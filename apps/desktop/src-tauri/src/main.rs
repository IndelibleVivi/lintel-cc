#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod network;
mod remote;
mod resources;

#[tauri::command]
async fn request(payload: serde_json::Value) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || lintel_core::handle_request(payload))
        .await
        .map_err(|error| format!("本地执行器未能完成请求：{error}"))
}

#[tauri::command]
async fn browser_request(payload: serde_json::Value) -> Result<serde_json::Value, String> {
    tauri::async_runtime::spawn_blocking(move || lintel_browser_host::control(payload))
        .await
        .map_err(|error| format!("浏览器桥接请求未完成：{error}"))
}

fn main() {
    tauri::Builder::default()
        .manage(network::NetworkState::default())
        .invoke_handler(tauri::generate_handler![
            request,
            browser_request,
            network::network_request,
            remote::remote_request,
            resources::open_resource
        ])
        .run(tauri::generate_context!())
        .expect("Lintel could not start");
}
