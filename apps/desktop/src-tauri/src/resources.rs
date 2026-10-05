//! User-clicked documentation links. The webview selects a resource, never an arbitrary URL.
fn resource_url(resource: &str) -> Option<String> {
    resource_url_at_revision(resource, env!("LINTEL_DOCS_REVISION"))
}

fn resource_url_at_revision(resource: &str, revision: &str) -> Option<String> {
    let table: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../contracts/documentation-resources.json"
    ))
    .ok()?;
    let url = table.get(resource)?.as_str()?;
    if url != "https://github.com/IndelibleVivi"
        && !url.starts_with("https://github.com/IndelibleVivi/")
    {
        return None;
    }
    let source = table.get("source")?.as_str()?;
    Some(if revision == "unknown" {
        url.into()
    } else {
        url.replace(
            &format!("{source}/blob/main/docs/"),
            &format!("{source}/blob/{revision}/docs/"),
        )
    })
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
    fn revision_pins_only_this_products_documentation() {
        let revision = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(resource_url_at_revision("policy-guide", revision).unwrap(),
            format!("https://github.com/IndelibleVivi/lintel-cc/blob/{revision}/docs/operator-guide.md#protect"));
        assert_eq!(
            resource_url_at_revision("operator-guide", "unknown").unwrap(),
            "https://github.com/IndelibleVivi/lintel-cc/blob/main/docs/operator-guide.md"
        );
        for (resource, file) in [
            ("vps-basics", "01-vps-basics.md"),
            ("ssh-troubleshooting", "08-troubleshooting.md"),
        ] {
            assert_eq!(
                resource_url_at_revision(resource, revision).unwrap(),
                format!("https://github.com/IndelibleVivi/infra-field-guide/blob/main/docs/{file}")
            );
        }
    }
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
