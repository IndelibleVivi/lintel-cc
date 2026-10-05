#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let result = if args.len() == 2 && args[0] == "--config" {
        lintel_egress::serve_config(std::path::Path::new(&args[1])).await
    } else {
        Err("usage_lintel_egress_--config_PATH".into())
    };
    if let Err(code) = result {
        eprintln!("{}", serde_json::json!({"ok":false,"error":{"code":code}}));
        std::process::exit(1);
    }
}
