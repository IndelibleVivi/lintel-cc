//! The single finite HTTPS IP-echo probe used by `lintel-core`.
//!
//! It performs exactly four cells: host-default/ipv4, host-default/ipv6,
//! lintel-channel/ipv4 and lintel-channel/ipv6. With no loopback proxy the two
//! channel cells are `not_tested`. It never inspects business TLS, never keeps
//! response data beyond the validated echo address, and never runs in the
//! background. Everything is bounded: endpoints are validated as finite HTTPS
//! URLs, the response is capped, and each request has a caller-supplied
//! deadline. Failures keep their specific cause and never turn into a
//! "no leak" conclusion.
use crate::{AddressFamily, Destination};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::timeout,
};
use tokio_rustls::{
    rustls::{self, pki_types::ServerName},
    TlsConnector,
};

const MAX_ENDPOINT_URL: usize = 2048;
const MAX_BODY: usize = 256;
const MAX_ENCODED_BODY: usize = 2048;
const MAX_HEADER: usize = 8192;

/// One finite, validated HTTPS echo endpoint. The caller supplies the URL; we
/// freeze the parsed pieces and never re-read the raw string after validation.
#[derive(Clone, Debug)]
pub struct Endpoint {
    host: String,
    port: u16,
    path: String,
}

impl Endpoint {
    /// Accept only a finite HTTPS echo URL: no credentials, query, fragment,
    /// control characters, or non-HTTPS scheme. Host/port/path stay bounded.
    pub fn parse(url: &str) -> Result<Self, &'static str> {
        if url.len() > MAX_ENDPOINT_URL
            || !url.bytes().all(|b| b.is_ascii_graphic())
            || url.contains('\\')
        {
            return Err("invalid_endpoint");
        }
        let rest = url.strip_prefix("https://").ok_or("invalid_endpoint")?;
        if rest.contains(['@', '?', '#']) || rest.is_empty() {
            return Err("invalid_endpoint");
        }
        let (authority, path) = match rest.find('/') {
            Some(index) => (&rest[..index], rest[index..].to_string()),
            None => (rest, "/".to_string()),
        };
        if path.len() > 1024 {
            return Err("invalid_endpoint");
        }
        let (host, port) = if let Some(bracketed) = authority.strip_prefix('[') {
            // Bracketed IPv6 literal, optionally with a port.
            let (host, tail) = bracketed.split_once(']').ok_or("invalid_endpoint")?;
            host.parse::<std::net::Ipv6Addr>()
                .map_err(|_| "invalid_endpoint")?;
            let port = if tail.is_empty() {
                443
            } else {
                tail.strip_prefix(':')
                    .and_then(|p| p.parse::<u16>().ok())
                    .filter(|p| *p != 0)
                    .ok_or("invalid_endpoint")?
            };
            (host.to_string(), port)
        } else {
            match authority.rsplit_once(':') {
                Some((host, port)) => {
                    let port: u16 = port.parse().map_err(|_| "invalid_endpoint")?;
                    if port == 0 {
                        return Err("invalid_endpoint");
                    }
                    (host.to_string(), port)
                }
                None => (authority.to_string(), 443),
            }
        };
        let host = crate::normalize_host(&host).map_err(|_| "invalid_endpoint")?;
        if host.is_empty()
            || host.len() > 253
            || host.contains(char::is_control)
            || host.contains(['/', '[', ']'])
        {
            return Err("invalid_endpoint");
        }
        Ok(Self { host, port, path })
    }
    fn authority(&self) -> String {
        if self.host.contains(':') {
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
    pub fn describe(&self) -> String {
        let host = if self.host.contains(':') {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };
        if self.port == 443 {
            format!("https://{}{}", host, self.path)
        } else {
            format!("https://{}:{}{}", host, self.port, self.path)
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProbeRequest {
    pub ipv4_url: String,
    pub ipv6_url: String,
    #[serde(default)]
    pub proxy_url: Option<String>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

fn default_timeout() -> u64 {
    10
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    NotTested,
    DnsFailed,
    NoRoute,
    Refused,
    TimedOut,
    TlsFailed,
    ProxyDenied,
    HttpError,
    InvalidResponse,
    FamilyMismatch,
    Ipv4Unavailable,
    Ipv6Unavailable,
    ConnectionFailed,
    PermissionDenied,
    ProxyFailed,
}

#[derive(Clone, Debug, Serialize)]
pub struct Cell {
    pub path: &'static str,
    pub family: &'static str,
    pub status: Status,
    pub public_ip: Option<String>,
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl Cell {
    fn not_tested(path: &'static str, family: &'static str) -> Self {
        Self {
            path,
            family,
            status: Status::NotTested,
            public_ip: None,
            elapsed_ms: 0,
            peer_family: None,
            message: Some(match path {
                "lintel_channel" => "未提供 loopback 通道；此路径未测试".into(),
                _ => "此路径未测试".into(),
            }),
        }
    }
}

/// A loopback CONNECT proxy owned by the App. The `authority` already encodes
/// the frozen runner address; `family` is the constrained family for the
/// Lintel-created segment to the proxy.
#[derive(Clone, Debug)]
pub struct ProxyChannel {
    pub authority: String,
    pub family: AddressFamily,
}

/// Run the four probe cells. The caller has already decided this is an explicit
/// network action; this function itself only performs the bounded requests.
pub async fn run(
    request: &ProbeRequest,
    proxy: Option<ProxyChannel>,
) -> Result<[Cell; 4], &'static str> {
    run_with_trust(request, proxy, None).await
}

/// Internal entry that may substitute a trust anchor set. Production always
/// passes `None` (Mozilla roots). Tests pass a pinned self-signed root so the
/// full TLS certificate/hostname verification path runs against a synthetic
/// local server without ever contacting a real endpoint.
async fn run_with_trust(
    request: &ProbeRequest,
    proxy: Option<ProxyChannel>,
    trust: Option<Arc<rustls::ClientConfig>>,
) -> Result<[Cell; 4], &'static str> {
    if !(1..=15).contains(&request.timeout_seconds) {
        return Err("invalid_timeout");
    }
    let ipv4 = Endpoint::parse(&request.ipv4_url)?;
    let ipv6 = Endpoint::parse(&request.ipv6_url)?;
    match (&request.proxy_url, &proxy) {
        (None, None) => {}
        (Some(url), Some(channel)) => {
            let addr: SocketAddr = url
                .strip_prefix("http://")
                .ok_or("invalid_proxy")?
                .parse()
                .map_err(|_| "invalid_proxy")?;
            let actual: SocketAddr = channel.authority.parse().map_err(|_| "invalid_proxy")?;
            if !addr.ip().is_loopback() || addr.port() == 0 || addr != actual {
                return Err("invalid_proxy");
            }
        }
        _ => return Err("invalid_proxy"),
    }
    let deadline = Duration::from_secs(request.timeout_seconds);
    let channels = async {
        match &proxy {
            Some(channel) => tokio::join!(
                probe_via_proxy(&ipv4, "ipv4", channel, deadline, &trust),
                probe_via_proxy(&ipv6, "ipv6", channel, deadline, &trust)
            ),
            None => (
                Cell::not_tested("lintel_channel", "ipv4"),
                Cell::not_tested("lintel_channel", "ipv6"),
            ),
        }
    };
    let (host_v4, host_v6, (chan_v4, chan_v6)) = tokio::join!(
        probe_direct(&ipv4, "ipv4", AddressFamily::Ipv4Only, deadline, &trust),
        probe_direct(&ipv6, "ipv6", AddressFamily::System, deadline, &trust),
        channels
    );
    Ok([host_v4, host_v6, chan_v4, chan_v6])
}

async fn probe_direct(
    endpoint: &Endpoint,
    family: &'static str,
    addressing: AddressFamily,
    deadline: Duration,
    trust: &Option<Arc<rustls::ClientConfig>>,
) -> Cell {
    let started = Instant::now();
    let whole_request = async {
        let (stream, peer) = connect_direct(endpoint, addressing, family == "ipv6").await?;
        Ok::<_, &'static str>(
            finish_tls(stream, endpoint, family, started, peer, None, trust).await,
        )
    };
    match timeout(deadline, whole_request).await {
        Ok(Ok(cell)) => cell,
        Ok(Err(code)) => failure(family, started, code, None),
        Err(_) => failure(family, started, "timed_out", None),
    }
}

async fn probe_via_proxy(
    endpoint: &Endpoint,
    family: &'static str,
    channel: &ProxyChannel,
    deadline: Duration,
    trust: &Option<Arc<rustls::ClientConfig>>,
) -> Cell {
    let started = Instant::now();
    let whole_request = async {
        let (tcp, peer) = connect_direct(&proxy_endpoint(channel), channel.family, false).await?;
        let stream = established_through_proxy(tcp, endpoint).await?;
        Ok::<_, &'static str>(
            finish_tls(
                stream,
                endpoint,
                family,
                started,
                peer,
                Some("proxy"),
                trust,
            )
            .await,
        )
    };
    match timeout(deadline, whole_request).await {
        Ok(Ok(cell)) => cell,
        Ok(Err(code)) => failure(family, started, code, Some("proxy")),
        Err(_) => failure(family, started, "timed_out", Some("proxy")),
    }
}

fn proxy_endpoint(channel: &ProxyChannel) -> Endpoint {
    Endpoint {
        host: channel.authority.clone(),
        port: 0,
        path: String::new(),
    }
}

async fn connect_direct(
    endpoint: &Endpoint,
    addressing: AddressFamily,
    ipv6_only: bool,
) -> Result<(TcpStream, SocketAddr), &'static str> {
    // The proxy endpoint is a raw loopback authority (host:port); the echo
    // endpoints go through the same family-aware TCP helper.
    let destination = if endpoint.port == 0 {
        let addr: SocketAddr = endpoint.host.parse().map_err(|_| "invalid_proxy")?;
        Destination::raw(&addr.ip().to_string(), addr.port())?
    } else {
        Destination::raw(&endpoint.host, endpoint.port)?
    };
    crate::connect_for_probe(&destination, addressing, ipv6_only).await
}

async fn established_through_proxy(
    mut tcp: TcpStream,
    endpoint: &Endpoint,
) -> Result<TcpStream, &'static str> {
    let authority = endpoint.authority();
    let head = format!("CONNECT {authority} HTTP/1.1\r\nHost: {authority}\r\n\r\n");
    tcp.write_all(head.as_bytes())
        .await
        .map_err(crate::io_error_code)?;
    let mut buffer = Vec::new();
    loop {
        if let Some(end) = buffer.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = &buffer[..end + 4];
            let status = std::str::from_utf8(head)
                .map_err(|_| "invalid_response")?
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|code| code.parse::<u16>().ok())
                .ok_or("invalid_response")?;
            if status != 200 {
                return Err(if status == 403 {
                    "proxy_denied"
                } else {
                    "proxy_failed"
                });
            }
            if buffer.len() != end + 4 {
                return Err("invalid_response");
            }
            return Ok(tcp);
        }
        if buffer.len() >= MAX_HEADER {
            return Err("invalid_response");
        }
        let mut chunk = [0u8; 1024];
        let count = tcp.read(&mut chunk).await.map_err(crate::io_error_code)?;
        if count == 0 {
            return Err("proxy_failed");
        }
        if buffer.len() + count > MAX_HEADER {
            return Err("invalid_response");
        }
        buffer.extend_from_slice(&chunk[..count]);
    }
}

async fn finish_tls(
    tcp: TcpStream,
    endpoint: &Endpoint,
    family: &'static str,
    started: Instant,
    peer: SocketAddr,
    via: Option<&str>,
    trust: &Option<Arc<rustls::ClientConfig>>,
) -> Cell {
    let config = match trust {
        Some(config) => (**config).clone(),
        None => tls_config(),
    };
    let tls = match tls_connect_with(tcp, &endpoint.host, &config).await {
        Ok(stream) => stream,
        Err(code) => return failure(family, started, code, via),
    };
    match request_ip(tls, endpoint).await {
        Ok(value) => {
            let family_matches =
                (family == "ipv4" && value.is_ipv4()) || (family == "ipv6" && value.is_ipv6());
            if !family_matches {
                return failure(family, started, "family_mismatch", via);
            }
            Cell {
                path: if via.is_some() {
                    "lintel_channel"
                } else {
                    "host_default"
                },
                family,
                status: Status::Ok,
                public_ip: Some(value.to_string()),
                elapsed_ms: started.elapsed().as_millis() as u64,
                peer_family: Some(if peer.is_ipv4() {
                    "ipv4".into()
                } else {
                    "ipv6".into()
                }),
                message: None,
            }
        }
        Err(code) => failure(family, started, code, via),
    }
}

fn tls_config() -> rustls::ClientConfig {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .expect("ring provider supports TLS1.2/1.3")
        .with_root_certificates(roots)
        .with_no_client_auth()
}

async fn tls_connect_with(
    tcp: TcpStream,
    host: &str,
    config: &rustls::ClientConfig,
) -> Result<tokio_rustls::client::TlsStream<TcpStream>, &'static str> {
    let name = ServerName::try_from(host.to_string()).map_err(|_| "tls_failed")?;
    TlsConnector::from(Arc::new(config.clone()))
        .connect(name, tcp)
        .await
        .map_err(|_| "tls_failed")
}

/// Fetch the body and read back exactly one validated IP address. No URL, path,
/// header, or body content beyond the validated IP is retained or returned.
async fn request_ip<S>(mut stream: S, endpoint: &Endpoint) -> Result<IpAddr, &'static str>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nUser-Agent: lintel-ipv4-ipv6-probe\r\nAccept: text/plain\r\nConnection: close\r\n\r\n",
        endpoint.path, endpoint.authority()
    );
    stream
        .write_all(request.as_bytes())
        .await
        .map_err(|_| "http_error")?;
    let mut raw = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        if let Some(ip) = response_ip(&raw, false)? {
            return Ok(ip);
        }
        let count = stream.read(&mut chunk).await.map_err(|_| "http_error")?;
        if count == 0 {
            return response_ip(&raw, true)?.ok_or("invalid_response");
        }
        if raw.len() + count > MAX_HEADER + MAX_ENCODED_BODY {
            return Err("invalid_response");
        }
        raw.extend_from_slice(&chunk[..count]);
    }
}

#[cfg(test)]
fn parse_echo_ip(raw: &[u8]) -> Result<IpAddr, &'static str> {
    response_ip(raw, true)?.ok_or("invalid_response")
}

// One bounded response: honor framing and stop at its complete body, even if a
// peer ignores Connection: close. Never follow redirects or accept partial data.
fn response_ip(raw: &[u8], eof: bool) -> Result<Option<IpAddr>, &'static str> {
    let Some(head_end) = raw.windows(4).position(|w| w == b"\r\n\r\n") else {
        return if raw.len() >= MAX_HEADER || eof {
            Err("invalid_response")
        } else {
            Ok(None)
        };
    };
    if head_end + 4 > MAX_HEADER {
        return Err("invalid_response");
    }
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut response = httparse::Response::new(&mut headers);
    if !response
        .parse(&raw[..head_end + 4])
        .map_err(|_| "invalid_response")?
        .is_complete()
        || response.version != Some(1)
    {
        return Err("invalid_response");
    }
    if response.code != Some(200) {
        return Err("http_error");
    }
    let mut length = None;
    let mut chunked = false;
    for header in response.headers.iter() {
        if header.name.eq_ignore_ascii_case("content-length") {
            if length.is_some() {
                return Err("invalid_response");
            }
            let value = std::str::from_utf8(header.value)
                .map_err(|_| "invalid_response")?
                .trim();
            if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
                return Err("invalid_response");
            }
            let size: usize = value.parse().map_err(|_| "invalid_response")?;
            if size > MAX_BODY {
                return Err("invalid_response");
            }
            length = Some(size);
        } else if header.name.eq_ignore_ascii_case("transfer-encoding") {
            if chunked || !header.value.eq_ignore_ascii_case(b"chunked") {
                return Err("invalid_response");
            }
            chunked = true;
        } else if header.name.eq_ignore_ascii_case("content-encoding")
            && !header.value.eq_ignore_ascii_case(b"identity")
        {
            return Err("invalid_response");
        }
    }
    if length.is_some() && chunked {
        return Err("invalid_response");
    }
    let body = &raw[head_end + 4..];
    if chunked {
        if body.len() > MAX_ENCODED_BODY {
            return Err("invalid_response");
        }
        return match decode_chunks(body)? {
            Some(body) => Ok(Some(echo_ip(&body)?)),
            None if eof => Err("invalid_response"),
            None => Ok(None),
        };
    }
    if body.len() > MAX_BODY {
        return Err("invalid_response");
    }
    if let Some(size) = length {
        if body.len() > size {
            return Err("invalid_response");
        }
        if body.len() == size {
            return Ok(Some(echo_ip(body)?));
        }
        if eof {
            return Err("invalid_response");
        }
        return Ok(None);
    }
    if eof {
        Ok(Some(echo_ip(body)?))
    } else {
        Ok(None)
    }
}

fn decode_chunks(mut encoded: &[u8]) -> Result<Option<Vec<u8>>, &'static str> {
    let mut body = Vec::new();
    loop {
        let Some(end) = encoded.windows(2).position(|w| w == b"\r\n") else {
            return if encoded.len() > 8 {
                Err("invalid_response")
            } else {
                Ok(None)
            };
        };
        let size = &encoded[..end];
        if size.is_empty() || size.len() > 8 || !size.iter().all(u8::is_ascii_hexdigit) {
            return Err("invalid_response");
        }
        let size = usize::from_str_radix(
            std::str::from_utf8(size).map_err(|_| "invalid_response")?,
            16,
        )
        .map_err(|_| "invalid_response")?;
        encoded = &encoded[end + 2..];
        if size == 0 {
            if encoded.len() < 2 {
                return Ok(None);
            }
            if encoded != b"\r\n" {
                return Err("invalid_response");
            }
            return Ok(Some(body));
        }
        if size > MAX_BODY - body.len() {
            return Err("invalid_response");
        }
        if encoded.len() < size + 2 {
            return Ok(None);
        }
        if &encoded[size..size + 2] != b"\r\n" {
            return Err("invalid_response");
        }
        body.extend_from_slice(&encoded[..size]);
        encoded = &encoded[size + 2..];
    }
}

fn echo_ip(body: &[u8]) -> Result<IpAddr, &'static str> {
    let body = std::str::from_utf8(body)
        .map_err(|_| "invalid_response")?
        .trim();
    if body.is_empty() || body.len() > 64 {
        return Err("invalid_response");
    }
    let ip: IpAddr = body.parse().map_err(|_| "invalid_response")?;
    Ok(match ip {
        IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
        ip => ip,
    })
}

fn failure(family: &'static str, started: Instant, code: &str, via: Option<&str>) -> Cell {
    let status = match code {
        "dns_failed" => Status::DnsFailed,
        "no_route" => Status::NoRoute,
        "connect_refused" | "refused" => Status::Refused,
        "timed_out" | "destination_timeout" => Status::TimedOut,
        "tls_failed" => Status::TlsFailed,
        "proxy_denied" => Status::ProxyDenied,
        "http_error" => Status::HttpError,
        "family_mismatch" => Status::FamilyMismatch,
        "ipv4_unavailable" => Status::Ipv4Unavailable,
        "ipv6_unavailable" => Status::Ipv6Unavailable,
        "connect_failed" => Status::ConnectionFailed,
        "permission_denied" => Status::PermissionDenied,
        "proxy_failed" => Status::ProxyFailed,
        _ => Status::InvalidResponse,
    };
    Cell {
        path: if via.is_some() {
            "lintel_channel"
        } else {
            "host_default"
        },
        family,
        status,
        public_ip: None,
        elapsed_ms: started.elapsed().as_millis() as u64,
        peer_family: None,
        message: Some(finite_message(status)),
    }
}

fn finite_message(status: Status) -> String {
    match status {
        Status::DnsFailed => "域名解析失败".into(),
        Status::NoRoute => "连接或路由不可达".into(),
        Status::Refused => "连接被拒绝".into(),
        Status::TimedOut => "请求超时".into(),
        Status::TlsFailed => "TLS 校验失败".into(),
        Status::ProxyDenied => "通道拒绝或不接受此目标".into(),
        Status::HttpError => "响应状态非 200".into(),
        Status::FamilyMismatch => "回显地址与测试地址族不一致".into(),
        Status::Ipv4Unavailable => "此目标没有可用 IPv4 地址".into(),
        Status::Ipv6Unavailable => "此目标没有可用 IPv6 地址".into(),
        Status::ConnectionFailed => "连接失败；不推断为策略阻止".into(),
        Status::PermissionDenied => "系统拒绝此连接；不推断为 Lintel 策略".into(),
        Status::ProxyFailed => "通道未能建立此连接；不推断为策略拒绝".into(),
        Status::InvalidResponse => "响应不符合有限回显格式".into(),
        Status::NotTested => "此路径未测试".into(),
        Status::Ok => "成功".into(),
    }
}

/// Synchronous convenience wrapper for callers that are not async. Uses a
/// dedicated current-thread runtime, so the probe never requires a caller-owned
/// tokio runtime and never spawns a background worker.
pub fn run_blocking(
    request: &ProbeRequest,
    proxy: Option<ProxyChannel>,
) -> Result<[Cell; 4], &'static str> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| "runtime_unavailable")?;
    runtime.block_on(run(request, proxy))
}

/// Test-only entry that substitutes a pinned trust store for the Mozilla roots,
/// so the full certificate/hostname verification path runs against a synthetic
/// local server without contacting a real endpoint. Not used in production.
#[cfg(test)]
pub(crate) async fn run_with_trust_for_test(
    request: &ProbeRequest,
    proxy: Option<ProxyChannel>,
    trust: std::sync::Arc<tokio_rustls::rustls::ClientConfig>,
) -> Result<[Cell; 4], &'static str> {
    run_with_trust(request, proxy, Some(trust)).await
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn finite_http_framing_accepts_chunks_and_rejects_partial_or_ambiguous_bodies() {
        assert_eq!(parse_echo_ip(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nb\r\n203.0.113.7\r\n0\r\n\r\n").unwrap().to_string(), "203.0.113.7");
        assert!(response_ip(
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n203.0.",
            false
        )
        .unwrap()
        .is_none());
        for raw in [
            b"HTTP/1.1 200 OK\r\nContent-Length: 12\r\n\r\n203.0.113.7".as_slice(),
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nContent-Length: 11\r\n\r\n203.0.113.7",
            b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\nTransfer-Encoding: chunked\r\n\r\n203.0.113.7",
            b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nb\r\n203.0.113.7\r\n",
            b"HTTP/1.1 302 Found\r\nLocation: https://other.invalid\r\n\r\n203.0.113.7",
            b"HTTP/1.1 200 OK\r\nContent-Length: 999999\r\n\r\n203.0.113.7",
        ] { assert!(parse_echo_ip(raw).is_err(), "{raw:?}"); }
    }

    #[test]
    fn echo_body_must_be_exactly_one_ip() {
        assert_eq!(
            parse_echo_ip(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n203.0.113.7").unwrap(),
            "203.0.113.7".parse::<IpAddr>().unwrap()
        );
        assert_eq!(
            parse_echo_ip(b"HTTP/1.1 200 OK\r\n\r\n2001:db8::1\n").unwrap(),
            "2001:db8::1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn non_200_and_non_ip_bodies_are_invalid() {
        assert_eq!(
            parse_echo_ip(b"HTTP/1.1 500 Error\r\n\r\n1.2.3.4"),
            Err("http_error")
        );
        // A redirect or free-form body is never accepted as an echo.
        assert!(parse_echo_ip(b"HTTP/1.1 200 OK\r\n\r\n<html>hi</html>").is_err());
        assert!(parse_echo_ip(b"HTTP/1.1 200 OK\r\n\r\n1.2.3.4 and more").is_err());
        assert!(parse_echo_ip(b"not http at all").is_err());
    }

    #[test]
    fn address_family_constraint_treats_mapped_ipv6_as_ipv4() {
        // The ipv4_only branch of connect_tcp maps an IPv4-mapped IPv6 literal
        // to its embedded IPv4 before connecting; prove the literal handling.
        let mapped = "[::ffff:127.0.0.1]:443";
        let dest = crate::destination(mapped, None).unwrap();
        assert_eq!(dest.host, "127.0.0.1");
    }
}

#[cfg(test)]
#[path = "probe_tests.rs"]
mod socket_tests;
