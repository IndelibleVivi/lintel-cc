#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod activity;
mod app_updates;
mod bundled_browser_extension;
mod bundled_browser_host;
mod cli_inspector;
mod network;
mod remote;
mod resources;

#[tauri::command]
async fn request(
    state: tauri::State<'_, network::NetworkState>,
    gate: tauri::State<'_, activity::ActivityGate>,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let permit = match gate.operation() {
        Ok(permit) => permit,
        Err(code) => {
            return Ok(
                serde_json::json!({"ok":false,"error":{"code":code,"message":"App 正在安装更新，请完成后再执行操作"}}),
            )
        }
    };
    Ok(state
        .dispatch_core(payload, move |payload| {
            let _permit = &permit;
            lintel_core::handle_request(payload)
        })
        .await)
}

#[tauri::command]
async fn browser_request(app: tauri::AppHandle, payload: serde_json::Value) -> serde_json::Value {
    use bundled_browser_host::{envelope, Installer};
    use tauri::Manager;
    let gate = app.state::<activity::ActivityGate>();
    let permit = match gate.operation() {
        Ok(permit) => permit,
        Err(code) => {
            return envelope(Err(format!(
                "{code}:App 正在安装更新，请完成后再操作浏览器"
            )))
        }
    };
    let op = payload["op"].as_str();
    if matches!(op, Some("installation_plan" | "install_native_host")) {
        return envelope(Err("unsupported_browser_operation:Desktop 安装只接受 App 内置 host 的 frozen preview；手工路径 installer 由独立 CLI 提供".into()));
    }
    let bundle = match op {
        Some("bundled_host_plan" | "install_bundled_host") => {
            Some(("browser-host-bundle", "browser-host", false))
        }
        Some(
            "bundled_extension_plan" | "install_bundled_extension" | "reveal_bundled_extension",
        ) => Some(("browser-extension-bundle", "browser-extensions", true)),
        _ => None,
    };
    let resources = if let Some((development_path, packaged_path, extension)) = bundle {
        if cfg!(debug_assertions) {
            Some((
                std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(development_path),
                extension,
            ))
        } else {
            match app.path().resource_dir() {
                Ok(path) => Some((path.join(packaged_path), extension)),
                Err(_) => {
                    return envelope(Err(
                        "bundle_unavailable:无法定位 App 内的 browser resources".into(),
                    ))
                }
            }
        }
    } else {
        None
    };
    match tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        match resources {
            Some((resources, true)) => envelope(
                bundled_browser_extension::Installer::system(resources)
                    .and_then(|installer| installer.dispatch(payload)),
            ),
            Some((resources, false)) => envelope(
                Installer::system(resources).and_then(|installer| installer.dispatch(payload)),
            ),
            None => lintel_browser_host::control(payload),
        }
    })
    .await
    {
        Ok(response) => response,
        Err(error) => envelope(Err(format!(
            "browser_bridge_failed:浏览器桥接请求未完成：{error}"
        ))),
    }
}

#[tauri::command(rename = "network_request")]
async fn guarded_network_request(
    state: tauri::State<'_, network::NetworkState>,
    gate: tauri::State<'_, activity::ActivityGate>,
    payload: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let permit = match gate.operation() {
        Ok(permit) => permit,
        Err(code) => {
            return Ok(
                serde_json::json!({"ok":false,"error":{"code":code,"message":"App 正在安装更新，请稍后操作通道"}}),
            )
        }
    };
    network::network_request(state, payload, permit).await
}

#[tauri::command]
async fn remote_request(app: tauri::AppHandle, payload: serde_json::Value) -> serde_json::Value {
    use tauri::Manager;
    let gate = app.state::<activity::ActivityGate>();
    let permit = match gate.operation() {
        Ok(permit) => permit,
        Err(code) => {
            return serde_json::json!({"ok":false,"error":{"code":code,"message":"App 正在安装更新，请稍后操作远端"}})
        }
    };
    remote::remote_request(app, payload, permit).await
}

fn main() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_updater::Builder::new()
                .default_version_comparator(|current, release| {
                    current.cmp_precedence(&release.version).is_lt()
                })
                .build(),
        )
        .manage(network::NetworkState::default())
        .manage(activity::ActivityGate::default())
        .manage(app_updates::AppUpdateState::default())
        .invoke_handler(tauri::generate_handler![
            request,
            browser_request,
            guarded_network_request,
            remote_request,
            app_updates::app_update_request,
            resources::open_resource,
            cli_inspector::inspect_cli
        ])
        .run(tauri::generate_context!())
        .expect("Lintel could not start");
}
