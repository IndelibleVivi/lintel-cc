use lintel_egress::{Config, Proxy};
use std::sync::Arc;

#[tokio::main]
async fn main() {
    if let Err(code) = run().await {
        eprintln!("{}", serde_json::json!({"ok":false,"error":{"code":code}}));
        std::process::exit(1);
    }
}
async fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--config" {
        return Err("usage_lintel_egress_--config_PATH".into());
    }
    let bytes = std::fs::read(&args[2]).map_err(|_| "config_read_failed".to_string())?;
    let config: Config =
        serde_json::from_slice(&bytes).map_err(|_| "invalid_config_json".to_string())?;
    let proxy = Proxy::bind(
        config,
        Arc::new(|event| {
            // This typed event is the complete log surface: never serialize raw requests/errors.
            println!("{}", serde_json::to_string(&event).unwrap());
        }),
    )
    .await?;
    println!(
        "{}",
        serde_json::json!({"event":"listening","address":proxy.local_addr().map_err(|_|"listen_failed".to_string())?,"coverage":"proxy_connections_only","direct_connections_enforced":false})
    );
    proxy
        .serve_until(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .map_err(|_| "accept_failed".to_string())
}
