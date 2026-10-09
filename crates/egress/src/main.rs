#[tokio::main]
async fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    // Accepted grammars:
    //   --config PATH
    //   --config PATH --test-telemetry ID[,ID...]
    let result = match args.as_slice() {
        [flag, path] if flag == "--config" => {
            lintel_egress::serve_config(std::path::Path::new(path)).await
        }
        [flag, path, tests_flag, ids] if flag == "--config" && tests_flag == "--test-telemetry" => {
            let ids: Vec<String> = ids.split(',').map(str::trim).map(str::to_owned).collect();
            lintel_egress::serve_config_with_tests(std::path::Path::new(path), &ids).await
        }
        _ => Err("usage_lintel_egress_--config_PATH_[--test-telemetry_ID,...]".into()),
    };
    if let Err(code) = result {
        eprintln!("{}", serde_json::json!({"ok":false,"error":{"code":code}}));
        std::process::exit(1);
    }
}
