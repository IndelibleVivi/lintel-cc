//! Bounded authentication provenance, without exporting credential values.
//!
//! Inventory describes candidates, not Claude's effective precedence. The only
//! mutable sources are exact known fields and complete literal shell assignments.
//! This module reads/transforms; cleanup.rs owns approval, publication and jobs.
use crate::{components::startup, err, keychain, storage, Result};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::OsString,
    fs::{self, OpenOptions},
    io::Read,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
};

pub(crate) const FILE_BYTES: u64 = 1024 * 1024;
const SHELL_BYTES: u64 = 256 * 1024;
const TOTAL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SOURCES: usize = 256;
const MAX_ASSIGNMENTS: usize = 128;
const SCHEMA: &str = "lintel.auth-sources/1";
const FROZEN_SCHEMA: &str = "lintel.auth-selection/1";

// Selectors are inventoried separately from credential-bearing assignments.
// Removing a provider/profile switch is not part of removing its credentials.
const REMOVABLE_NAMES: &[&str] = &[
    "CLAUDE_CODE_OAUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_REFRESH_TOKEN",
    "CLAUDE_CODE_OAUTH_SCOPES",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_IDENTITY_TOKEN",
];
const SELECTOR_NAMES: &[&str] = &[
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_SECURESTORAGE_CONFIG_DIR",
    "ANTHROPIC_PROFILE",
    "ANTHROPIC_CONFIG_DIR",
    "ANTHROPIC_BASE_URL",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_OAUTH_TOKEN_FILE_DESCRIPTOR",
    "CLAUDE_CODE_API_KEY_FILE_DESCRIPTOR",
    "ANTHROPIC_IDENTITY_TOKEN_FILE",
    "ANTHROPIC_FEDERATION_RULE_ID",
    "ANTHROPIC_ORGANIZATION_ID",
    "ANTHROPIC_SERVICE_ACCOUNT_ID",
    "ANTHROPIC_WORKSPACE_ID",
    "ZDOTDIR",
    "XDG_CONFIG_HOME",
    "BASH_ENV",
    "ENV",
];
const SHELL_FILES: &[&str] = &[
    ".zshenv",
    ".zprofile",
    ".zshrc",
    ".zlogin",
    ".bash_profile",
    ".bash_login",
    ".bashrc",
    ".profile",
    ".config/fish/config.fish",
];

fn names() -> impl Iterator<Item = &'static str> {
    REMOVABLE_NAMES.iter().chain(SELECTOR_NAMES).copied()
}

fn source_id(kind: &str, path: Option<&Path>, locator: &str) -> String {
    // Structural identifiers never hash or encode credential values.
    let structural = json!([kind, path, locator]);
    format!(
        "auth-{}",
        &storage::digest(&serde_json::to_vec(&structural).unwrap())[..24]
    )
}

fn source(
    kind: &str,
    name: &str,
    path: Option<&Path>,
    locator: &str,
    scope: &str,
    evidence: &str,
    presence: &str,
    removable: bool,
    reason: &str,
) -> Value {
    let mut item = json!({
        "id":source_id(kind,path,locator),"kind":kind,"name":name,
        "locator":locator,"scope":scope,"evidence":evidence,
        "presence":presence,"removable":removable,"reason":reason,
        "actual_use":"unverified",
        "default_selected":removable && matches!(kind,"credential_field"|"client_state_field"),
    });
    if let Some(path) = path {
        item["path"] = json!(path);
    }
    item
}

fn identity(meta: &fs::Metadata) -> Value {
    // Rename changes ctime. Retain mtime and identity for quarantine checks,
    // while read_stable also checks ctime before/after its same-handle read.
    json!({"device":meta.dev(),"inode":meta.ino(),"owner":meta.uid(),
        "mode":meta.mode(),"bytes":meta.len(),"mtime":meta.mtime(),
        "mtime_nsec":meta.mtime_nsec(),"links":meta.nlink()})
}

fn parent_identity(path: &Path) -> Result<Value> {
    let parent = path
        .parent()
        .ok_or_else(|| err("invalid_path", "认证来源缺少父目录"))?;
    storage::guard(parent)?;
    let meta = fs::symlink_metadata(parent)?;
    if !meta.is_dir() {
        return Err(err("unsafe_auth_source", "认证来源父目录不可访问"));
    }
    Ok(json!({"device":meta.dev(),"inode":meta.ino(),"owner":meta.uid()}))
}

pub(crate) fn check_parent(group: &Value) -> Result<()> {
    if group["storage"] == "keychain" {
        return Ok(());
    }
    let path = Path::new(
        group["path"]
            .as_str()
            .ok_or_else(|| err("invalid_auth_selection", "认证来源缺少路径"))?,
    );
    if parent_identity(path)? != group["parent_identity"] {
        return Err(err("stale_auth_source", "认证来源父目录改变，请重新预览"));
    }
    Ok(())
}

fn full_identity(meta: &fs::Metadata) -> Value {
    let mut result = identity(meta);
    result["ctime"] = json!(meta.ctime());
    result["ctime_nsec"] = json!(meta.ctime_nsec());
    result
}

struct FileRead {
    bytes: Vec<u8>,
    identity: Value,
    editable: bool,
    parent_identity: Value,
}

fn read_stable(path: &Path, limit: u64) -> Result<Option<FileRead>> {
    storage::guard(path)?;
    let before = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => {
            return Err(err(
                "auth_source_unreadable",
                "无法读取认证来源；未将其视为空白",
            ))
        }
    };
    let parent_before = parent_identity(path)?;
    if !before.is_file() || before.len() > limit {
        return Err(err(
            "auth_source_limit",
            "认证来源类型或大小超出有界检查范围",
        ));
    }
    // Explicit mode check also makes synthetic permission tests deterministic
    // under privileged CI without reading a file the ordinary owner cannot read.
    if before.mode() & 0o444 == 0 {
        return Err(err(
            "auth_source_unreadable",
            "认证来源不可读；未将其视为空白",
        ));
    }
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(|_| err("auth_source_unreadable", "无法打开认证来源；未将其视为空白"))?;
    let opened = file.metadata()?;
    if !opened.is_file() || full_identity(&opened) != full_identity(&before) {
        return Err(err("stale_auth_source", "读取期间认证来源改变，请重新检查"));
    }
    let mut bytes = Vec::new();
    (&mut file).take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit
        || full_identity(&file.metadata()?) != full_identity(&before)
        || full_identity(&fs::symlink_metadata(path)?) != full_identity(&before)
        || parent_identity(path)? != parent_before
    {
        return Err(err("stale_auth_source", "读取期间认证来源改变，请重新检查"));
    }
    Ok(Some(FileRead {
        bytes,
        identity: identity(&before),
        editable: before.uid() == unsafe { libc::geteuid() } && before.nlink() == 1,
        parent_identity: parent_before,
    }))
}

fn salted_snapshot(read: &FileRead, nonce: &str) -> Value {
    let mut value = read.identity.clone();
    let mut bytes = b"lintel.auth-source/1\0".to_vec();
    bytes.extend_from_slice(nonce.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&read.bytes);
    value["digest"] = json!(storage::digest(&bytes));
    value
}

/// The only content fingerprint is nonce-bound and remains in private plans.
pub(crate) fn snapshot(path: &Path, nonce: &str) -> Result<Value> {
    let Some(read) = read_stable(path, FILE_BYTES)? else {
        return Ok(Value::Null);
    };
    if !read.editable {
        return Err(err(
            "unsafe_auth_source",
            "认证来源所有者或硬链接状态不支持精确修改",
        ));
    }
    Ok(salted_snapshot(&read, nonce))
}

pub(crate) fn matches_snapshot(path: &Path, expected: &Value, nonce: &str) -> Result<bool> {
    Ok(snapshot(path, nonce)? == *expected)
}

struct Candidate {
    path: PathBuf,
    kind: &'static str,
    locator: Value,
}

struct Scan {
    sources: Vec<Value>,
    candidates: BTreeMap<String, Candidate>,
    files: BTreeMap<PathBuf, FileRead>,
    keychains: BTreeMap<String, (String, keychain::Item)>,
    seen: BTreeSet<PathBuf>,
    remaining: u64,
    complete: bool,
    keychain_complete: bool,
}

impl Scan {
    fn new() -> Self {
        Self {
            sources: vec![],
            candidates: BTreeMap::new(),
            files: BTreeMap::new(),
            keychains: BTreeMap::new(),
            seen: BTreeSet::new(),
            remaining: TOTAL_BYTES,
            complete: true,
            keychain_complete: true,
        }
    }

    fn push(&mut self, mut item: Value, candidate: Option<Candidate>) {
        if self.sources.len() >= MAX_SOURCES {
            self.complete = false;
            return;
        }
        if let Some(candidate) = candidate {
            self.candidates
                .insert(item["id"].as_str().unwrap().to_owned(), candidate);
        } else {
            item["removable"] = json!(false);
            item["default_selected"] = json!(false);
        }
        self.sources.push(item);
    }

    fn keychain(&mut self, home: &Path, root: &Path) {
        for service in keychain::services(home, root) {
            let result = keychain::read(home, &service);
            let mut item = source(
                "keychain",
                "Keychain",
                None,
                "/claudeAiOauth",
                "selected_root",
                "unknown",
                "unknown",
                false,
                "keychain_unavailable",
            );
            item["id"] = json!(source_id("keychain", Some(root), &service));
            item["service"] = json!(service);
            match result {
                Ok(None) => {
                    item["evidence"] = json!("not_found");
                    item["presence"] = json!("absent");
                    item["reason"] = json!("not_found");
                }
                Ok(Some(read)) if read.bytes.len() as u64 <= FILE_BYTES.min(self.remaining) => {
                    self.remaining -= read.bytes.len() as u64;
                    match storage::parse(&read.bytes).filter_object() {
                        Ok(doc) => {
                            item["evidence"] = json!("confirmed_present");
                            item["presence"] = json!(if doc.get("claudeAiOauth").is_some() {
                                "present"
                            } else {
                                "absent"
                            });
                            item["reason"] = json!("not_found");
                            if let Some(value) = doc.get("claudeAiOauth") {
                                let recognized = value.is_object()
                                    && ["accessToken", "refreshToken"]
                                        .iter()
                                        .any(|key| value[*key].is_string());
                                item["kind"] = json!("keychain_field");
                                item["name"] = json!("claudeAiOauth");
                                item["removable"] = json!(recognized);
                                item["default_selected"] = json!(recognized);
                                item["reason"] = json!(if recognized {
                                    "known_claude_login_field"
                                } else {
                                    "unrecognized_format"
                                });
                                if recognized {
                                    if self.sources.len() < MAX_SOURCES {
                                        self.keychains.insert(
                                            item["id"].as_str().unwrap().to_owned(),
                                            (service.clone(), read),
                                        );
                                        self.sources.push(item);
                                    } else {
                                        self.keychain_complete = false;
                                    }
                                    continue;
                                }
                            }
                        }
                        Err(_) => {
                            item["reason"] = json!("keychain_invalid_format");
                            self.keychain_complete = false;
                        }
                    }
                }
                Ok(Some(_)) => {
                    item["reason"] = json!("keychain_too_large");
                    self.keychain_complete = false;
                }
                Err(error) => {
                    item["reason"] = json!(error.code);
                    self.keychain_complete = false;
                }
            }
            self.push(item, None);
        }
    }

    fn read_file(&mut self, path: &Path, kind: &str, scope: &str, limit: u64) -> Option<FileRead> {
        if !self.seen.insert(path.to_path_buf()) {
            return None;
        }
        let result = read_stable(path, limit.min(self.remaining));
        match result {
            Ok(Some(read)) => {
                self.remaining -= read.bytes.len() as u64;
                Some(read)
            }
            Ok(None) => {
                self.push(
                    source(
                        kind,
                        "source_file",
                        Some(path),
                        "",
                        scope,
                        "not_found",
                        "absent",
                        false,
                        "not_found",
                    ),
                    None,
                );
                None
            }
            Err(error) => {
                self.complete = false;
                self.push(
                    source(
                        kind,
                        "source_file",
                        Some(path),
                        "",
                        scope,
                        "inaccessible",
                        "unknown",
                        false,
                        if error.code == "auth_source_limit" {
                            "file_too_large"
                        } else {
                            "unreadable"
                        },
                    ),
                    None,
                );
                None
            }
        }
    }

    fn json_file(&mut self, path: PathBuf, purpose: &'static str, scope: &str, writable: bool) {
        let file_kind = if purpose == "credentials" {
            "credential_file"
        } else {
            "settings_source"
        };
        let Some(read) = self.read_file(&path, file_kind, scope, FILE_BYTES) else {
            return;
        };
        let document = storage::parse(&read.bytes);
        let Ok(document) = document.filter_object() else {
            self.complete = false;
            self.push(
                source(
                    file_kind,
                    "source_file",
                    Some(&path),
                    "",
                    scope,
                    "confirmed_present",
                    "present",
                    false,
                    "unrecognized_format",
                ),
                None,
            );
            return;
        };
        let editable = writable && read.editable;
        let count_before = self.sources.len();
        if purpose == "credentials" {
            if let Some(value) = document.get("claudeAiOauth") {
                let recognized = value.is_object()
                    && ["accessToken", "refreshToken"]
                        .iter()
                        .any(|key| value[*key].is_string());
                let mutable = editable && recognized;
                let item = source(
                    "credential_field",
                    "claudeAiOauth",
                    Some(&path),
                    "/claudeAiOauth",
                    scope,
                    "confirmed_present",
                    "present",
                    mutable,
                    if recognized {
                        "known_claude_login_field"
                    } else {
                        "unrecognized_format"
                    },
                );
                let candidate = mutable.then(|| Candidate {
                    path: path.clone(),
                    kind: "json",
                    locator: json!({"pointer":"/claudeAiOauth"}),
                });
                self.push(item, candidate);
            }
        } else if purpose == "client_state" {
            if let Some(value) = document.get("primaryApiKey") {
                let mutable = editable && value.is_string();
                self.push(
                    source(
                        "client_state_field",
                        "primaryApiKey",
                        Some(&path),
                        "/primaryApiKey",
                        scope,
                        "confirmed_present",
                        "present",
                        mutable,
                        if value.is_string() {
                            "known_claude_login_field"
                        } else {
                            "non_string_value"
                        },
                    ),
                    mutable.then(|| Candidate {
                        path: path.clone(),
                        kind: "json",
                        locator: json!({"pointer":"/primaryApiKey"}),
                    }),
                );
            }
        } else {
            if document.get("env").is_some_and(|env| !env.is_object()) {
                self.complete = false;
                self.push(
                    source(
                        "settings_source",
                        "env",
                        Some(&path),
                        "/env",
                        scope,
                        "declared",
                        "present",
                        false,
                        "unrecognized_format",
                    ),
                    None,
                );
            }
            for name in names() {
                if let Some(value) = document["env"].get(name) {
                    let removable_name = REMOVABLE_NAMES.contains(&name);
                    let mutable = editable && removable_name && value.is_string();
                    let pointer = format!("/env/{name}");
                    let reason = if scope == "project" && name.starts_with("CLAUDE_CODE_OAUTH_") {
                        "project_token_ignored"
                    } else if !removable_name {
                        "provider_or_profile_selector"
                    } else if !value.is_string() {
                        "non_string_value"
                    } else {
                        "settings_declaration"
                    };
                    let mut item = source(
                        "settings_env",
                        name,
                        Some(&path),
                        &pointer,
                        scope,
                        "declared",
                        "present",
                        mutable,
                        reason,
                    );
                    item["value_state"] = json!(match value.as_str() {
                        Some("") => "empty",
                        Some(_) => "nonempty",
                        None => "non_string",
                    });
                    self.push(
                        item,
                        mutable.then(|| Candidate {
                            path: path.clone(),
                            kind: "json",
                            locator: json!({"pointer":pointer}),
                        }),
                    );
                }
            }
            if let Some(value) = document.get("apiKeyHelper") {
                let mutable = editable && value.is_string();
                self.push(
                    source(
                        "api_key_helper",
                        "apiKeyHelper",
                        Some(&path),
                        "/apiKeyHelper",
                        scope,
                        "declared",
                        "present",
                        mutable,
                        "helper_not_executed",
                    ),
                    mutable.then(|| Candidate {
                        path: path.clone(),
                        kind: "json",
                        locator: json!({"pointer":"/apiKeyHelper"}),
                    }),
                );
            }
        }
        if count_before == self.sources.len() {
            self.push(
                source(
                    file_kind,
                    "source_file",
                    Some(&path),
                    "",
                    scope,
                    "confirmed_present",
                    "present",
                    false,
                    if purpose == "credentials" {
                        "unrecognized_format"
                    } else {
                        "no_auth_declaration"
                    },
                ),
                None,
            );
        }
        self.files.insert(path, read);
    }

    fn shell_file(&mut self, path: PathBuf, fish: bool) {
        let Some(read) = self.read_file(&path, "shell_file", "host_shared", SHELL_BYTES) else {
            return;
        };
        let Ok(text) = std::str::from_utf8(&read.bytes) else {
            self.complete = false;
            self.push(
                source(
                    "shell_file",
                    "source_file",
                    Some(&path),
                    "",
                    "host_shared",
                    "confirmed_present",
                    "present",
                    false,
                    "unrecognized_format",
                ),
                None,
            );
            return;
        };
        let parsed = shell_declarations(text);
        let simple = !fish && parsed.simple;
        let count_before = self.sources.len();
        for (name, declaration) in parsed.declarations {
            let mutable = simple
                && read.editable
                && declaration.exact
                && !declaration.lines.is_empty()
                && REMOVABLE_NAMES.contains(&name.as_str());
            let mut item = source(
                "shell_assignment",
                &name,
                Some(&path),
                &name,
                "host_shared",
                if declaration.declared {
                    "declared"
                } else {
                    "unknown"
                },
                "present",
                mutable,
                if !REMOVABLE_NAMES.contains(&name.as_str()) {
                    "provider_or_profile_selector"
                } else if mutable {
                    "literal_assignment"
                } else {
                    "dynamic_or_complex_shell"
                },
            );
            item["locations"] = json!(declaration
                .lines
                .iter()
                .map(|line| json!({"line":line}))
                .collect::<Vec<_>>());
            let candidate = mutable.then(|| Candidate {
                path: path.clone(),
                kind: "shell",
                locator: json!({"name":name,"lines":declaration.lines}),
            });
            self.push(item, candidate);
        }
        if count_before == self.sources.len() {
            self.push(
                source(
                    "shell_file",
                    "source_file",
                    Some(&path),
                    "",
                    "host_shared",
                    "confirmed_present",
                    "present",
                    false,
                    if simple {
                        "no_auth_declaration"
                    } else {
                        "dynamic_or_complex_shell"
                    },
                ),
                None,
            );
        }
        self.files.insert(path, read);
    }

    fn metadata(&mut self, path: PathBuf, name: &str, locator: &str) {
        let (evidence, presence, reason) =
            match storage::guard(&path).and_then(|_| match fs::symlink_metadata(&path) {
                Ok(meta) if meta.is_dir() || meta.is_file() => Ok(true),
                Ok(_) => Err(err("auth_source_unreadable", "不支持的认证来源类型")),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
                Err(_) => Err(err("auth_source_unreadable", "认证来源元数据不可访问")),
            }) {
                Ok(true) => ("confirmed_present", "present", "profile_shared"),
                Ok(false) => ("not_found", "absent", "not_found"),
                Err(_) => {
                    self.complete = false;
                    ("inaccessible", "unknown", "unreadable")
                }
            };
        self.push(
            source(
                "profile",
                name,
                Some(&path),
                locator,
                "host_shared",
                evidence,
                presence,
                false,
                reason,
            ),
            None,
        );
    }
}

trait ObjectDocument {
    fn filter_object(self) -> Result<Value>;
}
impl ObjectDocument for Result<Value> {
    fn filter_object(self) -> Result<Value> {
        match self? {
            value if value.is_object() => Ok(value),
            _ => Err(err("invalid_json", "认证来源需要 JSON 对象")),
        }
    }
}

#[derive(Default)]
struct Declaration {
    lines: Vec<usize>,
    exact: bool,
    declared: bool,
}
struct ShellDeclarations {
    simple: bool,
    declarations: BTreeMap<String, Declaration>,
}

fn name_mentioned(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(index, _)| {
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        (index == 0 || !word(line.as_bytes()[index - 1]))
            && (index + name.len() == line.len() || !word(line.as_bytes()[index + name.len()]))
    })
}

/// Return the non-comment portion only when one physical line is a complete
/// simple shell line. This does not evaluate shell and deliberately refuses
/// heredocs, substitutions, blocks, continuations and multi-line quoting.
fn complete_shell_line(line: &str) -> Option<&str> {
    let line = line.trim_end_matches('\r');
    let mut single = false;
    let mut double = false;
    let mut escaped = false;
    let mut end = line.len();
    let mut previous = None;
    for (index, c) in line.char_indices() {
        if escaped {
            escaped = false;
            previous = Some(c);
            continue;
        }
        if single {
            if c == '\'' {
                single = false;
            }
            previous = Some(c);
            continue;
        }
        if c == '\\' {
            escaped = true;
            previous = Some(c);
            continue;
        }
        if c == '\'' && !double {
            single = true;
            previous = Some(c);
            continue;
        }
        if c == '"' {
            double = !double;
            previous = Some(c);
            continue;
        }
        if c == '`' || (c == '(' && previous == Some('$')) {
            return None;
        }
        if !double {
            if c == '#' && (index == 0 || previous.is_some_and(char::is_whitespace)) {
                end = index;
                break;
            }
            if matches!(c, '(' | ')' | '{' | '}' | '|' | '&' | '<' | '>' | '!') {
                return None;
            }
            if c == '<' && previous == Some('<') {
                return None;
            }
        }
        previous = Some(c);
    }
    if single || double || escaped {
        return None;
    }
    let code = line[..end].trim();
    // Shell reserved words can open constructs even with punctuation. Deny
    // them at every unquoted token boundary, not only at the start of a line.
    let tokens = unquoted_words(code);
    if tokens.iter().any(|word| {
        matches!(
            word.as_str(),
            "if" | "then"
                | "elif"
                | "else"
                | "fi"
                | "for"
                | "while"
                | "until"
                | "do"
                | "done"
                | "case"
                | "esac"
                | "select"
                | "function"
                | "coproc"
                | "time"
                | "repeat"
                | "foreach"
                | "end"
                | "switch"
        )
    }) {
        return None;
    }
    Some(code)
}

fn unquoted_words(code: &str) -> Vec<String> {
    let mut result = vec![];
    let mut word = String::new();
    let mut single = false;
    let mut double = false;
    let mut escape = false;
    for c in code.chars() {
        if escape {
            escape = false;
            continue;
        }
        if !single && c == '\\' {
            escape = true;
            continue;
        }
        if c == '\'' && !double {
            single = !single;
            continue;
        }
        if c == '"' && !single {
            double = !double;
            continue;
        }
        if single || double {
            continue;
        }
        if c.is_ascii_alphanumeric() || c == '_' {
            word.push(c);
        } else if !word.is_empty() {
            result.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        result.push(word);
    }
    result
}

fn literal_assignment(code: &str) -> Option<&str> {
    let code = code
        .strip_prefix("export ")
        .or_else(|| code.strip_prefix("export\t"))
        .map(str::trim_start)
        .unwrap_or(code);
    let (name, value) = code.split_once('=')?;
    if name.is_empty()
        || !name
            .bytes()
            .enumerate()
            .all(|(i, c)| c == b'_' || c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()))
    {
        return None;
    }
    // One literal shell word only. No second assignment, command, expansion,
    // concatenation, redirect or trailing semicolon is edited automatically.
    if value.starts_with('\'') {
        let inner = value.strip_prefix('\'')?.strip_suffix('\'')?;
        if inner.contains('\'') {
            return None;
        }
    } else if value.starts_with('"') {
        let inner = value.strip_prefix('"')?.strip_suffix('"')?;
        if inner.chars().any(|c| matches!(c, '"' | '$' | '`' | '\\')) {
            return None;
        }
    } else if value.chars().any(|c| {
        c.is_whitespace()
            || matches!(
                c,
                '\'' | '"'
                    | '$'
                    | '`'
                    | '\\'
                    | ';'
                    | '&'
                    | '|'
                    | '<'
                    | '>'
                    | '('
                    | ')'
                    | '{'
                    | '}'
                    | '~'
                    | '*'
                    | '?'
                    | '['
                    | ']'
            )
    }) {
        return None;
    }
    Some(name)
}

fn declaration_shape(code: &str, name: &str) -> bool {
    let code = code.trim_start();
    let exported = code
        .strip_prefix("export ")
        .or_else(|| code.strip_prefix("export\t"));
    let candidate = exported.map(str::trim_start).unwrap_or(code);
    if candidate
        .strip_prefix(name)
        .is_some_and(|tail| tail.starts_with('='))
    {
        return true;
    }
    if exported.is_some() && candidate == name {
        return true;
    }
    // Fish declarations are observable but never admitted for editing.
    code.split_whitespace().next() == Some("set")
        && code.split_whitespace().any(|word| word == name)
}

fn shell_declarations(text: &str) -> ShellDeclarations {
    let mut simple = true;
    let mut declarations = BTreeMap::<String, Declaration>::new();
    let mut assignments = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        let line = line.trim_end_matches('\n');
        let trim = line.trim();
        if trim.is_empty() || trim.starts_with('#') {
            continue;
        }
        let code = complete_shell_line(line);
        if code.is_none() {
            simple = false;
        }
        let visible = code.unwrap_or(line);
        let literal = code.and_then(literal_assignment);
        for name in names() {
            if !name_mentioned(visible, name) {
                continue;
            }
            let declaration = declarations
                .entry(name.to_owned())
                .or_insert_with(|| Declaration {
                    lines: vec![],
                    exact: true,
                    declared: false,
                });
            declaration.exact &= literal == Some(name);
            declaration.declared |= declaration_shape(visible, name);
            if assignments >= MAX_ASSIGNMENTS {
                simple = false;
                continue;
            }
            declaration.lines.push(index + 1);
            assignments += 1;
        }
    }
    ShellDeclarations {
        simple,
        declarations,
    }
}

fn env_value(name: &str) -> Option<OsString> {
    std::env::var_os(name)
}

fn scan(
    home: &Path,
    root: &Path,
    project: Option<&Path>,
    env: &dyn Fn(&str) -> Option<OsString>,
) -> Result<(Scan, Value)> {
    storage::guard(home)?;
    storage::guard(root)?;
    if let Some(project) = project {
        storage::guard(project)?;
    }
    // Validate the existing synthetic fixture boundary before reading any
    // credential or shell content. Reuse its finite candidate scan afterward.
    // Its 16 MiB body limit is independent of this module's 8 MiB budget.
    let startup = startup::snapshot(home, root, project, "readonly-auth-candidates")?;
    let synthetic_boundary = startup.binding["synthetic_boundary"]
        .as_str()
        .map(PathBuf::from);
    let mut scan = Scan::new();
    scan.json_file(
        root.join(".credentials.json"),
        "credentials",
        "selected_root",
        true,
    );
    let global = if root == home.join(".claude") {
        home.join(".claude.json")
    } else {
        root.join(".claude.json")
    };
    scan.json_file(global, "client_state", "selected_root", true);
    scan.json_file(
        root.join("settings.json"),
        "settings",
        "selected_root",
        true,
    );
    for name in SHELL_FILES {
        scan.shell_file(home.join(name), name.ends_with(".fish"));
    }
    scan.complete &= startup.view["candidate_scan_complete"] == true;
    for item in startup.view["sources"].as_array().into_iter().flatten() {
        let Some(path) = item["path"].as_str().map(PathBuf::from) else {
            continue;
        };
        let scope = item["scope"].as_str().unwrap_or("");
        let file = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let settings = matches!(
            file,
            "settings.json" | "settings.local.json" | "managed-settings.json"
        ) || scope == "managed_drop_in";
        if !settings || scope == "user" {
            continue;
        }
        let scope = if scope.starts_with("managed") {
            "managed"
        } else {
            "project"
        };
        scan.json_file(path, "settings", scope, false);
    }
    for name in names() {
        let value = env(name);
        let present = value.is_some();
        let mut item = source(
            "process_env",
            name,
            None,
            name,
            "current_process",
            if present {
                "observed_context"
            } else {
                "not_found"
            },
            if present { "present" } else { "absent" },
            false,
            "context_only",
        );
        item["value_state"] = json!(match value {
            None => "not_set",
            Some(value) if value.is_empty() => "empty",
            Some(_) => "nonempty",
        });
        scan.push(item, None);
    }
    let default_profiles = home.join(".config/anthropic");
    scan.metadata(default_profiles.clone(), "profile_store", "default_store");
    if let Some(raw) = env("ANTHROPIC_CONFIG_DIR") {
        let path = PathBuf::from(raw);
        if synthetic_boundary
            .as_deref()
            .is_some_and(|boundary| !path.starts_with(boundary))
        {
            // A test process may inherit a real profile pointer. Synthetic
            // inspection never follows it outside the temporary fixture.
            scan.push(
                source(
                    "profile",
                    "profile_store",
                    None,
                    "environment_selected_store",
                    "host_shared",
                    "unknown",
                    "unknown",
                    false,
                    "invalid_profile_pointer",
                ),
                None,
            );
        } else if path != default_profiles && path.is_absolute() && path.as_os_str().len() <= 4096 {
            scan.metadata(path, "profile_store", "environment_selected_store");
        } else if !path.is_absolute() || path.as_os_str().len() > 4096 {
            scan.push(
                source(
                    "profile",
                    "profile_store",
                    None,
                    "environment_selected_store",
                    "host_shared",
                    "unknown",
                    "unknown",
                    false,
                    "invalid_profile_pointer",
                ),
                None,
            );
        }
    }
    scan.keychain(home, root);
    scan.push(
        source(
            "external_context",
            "other_contexts",
            None,
            "external_contexts",
            "external",
            "unknown",
            "unknown",
            false,
            "external_contexts",
        ),
        None,
    );
    let inherited = match env("CLAUDE_CONFIG_DIR") {
        None => "not_set",
        Some(value) if Path::new(&value) == root => "matches_selected",
        Some(_) => "different",
    };
    let view = json!({"schema":SCHEMA,"sources":scan.sources,
        "context":{"execution":"current_lintel_process","selected_config_directory":root,
            "inherited_config_dir":inherited,"project_selected":project.is_some(),
            "shell_environment":"not_reconstructed","credential_values_returned":false},
        "limits":{"file_bytes":FILE_BYTES,"shell_file_bytes":SHELL_BYTES,"total_bytes":TOTAL_BYTES,
            "startup_candidate_bytes":16*1024*1024,"max_sources":MAX_SOURCES,
            "scanned_files":scan.seen.len(),"bounded_scan_complete":scan.complete,
            "coverage_complete":false,"keychain_scan_complete":scan.keychain_complete,"keychain":"selected_services_only","other_contexts":"not_inspected",
            "server_revocation":"not_checked"}});
    Ok((scan, view))
}

pub(crate) fn inspect(home: &Path, root: &Path, project: Option<&Path>) -> Result<Value> {
    scan(home, root, project, &env_value).map(|(_, view)| view)
}

/// Freeze only locators/identity and salted digests; no original or replacement
/// text and no credential value is serialized, even in private plan extras.
pub(crate) fn freeze(
    home: &Path,
    root: &Path,
    project: Option<&Path>,
    source_ids: &[String],
    nonce: &str,
) -> Result<Value> {
    if source_ids.len() > MAX_SOURCES || nonce.is_empty() || nonce.len() > 256 {
        return Err(err("invalid_auth_selection", "认证来源选择超出限制"));
    }
    let (scan, _) = scan(home, root, project, &env_value)?;
    let mut seen = BTreeSet::new();
    let mut groups = BTreeMap::<PathBuf, Value>::new();
    let mut selected = vec![];
    let mut keychain_groups = vec![];
    for source_id in source_ids {
        if !seen.insert(source_id) {
            return Err(err("invalid_auth_selection", "认证来源选择重复"));
        }
        if let Some((service, item)) = scan.keychains.get(source_id) {
            keychain_groups.push(json!({"storage":"keychain","home":home,"service":service,"identity":item.identity,
                "snapshot":keychain_digest(&item.bytes, nonce),"source_ids":[source_id],"locators":[{"pointer":"/claudeAiOauth"}],"kind":"json"}));
            selected.push(
                scan.sources
                    .iter()
                    .find(|s| s["id"] == *source_id)
                    .unwrap()
                    .clone(),
            );
            continue;
        }
        let candidate = scan.candidates.get(source_id).ok_or_else(|| {
            err(
                "auth_source_not_removable",
                "来源不存在、已改变或不支持精确清理，请重新检查",
            )
        })?;
        let file = scan
            .files
            .get(&candidate.path)
            .ok_or_else(|| err("auth_source_not_removable", "来源无法冻结，请重新检查"))?;
        let group = groups.entry(candidate.path.clone()).or_insert_with(|| {
            json!({
            "path":candidate.path,"kind":candidate.kind,"source_ids":[],"locators":[],
            "snapshot":salted_snapshot(file,nonce),"parent_identity":file.parent_identity})
        });
        group["source_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!(source_id));
        group["locators"]
            .as_array_mut()
            .unwrap()
            .push(candidate.locator.clone());
        selected.push(
            scan.sources
                .iter()
                .find(|s| s["id"] == *source_id)
                .unwrap()
                .clone(),
        );
    }
    let mut groups = groups.into_values().collect::<Vec<_>>();
    groups.extend(keychain_groups);
    Ok(json!({"schema":FROZEN_SCHEMA,"groups":groups,"sources":selected}))
}

pub(crate) fn check(groups: &Value, nonce: &str) -> Result<()> {
    let groups = groups
        .as_array()
        .ok_or_else(|| err("invalid_auth_selection", "认证来源冻结清单不完整"))?;
    for group in groups {
        if group["storage"] == "keychain" {
            keychain_original(group, nonce)?;
            continue;
        }
        check_parent(group)?;
        let path = group["path"]
            .as_str()
            .ok_or_else(|| err("invalid_auth_selection", "认证来源缺少路径"))?;
        if !matches_snapshot(Path::new(path), &group["snapshot"], nonce)? {
            return Err(err(
                "stale_auth_source",
                "预览后认证来源改变，未覆盖外部编辑，请重新预览",
            ));
        }
    }
    Ok(())
}

pub(crate) fn replacement(group: &Value, nonce: &str) -> Result<Vec<u8>> {
    check_parent(group)?;
    let path = Path::new(
        group["path"]
            .as_str()
            .ok_or_else(|| err("invalid_auth_selection", "认证来源缺少路径"))?,
    );
    let read = read_stable(path, FILE_BYTES)?
        .ok_or_else(|| err("stale_auth_source", "认证来源已消失，请重新预览"))?;
    if !read.editable || salted_snapshot(&read, nonce) != group["snapshot"] {
        return Err(err(
            "stale_auth_source",
            "预览后认证来源改变，原内容保持不变",
        ));
    }
    let locators = group["locators"]
        .as_array()
        .ok_or_else(|| err("invalid_auth_selection", "认证来源缺少冻结位置"))?;
    match group["kind"].as_str() {
        Some("json") => json_replacement(&read.bytes, locators, false),
        Some("shell") => {
            let text = std::str::from_utf8(&read.bytes)
                .map_err(|_| err("stale_auth_source", "Shell 来源格式改变"))?;
            let declarations = shell_declarations(text);
            if !declarations.simple {
                return Err(err("stale_auth_source", "Shell 来源无法安全精确编辑"));
            }
            let mut lines = BTreeSet::new();
            for locator in locators {
                let name = locator["name"]
                    .as_str()
                    .filter(|name| REMOVABLE_NAMES.contains(name))
                    .ok_or_else(|| err("invalid_auth_selection", "不支持的 Shell 认证变量"))?;
                let declaration = declarations
                    .declarations
                    .get(name)
                    .filter(|d| d.exact && json!(d.lines) == locator["lines"])
                    .ok_or_else(|| err("stale_auth_source", "Shell 认证赋值位置改变"))?;
                lines.extend(declaration.lines.iter().copied());
            }
            Ok(text
                .split_inclusive('\n')
                .enumerate()
                .filter(|(index, _)| !lines.contains(&(index + 1)))
                .flat_map(|(_, line)| line.bytes())
                .collect())
        }
        _ => Err(err("invalid_auth_selection", "不支持的认证来源编辑类型")),
    }
}

fn json_replacement(bytes: &[u8], locators: &[Value], keychain_only: bool) -> Result<Vec<u8>> {
    let mut doc = storage::parse(bytes).filter_object()?;
    for locator in locators {
        let pointer = locator["pointer"]
            .as_str()
            .ok_or_else(|| err("invalid_auth_selection", "缺少认证字段位置"))?;
        if keychain_only && pointer != "/claudeAiOauth" {
            return Err(err(
                "invalid_auth_selection",
                "Keychain 只支持 Claude 登录字段",
            ));
        }
        if let Some(name) = pointer
            .strip_prefix("/env/")
            .filter(|name| REMOVABLE_NAMES.contains(name))
        {
            let env = doc["env"]
                .as_object_mut()
                .ok_or_else(|| err("stale_auth_source", "认证字段结构改变"))?;
            if env.remove(name).is_none() {
                return Err(err("stale_auth_source", "认证字段已改变"));
            }
        } else if ["/claudeAiOauth", "/primaryApiKey", "/apiKeyHelper"].contains(&pointer) {
            if doc.as_object_mut().unwrap().remove(&pointer[1..]).is_none() {
                return Err(err("stale_auth_source", "认证字段已改变"));
            }
        } else {
            return Err(err("invalid_auth_selection", "不支持的认证字段位置"));
        }
    }
    if keychain_only {
        return Ok(serde_json::to_vec(&doc)?);
    }
    let mut bytes = serde_json::to_vec_pretty(&doc)?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn keychain_digest(bytes: &[u8], nonce: &str) -> String {
    let mut content = b"lintel.auth-keychain/1\0".to_vec();
    content.extend_from_slice(nonce.as_bytes());
    content.push(0);
    content.extend_from_slice(bytes);
    storage::digest(&content)
}

pub(crate) fn keychain_original(group: &Value, nonce: &str) -> Result<keychain::Item> {
    let home = Path::new(crate::string(group, "home")?);
    let item = keychain::read_bound(home, crate::string(group, "service")?, &group["identity"])?
        .ok_or_else(|| err("stale_auth_source", "原 Keychain 项目已消失，请重新预览"))?;
    if keychain_digest(&item.bytes, nonce) != group["snapshot"].as_str().unwrap_or("") {
        return Err(err(
            "stale_auth_source",
            "Keychain 容器在预览后变化，请重新预览",
        ));
    }
    Ok(item)
}

pub(crate) fn keychain_replacement(group: &Value, item: &keychain::Item) -> Result<Vec<u8>> {
    let locators = group["locators"]
        .as_array()
        .ok_or_else(|| err("invalid_auth_selection", "缺少 Keychain 字段位置"))?;
    json_replacement(&item.bytes, locators, true)
}

pub(crate) fn source_state_bound(source: &Value, group: &Value) -> Result<String> {
    if source["kind"] != "keychain_field" {
        return source_state(source);
    }
    let home = Path::new(crate::string(group, "home")?);
    let service = crate::string(group, "service")?;
    // Fresh service observation detects credentials recreated by a status command,
    // even when the original item was replaced. An absent field in another item
    // does not prove the approved original was edited.
    let fresh = keychain::read(home, service)?;
    if let Some(ref item) = fresh {
        let doc = storage::parse(&item.bytes).filter_object()?;
        if doc.get("claudeAiOauth").is_some() {
            return Ok("still_present".into());
        }
    }
    match keychain::read_bound(home, service, &group["identity"]) {
        Ok(Some(item)) => {
            let doc = storage::parse(&item.bytes).filter_object()?;
            Ok(if doc.get("claudeAiOauth").is_some() {
                "still_present"
            } else {
                "removed"
            }
            .into())
        }
        Ok(None) if fresh.is_none() => Ok("removed".into()),
        _ => Ok("unknown".into()),
    }
}

/// Independently read the exact original source. Absence from a truncated
/// inventory is never evidence that a selected credential was removed.
pub(crate) fn source_state(source: &Value) -> Result<String> {
    let Some(path) = source["path"].as_str() else {
        return Ok("unknown".into());
    };
    let read = match read_stable(Path::new(path), FILE_BYTES) {
        Ok(Some(read)) => read,
        Ok(None) => return Ok("removed".into()),
        Err(_) => return Ok("unknown".into()),
    };
    match source["kind"].as_str() {
        Some("credential_field" | "client_state_field" | "settings_env" | "api_key_helper") => {
            let Ok(doc) = storage::parse(&read.bytes).filter_object() else {
                return Ok("unknown".into());
            };
            let Some(pointer) = source["locator"].as_str() else {
                return Ok("unknown".into());
            };
            if doc.pointer(pointer).is_some() {
                return Ok("still_present".into());
            }
            if pointer.starts_with("/env/")
                && doc.get("env").is_some_and(|value| !value.is_object())
            {
                return Ok("unknown".into());
            }
            Ok("removed".into())
        }
        Some("shell_assignment") => {
            let Ok(text) = std::str::from_utf8(&read.bytes) else {
                return Ok("unknown".into());
            };
            let parsed = shell_declarations(text);
            let Some(name) = source["name"].as_str() else {
                return Ok("unknown".into());
            };
            if parsed
                .declarations
                .get(name)
                .is_some_and(|declaration| declaration.declared)
            {
                Ok("still_present".into())
            } else if parsed.declarations.contains_key(name) {
                Ok("unknown".into())
            } else if parsed.simple {
                Ok("removed".into())
            } else {
                Ok("unknown".into())
            }
        }
        _ => Ok("unknown".into()),
    }
}

#[cfg(test)]
#[path = "auth_sources_tests.rs"]
mod tests;
