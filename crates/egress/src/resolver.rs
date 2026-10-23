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
    fn thirty_two_blocked_owners_cancel_without_queue_or_late_connection() {
        use std::sync::{
            atomic::{AtomicUsize, Ordering},
            Condvar, Mutex,
        };
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let slots = Arc::new(Semaphore::new(32));
        let entered = Arc::new(AtomicUsize::new(0));
        let connected = Arc::new(AtomicUsize::new(0));
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let (finished, done) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut tasks = Vec::new();
            for _ in 0..32 {
                let (slots, entered, connected, gate, finished) = (
                    slots.clone(),
                    entered.clone(),
                    connected.clone(),
                    gate.clone(),
                    finished.clone(),
                );
                tasks.push(tokio::spawn(async move {
                    let addresses = resolve_owned(slots, move || {
                        entered.fetch_add(1, Ordering::SeqCst);
                        let (lock, wake) = &*gate;
                        let _guard = wake
                            .wait_while(lock.lock().unwrap(), |released| !*released)
                            .unwrap();
                        finished.send(()).unwrap();
                        Ok(vec![address])
                    })
                    .await;
                    if let Ok(addresses) = addresses {
                        connected.fetch_add(1, Ordering::SeqCst);
                        let _ = tokio::net::TcpStream::connect(addresses[0]).await;
                    }
                }));
            }
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                while entered.load(Ordering::SeqCst) != 32 {
                    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
                }
            })
            .await
            .unwrap();
            for _ in 0..100 {
                assert!(resolve_owned(slots.clone(), || panic!(
                    "saturation must not spawn or queue"
                ))
                .await
                .is_err());
            }
            assert_eq!(entered.load(Ordering::SeqCst), 32);
            for task in &tasks {
                task.abort();
            }
            for task in tasks {
                assert!(task.await.unwrap_err().is_cancelled());
            }
        });
        let started = std::time::Instant::now();
        drop(runtime);
        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert_eq!(
            slots.available_permits(),
            0,
            "canceled callers cannot free running owners"
        );
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        for _ in 0..32 {
            done.recv_timeout(std::time::Duration::from_secs(3))
                .unwrap();
        }
        let limit = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while slots.available_permits() != 32 && std::time::Instant::now() < limit {
            std::thread::yield_now();
        }
        assert_eq!(slots.available_permits(), 32);
        assert_eq!(
            connected.load(Ordering::SeqCst),
            0,
            "late addresses cannot reach connection continuation"
        );
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }
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
