//! Fixed system-resolver ownership. A stalled OS resolver occupies one of 32
//! slots; callers never queue more work or make runtime shutdown wait for it.
use std::{
    net::{SocketAddr, ToSocketAddrs},
    sync::{Arc, OnceLock},
};
use tokio::sync::{oneshot, Semaphore};

pub(crate) async fn resolve(host: String, port: u16) -> Result<Vec<SocketAddr>, ()> {
    resolve_with(move || {
        (host.as_str(), port)
            .to_socket_addrs()
            .map(|addresses| addresses.take(64).collect())
            .map_err(|_| ())
    })
    .await
}

async fn resolve_with<F>(lookup: F) -> Result<Vec<SocketAddr>, ()>
where
    F: FnOnce() -> Result<Vec<SocketAddr>, ()> + Send + 'static,
{
    static SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    resolve_owned(
        SLOTS.get_or_init(|| Arc::new(Semaphore::new(32))).clone(),
        lookup,
    )
    .await
}

async fn resolve_owned<F>(slots: Arc<Semaphore>, lookup: F) -> Result<Vec<SocketAddr>, ()>
where
    F: FnOnce() -> Result<Vec<SocketAddr>, ()> + Send + 'static,
{
    let permit = slots.try_acquire_owned().map_err(|_| ())?;
    let (send, receive) = oneshot::channel();
    std::thread::Builder::new()
        .name("lintel-system-dns".into())
        .spawn(move || {
            let _permit = permit;
            // Only address resolution happens here. A canceled caller can never
            // cause a late connection, TLS handshake or HTTP request.
            let result = lookup();
            let _ = send.send(result);
        })
        .map_err(|_| ())?;
    receive.await.map_err(|_| ())?
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saturated_owner_rejects_without_invoking_resolver() {
        let slots = Arc::new(Semaphore::new(1));
        let permit = slots.clone().try_acquire_owned().unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        for _ in 0..100 {
            assert!(runtime
                .block_on(resolve_owned(slots.clone(), || panic!("must not enqueue")))
                .is_err());
        }
        drop(permit);
        assert!(runtime
            .block_on(resolve_owned(slots, || Ok(vec![])))
            .is_ok());
    }
    #[test]
    fn stalled_resolution_is_bounded_and_runtime_drop_does_not_wait() {
        let (release, wait) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let started = std::time::Instant::now();
        runtime.block_on(async {
            assert!(tokio::time::timeout(
                std::time::Duration::from_millis(30),
                resolve_owned(Arc::new(Semaphore::new(1)), move || {
                    wait.recv().unwrap();
                    Ok(vec![])
                })
            )
            .await
            .is_err());
        });
        drop(runtime);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        release.send(()).unwrap();
    }
}
