//! Finder reveal for one exact, freshly observed local inventory item.
//! The webview supplies an environment ID and a selected path, never an open
//! command, URL, executable, or remote host. No file content is opened here.
use serde_json::{json, Value};
use std::{
    fs,
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

fn selected_path(payload: &Value, inventory: &Value) -> Result<PathBuf, String> {
    let fields = payload
        .as_object()
        .ok_or("invalid_request:请求必须是对象")?;
    if fields
        .keys()
        .any(|k| !["environment_id", "path", "extra_roots"].contains(&k.as_str()))
        || !fields.contains_key("environment_id")
        || !fields.contains_key("path")
    {
        return Err("invalid_request:只接受明确环境 ID 与原件路径".into());
    }
    let environment = payload["environment_id"]
        .as_str()
        .ok_or("invalid_request:缺少环境 ID")?;
    // The preceding core inventory request validates the UUID and resolves the
    // exact registered environment. This projection adds no independent ID rule.
    if environment.is_empty() || environment.len() > 4096 {
        return Err("invalid_request:环境 ID 无效".into());
    }
    let path = payload["path"]
        .as_str()
        .filter(|path| {
            !path.is_empty() && path.len() <= 4096 && !path.chars().any(char::is_control)
        })
        .ok_or("invalid_request:需要有限的绝对原件路径")?;
    let path = Path::new(path);
    if !path.is_absolute()
        || path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    {
        return Err("invalid_request:需要准确的绝对原件路径".into());
    }
    let observed = inventory["locations"]
        .as_array()
        .into_iter()
        .flatten()
        .any(|location| {
            location["state"] == "present"
                && (location["path"].as_str() == path.to_str()
                    || location["entries"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .any(|entry| entry["path"].as_str() == path.to_str()))
        });
    let observed = observed
        || inventory["rows"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|row| row["path"].as_str() == path.to_str());
    if !observed {
        return Err("path_not_observed:此路径不在当前环境的有限清单；请刷新后重新选择".into());
    }
    for ancestor in path.ancestors() {
        let metadata =
            fs::symlink_metadata(ancestor).map_err(|_| "path_unreadable:原件路径已无法检查")?;
        if metadata.file_type().is_symlink() {
            return Err("symlink_target:原件路径已经变为符号链接；请刷新清单".into());
        }
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "path_unreadable:原件已不存在")?;
    if metadata.uid() != unsafe { libc::geteuid() }
        || !(metadata.is_file() || metadata.is_dir())
        || (metadata.is_file() && metadata.nlink() != 1)
    {
        return Err("unsupported_path:原件类型或归属已变化；未打开 Finder".into());
    }
    Ok(path.to_path_buf())
}

#[tauri::command]
pub async fn reveal_local_item(
    gate: tauri::State<'_, crate::activity::ActivityGate>,
    payload: Value,
) -> Result<Value, String> {
    let permit = gate
        .operation()
        .map_err(|_| "app_update_busy:App 正在安装更新，请稍后定位文件")?;
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (permit, payload);
        Err("platform_unsupported:此 App 定位操作只支持 macOS Finder；可复制目标主机路径".into())
    }
    #[cfg(target_os = "macos")]
    {
        tauri::async_runtime::spawn_blocking(move || {
            let _permit = permit;
            let inventory = lintel_core::handle_request(
                json!({"command":"residue_inventory","environment_id":payload["environment_id"],"project":payload["path"],"extra_roots":payload.get("extra_roots").cloned().unwrap_or(json!([]))}),
            );
            if inventory["ok"] != true {
                return Err(format!(
                    "observation_failed:{}",
                    inventory["error"]["message"]
                        .as_str()
                        .unwrap_or("无法刷新当前环境清单")
                ));
            }
            let path = selected_path(&payload, &inventory["data"])?;
            let status = std::process::Command::new("/usr/bin/open")
                .arg("-R")
                .arg(&path)
                .status()
                .map_err(|_| "reveal_failed:无法启动 Finder 定位")?;
            if !status.success() {
                return Err("reveal_failed:Finder 未完成定位".into());
            }
            Ok(json!({"status":"revealed","environment_id":payload["environment_id"],"path":path}))
        })
        .await
        .map_err(|_| "reveal_failed:定位请求未完成")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ID: &str = "11111111-2222-4333-8444-555555555555";

    #[test]
    fn only_exact_observed_items_can_be_revealed() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let selected = root.join("synthetic.log");
        let neighbor = root.join("neighbor.log");
        fs::write(&selected, b"synthetic").unwrap();
        fs::write(&neighbor, b"not-selected").unwrap();
        let inventory =
            json!({"locations":[{"path":root,"state":"present","entries":[{"path":selected}]}]});
        let mut request = json!({"environment_id":ID,"path":selected});
        assert_eq!(selected_path(&request, &inventory).unwrap(), selected);
        request["path"] = json!(neighbor);
        assert!(selected_path(&request, &inventory)
            .unwrap_err()
            .starts_with("path_not_observed:"));
        request["path"] = json!(selected);
        request["shell"] = json!("do-not-run");
        assert!(selected_path(&request, &inventory).is_err());
        request.as_object_mut().unwrap().remove("shell");
        request["environment_id"] = json!("");
        assert!(selected_path(&request, &inventory).is_err());
    }

    #[test]
    fn replaced_or_nonlocal_items_fail_before_any_opener() {
        use std::os::unix::fs::symlink;
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().canonicalize().unwrap();
        let original = root.join("synthetic.log");
        let target = root.join("other.log");
        fs::write(&target, b"synthetic").unwrap();
        symlink(&target, &original).unwrap();
        let inventory =
            json!({"locations":[{"path":root,"state":"present","entries":[{"path":original}]}]});
        let request = json!({"environment_id":ID,"path":original});
        assert!(selected_path(&request, &inventory)
            .unwrap_err()
            .starts_with("symlink_target:"));
        for path in [
            "https://example.invalid",
            "-R",
            "/tmp/../synthetic.log",
            "/tmp/synthetic\nlog",
        ] {
            assert!(selected_path(&json!({"environment_id":ID,"path":path}), &inventory).is_err());
        }
    }
}
