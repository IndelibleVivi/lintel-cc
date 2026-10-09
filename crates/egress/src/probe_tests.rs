//! Synthetic evidence for the finite HTTPS IP-echo probe.
//!
//! Everything here runs on loopback with self-signed TLS and a synthetic
//! CONNECT proxy. No public endpoint, real certificate, host network change, or
//! DNS of a real name is involved. The probe's production trust store (Mozilla
//! roots) is replaced with a pinned synthetic root so the *full* TLS
//! certificate/hostname verification path runs, not a bypass.
use crate::probe::{run, run_with_trust_for_test, Endpoint, ProbeRequest, ProxyChannel, Status};
use std::sync::Arc;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinHandle,
};
use tokio_rustls::rustls;

fn server_config(
    cert: rustls::pki_types::CertificateDer<'static>,
    key: Vec<u8>,
) -> rustls::ServerConfig {
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(key.into());
    rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .unwrap()
}

fn pinned_trust(cert: &rcgen::Certificate) -> Arc<rustls::ClientConfig> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(cert.der().clone())
        .expect("synthetic root is a valid certificate");
    Arc::new(
        rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_root_certificates(roots)
        .with_no_client_auth(),
    )
}

/// A synthetic TLS IP-echo HTTP/1.1 server. Replies with `body` on any request
/// path. Bind address is chosen by the caller so the caller knows the port.
async fn tls_echo(
    bind: &str,
    certified: &rcgen::CertifiedKey<rcgen::KeyPair>,
    body: &'static str,
) -> (std::net::SocketAddr, JoinHandle<()>) {
    let config = server_config(
        certified.cert.der().clone(),
        certified.signing_key.serialize_der(),
    );
    let listener = TcpListener::bind(bind).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((tcp, _)) = listener.accept().await else {
                return;
            };
            let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config.clone()));
            tokio::spawn(async move {
                let Ok(mut stream) = acceptor.accept(tcp).await else {
                    return;
                };
                let mut head = Vec::new();
                let mut buf = [0u8; 512];
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => head.extend_from_slice(&buf[..n]),
                    }
                }
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(reply.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });
    (addr, handle)
}

/// A synthetic loopback CONNECT proxy. `allow=false` denies every tunnel with a
/// 403 so the `proxy_denied` path is exercised without touching the endpoint.
async fn connect_proxy(bind: &str, allow: bool) -> (std::net::SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind(bind).await.unwrap();
    let address = listener.local_addr().unwrap();
    let handle = tokio::spawn(async move {
        loop {
            let Ok((mut client, _)) = listener.accept().await else {
                return;
            };
            tokio::spawn(async move {
                let mut head = Vec::new();
                let mut buf = [0u8; 512];
                while !head.windows(4).any(|w| w == b"\r\n\r\n") {
                    match client.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => head.extend_from_slice(&buf[..n]),
                    }
                }
                let text = String::from_utf8_lossy(&head);
                let authority = text
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .map(str::to_owned)
                    .unwrap_or_default();
                if !allow {
                    let _ = client
                        .write_all(b"HTTP/1.1 403 Forbidden\r\nContent-Length: 0\r\n\r\n")
                        .await;
                    return;
                }
                let Ok(mut upstream) = TcpStream::connect(authority.as_str()).await else {
                    let _ = client
                        .write_all(b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\n\r\n")
                        .await;
                    return;
                };
                let _ = client
                    .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
                    .await;
                let _ = tokio::io::copy_bidirectional(&mut client, &mut upstream).await;
            });
        }
    });
    (address, handle)
}

fn request(ipv4_url: String, ipv6_url: String, proxy_url: Option<String>) -> ProbeRequest {
    ProbeRequest {
        ipv4_url,
        ipv6_url,
        proxy_url,
        timeout_seconds: 5,
    }
}

fn url(host: &str, port: u16) -> String {
    format!("https://{host}:{port}/")
}

fn cell_kind(status: Status) -> &'static str {
    match status {
        Status::Ok => "ok",
        _ => "error",
    }
}

#[tokio::test]
async fn four_cells_without_proxy_mark_channel_not_tested() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let (v4_addr, _v4) = tls_echo("127.0.0.1:0", &certified, "203.0.113.7").await;
    let (v6_addr, _v6) = tls_echo("[::1]:0", &certified, "2001:db8::1").await;
    let trust = pinned_trust(&certified.cert);
    let req = request(
        url("localhost", v4_addr.port()),
        url("localhost", v6_addr.port()),
        None,
    );
    let cells = run_with_trust_for_test(&req, None, trust).await.unwrap();
    assert_eq!(cells.len(), 4);
    // host_default/ipv4 reaches the IPv4 echo and echoes an IPv4.
    assert_eq!(cells[0].status, Status::Ok);
    assert_eq!(cells[0].public_ip.as_deref(), Some("203.0.113.7"));
    assert_eq!(cells[0].peer_family.as_deref(), Some("ipv4"));
    assert_eq!(cell_kind(cells[0].status), "ok");
    // The IPv6 cell must actually connect via IPv6.
    assert_eq!(cells[1].status, Status::Ok);
    assert_eq!(cells[1].peer_family.as_deref(), Some("ipv6"));
    assert_eq!(cells[1].public_ip.as_deref(), Some("2001:db8::1"));
    // No proxy: both channel cells are explicitly not tested.
    assert_eq!(cells[2].path, "lintel_channel");
    assert_eq!(cells[2].family, "ipv4");
    assert_eq!(cells[2].status, Status::NotTested);
    assert_eq!(cells[3].status, Status::NotTested);
    assert_eq!(cells[3].family, "ipv6");
}

#[tokio::test]
async fn channel_uses_proxy_connect_and_echoes() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let (v4_addr, _v4) = tls_echo("127.0.0.1:0", &certified, "203.0.113.7").await;
    let (v6_addr, _v6) = tls_echo("[::1]:0", &certified, "2001:db8::1").await;
    let (proxy_addr, _proxy) = connect_proxy("127.0.0.1:0", true).await;
    let trust = pinned_trust(&certified.cert);
    let req = request(
        url("localhost", v4_addr.port()),
        url("localhost", v6_addr.port()),
        Some(format!("http://127.0.0.1:{}", proxy_addr.port())),
    );
    let channel = ProxyChannel {
        authority: format!("127.0.0.1:{}", proxy_addr.port()),
        family: crate::AddressFamily::Ipv4Only,
    };
    let cells = run_with_trust_for_test(&req, Some(channel), trust)
        .await
        .unwrap();
    assert_eq!(
        cells
            .iter()
            .map(|cell| (cell.path, cell.family))
            .collect::<Vec<_>>(),
        vec![
            ("host_default", "ipv4"),
            ("host_default", "ipv6"),
            ("lintel_channel", "ipv4"),
            ("lintel_channel", "ipv6"),
        ]
    );
    assert_eq!(cells[2].status, Status::Ok, "{:?}", cells[2]);
    assert_eq!(cells[2].public_ip.as_deref(), Some("203.0.113.7"));
    assert_eq!(cells[3].status, Status::Ok, "{:?}", cells[3]);
    assert_eq!(cells[3].public_ip.as_deref(), Some("2001:db8::1"));
}

#[tokio::test]
async fn proxy_denied_channel_cell_keeps_specific_reason() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let (v4_addr, _v4) = tls_echo("127.0.0.1:0", &certified, "203.0.113.7").await;
    let (proxy_addr, _proxy) = connect_proxy("127.0.0.1:0", false).await;
    let trust = pinned_trust(&certified.cert);
    let req = request(
        url("localhost", v4_addr.port()),
        url("localhost", v4_addr.port()),
        Some(format!("http://127.0.0.1:{}", proxy_addr.port())),
    );
    let channel = ProxyChannel {
        authority: format!("127.0.0.1:{}", proxy_addr.port()),
        family: crate::AddressFamily::Ipv4Only,
    };
    let cells = run_with_trust_for_test(&req, Some(channel), trust)
        .await
        .unwrap();
    assert_eq!(cells[2].status, Status::ProxyDenied);
    assert!(cells[2].public_ip.is_none());
}

#[tokio::test]
async fn family_mismatch_when_echo_does_not_match_requested_family() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    // The IPv4 cell reaches an echo that returns an IPv6 address.
    let (v4_addr, _v4) = tls_echo("127.0.0.1:0", &certified, "2001:db8::1").await;
    let trust = pinned_trust(&certified.cert);
    let req = request(
        url("localhost", v4_addr.port()),
        url("localhost", v4_addr.port()),
        None,
    );
    let cells = run_with_trust_for_test(&req, None, trust).await.unwrap();
    assert_eq!(cells[0].status, Status::FamilyMismatch);
}

#[tokio::test]
async fn unreachable_endpoint_reports_connect_failure_not_success() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let trust = pinned_trust(&certified.cert);
    // Nothing is listening on this port; the probe must fail, never fabricate.
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead = listener.local_addr().unwrap().port();
    drop(listener);
    let req = request(url("localhost", dead), url("localhost", dead), None);
    let cells = run_with_trust_for_test(&req, None, trust).await.unwrap();
    assert!(matches!(
        cells[0].status,
        Status::Refused | Status::NoRoute | Status::TimedOut
    ));
    assert!(cells[0].public_ip.is_none());
}

#[tokio::test]
async fn ipv6_probe_cannot_accept_an_ipv4_socket_even_with_ipv6_echo() {
    let certified = rcgen::generate_simple_self_signed(vec!["localhost".into()]).unwrap();
    let (addr, _server) = tls_echo("127.0.0.1:0", &certified, "2001:db8::1").await;
    let req = request(
        url("localhost", addr.port()),
        url("127.0.0.1", addr.port()),
        None,
    );
    let cells = run_with_trust_for_test(&req, None, pinned_trust(&certified.cert))
        .await
        .unwrap();
    assert_eq!(cells[1].status, Status::Ipv6Unavailable);
    assert!(cells[1].public_ip.is_none());
}

#[tokio::test]
async fn whole_request_timeout_includes_stalled_tls_and_cells_run_together() {
    async fn silent(bind: &str) -> (std::net::SocketAddr, JoinHandle<()>) {
        let listener = TcpListener::bind(bind).await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            loop {
                let (tcp, _) = listener.accept().await.unwrap();
                tokio::spawn(async move {
                    let _held = tcp;
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                });
            }
        });
        (addr, handle)
    }
    let (v4, h4) = silent("127.0.0.1:0").await;
    let (v6, h6) = silent("[::1]:0").await;
    let mut req = request(
        url("localhost", v4.port()),
        url("localhost", v6.port()),
        None,
    );
    req.timeout_seconds = 1;
    let started = std::time::Instant::now();
    let cells = tokio::time::timeout(std::time::Duration::from_secs(2), run(&req, None))
        .await
        .expect("TLS must share the whole request deadline")
        .unwrap();
    assert_eq!(cells[0].status, Status::TimedOut);
    assert_eq!(cells[1].status, Status::TimedOut);
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
    h4.abort();
    h6.abort();
}

#[tokio::test]
async fn pinned_certificate_still_requires_the_requested_hostname() {
    let certified = rcgen::generate_simple_self_signed(vec!["other.invalid".into()]).unwrap();
    let (addr, _server) = tls_echo("127.0.0.1:0", &certified, "203.0.113.7").await;
    let req = request(
        url("localhost", addr.port()),
        url("localhost", addr.port()),
        None,
    );
    let cells = run_with_trust_for_test(&req, None, pinned_trust(&certified.cert))
        .await
        .unwrap();
    assert_eq!(cells[0].status, Status::TlsFailed);
    assert!(cells[0].public_ip.is_none());
}

#[test]
fn endpoint_validation_rejects_unsafe_urls() {
    for bad in [
        "http://example.invalid/",
        "https://user:pass@example.invalid/",
        "https://example.invalid/?x=1",
        "https://example.invalid/#frag",
        "https://example.invalid:0/",
        "https:///",
        "not a url",
    ] {
        assert!(Endpoint::parse(bad).is_err(), "{bad}");
    }
    assert_eq!(
        Endpoint::parse("https://api.ipify.org/")
            .unwrap()
            .describe(),
        "https://api.ipify.org/"
    );
    assert!(Endpoint::parse("https://[::1]:8443/echo").is_ok());
}

#[test]
fn timeout_out_of_range_is_rejected() {
    let req = ProbeRequest {
        ipv4_url: "https://api.ipify.org/".into(),
        ipv6_url: "https://api6.ipify.org/".into(),
        proxy_url: None,
        timeout_seconds: 0,
    };
    let outcome = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(run(&req, None));
    assert!(outcome.is_err());
}
