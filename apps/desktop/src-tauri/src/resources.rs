//! User-clicked documentation links. The webview selects a resource, never an arbitrary URL.
fn resource_url(resource: &str) -> Option<&'static str> {
    match resource {
        "developer" => Some("https://github.com/IndelibleVivi"),
        "source" => Some("https://github.com/IndelibleVivi/lintel-cc"),
        "operator-guide" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md"),
        "policy-guide" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#protect"),
        "work-guide" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#work"),
        "cleanup-guide" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#cleanup"),
        "recovery-guide" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md#recovery"),
        "agent-guide" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/agents.md"),
        "remote-setup" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/remote.md"),
        "browser-setup" => Some("https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/browser.md"),
        "vps-guide" => Some("https://github.com/IndelibleVivi/infra-field-guide"),
        "vps-basics" => Some("https://github.com/IndelibleVivi/infra-field-guide/blob/main/docs/01-vps-basics.md"),
        "ssh-troubleshooting" => Some("https://github.com/IndelibleVivi/infra-field-guide/blob/main/docs/08-troubleshooting.md"),
        _ => None,
    }
}

#[tauri::command]
pub async fn open_resource(resource: String) -> Result<(), String> {
    let url = resource_url(&resource).ok_or("未知的帮助资源")?;
    #[cfg(target_os = "macos")]
    {
        let status = tauri::async_runtime::spawn_blocking(move || {
            std::process::Command::new("/usr/bin/open")
                .arg(url)
                .status()
        })
        .await
        .map_err(|_| "无法打开系统浏览器")?
        .map_err(|_| "无法打开系统浏览器")?;
        if status.success() {
            Ok(())
        } else {
            Err("系统浏览器未能打开链接".into())
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = url;
        Err("此平台请复制帮助链接到浏览器打开".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_named_https_resources_can_open() {
        for name in [
            "developer",
            "source",
            "operator-guide",
            "policy-guide",
            "work-guide",
            "cleanup-guide",
            "recovery-guide",
            "agent-guide",
            "remote-setup",
            "browser-setup",
            "vps-guide",
            "vps-basics",
            "ssh-troubleshooting",
        ] {
            assert!(resource_url(name)
                .unwrap()
                .starts_with("https://github.com/IndelibleVivi"));
        }
        for value in [
            "https://example.invalid",
            "file:///tmp/synthetic",
            "javascript:alert(1)",
            "--args",
            "",
        ] {
            assert!(resource_url(value).is_none());
        }
    }
}
