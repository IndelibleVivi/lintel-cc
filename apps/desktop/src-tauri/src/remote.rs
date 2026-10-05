//! Tauri resource lookup and async wrapper for the shared SSH controller.
use serde_json::{json, Value};
use std::path::PathBuf;

#[tauri::command]
pub async fn remote_request(app: tauri::AppHandle, payload: Value) -> Value {
    use tauri::Manager;
    let bundles = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("runner-bundles")
    } else {
        match app.path().resource_dir() {
            Ok(path) => path.join("remote-runners"),
            Err(_) => {
                return json!({"ok":false,"error":{"code":"bundle_unavailable","message":"无法定位 App 内的远端 runner"}})
            }
        }
    };
    match tauri::async_runtime::spawn_blocking(move || lintel_remote::control(payload, bundles))
        .await
    {
        Ok(response) => response,
        Err(_) => {
            json!({"ok":false,"error":{"code":"remote_bridge_failed","message":"远程桥接中断；请保留原任务 ID 并查询"}})
        }
    }
}
