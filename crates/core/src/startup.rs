//! Finite static startup candidates. These observations never attest what the
//! client actually loads, authenticates with, or enforces after it starts.
use crate::{err, id, storage::*, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

const REVISION: u32 = 1;
const MAX_ANCESTORS: usize = 32;
const MAX_DROP_INS: usize = 64;
const FILE_BYTES: u64 = 1024 * 1024;
const TOTAL_BYTES: u64 = 16 * 1024 * 1024;
const AUTH_SELECTORS: &[&str] = &[
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
    "ANTHROPIC_PROFILE",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
];

pub(crate) struct StartupSnapshot {
    pub(crate) view: Value,
    pub(crate) binding: Value,
}

fn identity(m: &fs::Metadata) -> Value {
    json!({"device":m.dev(),"inode":m.ino(),"owner":m.uid(),"mode":m.mode(),
        "bytes":m.len(),"mtime":m.mtime(),"mtime_nsec":m.mtime_nsec(),
        "ctime":m.ctime(),"ctime_nsec":m.ctime_nsec()})
}

struct Observer<'a> {
    salt: &'a str,
    bytes_left: u64,
    sources: Vec<Value>,
    bindings: Vec<Value>,
    seen: BTreeSet<PathBuf>,
}

impl Observer<'_> {
    fn observe(&mut self, path: PathBuf, scope: &str, json_body: bool) {
        if !self.seen.insert(path.clone()) {
            return;
        }
        let mut fact = json!({"path":path,"scope":scope});
        let mut binding = json!({"path":path,"scope":scope});
        let inspected = (|| -> Result<()> {
            guard(&path)?;
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    fact["state"] = json!("not_found");
                    return Ok(());
                }
                Err(error) => return Err(error.into()),
            };
            // Credential/global-state/instruction bodies are never read here.
            // Directories only identify an entry; descendants remain unknown.
            binding["identity"] = identity(&meta);
            fact["state"] = json!(if meta.is_file() {
                "observed"
            } else if meta.is_dir() {
                "observed_directory"
            } else {
                "unsupported_type"
            });
            if !json_body || !meta.is_file() {
                return Ok(());
            }
            if meta.len() > FILE_BYTES || meta.len() > self.bytes_left {
                fact["content_state"] = json!("limit");
                return Ok(());
            }
            let mut file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
                .open(&path)?;
            if identity(&file.metadata()?) != identity(&meta) {
                return Err(err("source_changed", "读取时配置来源变化"));
            }
            let mut bytes = vec![];
            (&mut file)
                .take(FILE_BYTES.min(self.bytes_left) + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() as u64 > FILE_BYTES.min(self.bytes_left)
                || identity(&file.metadata()?) != identity(&meta)
                || identity(&fs::symlink_metadata(&path)?) != identity(&meta)
            {
                return Err(err("source_changed", "读取时配置来源变化"));
            }
            self.bytes_left -= bytes.len() as u64;
            // Kept only inside the private frozen plan. Salt avoids exposing a
            // reusable digest of settings that might contain a credential.
            let mut salted = self.salt.as_bytes().to_vec();
            salted.extend_from_slice(&bytes);
            binding["content_digest"] = json!(digest(&salted));
            match parse(&bytes) {
                Ok(doc) if doc.is_object() => {
                    fact["content_state"] = json!("observed");
                    for (key, flag) in [
                        ("hooks", "hooks_declared"),
                        ("mcpServers", "mcp_declared"),
                        ("apiKeyHelper", "api_key_helper_declared"),
                        ("policyHelper", "policy_helper_declared"),
                        ("enabledPlugins", "plugins_declared"),
                    ] {
                        fact[flag] = json!(doc.get(key).is_some());
                    }
                    let selectors: Vec<_> = AUTH_SELECTORS
                        .iter()
                        .filter(|name| doc["env"].get(**name).is_some())
                        .copied()
                        .collect();
                    fact["auth_selectors_declared"] = json!(selectors);
                }
                _ => fact["content_state"] = json!("unknown"),
            }
            Ok(())
        })();
        if inspected.is_err() {
            fact["state"] = json!("access_limited");
            fact["content_state"] = json!("unknown");
            binding.as_object_mut().unwrap().remove("content_digest");
        }
        binding["state"] = fact["state"].clone();
        binding["content_state"] = fact["content_state"].clone();
        self.sources.push(fact);
        self.bindings.push(binding);
    }
}

fn synthetic_boundary(home: &Path, root: &Path, project: Option<&Path>) -> Result<Option<PathBuf>> {
    let synthetic = cfg!(test)
        || std::env::var_os("LINTEL_TEST_HOME")
            .and_then(|p| PathBuf::from(p).canonicalize().ok())
            .is_some_and(|p| p == home);
    if !synthetic {
        return Ok(None);
    }
    // CLI journeys compile production code. The explicit test home must also
    // isolate readonly observations, including sibling home/project fixtures.
    let boundary = home
        .ancestors()
        .find(|p| root.starts_with(p) && project.is_none_or(|project| project.starts_with(p)))
        .ok_or_else(|| err("invalid_path", "合成启动来源需要共同的临时 fixture"))?;
    let temp = std::env::temp_dir().canonicalize()?;
    if boundary == temp || !boundary.starts_with(&temp) {
        return Err(err(
            "invalid_path",
            "合成启动来源必须限定在独立临时 fixture 内",
        ));
    }
    Ok(Some(boundary.to_path_buf()))
}

fn managed_directory(home: &Path, synthetic: bool) -> PathBuf {
    if synthetic {
        home.join("synthetic-managed")
    } else if cfg!(target_os = "macos") {
        PathBuf::from("/Library/Application Support/ClaudeCode")
    } else {
        PathBuf::from("/etc/claude-code")
    }
}

pub(crate) fn snapshot(
    home: &Path,
    root: &Path,
    project: Option<&Path>,
    salt: &str,
) -> Result<StartupSnapshot> {
    guard(root)?;
    if let Some(project) = project {
        guard(project)?;
    }
    let synthetic_boundary = synthetic_boundary(home, root, project)?;
    let mut observer = Observer {
        salt,
        bytes_left: TOTAL_BYTES,
        sources: vec![],
        bindings: vec![],
        seen: BTreeSet::new(),
    };
    for (name, scope, body) in [
        ("settings.json", "user", true),
        ("CLAUDE.md", "user_instructions", false),
        ("rules", "user_rules", false),
        ("agents", "user_agents", false),
        ("skills", "user_skills", false),
        ("managed-settings.json", "managed_candidate", true),
        (".credentials.json", "credential_metadata", false),
        (".claude.json", "config_global_state_metadata", false),
    ] {
        observer.observe(root.join(name), scope, body);
    }
    observer.observe(
        home.join(".claude.json"),
        "shared_global_state_metadata",
        false,
    );
    observer.observe(
        home.join(".config/anthropic"),
        "shared_profile_metadata",
        false,
    );
    if root != home.join(".claude") {
        observer.observe(
            home.join(".claude/managed-settings.json"),
            "managed_candidate",
            true,
        );
    }
    let mut ancestors_complete = true;
    if let Some(project) = project {
        for (index, cwd) in project.ancestors().enumerate() {
            if index >= MAX_ANCESTORS {
                ancestors_complete = false;
                break;
            }
            for (name, scope, body) in [
                (".claude/settings.json", "project", true),
                (".claude/settings.local.json", "project_local", true),
                (".mcp.json", "project_mcp", true),
                ("CLAUDE.md", "project_instructions", false),
                (".claude/CLAUDE.md", "project_instructions", false),
                ("CLAUDE.local.md", "project_local_instructions", false),
                ("AGENTS.md", "project_agents_instructions", false),
                (".claude/rules", "project_rules", false),
            ] {
                observer.observe(
                    cwd.join(name),
                    if index == 0 {
                        scope
                    } else {
                        "ancestor_candidate"
                    },
                    body,
                );
            }
            if synthetic_boundary.as_deref() == Some(cwd) {
                break;
            }
        }
    }
    let managed = managed_directory(home, synthetic_boundary.is_some());
    for (name, scope, body) in [
        ("managed-settings.json", "managed_file", true),
        ("managed-mcp.json", "managed_mcp", true),
        ("CLAUDE.md", "managed_instructions", false),
    ] {
        observer.observe(managed.join(name), scope, body);
    }
    let drop_dir = managed.join("managed-settings.d");
    observer.observe(drop_dir.clone(), "managed_drop_in_directory", false);
    let mut drops_complete = true;
    let drops = (|| -> Result<Vec<PathBuf>> {
        guard(&drop_dir)?;
        let entries = match fs::read_dir(&drop_dir) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(error) => return Err(error.into()),
        };
        let mut paths = vec![];
        for (index, entry) in entries.enumerate() {
            if index >= MAX_DROP_INS {
                drops_complete = false;
                break;
            }
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|n| !n.starts_with('.') && n.ends_with(".json"))
            {
                paths.push(entry.path());
            }
        }
        paths.sort();
        Ok(paths)
    })();
    match drops {
        Ok(paths) => {
            for path in paths {
                observer.observe(path, "managed_drop_in", true);
            }
        }
        Err(_) => drops_complete = false,
    }
    let limited = observer.sources.iter().any(|f| {
        f["state"] == "access_limited"
            || f["state"] == "unsupported_type"
            || f["content_state"] == "limit"
            || f["content_state"] == "unknown"
    });
    let auth_variables: Vec<_> = AUTH_SELECTORS
        .iter()
        .map(|name| json!({"name":name,"present":std::env::var_os(name).is_some()}))
        .collect();
    let view = json!({"schema":"lintel.startup/1","config_root":root,"project_cwd":project,
        "observation_context":if synthetic_boundary.is_some() {"synthetic"} else {"host"},
        "sources":observer.sources,"candidate_scan_complete":ancestors_complete && drops_complete,
        "content_limited":limited,"actual_loaded":"unverified",
        "authentication":{"state":"unverified","observation":"metadata_only",
            "runner_environment":auth_variables,"terminal_environment":"unverified",
            "keychain":"not_checked","account":"not_checked"},
        "unknown_sources":["组织服务器／MDM 与 embedding host 策略","Git worktree 主 checkout、指令 imports、rules／skills／agents 的目录内容","启动 Terminal 的 shell 环境、认证优先级与运行中的外部写入者"],
        "next_steps":["确认下列候选来源仍适合这次项目；新 root 不会移除项目或 managed 配置。",
            "在打开的终端中核对 /status 的配置来源与 Login，并用 /mcp 核对连接。",
            "需要登录时在目标终端显式登录；Lintel 不会代为登录或自动发送上下文。"],
        "note":"只观察有限路径和声明，不执行 hooks、MCP、credential helper 或 Claude；文件存在不证明已加载，权限规则不证明 OS 隔离。"});
    let binding = json!({"revision":REVISION,"salt":salt,"sources":observer.bindings,
        "synthetic_boundary":synthetic_boundary,
        "candidate_scan_complete":ancestors_complete && drops_complete});
    Ok(StartupSnapshot { view, binding })
}

pub(crate) fn freeze(home: &Path, root: &Path, project: &Path) -> Result<StartupSnapshot> {
    snapshot(home, root, Some(project), &id())
}

pub(crate) fn recheck(home: &Path, root: &Path, project: &Path, binding: &Value) -> Result<()> {
    if binding["revision"] != REVISION || binding["salt"].as_str().is_none() {
        return Err(err(
            "stale_plan",
            "启动来源核对规则已更新，请重新预览；原启动记录仍只查询。",
        ));
    }
    let current = snapshot(home, root, Some(project), binding["salt"].as_str().unwrap())?;
    if &current.binding != binding {
        return Err(err(
            "stale_plan",
            "预览后配置候选来源发生变化，请重新核对启动目标。",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_observations_include_sibling_project_but_never_escape_fixture() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tmp.path().canonicalize().unwrap();
        let fixture = outside.join("fixture");
        let home = fixture.join("home");
        let root = home.join("root");
        let project = fixture.join("project");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&project).unwrap();
        fs::write(outside.join(".mcp.json"), "{}").unwrap();
        fs::write(fixture.join(".mcp.json"), "{}").unwrap();
        let snap = freeze(&home, &root, &project).unwrap();
        assert_eq!(snap.view["observation_context"], "synthetic");
        for source in snap.view["sources"].as_array().unwrap() {
            let path = Path::new(source["path"].as_str().unwrap());
            assert!(path.starts_with(&fixture), "escaped fixture: {path:?}");
        }
        assert!(snap.view["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["path"] == fixture.join(".mcp.json").to_str().unwrap()));
        assert!(freeze(&home, &root, &outside).is_ok());
        assert!(freeze(&home, &root, Path::new("/")).is_err());
    }
    #[test]
    fn finite_sources_are_sanitized_and_frozen_without_credentials_or_execution() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        let root = home.join("new-root");
        let project = home.join("old/project");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(project.join(".claude")).unwrap();
        fs::create_dir_all(home.join("synthetic-managed/managed-settings.d")).unwrap();
        fs::write(project.join(".claude/settings.json"),r#"{"hooks":{"x":"PRIVATE_COMMAND"},"env":{"ANTHROPIC_API_KEY":"PRIVATE_KEY"},"apiKeyHelper":"PRIVATE_HELPER"}"#).unwrap();
        // Malformed credential/global state bodies must remain unread.
        fs::write(root.join(".credentials.json"), "PRIVATE_CREDENTIAL").unwrap();
        fs::write(home.join(".claude.json"), "PRIVATE_GLOBAL_STATE").unwrap();
        fs::write(home.join("old/CLAUDE.md"), "PRIVATE_INSTRUCTIONS").unwrap();
        fs::write(
            home.join("synthetic-managed/managed-settings.d/10-policy.json"),
            r#"{"mcpServers":{"x":{"command":"PRIVATE_MCP"}}}"#,
        )
        .unwrap();
        let snap = freeze(&home, &root, &project).unwrap();
        let text = snap.view.to_string();
        for secret in [
            "PRIVATE_COMMAND",
            "PRIVATE_KEY",
            "PRIVATE_HELPER",
            "PRIVATE_CREDENTIAL",
            "PRIVATE_GLOBAL_STATE",
            "PRIVATE_INSTRUCTIONS",
            "PRIVATE_MCP",
        ] {
            assert!(!text.contains(secret));
            assert!(!snap.binding.to_string().contains(secret));
        }
        assert!(snap.view["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["scope"] == "managed_drop_in" && f["mcp_declared"] == true));
        assert!(snap.view["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["scope"] == "ancestor_candidate"
                && f["path"] == home.join("old/CLAUDE.md").to_str().unwrap()));
        recheck(&home, &root, &project, &snap.binding).unwrap();
        fs::write(project.join("ordinary.txt"), "ordinary project change").unwrap();
        recheck(&home, &root, &project, &snap.binding).unwrap();
        fs::write(project.join(".mcp.json"), "{}").unwrap();
        assert_eq!(
            recheck(&home, &root, &project, &snap.binding)
                .unwrap_err()
                .code,
            "stale_plan"
        );
        let snap = freeze(&home, &root, &project).unwrap();
        let settings = project.join(".claude/settings.json");
        let replacement = project.join(".claude/replacement.json");
        fs::write(&replacement, fs::read(&settings).unwrap()).unwrap();
        fs::rename(&replacement, &settings).unwrap();
        assert_eq!(
            recheck(&home, &root, &project, &snap.binding)
                .unwrap_err()
                .code,
            "stale_plan"
        );
    }
    #[test]
    fn unreadable_fifo_symlink_and_large_sources_are_explicit_and_new_plans_need_binding() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path().canonicalize().unwrap();
        let root = home.join("root");
        let project = home.join("project");
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&project).unwrap();
        std::os::unix::fs::symlink(home.join("outside"), root.join("settings.json")).unwrap();
        let snap = freeze(&home, &root, &project).unwrap();
        assert_eq!(snap.view["sources"][0]["state"], "access_limited");
        assert_eq!(snap.view["content_limited"], true);
        assert_eq!(
            recheck(&home, &root, &project, &Value::Null)
                .unwrap_err()
                .code,
            "stale_plan"
        );
        let fifo = project.join(".mcp.json");
        let name = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        let large = project.join(".claude/settings.json");
        fs::create_dir(large.parent().unwrap()).unwrap();
        fs::File::create(&large)
            .unwrap()
            .set_len(FILE_BYTES + 1)
            .unwrap();
        let snap = freeze(&home, &root, &project).unwrap();
        assert!(snap.view["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["path"] == fifo.to_str().unwrap() && s["state"] == "unsupported_type"));
        assert!(snap.view["sources"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["path"] == large.to_str().unwrap() && s["content_state"] == "limit"));
        fs::create_dir_all(home.join("synthetic-managed/managed-settings.d")).unwrap();
        let plan = freeze(&home, &root, &project).unwrap();
        fs::write(
            home.join("synthetic-managed/managed-settings.d/new.json"),
            "{}",
        )
        .unwrap();
        assert_eq!(
            recheck(&home, &root, &project, &plan.binding)
                .unwrap_err()
                .code,
            "stale_plan"
        );
    }
}
