use lintel_egress::{Config, DefaultAction, Event, Proxy, Rule};
use std::sync::{Arc, Mutex};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc,
    task::JoinHandle,
    time::{timeout, Duration},
};

async fn proxy(
    mut config: Config,
) -> (
    std::net::SocketAddr,
    Arc<Mutex<Vec<Event>>>,
    mpsc::UnboundedReceiver<()>,
    JoinHandle<()>,
) {
    config.environment_id = "synthetic-fixture".into();
    config.connect_timeout_seconds = 2;
    config.connection_lifetime_seconds = 5;
    let events = Arc::new(Mutex::new(Vec::new()));
    let copy = events.clone();
    let (tx, rx) = mpsc::unbounded_channel();
    let proxy = Proxy::bind(
        config,
        Arc::new(move |e| {
            copy.lock().unwrap().push(e);
            let _ = tx.send(());
        }),
    )
    .await
    .unwrap();
    let address = proxy.local_addr().unwrap();
    let task = tokio::spawn(async move {
        proxy.serve().await.unwrap();
    });
    (address, events, rx, task)
}
async fn request(address: std::net::SocketAddr, request: &[u8]) -> Vec<u8> {
    let mut socket = TcpStream::connect(address).await.unwrap();
    socket.write_all(request).await.unwrap();
    socket.shutdown().await.unwrap();
    let mut response = Vec::new();
    timeout(Duration::from_secs(4), socket.read_to_end(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}
#[tokio::test]
async fn actual_connect_allow_deny_and_metadata_only() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = origin.local_addr().unwrap().port();
    let echo = tokio::spawn(async move {
        let (mut socket, _) = origin.accept().await.unwrap();
        let mut body = Vec::new();
        socket.read_to_end(&mut body).await.unwrap();
        socket.write_all(&body).await.unwrap();
    });
    let mut config = Config {
        default_action: DefaultAction::Deny,
        ..Config::default()
    };
    config.allowed.push(Rule {
        host: "127.0.0.1".into(),
        ports: vec![port],
    });
    config.blocked.push(Rule {
        host: "localhost".into(),
        ports: vec![],
    });
    let (address, events, mut finished, task) = proxy(config).await;
    let allowed = format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: SYNTHETIC_TOKEN\r\nCookie: SYNTHETIC_COOKIE\r\n\r\nSYNTHETIC_PROMPT");
    let reply = request(address, allowed.as_bytes()).await;
    assert!(reply.starts_with(b"HTTP/1.1 200"));
    assert!(reply.ends_with(b"SYNTHETIC_PROMPT"));
    echo.await.unwrap();
    let denied = request(
        address,
        format!("CONNECT localhost:{port} HTTP/1.1\r\nHost: localhost:{port}\r\n\r\n").as_bytes(),
    )
    .await;
    assert!(denied.starts_with(b"HTTP/1.1 403"));
    for _ in 0..2 {
        timeout(Duration::from_secs(2), finished.recv())
            .await
            .unwrap();
    }
    let events = events.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].outcome, "completed");
    assert_eq!(events[1].outcome, "blocked");
    for event in events.iter() {
        assert!(!event.direct_connections_enforced);
        assert_eq!(event.coverage, "proxy_connections_only");
    }
    let logs = serde_json::to_string(&*events).unwrap();
    for secret in [
        "SYNTHETIC_TOKEN",
        "SYNTHETIC_COOKIE",
        "SYNTHETIC_PROMPT",
        "Authorization",
        "Cookie",
    ] {
        assert!(!logs.contains(secret));
    }
    task.abort();
}
#[tokio::test]
async fn http_forwards_body_strips_proxy_credentials_and_never_relays_pipeline() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = origin.local_addr().unwrap().port();
    let seen = tokio::spawn(async move {
        let (mut socket, _) = origin.accept().await.unwrap();
        let mut bytes = vec![];
        loop {
            let mut chunk = [0; 2048];
            let n = socket.read(&mut chunk).await.unwrap();
            if n == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..n]);
            if bytes.ends_with(b"BODY") {
                break;
            }
        }
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
            .await
            .unwrap();
        bytes
    });
    let (address, events, mut finished, task) = proxy(Config::default()).await;
    let req = format!("POST http://127.0.0.1:{port}/private?SYNTHETIC_QUERY HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nContent-Length: 4\r\nAuthorization: SYNTHETIC_AUTH\r\nProxy-Authorization: SYNTHETIC_PROXY\r\n\r\nBODYGET http://blocked.invalid/ HTTP/1.1\r\nHost: blocked.invalid\r\n\r\n");
    let response = request(address, req.as_bytes()).await;
    assert!(response.ends_with(b"OK"));
    let forwarded = String::from_utf8(seen.await.unwrap()).unwrap();
    assert!(forwarded.starts_with("POST /private?SYNTHETIC_QUERY HTTP/1.1"));
    assert!(forwarded.contains("SYNTHETIC_AUTH"));
    assert!(!forwarded.contains("SYNTHETIC_PROXY"));
    assert!(!forwarded.contains("blocked.invalid"));
    timeout(Duration::from_secs(2), finished.recv())
        .await
        .unwrap();
    let logs = serde_json::to_string(&*events.lock().unwrap()).unwrap();
    for secret in [
        "SYNTHETIC_QUERY",
        "SYNTHETIC_AUTH",
        "SYNTHETIC_PROXY",
        "BODY",
        "/private",
    ] {
        assert!(!logs.contains(secret));
    }
    task.abort();
}
#[tokio::test]
async fn upstream_connect_used_and_failure_has_no_direct_fallback() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_port = upstream.local_addr().unwrap().port();
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_port = origin.local_addr().unwrap().port();
    let saw = tokio::spawn(async move {
        let (mut socket, _) = upstream.accept().await.unwrap();
        let mut data = [0; 1024];
        let n = socket.read(&mut data).await.unwrap();
        assert!(String::from_utf8_lossy(&data[..n])
            .starts_with(&format!("CONNECT 127.0.0.1:{origin_port} HTTP/1.1")));
        socket
            .write_all(b"HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n")
            .await
            .unwrap();
    });
    let config = Config {
        upstream: Some(format!("http://127.0.0.1:{upstream_port}")),
        ..Config::default()
    };
    let (address, _, _, task) = proxy(config).await;
    let response = request(
        address,
        format!(
            "CONNECT 127.0.0.1:{origin_port} HTTP/1.1\r\nHost: 127.0.0.1:{origin_port}\r\n\r\n"
        )
        .as_bytes(),
    )
    .await;
    assert!(response.starts_with(b"HTTP/1.1 502"));
    saw.await.unwrap();
    assert!(timeout(Duration::from_millis(80), origin.accept())
        .await
        .is_err());
    task.abort();
}
#[tokio::test]
async fn upstream_http_preserves_absolute_target_and_tunnel_transfers_bytes() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = upstream.local_addr().unwrap().port();
    let saw = tokio::spawn(async move {
        let (mut socket, _) = upstream.accept().await.unwrap();
        let mut data = [0; 2048];
        let n = socket.read(&mut data).await.unwrap();
        assert!(String::from_utf8_lossy(&data[..n])
            .starts_with("GET http://synthetic.invalid:80/one HTTP/1.1"));
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
            .await
            .unwrap();
        drop(socket);
        let (mut socket, _) = upstream.accept().await.unwrap();
        let n = socket.read(&mut data).await.unwrap();
        assert!(String::from_utf8_lossy(&data[..n])
            .starts_with("CONNECT synthetic.invalid:443 HTTP/1.1"));
        socket
            .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
            .await
            .unwrap();
        let mut payload = Vec::new();
        socket.read_to_end(&mut payload).await.unwrap();
        socket.write_all(&payload).await.unwrap();
    });
    let config = Config {
        upstream: Some(format!("http://127.0.0.1:{port}")),
        ..Config::default()
    };
    let (address, _, _, task) = proxy(config).await;
    let response = request(
        address,
        b"GET http://synthetic.invalid/one HTTP/1.1\r\nHost: synthetic.invalid\r\n\r\n",
    )
    .await;
    assert!(response.ends_with(b"OK"));
    let response = request(address, b"CONNECT synthetic.invalid:443 HTTP/1.1\r\nHost: synthetic.invalid:443\r\n\r\nopaque bytes").await;
    assert!(response.starts_with(b"HTTP/1.1 200"));
    assert!(response.ends_with(b"opaque bytes"));
    saw.await.unwrap();
    task.abort();
}
#[tokio::test]
async fn refuses_non_loopback_binding_and_socks_before_network_io() {
    let observer = Arc::new(|_| {});
    assert!(Proxy::bind(
        Config {
            bind: "0.0.0.0:12345".parse().unwrap(),
            ..Config::default()
        },
        observer.clone()
    )
    .await
    .is_err());
    assert!(Proxy::bind(
        Config {
            upstream: Some("socks5://localhost:1080".into()),
            ..Config::default()
        },
        observer
    )
    .await
    .is_err());
}

#[tokio::test]
async fn stopping_proxy_closes_existing_tunnels_before_completion() {
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = origin.local_addr().unwrap().port();
    let config = Config {
        environment_id: "stop-fixture".into(),
        ..Config::default()
    };
    let (events_tx, mut events) = mpsc::unbounded_channel();
    let proxy = Proxy::bind(
        config,
        Arc::new(move |e| {
            let _ = events_tx.send(e);
        }),
    )
    .await
    .unwrap();
    let address = proxy.local_addr().unwrap();
    let (stop, stopped) = tokio::sync::oneshot::channel();
    let service = tokio::spawn(async move {
        proxy
            .serve_until(async {
                let _ = stopped.await;
            })
            .await
            .unwrap();
    });
    let mut client = TcpStream::connect(address).await.unwrap();
    client
        .write_all(
            format!("CONNECT 127.0.0.1:{port} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let (mut remote, _) = origin.accept().await.unwrap();
    let mut head = [0; 39];
    client.read_exact(&mut head).await.unwrap();
    assert!(head.starts_with(b"HTTP/1.1 200"));
    stop.send(()).unwrap();
    timeout(Duration::from_secs(2), service)
        .await
        .unwrap()
        .unwrap();
    let mut byte = [0];
    assert_eq!(
        timeout(Duration::from_secs(2), client.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    assert_eq!(
        timeout(Duration::from_secs(2), remote.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0
    );
    assert_eq!(events.recv().await.unwrap().outcome, "proxy_stopped");
    assert!(TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn https_upstream_starts_tls_and_failure_does_not_fall_back_to_plaintext() {
    let upstream = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let upstream_port = upstream.local_addr().unwrap().port();
    let origin = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin_port = origin.local_addr().unwrap().port();
    let handshake = tokio::spawn(async move {
        let (mut socket, _) = upstream.accept().await.unwrap();
        let mut header = [0; 5];
        socket.read_exact(&mut header).await.unwrap();
        assert_eq!(
            header[0], 22,
            "TLS handshake record, never plaintext CONNECT"
        );
        assert_eq!(header[1], 3);
        // A plaintext/erroring upstream cannot make the proxy downgrade to HTTP.
        socket.write_all(b"HTTP/1.1 200 OK\r\n\r\n").await.unwrap();
    });
    let config = Config {
        upstream: Some(format!("https://127.0.0.1:{upstream_port}")),
        ..Config::default()
    };
    let (address, _, _, task) = proxy(config).await;
    let response = request(
        address,
        format!(
            "CONNECT 127.0.0.1:{origin_port} HTTP/1.1\r\nHost: 127.0.0.1:{origin_port}\r\n\r\n"
        )
        .as_bytes(),
    )
    .await;
    assert!(response.starts_with(b"HTTP/1.1 502"));
    handshake.await.unwrap();
    assert!(timeout(Duration::from_millis(80), origin.accept())
        .await
        .is_err());
    task.abort();
}
