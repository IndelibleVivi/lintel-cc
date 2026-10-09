//! An explicit, per-environment TCP proxy. It does not prevent direct connections.
pub mod probe;
pub mod telemetry;

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

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AddressFamily {
    /// Follow the system resolver/host order for the address family.
    #[default]
    System,
    /// Only ever resolve/connect IPv4; an IPv6-only target fails explicitly.
    Ipv4Only,
}

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
    /// Additive connection-family constraint. `system` keeps the historic
    /// behaviour; `ipv4_only` restricts every Lintel-established TCP segment
    /// (destination or HTTP/HTTPS upstream) to IPv4 and never falls back.
    #[serde(default)]
    pub address_family: AddressFamily,
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
            address_family: AddressFamily::System,
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
    /// Which address family Lintel's own client-to-endpoint TCP segment used.
    /// It describes only the segment Lintel created; it never classifies the
    /// destination's own traffic. Null until a connection is established.
    pub peer_family: Option<&'static str>,
    pub classification: &'static str,
    pub coverage: &'static str,
    pub direct_connections_enforced: bool,
    pub byte_counts_complete: bool,
    pub bytes_to_destination: u64,
    pub bytes_to_client: u64,
    /// Additive, finite origin marker. `Some("rule_test")` marks a controlled
    /// owner-origin test request; ordinary client traffic leaves it `None`.
    pub origin: Option<&'static str>,
    /// The one-use controlled-test id, present only for controlled requests.
    pub test_id: Option<String>,
}
pub type Observer = Arc<dyn Fn(Event) + Send + Sync>;

#[derive(Clone, Debug)]
pub struct Destination {
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
    /// Build a destination from an already-separated host and port. Used by the
    /// probe for a loopback proxy authority after it validated the split.
    pub(crate) fn raw(host: &str, port: u16) -> Result<Self, &'static str> {
        if port == 0 {
            return Err("invalid_port");
        }
        Ok(Destination {
            host: normalize_host(host)?,
            port,
        })
    }
}

/// Family-aware TCP connect shared with the probe so both the proxy relay and
/// the probe apply the exact same `address_family` constraint. Returns the
/// established stream plus the family Lintel actually connected with.
pub(crate) async fn connect_for_probe(
    destination: &Destination,
    family: AddressFamily,
    ipv6_only: bool,
) -> Result<(TcpStream, std::net::SocketAddr), &'static str> {
    let selected = if ipv6_only {
        Some(true)
    } else if family == AddressFamily::Ipv4Only {
        Some(false)
    } else {
        None
    };
    match connect_ip_family(destination, selected).await {
        Ok((stream, _)) => {
            let peer = stream.peer_addr().map_err(|_| "connect_failed")?;
            Ok((stream, peer))
        }
        Err(code) => Err(code),
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
            match label
                .strip_prefix("0x")
                .or_else(|| label.strip_prefix("0X"))
            {
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
    // Owned controlled-test registry for this exact Proxy instance. Provenance
    // comes only from a registration matched to a private loopback peer and the
    // exact target; a client header cannot manufacture a rule_test origin.
    rule_tests: Arc<telemetry::RuleTestRegistry>,
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
            rule_tests: Arc::new(telemetry::RuleTestRegistry::default()),
        })
    }

    /// Run one real controlled rule test against this exact serving instance.
    ///
    /// The `id` must be a catalog id; no arbitrary host/port/wildcard is
    /// accepted. Admission requires the exact target to be *explicitly blocked*
    /// by this instance's frozen config BEFORE any request; otherwise the call
    /// refuses without any network activity. On admission a private loopback
    /// client socket is bound (its local endpoint becomes the owner origin),
    /// registered with the exact host/443 and a one-use test id, then a real
    /// CONNECT `host:443` is sent to this Proxy's own listener. Success requires
    /// the proxy's own parser/decision path to refuse it with 403 and the
    /// observer event to carry the controlled origin. No public DNS, upstream,
    /// or destination socket is ever used; an unexpected allow is refused closed
    /// and reported `failed`, never `blocked_explicit`.
    pub async fn rule_test(&self, id: &str) -> Result<telemetry::RuleTestOutcome, &'static str> {
        let host = telemetry::host_for(id).ok_or("unknown_telemetry_destination")?;
        let destination = Destination::raw(host, 443)?;
        let (_, reason) = self.config.decide(&destination);
        if reason != "explicit_block" {
            return Err("telemetry_target_not_explicitly_blocked");
        }
        let address = self
            .listener
            .local_addr()
            .map_err(|_| "address_unavailable")?;
        // Bind a private loopback client socket first so its local endpoint is
        // the origin the handler can match; no other process can own this peer.
        let socket = if address.is_ipv4() {
            tokio::net::TcpSocket::new_v4()
        } else {
            tokio::net::TcpSocket::new_v6()
        }
        .map_err(|_| "rule_test_socket_failed")?;
        let loopback = if address.is_ipv4() {
            "127.0.0.1:0"
        } else {
            "[::1]:0"
        };
        socket
            .bind(loopback.parse().unwrap())
            .map_err(|_| "rule_test_bind_failed")?;
        let local = socket.local_addr().map_err(|_| "rule_test_bind_failed")?;
        let mut lease = self
            .rule_tests
            .register(local, host, 443, Duration::from_secs(5))?;
        let exchange = async {
            let mut stream = socket
                .connect(address)
                .await
                .map_err(|_| "rule_test_connect_failed")?;
            let request = format!("CONNECT {host}:443 HTTP/1.1\r\nHost: {host}:443\r\n\r\n");
            stream
                .write_all(request.as_bytes())
                .await
                .map_err(|_| "rule_test_write_failed")?;
            stream.flush().await.map_err(|_| "rule_test_write_failed")?;
            let (response, _) = read_head(&mut stream).await?;
            let record = (&mut lease.completion)
                .await
                .map_err(|_| "rule_test_missing_event")?;
            Ok::<_, &'static str>((response, record))
        };
        let (response, record) = timeout(Duration::from_secs(5), exchange)
            .await
            .map_err(|_| "rule_test_timeout")??;
        // The response and the completed canonical event must agree. A fail-
        // closed fallback, a partial reply or a cancelled handler cannot pass.
        let refused = response.starts_with(b"HTTP/1.1 403 Forbidden\r\n")
            && record.origin == Some("rule_test")
            && record.test_id.as_deref() == Some(lease.test_id.as_str())
            && record.destination_host.as_deref() == Some(host)
            && record.destination_port == Some(443)
            && record.decision == "deny"
            && record.provenance == "explicit_block"
            && record.outcome == "blocked"
            && record.peer_family.is_none();
        Ok(telemetry::RuleTestOutcome {
            kind: "rule_test",
            timestamp_unix_ms: record.timestamp_unix_ms,
            environment_id: record.environment_id,
            destination_host: host.to_string(),
            destination_port: 443,
            provenance: "rule_test",
            result: if refused {
                "blocked_explicit"
            } else {
                "failed"
            },
            decision: record.decision,
            reason: if refused {
                "explicit_block"
            } else {
                "unexpected_rule_result"
            },
            explicit_block: refused,
            connection_attempted: false,
            test_id: lease.test_id.clone(),
            origin: "rule_test",
            coverage: "proxy_connections_only",
        })
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }
    pub async fn serve(self) -> std::io::Result<()> {
        self.serve_until(std::future::pending()).await
    }
    /// Shared-handle variant of [`Proxy::serve_until`] so a caller (the App)
    /// can keep the same instance reachable for controlled rule tests while it
    /// serves. The listener is still owned by this one Proxy.
    pub async fn serve_until_shared(
        self: std::sync::Arc<Self>,
        shutdown: impl std::future::Future<Output = ()>,
    ) -> std::io::Result<()> {
        let permits = Arc::new(Semaphore::new(self.config.max_connections));
        let (stop, _) = watch::channel(false);
        let rule_tests = self.rule_tests.clone();
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
                    let rule_tests = rule_tests.clone();
                    let stopped = stop.subscribe();
                    tasks.spawn(async move { let _permit = permit; handle(client, config, observer, stopped, rule_tests).await; });
                }
            }
        };
        let _ = stop.send(true);
        while tasks.join_next().await.is_some() {}
        result
    }
    /// Resolve shutdown to stop accepting and close all active connections.
    /// Await completion before showing a stopped state or replacing the rules.
    pub async fn serve_until(
        self,
        shutdown: impl std::future::Future<Output = ()>,
    ) -> std::io::Result<()> {
        Arc::new(self).serve_until_shared(shutdown).await
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
        peer_family: None,
        classification: "unclassified",
        coverage: "proxy_connections_only",
        direct_connections_enforced: false,
        byte_counts_complete: false,
        bytes_to_destination: 0,
        bytes_to_client: 0,
        origin: None,
        test_id: None,
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
    family: AddressFamily,
) -> Result<(Stream, &'static str), &'static str> {
    let endpoint = upstream.map(|u| &u.dest).unwrap_or(dest);
    let (tcp, peer_family) = connect_tcp(endpoint, family).await?;
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
        Ok((Box::new(stream), peer_family))
    } else {
        Ok((Box::new(tcp), peer_family))
    }
}

/// Establish the TCP segment for a Lintel-created connection.
///
/// `system` keeps the historic resolver/`TcpStream::connect` behaviour. In
/// `ipv4_only` mode we resolve ourselves, keep only IPv4 addresses, map an
/// IPv4-mapped-IPv6 literal to its embedded IPv4, and refuse an address set
/// with no IPv4 entry. There is deliberately no IPv6 fallback: an IPv6-only
/// destination is an explicit `ipv4_unavailable` failure, never a silent
/// upgrade to IPv6 or a bypass of the configured upstream.
async fn connect_tcp(
    endpoint: &Destination,
    family: AddressFamily,
) -> Result<(TcpStream, &'static str), &'static str> {
    connect_ip_family(
        endpoint,
        if family == AddressFamily::Ipv4Only {
            Some(false)
        } else {
            None
        },
    )
    .await
}

async fn connect_ip_family(
    endpoint: &Destination,
    ipv6: Option<bool>,
) -> Result<(TcpStream, &'static str), &'static str> {
    // Canonicalize mapped IPv6 before family filtering. Do not let the resolver
    // or connect fallback reintroduce a family explicitly excluded by the caller.
    let candidates: Vec<IpAddr> = if let Ok(ip) = endpoint.host.parse::<IpAddr>() {
        vec![match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
            _ => ip,
        }]
    } else {
        let mut addresses = Vec::new();
        for address in tokio::net::lookup_host((endpoint.host.as_str(), endpoint.port))
            .await
            .map_err(|_| "dns_failed")?
        {
            if addresses.len() == 64 {
                break;
            }
            let ip = match address.ip() {
                IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(address.ip()),
                ip => ip,
            };
            if !addresses.contains(&ip) {
                addresses.push(ip);
            }
        }
        addresses
    };
    let candidates: Vec<_> = candidates
        .into_iter()
        .filter(|ip| ipv6.is_none_or(|v6| ip.is_ipv6() == v6))
        .collect();
    if candidates.is_empty() {
        return Err(if ipv6 == Some(true) {
            "ipv6_unavailable"
        } else if ipv6 == Some(false) {
            "ipv4_unavailable"
        } else {
            "no_route"
        });
    }
    let mut last = None;
    for ip in candidates {
        match TcpStream::connect(SocketAddr::new(ip, endpoint.port)).await {
            Ok(tcp) => {
                let peer = tcp.peer_addr().map_err(|_| "connect_failed")?;
                // Guard the constraint even if a future platform ever surprised us.
                if ipv6.is_some_and(|v6| peer.is_ipv6() != v6) {
                    return Err("family_mismatch");
                }
                return Ok((tcp, family_name(peer)));
            }
            Err(error) => {
                // Remember a specific refusal so the probe never reports a
                // generic failure when the reason is exactly known.
                last = Some(error);
            }
        }
    }
    Err(last.map(io_error_code).unwrap_or("no_route"))
}

fn family_name(addr: SocketAddr) -> &'static str {
    if addr.is_ipv4() {
        "ipv4"
    } else {
        "ipv6"
    }
}

/// Map a std IO error to the finite reason vocabulary the probe and callers
/// keep. A refused connection is never folded into an unreachable one.
fn io_error_code(error: std::io::Error) -> &'static str {
    match error.kind() {
        std::io::ErrorKind::ConnectionRefused => "refused",
        std::io::ErrorKind::TimedOut => "timed_out",
        std::io::ErrorKind::NetworkUnreachable
        | std::io::ErrorKind::HostUnreachable
        | std::io::ErrorKind::AddrNotAvailable => "no_route",
        std::io::ErrorKind::PermissionDenied => "permission_denied",
        _ => "connect_failed",
    }
}
async fn handle(
    mut client: TcpStream,
    config: Arc<Config>,
    observer: Observer,
    mut stopped: watch::Receiver<bool>,
    rule_tests: Arc<telemetry::RuleTestRegistry>,
) {
    let mut record = event(&config);
    // The private peer that this accept came from. Only a registration bound to
    // this exact peer + target can make the request a controlled origin.
    let peer = client.peer_addr().ok();
    let mut test_completion = None;
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
        // Controlled-test origin: consume the one-use registration only for the
        // exact owned peer and exact target. A header can never create this.
        if let Some(peer) = peer {
            if let Some(completion) =
                rule_tests.consume(peer, &request.dest.host, request.dest.port)
            {
                record.origin = Some("rule_test");
                record.test_id = Some(completion.test_id.clone());
                test_completion = Some(completion);
            }
        }
        if record.decision == "deny" {
            reject(&mut client, "403 Forbidden").await;
            return Err("blocked");
        }
        // Defense in depth: a registered controlled request must never reach a
        // destination. If a misconfiguration let it past an explicit block, fail
        // it closed here rather than connecting.
        if record.origin == Some("rule_test") {
            reject(&mut client, "503 Service Unavailable").await;
            return Err("rule_test_not_blocked");
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
            connect_transport(&request.dest, upstream.as_ref(), config.address_family),
        )
        .await;
        let mut remote = match connect_result {
            Ok(Ok((stream, peer_family))) => {
                // Only describes the segment Lintel created to the endpoint.
                record.peer_family = Some(peer_family);
                stream
            }
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
    observer(record.clone());
    if let Some(completion) = test_completion {
        let _ = completion.sender.send(record);
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    #[tokio::test]
    async fn controlled_unexpected_allow_fails_closed_with_completed_event() {
        let proxy = Arc::new(
            Proxy::bind(Config::default(), Arc::new(|_| {}))
                .await
                .unwrap(),
        );
        let socket = tokio::net::TcpSocket::new_v4().unwrap();
        socket.bind("127.0.0.1:0".parse().unwrap()).unwrap();
        let host = telemetry::host_for("datadog_logs_intake").unwrap();
        let mut lease = proxy
            .rule_tests
            .register(
                socket.local_addr().unwrap(),
                host,
                443,
                Duration::from_secs(5),
            )
            .unwrap();
        let serving = proxy.clone();
        let task = tokio::spawn(serving.serve_until_shared(std::future::pending()));
        let mut stream = socket.connect(proxy.local_addr().unwrap()).await.unwrap();
        stream
            .write_all(
                format!("CONNECT {host}:443 HTTP/1.1\r\nHost: {host}:443\r\n\r\n").as_bytes(),
            )
            .await
            .unwrap();
        let (response, _) = read_head(&mut stream).await.unwrap();
        let event = (&mut lease.completion).await.unwrap();
        assert!(
            response.starts_with(b"HTTP/1.1 503 "),
            "fallback must not claim the block's 403"
        );
        assert_eq!(event.origin, Some("rule_test"));
        assert_eq!(event.test_id.as_deref(), Some(lease.test_id.as_str()));
        assert_eq!(event.provenance, "default_allow");
        assert_eq!(event.outcome, "rule_test_not_blocked");
        assert!(
            event.peer_family.is_none(),
            "never connect public target or upstream"
        );
        task.abort();
    }
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
        assert!(
            serde_json::from_value::<Config>(serde_json::json!({"default_action":"deny"})).is_ok()
        );
        assert!(serde_json::from_value::<Config>(
            serde_json::json!({"blocked":[{"host":"example.com","ports":[]}]})
        )
        .is_err());
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

/// One foreground channel owner for both CLI executables. Stdout is NDJSON.
pub async fn serve_config(path: &std::path::Path) -> Result<(), String> {
    serve_config_with_tests(path, &[]).await
}

/// Foreground channel owner with an optional finite `--test-telemetry` id list.
/// Each requested catalog host is proven explicitly blocked by the frozen draft
/// *before* any public connection: an id that is not explicitly blocked is a
/// startup refusal, never a silent success. No telemetry body is ever sent.
pub async fn serve_config_with_tests(
    path: &std::path::Path,
    test_telemetry: &[String],
) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|_| "config_read_failed".to_string())?;
    let mut config: Config =
        serde_json::from_slice(&bytes).map_err(|_| "invalid_config_json".to_string())?;
    config.validate().map_err(str::to_owned)?;
    // Refuse an invalid or non-explicitly-blocked selection BEFORE binding or
    // connecting anything, so a bad `--test-telemetry` never opens a listener.
    if !telemetry::valid_telemetry_ids(test_telemetry) && !test_telemetry.is_empty() {
        return Err("invalid_telemetry_test_ids".to_string());
    }
    for id in test_telemetry {
        let host = telemetry::host_for(id).ok_or("unknown_telemetry_destination")?;
        let destination = Destination::raw(host, 443)?;
        let (_, reason) = config.decide(&destination);
        if reason != "explicit_block" {
            return Err(format!("telemetry_target_not_explicitly_blocked:{host}"));
        }
    }
    let proxy = Proxy::bind(
        config,
        Arc::new(|event| println!("{}", serde_json::to_string(&event).unwrap())),
    )
    .await?;
    let address = proxy
        .local_addr()
        .map_err(|_| "listen_failed".to_string())?;
    println!(
        "{}",
        serde_json::json!({"event":"listening","owner":"foreground_process","pid":std::process::id(),"address":address,"active_config":*proxy.config,"coverage":"proxy_connections_only","direct_connections_enforced":false})
    );
    let proxy = Arc::new(proxy);
    // The proxy must actually be serving before the controlled loopback request,
    // so start it, run the finite tests, then keep serving until Ctrl-C.
    let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
    let serving = proxy.clone();
    let server = tokio::spawn(async move {
        serving
            .serve_until_shared(async move {
                let _ = stopped.await;
            })
            .await
    });
    // Controlled rule tests go through the same serving Proxy instance. A target
    // that is not explicitly blocked refuses the whole start; no destination
    // connection is ever attempted. Owner origin arises from the registration,
    // not from any header.
    for id in test_telemetry {
        let outcome = match proxy.rule_test(id).await {
            Ok(outcome) => outcome,
            Err(reason) => {
                let _ = stop.send(());
                let _ = server.await;
                return Err(reason.to_string());
            }
        };
        println!(
            "{}",
            serde_json::json!({"event":"rule_test","owner_origin":"lintel_egress_foreground","telemetry_id":id,"outcome":outcome})
        );
        if outcome.result != "blocked_explicit" {
            let _ = stop.send(());
            let _ = server.await;
            return Err(format!(
                "telemetry_target_not_explicitly_blocked:{}",
                outcome.destination_host
            ));
        }
    }
    let mut server = server;
    let result = tokio::select! {
        response = &mut server => response,
        _ = tokio::signal::ctrl_c() => {
            let _ = stop.send(());
            server.await
        }
    };
    result
        .map_err(|_| "accept_failed".to_string())?
        .map_err(|_| "accept_failed".to_string())
}
