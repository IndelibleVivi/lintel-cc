//! An explicit, per-environment TCP proxy. It does not prevent direct connections.
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::{watch, Semaphore},
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::{
    rustls::{self, pki_types::ServerName},
    TlsConnector,
};

const HEADER_LIMIT: usize = 32 * 1024;
const BODY_LIMIT: u64 = 64 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub host: String,
    /// An empty list matches every port on this exact host; no wildcard expansion.
    #[serde(default)]
    pub ports: Vec<u16>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefaultAction {
    #[default]
    Allow,
    Deny,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default = "default_environment_id")]
    pub environment_id: String,
    #[serde(default = "default_bind")]
    pub bind: SocketAddr,
    /// Required on the wire: an omitted action must fail closed, not silently
    /// become allow-all. `DefaultAction::default()` stays for programmatic use.
    pub default_action: DefaultAction,
    #[serde(default)]
    pub allowed: Vec<Rule>,
    #[serde(default)]
    pub blocked: Vec<Rule>,
    /// HTTP or HTTPS proxy, with no userinfo. SOCKS requires a separate adapter.
    #[serde(default)]
    pub upstream: Option<String>,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_connect_timeout")]
    pub connect_timeout_seconds: u64,
    #[serde(default = "default_connection_lifetime")]
    pub connection_lifetime_seconds: u64,
}
fn default_environment_id() -> String {
    "unassigned".into()
}
fn default_bind() -> SocketAddr {
    "127.0.0.1:0".parse().unwrap()
}
fn default_max_connections() -> usize {
    64
}
fn default_connect_timeout() -> u64 {
    10
}
fn default_connection_lifetime() -> u64 {
    3600
}
impl Default for Config {
    fn default() -> Self {
        Self {
            environment_id: "unassigned".into(),
            bind: "127.0.0.1:0".parse().unwrap(),
            default_action: DefaultAction::Allow,
            allowed: vec![],
            blocked: vec![],
            upstream: None,
            max_connections: 64,
            connect_timeout_seconds: 10,
            connection_lifetime_seconds: 3600,
        }
    }
}
impl Config {
    pub fn validate(&mut self) -> Result<(), &'static str> {
        if !self.bind.ip().is_loopback() {
            return Err("bind_must_be_loopback");
        }
        if self.environment_id.is_empty()
            || self.environment_id.len() > 128
            || !self
                .environment_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_".contains(&b))
        {
            return Err("invalid_environment_id");
        }
        if !(1..=1024).contains(&self.max_connections)
            || !(1..=120).contains(&self.connect_timeout_seconds)
            || !(1..=86400).contains(&self.connection_lifetime_seconds)
        {
            return Err("invalid_resource_limit");
        }
        for rule in self.allowed.iter_mut().chain(self.blocked.iter_mut()) {
            rule.host = normalize_host(&rule.host)?;
            if rule.ports.contains(&0) {
                return Err("invalid_port");
            }
        }
        if let Some(upstream) = &self.upstream {
            parse_upstream(upstream)?;
        }
        Ok(())
    }
    fn decide(&self, dest: &Destination) -> (&'static str, &'static str) {
        let matches = |rule: &Rule| {
            rule.host == dest.host && (rule.ports.is_empty() || rule.ports.contains(&dest.port))
        };
        if self.blocked.iter().any(matches) {
            return ("deny", "explicit_block");
        }
        if self.allowed.iter().any(matches) {
            return ("allow", "explicit_allow");
        }
        match self.default_action {
            DefaultAction::Allow => ("allow", "default_allow"),
            DefaultAction::Deny => ("deny", "default_deny"),
        }
    }
}

/// Only this allowlisted record is exposed to the observer. No raw requests/errors.
#[derive(Clone, Debug, Serialize)]
pub struct Event {
    pub timestamp_unix_ms: u128,
    pub environment_id: String,
    pub destination_host: Option<String>,
    pub destination_port: Option<u16>,
    pub channel: &'static str,
    pub decision: &'static str,
    pub provenance: &'static str,
    pub outcome: &'static str,
    pub classification: &'static str,
    pub coverage: &'static str,
    pub direct_connections_enforced: bool,
    pub byte_counts_complete: bool,
    pub bytes_to_destination: u64,
    pub bytes_to_client: u64,
}
pub type Observer = Arc<dyn Fn(Event) + Send + Sync>;

#[derive(Clone, Debug)]
struct Destination {
    host: String,
    port: u16,
}
impl Destination {
    fn authority(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}
fn normalize_host(host: &str) -> Result<String, &'static str> {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    if let Ok(ip) = host.parse::<IpAddr>() {
        // IPv4-mapped IPv6 names the same socket as its embedded IPv4; match
        // rules on one canonical form so either spelling hits an exact-IP rule.
        if let IpAddr::V6(v6) = ip {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return Ok(v4.to_string());
            }
        }
        return Ok(ip.to_string());
    }
    // inet_aton-style spellings (decimal/octal/hex atoms, short or mixed
    // dotted forms) resolve to an IP at connect time but never parse above;
    // refusing them keeps exact-IP rules unbypassable instead of silently
    // string-matching. Each atom is all-digits or 0x-prefixed hex; ordinary
    // alphanumeric hostnames are unaffected.
    let ambiguous_numeric = !host.is_empty()
        && host.split('.').all(|label| {
            if label.is_empty() {
                return false;
            }
            match label.strip_prefix("0x").or_else(|| label.strip_prefix("0X")) {
                Some(hex) => !hex.is_empty() && hex.bytes().all(|b| b.is_ascii_hexdigit()),
                None => label.bytes().all(|b| b.is_ascii_digit()),
            }
        });
    if ambiguous_numeric {
        return Err("invalid_host");
    }
    if host.is_empty()
        || host.len() > 253
        || !host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
    {
        return Err("invalid_host");
    }
    Ok(host)
}
fn destination(authority: &str, default_port: Option<u16>) -> Result<Destination, &'static str> {
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (host, rest) = rest.split_once(']').ok_or("invalid_authority")?;
        if host.parse::<std::net::Ipv6Addr>().is_err() {
            return Err("invalid_authority");
        }
        (
            host,
            if rest.is_empty() {
                default_port.ok_or("missing_port")?
            } else {
                rest.strip_prefix(':')
                    .ok_or("invalid_authority")?
                    .parse()
                    .map_err(|_| "invalid_port")?
            },
        )
    } else if let Some((host, port)) = authority.rsplit_once(':') {
        (host, port.parse().map_err(|_| "invalid_port")?)
    } else {
        (authority, default_port.ok_or("missing_port")?)
    };
    if port == 0 {
        return Err("invalid_port");
    }
    Ok(Destination {
        host: normalize_host(host)?,
        port,
    })
}
#[derive(Clone)]
struct Upstream {
    tls: bool,
    dest: Destination,
}
fn parse_upstream(value: &str) -> Result<Upstream, &'static str> {
    let (tls, authority) = if let Some(v) = value.strip_prefix("http://") {
        (false, v)
    } else if let Some(v) = value.strip_prefix("https://") {
        (true, v)
    } else {
        return Err("unsupported_upstream_scheme_use_http_or_https_not_socks");
    };
    // No URL credentials are accepted, stored in events, or forwarded as proxy auth.
    if authority.contains(['/', '@', '?', '#']) {
        return Err("upstream_requires_authority_without_credentials");
    }
    Ok(Upstream {
        tls,
        dest: destination(authority, Some(if tls { 443 } else { 80 }))?,
    })
}
trait Transport: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Transport for T {}
type Stream = Box<dyn Transport>;

pub struct Proxy {
    listener: TcpListener,
    config: Arc<Config>,
    observer: Observer,
}
impl Proxy {
    pub async fn bind(mut config: Config, observer: Observer) -> Result<Self, String> {
        config.validate().map_err(str::to_owned)?;
        let listener = TcpListener::bind(config.bind)
            .await
            .map_err(|_| "listen_failed".to_string())?;
        Ok(Self {
            listener,
            config: Arc::new(config),
            observer,
        })
    }
    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }
    pub async fn serve(self) -> std::io::Result<()> {
        self.serve_until(std::future::pending()).await
    }
    /// Resolve shutdown to stop accepting and close all active connections.
    /// Await completion before showing a stopped state or replacing the rules.
    pub async fn serve_until(
        self,
        shutdown: impl std::future::Future<Output = ()>,
    ) -> std::io::Result<()> {
        let permits = Arc::new(Semaphore::new(self.config.max_connections));
        let (stop, _) = watch::channel(false);
        let mut tasks = JoinSet::new();
        tokio::pin!(shutdown);
        let result = loop {
            let accept = async {
                let permit = permits.clone().acquire_owned().await.unwrap();
                self.listener
                    .accept()
                    .await
                    .map(|(client, _)| (client, permit))
            };
            tokio::select! {
                _ = &mut shutdown => break Ok(()),
                Some(_) = tasks.join_next(), if !tasks.is_empty() => {},
                accepted = accept => {
                    let (client, permit) = match accepted { Ok(value) => value, Err(error) => break Err(error) };
                    let config = self.config.clone(); let observer = self.observer.clone();
                    let stopped = stop.subscribe();
                    tasks.spawn(async move { let _permit = permit; handle(client, config, observer, stopped).await; });
                }
            }
        };
        let _ = stop.send(true);
        while tasks.join_next().await.is_some() {}
        result
    }
}
fn event(config: &Config) -> Event {
    Event {
        timestamp_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        environment_id: config.environment_id.clone(),
        destination_host: None,
        destination_port: None,
        channel: "proxy",
        decision: "reject",
        provenance: "request_validation",
        outcome: "invalid_request",
        classification: "unclassified",
        coverage: "proxy_connections_only",
        direct_connections_enforced: false,
        byte_counts_complete: false,
        bytes_to_destination: 0,
        bytes_to_client: 0,
    }
}
async fn reject(client: &mut TcpStream, status: &str) {
    let reply = format!("HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: 0\r\n\r\n");
    let _ = client.write_all(reply.as_bytes()).await;
    let _ = client.shutdown().await;
}
async fn read_head<S: AsyncRead + Unpin + ?Sized>(
    stream: &mut S,
) -> Result<(Vec<u8>, usize), &'static str> {
    let mut buf = Vec::new();
    loop {
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            return Ok((buf, end + 4));
        }
        if buf.len() >= HEADER_LIMIT {
            return Err("header_too_large");
        }
        let mut chunk = [0u8; 2048];
        let count = stream.read(&mut chunk).await.map_err(|_| "read_failed")?;
        if count == 0 {
            return Err("incomplete_header");
        }
        buf.extend_from_slice(&chunk[..count]);
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            if end + 4 > HEADER_LIMIT {
                return Err("header_too_large");
            }
            return Ok((buf, end + 4));
        }
    }
}
struct Request {
    dest: Destination,
    connect: bool,
    method: String,
    path: String,
    headers: Vec<(String, Vec<u8>)>,
    body_length: u64,
}
fn parse_request(head: &[u8]) -> Result<Request, &'static str> {
    let mut headers = [httparse::EMPTY_HEADER; 100];
    let mut parsed = httparse::Request::new(&mut headers);
    if !parsed
        .parse(head)
        .map_err(|_| "invalid_request")?
        .is_complete()
    {
        return Err("incomplete_header");
    }
    if parsed.version != Some(1) {
        return Err("http_1_1_required");
    }
    let method = parsed.method.ok_or("missing_method")?;
    let target = parsed.path.ok_or("missing_target")?;
    let connect = method == "CONNECT";
    let (dest, path) = if connect {
        (destination(target, None)?, String::new())
    } else {
        let rest = target
            .strip_prefix("http://")
            .ok_or("absolute_http_url_required")?;
        let end = rest.find(['/', '?']).unwrap_or(rest.len());
        let dest = destination(&rest[..end], Some(80))?;
        let path = if end == rest.len() {
            "/".to_string()
        } else if rest.as_bytes()[end] == b'?' {
            format!("/{}", &rest[end..])
        } else {
            rest[end..].to_string()
        };
        if path.contains('#') {
            return Err("invalid_target");
        }
        (dest, path)
    };
    let mut body_length = None;
    let mut host_count = 0;
    let mut output_headers = Vec::new();
    for h in parsed.headers.iter() {
        let name = h.name.to_ascii_lowercase();
        if name == "host" {
            host_count += 1;
            let host = destination(
                std::str::from_utf8(h.value).map_err(|_| "invalid_host")?,
                Some(if connect { dest.port } else { 80 }),
            )?;
            if host.host != dest.host || host.port != dest.port {
                return Err("host_authority_mismatch");
            }
        }
        if name == "content-length" {
            if body_length.is_some() {
                return Err("duplicate_content_length");
            }
            let length = std::str::from_utf8(h.value).map_err(|_| "invalid_content_length")?;
            if length.is_empty() || !length.bytes().all(|v| v.is_ascii_digit()) {
                return Err("invalid_content_length");
            }
            body_length = Some(
                length
                    .parse::<u64>()
                    .map_err(|_| "invalid_content_length")?,
            );
        }
        // Explicitly bounded HTTP support: unknown framing must never become a tunnel.
        if name == "transfer-encoding" || name == "upgrade" || name == "expect" {
            return Err("unsupported_http_framing");
        }
        output_headers.push((name, h.value.to_vec()));
    }
    if host_count > 1 {
        return Err("duplicate_host");
    }
    let body_length = body_length.unwrap_or(0);
    if body_length > BODY_LIMIT || (connect && body_length != 0) {
        return Err("invalid_body_length");
    }
    Ok(Request {
        dest,
        connect,
        method: method.into(),
        path,
        headers: output_headers,
        body_length,
    })
}
fn forward_head(request: &Request, upstream: bool) -> Result<Vec<u8>, &'static str> {
    let target = if upstream {
        format!("http://{}{}", request.dest.authority(), request.path)
    } else {
        request.path.clone()
    };
    let mut output = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        request.method,
        target,
        request.dest.authority()
    )
    .into_bytes();
    let mut hop_headers = vec![
        "host".to_string(),
        "connection".into(),
        "proxy-connection".into(),
        "proxy-authorization".into(),
        "proxy-authenticate".into(),
        "keep-alive".into(),
        "te".into(),
        "trailer".into(),
    ];
    for (name, value) in &request.headers {
        if name == "connection" {
            let value = std::str::from_utf8(value).map_err(|_| "invalid_connection_header")?;
            for token in value.split(',') {
                let token = token.trim().to_ascii_lowercase();
                if token == "content-length" || token == "host" {
                    return Err("invalid_connection_header");
                }
                hop_headers.push(token);
            }
        }
    }
    for (name, value) in &request.headers {
        if !hop_headers.contains(name) {
            output.extend_from_slice(name.as_bytes());
            output.extend_from_slice(b": ");
            output.extend_from_slice(value);
            output.extend_from_slice(b"\r\n");
        }
    }
    output.extend_from_slice(b"\r\n");
    Ok(output)
}
async fn connect_transport(
    dest: &Destination,
    upstream: Option<&Upstream>,
) -> Result<Stream, &'static str> {
    let endpoint = upstream.map(|u| &u.dest).unwrap_or(dest);
    let tcp = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .await
        .map_err(|_| "connect_failed")?;
    if upstream.is_some_and(|u| u.tls) {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        let tls = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .map_err(|_| "upstream_tls_configuration_failed")?
        .with_root_certificates(roots)
        .with_no_client_auth();
        let name = ServerName::try_from(endpoint.host.clone()).map_err(|_| "invalid_tls_name")?;
        let stream = TlsConnector::from(Arc::new(tls))
            .connect(name, tcp)
            .await
            .map_err(|_| "upstream_tls_failed")?;
        Ok(Box::new(stream))
    } else {
        Ok(Box::new(tcp))
    }
}
async fn handle(
    mut client: TcpStream,
    config: Arc<Config>,
    observer: Observer,
    mut stopped: watch::Receiver<bool>,
) {
    let mut record = event(&config);
    let transfer = async {
        let (buf, head_length) = timeout(
            Duration::from_secs(config.connect_timeout_seconds),
            read_head(&mut client),
        )
        .await
        .map_err(|_| "header_timeout")??;
        let request = match parse_request(&buf[..head_length]) {
            Ok(r) => r,
            Err(e) => {
                reject(
                    &mut client,
                    if e == "unsupported_http_framing" {
                        "501 Not Implemented"
                    } else {
                        "400 Bad Request"
                    },
                )
                .await;
                return Err(e);
            }
        };
        record.destination_host = Some(request.dest.host.clone());
        record.destination_port = Some(request.dest.port);
        record.channel = if request.connect { "connect" } else { "http" };
        record.classification = if request.connect {
            "tunnel_content_unclassified"
        } else {
            "http_content_not_inspected"
        };
        (record.decision, record.provenance) = config.decide(&request.dest);
        if record.decision == "deny" {
            reject(&mut client, "403 Forbidden").await;
            return Err("blocked");
        }
        let upstream = config.upstream.as_deref().map(parse_upstream).transpose()?;
        // Construct/validate HTTP framing before connecting to the destination.
        let forwarded = if request.connect {
            vec![]
        } else {
            forward_head(&request, upstream.is_some())?
        };
        let connect_result = timeout(
            Duration::from_secs(config.connect_timeout_seconds),
            connect_transport(&request.dest, upstream.as_ref()),
        )
        .await;
        let mut remote = match connect_result {
            Ok(Ok(stream)) => stream,
            _ => {
                reject(&mut client, "502 Bad Gateway").await;
                return Err("route_connection_failed");
            }
        };
        if request.connect {
            if upstream.is_some() {
                let authority = request.dest.authority();
                let connect_head =
                    format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n");
                remote
                    .write_all(connect_head.as_bytes())
                    .await
                    .map_err(|_| "upstream_write_failed")?;
                let (response, length) = timeout(
                    Duration::from_secs(config.connect_timeout_seconds),
                    read_head(remote.as_mut()),
                )
                .await
                .map_err(|_| "upstream_timeout")??;
                let mut headers = [httparse::EMPTY_HEADER; 100];
                let mut parsed = httparse::Response::new(&mut headers);
                parsed
                    .parse(&response[..length])
                    .map_err(|_| "invalid_upstream_response")?;
                if parsed.code != Some(200) {
                    reject(&mut client, "502 Bad Gateway").await;
                    return Err("upstream_connect_rejected");
                }
                client
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await
                    .map_err(|_| "client_write_failed")?;
                client
                    .write_all(&response[length..])
                    .await
                    .map_err(|_| "client_write_failed")?;
                record.bytes_to_client += (response.len() - length) as u64;
            } else {
                client
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await
                    .map_err(|_| "client_write_failed")?;
            }
            remote
                .write_all(&buf[head_length..])
                .await
                .map_err(|_| "remote_write_failed")?;
            record.bytes_to_destination += (buf.len() - head_length) as u64;
            let (sent, received) = tokio::io::copy_bidirectional(&mut client, &mut remote)
                .await
                .map_err(|_| "relay_failed")?;
            record.bytes_to_destination += sent;
            record.bytes_to_client += received;
        } else {
            remote
                .write_all(&forwarded)
                .await
                .map_err(|_| "remote_write_failed")?;
            let available = &buf[head_length..];
            let initial = (available.len() as u64).min(request.body_length) as usize;
            remote
                .write_all(&available[..initial])
                .await
                .map_err(|_| "remote_write_failed")?;
            record.bytes_to_destination = forwarded.len() as u64 + initial as u64;
            let remaining = request.body_length - initial as u64;
            let copied = tokio::io::copy(&mut (&mut client).take(remaining), &mut remote)
                .await
                .map_err(|_| "body_read_failed")?;
            record.bytes_to_destination += copied;
            if copied != remaining {
                return Err("incomplete_body");
            }
            remote.flush().await.map_err(|_| "remote_write_failed")?;
            // One HTTP request per connection. Extra bytes are never forwarded as a second request.
            record.bytes_to_client = tokio::io::copy(&mut remote, &mut client)
                .await
                .map_err(|_| "response_read_failed")?;
            client.shutdown().await.map_err(|_| "client_write_failed")?;
        }
        Ok(())
    };
    record.outcome = tokio::select! {
        result = timeout(Duration::from_secs(config.connection_lifetime_seconds), transfer) => match result {
            Ok(Ok(())) => "completed", Ok(Err(code)) => code, Err(_) => "connection_lifetime_exceeded",
        },
        _ = stopped.changed() => "proxy_stopped",
    };
    record.byte_counts_complete = record.outcome == "completed";
    observer(record);
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    #[test]
    fn mixed_api_is_not_telemetry_or_implicitly_blocked() {
        let config = Config::default();
        assert_eq!(
            config.decide(&destination("api.anthropic.com:443", None).unwrap()),
            ("allow", "default_allow")
        );
    }
    #[test]
    fn ipv6_rules_match_equivalent_literal_spellings() {
        let mut config = Config::default();
        config.blocked.push(Rule {
            host: "::1".into(),
            ports: vec![443],
        });
        config.validate().unwrap();
        assert_eq!(
            config
                .decide(&destination("[0:0:0:0:0:0:0:1]:443", None).unwrap())
                .0,
            "deny"
        );
    }
    #[test]
    fn exact_ip_rules_cannot_be_bypassed_by_alternate_spellings() {
        let mut config = Config::default();
        config.blocked.push(Rule {
            host: "127.0.0.1".into(),
            ports: vec![],
        });
        config.validate().unwrap();
        // IPv4-mapped IPv6 names the same socket and must hit the IPv4 rule.
        assert_eq!(
            config
                .decide(&destination("[::ffff:127.0.0.1]:443", None).unwrap())
                .0,
            "deny"
        );
        assert_eq!(
            config
                .decide(&destination("[::ffff:7f00:1]:443", None).unwrap())
                .0,
            "deny"
        );
        // inet_aton-style spellings resolve at connect time but never reach
        // the matcher: they are refused outright instead of bypassing rules.
        for spelling in [
            "127.1:443",
            "127.000.000.001:443",
            "2130706433:443",
            "0x7f000001:443",
            "017700000001:443",
            "0x7f.0.0.1:443",
        ] {
            assert!(destination(spelling, None).is_err(), "{spelling}");
        }
        // Ordinary hostnames and valid IPv4 keep working.
        assert!(destination("example.com:443", None).is_ok());
        assert_eq!(
            destination("192.168.0.1:443", None).unwrap().authority(),
            "192.168.0.1:443"
        );
    }
    #[test]
    fn wire_config_requires_an_explicit_default_action() {
        assert!(serde_json::from_value::<Config>(serde_json::json!({"default_action":"deny"})).is_ok());
        assert!(serde_json::from_value::<Config>(serde_json::json!({"blocked":[{"host":"example.com","ports":[]}]})).is_err());
    }
    #[test]
    fn rejects_unsupported_route_and_ambiguous_framing() {
        assert!(parse_upstream("socks5://localhost:1080").is_err());
        assert!(parse_upstream("http://user:secret@localhost:8080").is_err());
        for req in [
            "GET http://example.test/ HTTP/1.1\r\nHost: different.test\r\n\r\n",
            "POST http://example.test/ HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 1\r\n\r\n",
            "POST http://example.test/ HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n",
        ] {
            assert!(parse_request(req.as_bytes()).is_err());
        }
    }
}
