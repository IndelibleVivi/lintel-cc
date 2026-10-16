//! Explicit user configuration sources and field-scoped edits. Publication,
//! approval, journal ownership and recovery stay in the canonical Engine.
use crate::{archive, components::startup, err, policy, storage::*, string, work, Engine, Result};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    fs,
    os::unix::fs::MetadataExt,
    path::{Component, Path, PathBuf},
};

const CONFIG_BYTES: u64 = 1024 * 1024;
const MAX_PATCHES: usize = 4096;

pub(crate) fn masked_setting(key: &str, value: &Value) -> bool {
    key == policy::MASKED_KEY
        || (key == "OTEL_EXPORTER_OTLP_ENDPOINT"
            && value
                .as_str()
                .is_some_and(|text| text.contains(['@', '?', '#'])))
}

fn project(r: &Value) -> Result<Option<&Path>> {
    match r.get("project_cwd") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if Path::new(value).is_absolute() => {
            let path = Path::new(value);
            guard(path)?;
            if !path.is_dir() {
                return Err(err("invalid_project", "项目目录不存在；未创建目录"));
            }
            Ok(Some(path))
        }
        _ => Err(err("invalid_project", "project_cwd 需要明确的绝对目录")),
    }
}

fn source_scope(root: &Path, path: &Path) -> Result<&'static str> {
    if !path.is_absolute() || path.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(err("invalid_path", "配置路径需要不含父目录跳转的绝对路径"));
    }
    guard(path)?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    if name.starts_with("managed-") || name == ".claude.json" || name == ".credentials.json" {
        return Err(err(
            "config_readonly",
            "该文件属于管理配置或身份材料；请使用对应的独立操作",
        ));
    }
    if path == root.join("settings.json") {
        return Ok("user");
    }
    if matches!(name, "settings.json" | "settings.local.json")
        && path
            .parent()
            .and_then(Path::file_name)
            .is_some_and(|n| n == ".claude")
    {
        return Ok(if name == "settings.local.json" {
            "project_local"
        } else {
            "project"
        });
    }
    if name == ".mcp.json" {
        return Ok("project_mcp");
    }
    if path.parent() == Some(root) && path.extension().is_some_and(|ext| ext == "json") {
        return Ok("explicit_user_config");
    }
    Err(err(
        "unsupported_config_source",
        "请选择环境内的 JSON 配置、项目 settings/settings.local 或 .mcp.json",
    ))
}

fn writable_anchor(path: &Path) -> Result<(PathBuf, Value)> {
    let parent = path
        .parent()
        .ok_or_else(|| err("invalid_path", "配置路径缺少父目录"))?;
    let anchor = parent
        .ancestors()
        .find(|p| p.exists())
        .ok_or_else(|| err("path_unreadable", "配置父目录不可访问"))?;
    guard(anchor)?;
    let meta = fs::metadata(anchor)?;
    if !meta.is_dir() || meta.uid() != unsafe { libc::geteuid() } || meta.mode() & 0o300 != 0o300 {
        return Err(err(
            "config_readonly",
            "当前用户不能在配置父目录中写入；未改变权限",
        ));
    }
    // Only the ordinary .claude directory may be prepared for a new project
    // source. Do not let an explicit path create an arbitrary directory tree.
    if anchor != parent
        && !(parent.file_name().is_some_and(|name| name == ".claude")
            && parent.parent() == Some(anchor))
    {
        return Err(err(
            "config_parent_missing",
            "配置父目录不存在；请先选择现有目录",
        ));
    }
    Ok((
        anchor.to_path_buf(),
        json!([meta.dev(), meta.ino(), meta.uid()]),
    ))
}

fn read_document(path: &Path) -> Result<(Value, String, Value)> {
    let before = snapshot(path)?;
    if before.is_null() {
        return Ok((json!({}), digest(b""), before));
    }
    let bytes = read(path, CONFIG_BYTES)?;
    let doc = parse(&bytes)?;
    if !doc.is_object() {
        return Err(err(
            "unsupported_settings",
            "配置必须是 JSON 对象；未改动原件",
        ));
    }
    if snapshot(path)? != before {
        return Err(err("source_changed", "读取期间配置来源改变"));
    }
    Ok((doc, digest(&bytes), before))
}

fn observe_document(path: &Path) -> Result<Value> {
    use std::{io::Read, os::unix::fs::OpenOptionsExt};
    guard(path)?;
    let before = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(json!({})),
        Err(error) => return Err(error.into()),
    };
    if !before.is_file() || before.len() > CONFIG_BYTES {
        return Err(err("config_limit", "配置来源不是有限普通文件"));
    }
    let identity = |m: &fs::Metadata| {
        json!([
            m.dev(),
            m.ino(),
            m.uid(),
            m.len(),
            m.mtime(),
            m.mtime_nsec(),
            m.ctime(),
            m.ctime_nsec()
        ])
    };
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    if identity(&file.metadata()?) != identity(&before) {
        return Err(err("source_changed", "读取配置来源时对象改变"));
    }
    let mut bytes = vec![];
    (&mut file).take(CONFIG_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > CONFIG_BYTES
        || identity(&file.metadata()?) != identity(&before)
        || identity(&fs::symlink_metadata(path)?) != identity(&before)
    {
        return Err(err("source_changed", "读取配置来源时内容改变"));
    }
    let doc = parse(&bytes)?;
    if !doc.is_object() {
        return Err(err("unsupported_settings", "配置来源不是JSON对象"));
    }
    Ok(doc)
}

fn writable_source(path: &Path) -> Result<(PathBuf, Value)> {
    let anchor = writable_anchor(path)?;
    if let Ok(meta) = fs::symlink_metadata(path) {
        if !meta.is_file()
            || meta.uid() != unsafe { libc::geteuid() }
            || meta.nlink() != 1
            || meta.mode() & 0o200 == 0
        {
            return Err(err(
                "config_readonly",
                "配置文件不是当前用户可写的独占普通文件",
            ));
        }
    }
    Ok(anchor)
}

fn validate_source_values(scope: &str, patches: &[Value]) -> Result<()> {
    if !matches!(scope, "project" | "project_local") {
        return Ok(());
    }
    for patch in patches {
        if patch["parts"] == json!(["env"]) && patch["after_present"] == true {
            if let Some(fields) = patch["after"].as_object() {
                let children:Vec<_>=fields.iter().map(|(key,value)|json!({"parts":["env",key],"after_present":true,"after":value})).collect();
                validate_source_values(scope, &children)?;
            }
        }
        let Some(parts) = patch["parts"]
            .as_array()
            .filter(|p| p.len() == 2 && p[0] == "env")
        else {
            continue;
        };
        let Some(rule) = parts[1]
            .as_str()
            .and_then(|key| policy::catalog().iter().find(|rule| rule.key == key))
        else {
            continue;
        };
        if patch["after_present"] != true {
            continue;
        }
        let value = patch["after"]
            .as_str()
            .ok_or_else(|| err("unsupported_settings", "env 设置值需要字符串"))?;
        let off = matches!(
            value.to_ascii_lowercase().as_str(),
            "" | "0" | "false" | "no" | "off" | "none"
        );
        let allowed = if off {
            rule.sources.turn_off
        } else {
            rule.sources.turn_on
        };
        if !allowed.contains(&scope) {
            return Err(err(
                "config_source_not_permitted",
                &format!("{} 不能从此来源设置；请选择 user 配置", rule.key),
            ));
        }
    }
    Ok(())
}

fn pointer(parts: &[String]) -> String {
    parts
        .iter()
        .map(|part| format!("/{}", part.replace('~', "~0").replace('/', "~1")))
        .collect()
}

fn changes(
    before: Option<&Value>,
    after: Option<&Value>,
    path: &mut Vec<String>,
    out: &mut Vec<Value>,
) {
    if before == after {
        return;
    }
    if let (None, Some(Value::Object(right))) = (before, after) {
        if !right.is_empty() {
            let start = out.len();
            for (key, value) in right {
                path.push(key.clone());
                changes(None, Some(value), path, out);
                path.pop();
            }
            for patch in &mut out[start..] {
                if patch.get("missing_parents").is_none() {
                    patch["missing_parents"] = json!([]);
                }
                patch["missing_parents"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!(path));
            }
            return;
        }
    }
    if let (Some(Value::Object(left)), Some(Value::Object(right))) = (before, after) {
        let keys: BTreeSet<_> = left.keys().chain(right.keys()).collect();
        for key in keys {
            path.push(key.clone());
            changes(left.get(key), right.get(key), path, out);
            path.pop();
        }
    } else {
        out.push(json!({"key":pointer(path),"parts":path,"before_present":before.is_some(),"after_present":after.is_some(),"before":before,"after":after}));
    }
}

fn current_value<'a>(doc: &'a Value, patch: &Value) -> Option<&'a Value> {
    let mut current = doc;
    for part in patch["parts"].as_array()? {
        current = current.as_object()?.get(part.as_str()?)?;
    }
    Some(current)
}

fn apply_patch(doc: &mut Value, patch: &Value) -> Result<()> {
    let parts = patch["parts"]
        .as_array()
        .ok_or_else(|| err("invalid_plan", "缺少配置字段路径"))?;
    if parts.is_empty() {
        return Err(err("invalid_plan", "不替换整个配置根"));
    }
    let mut target = &mut *doc;
    for part in &parts[..parts.len() - 1] {
        let key = part
            .as_str()
            .ok_or_else(|| err("invalid_plan", "配置字段路径损坏"))?;
        let object = target
            .as_object_mut()
            .ok_or_else(|| err("restore_conflict", "字段上级类型已改变"))?;
        target = object.entry(key).or_insert_with(|| json!({}));
    }
    let key = parts
        .last()
        .unwrap()
        .as_str()
        .ok_or_else(|| err("invalid_plan", "配置字段路径损坏"))?;
    let object = target
        .as_object_mut()
        .ok_or_else(|| err("restore_conflict", "字段上级类型已改变"))?;
    if patch["after_present"] == true {
        object.insert(key.into(), patch["after"].clone());
    } else {
        object.remove(key);
    }
    if patch["after_present"] != true {
        for parent in patch["missing_parents"].as_array().into_iter().flatten() {
            let parts: Vec<String> = parent
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            if !parts.is_empty()
                && doc
                    .pointer(&pointer(&parts))
                    .and_then(Value::as_object)
                    .is_some_and(|o| o.is_empty())
            {
                apply_patch(doc, &json!({"parts":parts,"after_present":false}))?;
            }
        }
    }
    Ok(())
}

fn public_change(patch: &Value, path: &Path) -> Value {
    let parts = patch["parts"].as_array();
    let known = parts
        .filter(|p| p.len() == 2 && p[0] == "env")
        .and_then(|p| p[1].as_str())
        .and_then(|key| policy::catalog().iter().find(|r| r.key == key));
    let sensitive = known.is_none()
        || known.is_some_and(|r| {
            masked_setting(r.key, &patch["before"]) || masked_setting(r.key, &patch["after"])
        });
    json!({"key":patch["key"],"label":known.map(|r|r.label).unwrap_or("明确选择的配置字段"),"path":path,
        "before":if sensitive{Value::Null}else{patch["before"].clone()},"after":if sensitive{Value::Null}else{patch["after"].clone()},"masked":sensitive,
        "before_present":patch["before_present"],"after_present":patch["after_present"],"operation":if patch["after_present"]!=true{"remove"}else if patch["before_present"]!=true{"add"}else{"replace"}})
}

fn selected_keys(r: &Value) -> Result<Vec<String>> {
    let keys = r["selected_keys"]
        .as_array()
        .filter(|a| !a.is_empty() && a.len() <= MAX_PATCHES)
        .ok_or_else(|| err("invalid_selection", "需要选择至少一个明确配置字段"))?;
    let mut unique = BTreeSet::new();
    for key in keys {
        let key = key
            .as_str()
            .filter(|s| !s.is_empty() && s.len() <= 4096 && !s.chars().any(char::is_control))
            .ok_or_else(|| err("invalid_selection", "配置字段选择无效"))?;
        if !unique.insert(key.to_owned()) {
            return Err(err("invalid_selection", "不能重复选择同一字段"));
        }
    }
    Ok(unique.into_iter().collect())
}

impl Engine {
    pub(crate) fn config_inspect(&self, r: &Value) -> Result<Value> {
        let environment = policy::environment(self.env(r)?, self.executable());
        let root = Path::new(string(&environment, "root")?);
        let project = project(r)?;
        let observed = startup::snapshot(&self.home, root, project, "readonly-config-sources")?;
        let mut files = vec![];
        let mut settings = vec![];
        let mut bytes_left = 16 * CONFIG_BYTES;
        for source in observed.view["sources"].as_array().into_iter().flatten() {
            let Some(raw) = source["path"].as_str() else {
                continue;
            };
            let path = Path::new(raw);
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !matches!(
                name,
                "settings.json"
                    | "settings.local.json"
                    | ".mcp.json"
                    | "managed-settings.json"
                    | "managed-mcp.json"
            ) && source["scope"] != "managed_drop_in"
            {
                continue;
            }
            let writable = source_scope(root, path).is_ok() && writable_source(path).is_ok();
            let bytes = fs::symlink_metadata(path).map(|m| m.len()).unwrap_or(0);
            let loaded = if bytes > bytes_left {
                Err(err("config_limit", "配置来源总读取预算已耗尽"))
            } else {
                bytes_left -= bytes;
                observe_document(path)
            };
            let mut file = json!({"path":path,"scope":source["scope"],"state":source["state"],"writable":writable,"content_state":source["content_state"]});
            match loaded {
                Ok(doc) => {
                    for rule in policy::catalog() {
                        if let Some(value) = doc["env"].get(rule.key) {
                            let patch = json!({"key":format!("/env/{}",rule.key),"parts":["env",rule.key],"before_present":true,"after_present":true,"before":value,"after":value});
                            let safe = public_change(&patch, path);
                            let scope = source["scope"].as_str().unwrap_or("unknown");
                            settings.push(json!({"key":rule.key,"label":rule.label,"value":safe["after"],"masked":safe["masked"],"source":path,"scope":scope,
                                "status":policy::setting_status(rule.key,value),"impact":rule.impact,"version_floor":rule.version_floor,"depends_on":rule.depends_on,
                                "allowed_sources":{"turn_on":rule.sources.turn_on,"turn_off":rule.sources.turn_off},"runtime_verified":false,"effect_timing":"next_launch"}));
                        }
                    }
                }
                Err(error) => {
                    file["state"] = json!("access_limited");
                    file["error"] = json!({"code":error.code});
                }
            }
            files.push(file);
        }
        if let Some(cwd) = project {
            for name in [
                ".claude/settings.json",
                ".claude/settings.local.json",
                ".mcp.json",
            ] {
                let path = cwd.join(name);
                if !files.iter().any(|v| v["path"] == json!(path)) {
                    files.push(json!({"path":path,"scope":if name.ends_with("local.json"){"project_local"}else{"project"},"state":"not_found","writable":writable_anchor(&path).is_ok()}));
                }
            }
        }
        let mut conflicts = vec![];
        for rule in policy::catalog() {
            let rows: Vec<_> = settings
                .iter()
                .filter(|row| row["key"] == rule.key)
                .collect();
            if rows.len() > 1 {
                let distinct: BTreeSet<_> =
                    rows.iter().map(|row| row["value"].to_string()).collect();
                conflicts.push(json!({"key":rule.key,"sources":rows.iter().map(|row|&row["source"]).collect::<Vec<_>>(),"state":if rows.iter().any(|row|row["masked"]==true){"unknown"}else if distinct.len()>1{"conflicting"}else{"multiple_sources"}}));
            }
        }
        let complete = observed.view["candidate_scan_complete"] == true
            && files.iter().all(|file| file["state"] != "access_limited");
        Ok(
            json!({"environment_id":environment["id"],"root":root,"project_cwd":project,"files":files,"settings":settings,"conflicts":conflicts,"coverage":complete,"actual_loaded":"unverified","note":"只观察有限静态来源，不使用 App 环境变量推断终端／IDE 的真实生效值；managed、CLI --settings 与外部启动环境仍须核对。"}),
        )
    }

    pub(crate) fn config_read(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let root = Path::new(string(&e, "root")?);
        let path = Path::new(string(r, "config_path")?);
        let scope = source_scope(root, path)?;
        writable_source(path)?;
        let (doc, digest, _) = read_document(path)?;
        Ok(
            json!({"path":path,"scope":scope,"document":serde_json::to_string_pretty(&doc)?,"digest":digest,"private_content":true}),
        )
    }

    fn plan_config_patches(
        &self,
        e: &Value,
        path: &Path,
        doc: &Value,
        patches: Vec<Value>,
        original: Option<&str>,
        assessment: Option<Value>,
    ) -> Result<Value> {
        let root = Path::new(string(e, "root")?);
        let scope = source_scope(root, path)?;
        let (anchor, anchor_identity) = writable_source(path)?;
        if patches.len() > MAX_PATCHES {
            return Err(err("config_limit", "配置差异超过4096项；请缩小选择"));
        }
        if original.is_none() {
            validate_source_values(scope, &patches)?;
        }
        let mut next = doc.clone();
        for patch in &patches {
            apply_patch(&mut next, patch)?;
        }
        if !next.is_object()
            || next.get("env").is_some_and(|env| {
                !env.is_object() || env.as_object().unwrap().values().any(|v| !v.is_string())
            })
        {
            return Err(err(
                "unsupported_settings",
                "编辑后的配置必须是对象，env 只接受字符串值",
            ));
        }
        if serde_json::to_vec_pretty(&next)?.len() as u64 > CONFIG_BYTES {
            return Err(err("config_limit", "配置超过1MiB编辑上限"));
        }
        let public: Vec<_> = patches
            .iter()
            .map(|patch| public_change(patch, path))
            .collect();
        let mut actions =
            vec![json!({"id":"config_backup","label":"修改前加密保全当前配置","reversible":false})];
        if !path.parent().unwrap().exists() {
            actions.push(json!({"id":"config_parent","label":format!("建立明确选择的配置目录：{}",path.parent().unwrap().display()),"reversible":false}));
        }
        actions.push(json!({"id":"settings","label":"应用所选字段、同步并读回","reversible":true}));
        self.plan(e,"config_edit",if original.is_some(){"恢复选中的配置字段"}else{"修改明确选择的配置文件"},json!(public),
            vec!["未选择的配置字段与后续无关编辑","登录、持久身份与全部工作原件"],
            json!(actions),
            json!({"config_path":path,"config_scope":scope,"config_patches":patches,"config_parent":anchor,"config_parent_identity":anchor_identity,
                "config_directory":path.parent(),"config_directory_missing":!path.parent().unwrap().exists(),
                "original_job":original,"policy":assessment,"archive_passphrase_required":!public.is_empty()}))
    }

    pub(crate) fn plan_config_edit(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let path = Path::new(string(r, "config_path")?);
        source_scope(Path::new(string(&e, "root")?), path)?;
        let (before, digest, _) = read_document(path)?;
        if r["expected_digest"] != digest {
            return Err(err(
                "stale_plan",
                "配置已在编辑期间变化；请重新读取并核对差异",
            ));
        }
        let after = match &r["document"] {
            Value::String(text) if text.len() as u64 <= CONFIG_BYTES => parse(text.as_bytes())?,
            _ => return Err(err("invalid_request", "document 需要最多1MiB的JSON文本")),
        };
        if !after.is_object() {
            return Err(err("unsupported_settings", "配置根必须是JSON对象"));
        }
        let mut patches = vec![];
        changes(Some(&before), Some(&after), &mut vec![], &mut patches);
        self.plan_config_patches(&e, path, &before, patches, None, None)
    }

    pub(crate) fn plan_config_policy(&self, r: &Value) -> Result<Value> {
        let e = policy::environment(self.env(r)?, self.executable());
        let path = Path::new(string(r, "config_path")?);
        source_scope(Path::new(string(&e, "root")?), path)?;
        let (before, _, _) = read_document(path)?;
        let (policy_changes, assessment) = policy::plan(&before, &e["product_evidence"], r, path)?;
        let mut after = before.clone();
        if after.get("env").is_none() {
            after["env"] = json!({});
        }
        for change in policy_changes.as_array().unwrap() {
            let key = string(change, "key")?;
            if change["after"].is_null() {
                after["env"].as_object_mut().unwrap().remove(key);
            } else {
                after["env"][key] = change["after"].clone();
            }
        }
        let mut patches = vec![];
        changes(Some(&before), Some(&after), &mut vec![], &mut patches);
        self.plan_config_patches(&e, path, &before, patches, None, Some(assessment))
    }

    pub(crate) fn plan_config_apply(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let source = self.env(&json!({"environment_id":string(r,"source_environment_id")?}))?;
        if e["id"] == source["id"] {
            return Err(err("invalid_request", "请选择不同的目标环境"));
        }
        let (_, source_doc, _) = self.settings(&source)?;
        let path = r["config_path"]
            .as_str()
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(e["root"].as_str().unwrap()).join("settings.json"));
        source_scope(Path::new(string(&e, "root")?), &path)?;
        let (doc, _, _) = read_document(&path)?;
        let mut after = doc.clone();
        if after.get("env").is_none() {
            after["env"] = json!({});
        }
        if !after["env"].is_object() {
            return Err(err("unsupported_settings", "目标env不是对象"));
        }
        for key in selected_keys(r)? {
            if !policy::fields().any(|(known, _)| known == key) || key == policy::MASKED_KEY {
                return Err(err(
                    "private_setting_not_copyable",
                    "只复制已识别的普通配置，身份和接收端凭据不随方案复制",
                ));
            }
            let value = &source_doc["env"][&key];
            let patch = json!({"key":format!("/env/{key}"),"parts":["env",key],"before":value,"after":value});
            if public_change(&patch, &path)["masked"] == true {
                return Err(err(
                    "private_setting_not_copyable",
                    "带认证信息的配置不得跨环境复制",
                ));
            }
            if value.is_null() {
                after["env"].as_object_mut().unwrap().remove(&key);
            } else {
                after["env"][&key] = value.clone();
            }
        }
        let mut patches = vec![];
        changes(Some(&doc), Some(&after), &mut vec![], &mut patches);
        self.plan_config_patches(&e, &path, &doc, patches, None, None)
    }

    fn original_config(&self, r: &Value) -> Result<(Value, Value, Value)> {
        let job_id = crate::safe_id(r, "job_id")?;
        let job = self.dispatch(&json!({"command":"job","job_id":job_id}))?;
        let plan = load(&self.path("plans", string(&job, "plan_id")?))?;
        let mut unhashed = plan.clone();
        unhashed
            .as_object_mut()
            .ok_or_else(|| err("invalid_plan", "原计划损坏"))?
            .remove("hash");
        if plan["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
            return Err(err("plan_changed", "原计划已改变"));
        }
        let plan = if plan["kind"] == "client_workflow" {
            crate::client_workflow::config_step(&plan)
                .ok_or_else(|| err("not_restorable", "此组合没有配置步骤"))?
        } else {
            plan
        };
        if r["environment_id"] != plan["environment_id"] {
            return Err(err("environment_mismatch", "恢复仍绑定原任务的环境"));
        }
        let environment = self.settings_plan_environment(&plan)?;
        if plan["kind"] != "config_edit" || job["restorable"] != true {
            return Err(err(
                "not_restorable",
                "此任务没有已确认的配置编辑；旧保护任务请使用原字段恢复入口",
            ));
        }
        Ok((environment, plan, job))
    }

    pub(crate) fn config_restore_inspect(&self, r: &Value) -> Result<Value> {
        let (e, plan, job) = self.original_config(r)?;
        let path = Path::new(string(&plan["extra"], "config_path")?);
        let (doc, _, _) = read_document(path)?;
        let fields: Vec<_> = plan["extra"]["config_patches"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "原字段列表缺失"))?
            .iter()
            .map(|patch| {
                let available = current_value(&doc, patch)
                    == if patch["after_present"] == true {
                        Some(&patch["after"])
                    } else {
                        None
                    };
                let mut public = public_change(patch, path);
                public["available"] = json!(available);
                public["reason"] = json!(if available {
                    "original_value_matches"
                } else {
                    "field_changed"
                });
                public
            })
            .collect();
        Ok(json!({"job_id":job["id"],"environment_id":e["id"],"path":path,"fields":fields}))
    }

    pub(crate) fn plan_config_restore(&self, r: &Value) -> Result<Value> {
        let (e, plan, job) = self.original_config(r)?;
        let path = Path::new(string(&plan["extra"], "config_path")?);
        let (doc, _, _) = read_document(path)?;
        let original = plan["extra"]["config_patches"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "原字段列表缺失"))?;
        let mut patches = vec![];
        for key in selected_keys(r)? {
            let patch = original
                .iter()
                .find(|p| p["key"] == key)
                .ok_or_else(|| err("invalid_selection", "所选字段不属于原任务"))?;
            if current_value(&doc, patch)
                != if patch["after_present"] == true {
                    Some(&patch["after"])
                } else {
                    None
                }
            {
                return Err(err(
                    "restore_conflict",
                    &format!("{key} 已由后续编辑改变；请保留当前值或重新选择"),
                ));
            }
            let mut inverse = patch.clone();
            inverse["before"] = patch["after"].clone();
            inverse["before_present"] = patch["after_present"].clone();
            inverse["after"] = patch["before"].clone();
            inverse["after_present"] = patch["before_present"].clone();
            patches.push(inverse);
        }
        self.plan_config_patches(&e, path, &doc, patches, job["id"].as_str(), None)
    }

    pub(crate) fn config_plan_settings(
        &self,
        e: &Value,
        p: &Value,
    ) -> Result<(PathBuf, Value, Value)> {
        let path = PathBuf::from(string(&p["extra"], "config_path")?);
        source_scope(Path::new(string(e, "root")?), &path)?;
        writable_source(&path)?;
        let anchor = Path::new(string(&p["extra"], "config_parent")?);
        guard(anchor)?;
        let metadata = fs::metadata(anchor)?;
        if p["extra"]["config_parent_identity"]
            != json!([metadata.dev(), metadata.ino(), metadata.uid()])
        {
            return Err(err("stale_plan", "配置父目录实际对象已改变"));
        }
        if p["extra"]["config_directory_missing"] == true && path.parent().unwrap().exists() {
            let original = load(&self.path("jobs", string(p, "id")?))
                .map_err(|_| err("stale_plan", "预览后配置目录已由其他操作创建"))?;
            let meta = fs::metadata(path.parent().unwrap())?;
            if original["config_created_parent"] != json!([meta.dev(), meta.ino(), meta.uid()]) {
                return Err(err("stale_plan", "配置目录创建归属无法确认"));
            }
        }
        let (doc, _, snapshot) = read_document(&path)?;
        Ok((path, doc, snapshot))
    }

    pub(crate) fn execute_config_edit(
        &self,
        e: &Value,
        p: &Value,
        r: &Value,
        j: &mut Value,
        journal: &Path,
    ) -> Result<()> {
        let (path, mut doc, before) = self.config_plan_settings(e, p)?;
        if before != p["snapshot"] {
            return Err(err("stale_plan", "配置在批准后改变"));
        }
        let patches = p["extra"]["config_patches"]
            .as_array()
            .ok_or_else(|| err("invalid_plan", "缺少冻结配置差异"))?;
        if patches.is_empty() {
            j["steps"] = json!([{"id":"config_backup","status":"skipped","label":"无需备份","message":"配置无需修改"},{"id":"settings","status":"completed","label":"配置无需修改","message":"原文件字节、权限与存在性保持不变"}]);
            j["status"] = json!("completed");
            return Ok(());
        }
        let passphrase = work::check_passphrase(r)?;
        let sealed = archive::seal(
            &json!({"schema":"lintel.config/1","path":path,"document":doc}),
            passphrase,
        )?;
        let backup = self
            .state
            .join("archives")
            .join(format!("{}-config.age", string(j, "id")?));
        j["config_archive_path"] = json!(backup);
        j["steps"] = json!([{"id":"config_backup","status":"executing","label":"加密配置备份","message":"先保全，再修改"}]);
        save(journal, j)?;
        atomic_new(&backup, &sealed, 0o600)?;
        if read(&backup, STATE_JSON_LIMIT)? != sealed {
            return Err(err(
                "archive_readback_failed",
                "加密配置备份未能读回，未修改配置",
            ));
        }
        j["steps"][0]["status"] = json!("completed");
        save(journal, j)?;
        for patch in patches {
            let expected = if patch["before_present"] == true {
                Some(&patch["before"])
            } else {
                None
            };
            if current_value(&doc, patch) != expected {
                return Err(err("restore_conflict", "冻结字段与当前配置不同"));
            }
            apply_patch(&mut doc, patch)?;
        }
        if snapshot(&path)? != before {
            return Err(err("stale_plan", "备份后配置已变化；保留备份与当前文件"));
        }
        let parent = path.parent().unwrap();
        if !parent.exists() {
            use std::os::unix::fs::DirBuilderExt;
            j["config_parent_intent"] = json!(parent);
            j["steps"].as_array_mut().unwrap().push(json!({"id":"config_parent","label":"创建明确的配置目录","status":"executing","message":"只创建计划内的 .claude 目录"}));
            save(journal, j)?;
            fs::DirBuilder::new().mode(0o700).create(parent)?;
            fs::File::open(parent.parent().unwrap())?.sync_all()?;
            let meta = fs::metadata(parent)?;
            j["config_created_parent"] = json!([meta.dev(), meta.ino(), meta.uid()]);
            j["steps"].as_array_mut().unwrap().last_mut().unwrap()["status"] = json!("completed");
            save(journal, j)?;
        }
        let mode = if before.is_null() {
            0o600
        } else {
            fs::metadata(&path)?.mode() & 0o777 & !0o022
        };
        j["steps"].as_array_mut().unwrap().push(json!({"id":"settings","status":"executing","label":"发布选中的配置字段","message":"同步并读回后才确认完成"}));
        save(journal, j)?;
        atomic_recorded(&path, &serde_json::to_vec_pretty(&doc)?, mode, |staged| {
            if snapshot(&path)? != before {
                return Err(err("stale_plan", "发布前配置变化"));
            }
            j["settings_write"] = json!({"state":"prepared","plan_hash":p["hash"],"before":before,"after":snapshot(staged)?});
            save(journal, j)?;
            #[cfg(test)]
            if let Some(hook) = self.settings_write_hook {
                hook("prepared");
            }
            Ok(())
        })?;
        #[cfg(test)]
        if let Some(hook) = self.settings_write_hook {
            hook("published");
        }
        j["settings_write"]["state"] = json!("written");
        j["status"] = json!("verifying");
        save(journal, j)?;
        if load(&path)? != doc {
            return Err(err("readback_failed", "配置写后读回不一致"));
        }
        j["restorable"] = json!(true);
        j["steps"].as_array_mut().unwrap().last_mut().unwrap()["status"] = json!("completed");
        j["status"] = json!("completed");
        if path == Path::new(string(e, "root")?).join("settings.json") {
            self.baseline(e)?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "config_control_tests.rs"]
mod tests;
