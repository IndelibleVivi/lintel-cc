//! Immutable finite launch requests (SPEC §4.2) and the native resume adapter
//! (SPEC §8). These are *not* detached mutation jobs and never queue: a plan
//! freezes the exact target/mode/input reference under the plan's own ID, and
//! one explicit request ID drives the existing Terminal/PTY entry. The durable
//! intent is persisted before any Terminal attempt; a repeat only queries the
//! original request and never launches a second time.
use crate::{err, id, now, policy, safe_id, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs::OpenOptions,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};

/// Exclusive per-request lock so same-ID launch attempts serialize instead of
/// racing into two Terminal sessions. The durable intent is written under it.
fn lock(path: &Path) -> Result<std::fs::File> {
    use fs2::FileExt;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(file)
}

/// Validate a project working directory on the target host: exists, is a real
/// directory, is not a symlink, and is owned by the current user. Never creates,
/// moves or chmods it — a launch must not mutate the project.
fn check_project_cwd(raw: &str) -> Result<PathBuf> {
    let path = Path::new(raw);
    if !path.is_absolute() {
        return Err(err("invalid_path", "项目工作目录必须是绝对路径"));
    }
    guard(path)?;
    if fs_symlink(path) {
        return Err(err("symlink_target", "项目工作目录不能是符号链接"));
    }
    let metadata = std::fs::metadata(path)
        .map_err(|_| err("project_missing", "项目工作目录不存在或不可访问"))?;
    use std::os::unix::fs::MetadataExt;
    if !metadata.is_dir() {
        return Err(err("project_missing", "项目工作目录不是目录"));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(err("wrong_owner", "项目工作目录不属于当前用户"));
    }
    Ok(path.canonicalize()?)
}

fn fs_symlink(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink())
}

/// A directory's birth time distinguishes immediately reused inode numbers.
/// It stays stable when ordinary project files are added or removed. Refuse a
/// new frozen launch on filesystems that cannot provide this evidence.
fn directory_generation(metadata: &std::fs::Metadata) -> Result<Value> {
    let created = metadata
        .created()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .ok_or_else(|| err(
            "directory_identity_unsupported",
            "当前文件系统未提供可核对的目录创建时间，无法冻结启动目标；请使用支持目录创建时间的文件系统。",
        ))?;
    Ok(json!({"seconds":created.as_secs(),"nanoseconds":created.subsec_nanos()}))
}

/// Project working directory identity: device, inode, birth time and owner, frozen at
/// preview and rechecked before any launch so a same-path replacement is stale.
fn project_identity(path: &Path) -> Result<Value> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::metadata(path)
        .map_err(|_| err("project_missing", "项目工作目录不存在或不可访问"))?;
    if !metadata.is_dir() {
        return Err(err("project_missing", "项目工作目录不是目录"));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(err("wrong_owner", "项目工作目录不属于当前用户"));
    }
    Ok(json!({
        "inode": metadata.ino(),
        "device": metadata.dev(),
        "owner": metadata.uid(),
        "generation": directory_generation(&metadata)?,
    }))
}

/// Bounded declared native-resume support policy.
///
/// Only Claude Code versions we can name from the reviewed official material
/// (2026-10-05: `--resume <absolute .jsonl>` + `--fork-session`) are treated as
/// candidates. An unknown/absent version is *never* assumed supported. This is a
/// policy about a finite declared set, not a claim that any of them will
/// authenticate or attach as the caller expects.
const SUPPORTED_RESUME_VERSIONS: &[&str] = &["2.1.283", "2.1.285"];

/// The resume-adapter policy revision. A plan frozen under a different revision
/// must be re-previewed before its first launch (a code/policy change invalidates
/// the approved adapter behavior).
const RESUME_POLICY_VERSION: u32 = 1;
/// Largest head of a transcript scanned for resume-format assessment. A file
/// larger than this is explicitly unsupported for native resume rather than
/// loaded in full.
const RESUME_SCAN_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, PartialEq)]
enum ResumeSupport {
    /// Version is in the declared set and the transcript shape is recognized.
    Supported,
    Unsupported(&'static str),
}

impl ResumeSupport {
    fn supported(&self) -> bool {
        matches!(self, ResumeSupport::Supported)
    }
    fn reason(&self) -> Value {
        match self {
            ResumeSupport::Supported => Value::Null,
            ResumeSupport::Unsupported(message) => json!(message),
        }
    }
}

/// Directory identity for a config root: device, inode, birth time and owner. Frozen at
/// preview and rechecked at execution so a same-path replacement is stale.
fn dir_identity(path: &Path) -> Result<Value> {
    use std::os::unix::fs::MetadataExt;
    let metadata =
        std::fs::metadata(path).map_err(|_| err("root_unreadable", "配置 root 不存在或不可读"))?;
    if !metadata.is_dir() {
        return Err(err("root_unreadable", "配置 root 不是目录"));
    }
    if metadata.uid() != unsafe { libc::geteuid() } {
        return Err(err("root_unreadable", "配置 root 不属于当前用户"));
    }
    Ok(json!({
        "inode": metadata.ino(),
        "device": metadata.dev(),
        "owner": metadata.uid(),
        "generation": directory_generation(&metadata)?,
    }))
}

/// Executable identity: a regular executable owned by this user or a protected
/// root installation, with size, mode and nanosecond change timestamps. A same-path, same-version replacement
/// with different bytes changes the identity, so it is stale before launch.
fn exec_identity(path: &Path) -> Result<Value> {
    use std::os::unix::fs::MetadataExt;
    let metadata =
        std::fs::metadata(path).map_err(|_| err("executable_missing", "找不到客户端可执行文件"))?;
    if !metadata.is_file() || metadata.mode() & 0o111 == 0 {
        return Err(err("executable_missing", "客户端不是可执行文件"));
    }
    if metadata.uid() != unsafe { libc::geteuid() }
        && !(metadata.uid() == 0 && metadata.mode() & 0o022 == 0)
    {
        return Err(err(
            "wrong_owner",
            "客户端应属于当前用户，或为不可由其他用户改写的 root 安装",
        ));
    }
    Ok(json!({
        "inode": metadata.ino(),
        "device": metadata.dev(),
        "size": metadata.size(),
        "mtime": metadata.mtime(),
        "mtime_nsec": metadata.mtime_nsec(),
        "ctime": metadata.ctime(),
        "ctime_nsec": metadata.ctime_nsec(),
        "mode": metadata.mode(),
        "owner": metadata.uid(),
    }))
}

/// Normalize and bound the caller's input reference. This is bounded metadata
/// naming where the continuation text conceptually came from — never a body and
/// never a secret. Unknown shapes are a specific validation error.
fn normalize_input_reference(value: Option<&Value>) -> Result<Value> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    let object = value
        .as_object()
        .ok_or_else(|| err("invalid_reference", "input_reference 必须是对象"))?;
    for key in object.keys() {
        if key != "files" {
            return Err(err("invalid_reference", "input_reference 只接受 files"));
        }
    }
    let files = object
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| err("invalid_reference", "input_reference.files 必须是数组"))?;
    if files.len() > 256 {
        return Err(err("invalid_reference", "input_reference 条目超过上限"));
    }
    let mut out = vec![];
    for entry in files {
        let object = entry
            .as_object()
            .ok_or_else(|| err("invalid_reference", "files 条目必须是对象"))?;
        if object
            .keys()
            .any(|key| !["path", "digest", "package_digest", "index"].contains(&key.as_str()))
        {
            return Err(err(
                "invalid_reference",
                "files 条目只接受 path、digest、package_digest、index",
            ));
        }
        let path = string(entry, "path")?;
        if path.is_empty() || path.len() > 4096 || path.chars().any(|c| c.is_control()) {
            return Err(err("invalid_reference", "path 必须是有限的文件引用"));
        }
        let digest = string(entry, "digest")?;
        let valid_digest = |d: &str| {
            d.len() == 64
                && d.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if !valid_digest(digest) {
            return Err(err("invalid_reference", "digest 需要 64 位小写 hex"));
        }
        let package_digest = match entry.get("package_digest") {
            None => digest,
            Some(value) => value
                .as_str()
                .filter(|d| valid_digest(d))
                .ok_or_else(|| err("invalid_reference", "package_digest 需要 64 位小写 hex"))?,
        };
        let index = match entry.get("index") {
            None => 0,
            Some(value) => value
                .as_u64()
                .filter(|n| *n <= 1_000_000)
                .ok_or_else(|| err("invalid_reference", "index 必须是 0 到 1000000 的整数"))?,
        };
        out.push(json!({
            "path": path,
            "digest": digest,
            "package_digest": package_digest,
            "index": index,
        }));
    }
    Ok(json!({"files": out}))
}

impl Engine {
    /// Readonly query for one frozen launch/resume request. It reads the durable
    /// record (or, before any attempt, the frozen plan) and returns only
    /// metadata. It never takes an approval or passphrase, never opens a
    /// Terminal and never starts anything, so the App can find the original
    /// startup request after a lost reply or a restart.
    pub(crate) fn launch_query(&self, r: &Value) -> Result<Value> {
        let rid = safe_id(r, "request_id")?;
        let record_path = self.state.join("launches").join(format!("{rid}.json"));
        if record_path.exists() {
            let record = load(&record_path)?;
            // Allowlist projection: never return the whole record, so any future
            // private field (or a secret accidentally added) cannot leak.
            let mut view = json!({
                "status": record["status"],
                "observed": "record",
                "request_id": record["request_id"],
                "environment_id": record["environment_id"],
                "project_cwd": record["project_cwd"],
                "root": record["root"],
                "executable": record["executable"],
                "mode": record["mode"],
                "phase": record["phase"],
                "private_copy_path": record["private_copy_path"],
                "recorded_at": record["recorded_at"],
                "replayed_query": true,
            });
            if record["error"].is_object() {
                view["error"] = json!({
                    "code": record["error"]["code"],
                    "message": record["error"]["message"],
                });
            }
            if record["message"].is_string() {
                view["message"] = record["message"].clone();
            }
            return Ok(view);
        }
        // No attempt yet: fall back to the frozen plan metadata (still no
        // side effect, no approval required).
        let plan_path = self.path("plans", &rid);
        if plan_path.exists() {
            let plan = load(&plan_path)?;
            if plan["kind"] == "launch" {
                let request = &plan["extra"]["launch_request"];
                return Ok(json!({
                    "status": "planned",
                    "observed": "plan",
                    "request_id": rid,
                    "environment_id": plan["environment_id"],
                    "project_cwd": request["project_cwd"],
                    "root": request["config_root"],
                    "executable": request["executable"],
                    "mode": request["mode"],
                    "created_at": plan["created_at"],
                    "message": "尚未请求启动；可批准原计划或继续核对。",
                }));
            }
            if plan["kind"] == "resume" {
                let resume = &plan["extra"]["resume"];
                return Ok(json!({
                    "status": "planned",
                    "observed": "plan",
                    "request_id": rid,
                    "environment_id": plan["environment_id"],
                    "project_cwd": resume["project_cwd"],
                    "root": resume["config_root"],
                    "executable": resume["executable"],
                    "mode": "resume",
                    "supported": resume["supported"],
                    "private_copy_path": resume["private_copy_path"],
                    "created_at": plan["created_at"],
                    "message": "尚未请求续聊；可批准原计划或改用阅读入口。",
                }));
            }
        }
        Err(err("launch_not_found", "没有找到此启动请求或任务记录"))
    }

    /// Readonly list of durable launch/resume request records. Metadata only.
    pub(crate) fn launches(&self) -> Result<Value> {
        let dir = self.state.join("launches");
        let mut out = vec![];
        if dir.is_dir() {
            for entry in std::fs::read_dir(&dir)? {
                let path = entry?.path();
                if path.extension().is_some_and(|ext| ext == "json") {
                    let Ok(record) = load(&path) else { continue };
                    out.push(json!({
                        "request_id": record["request_id"],
                        "environment_id": record["environment_id"],
                        "status": record["status"],
                        "mode": record["mode"],
                        "root": record["root"],
                        "project_cwd": record["project_cwd"],
                        "executable": record["executable"],
                        "recorded_at": record["recorded_at"],
                    }));
                }
            }
        }
        out.sort_by(|a, b| b["recorded_at"].as_str().cmp(&a["recorded_at"].as_str()));
        Ok(json!({"launches": out}))
    }

    fn launch_environment(&self, r: &Value) -> Result<(Value, PathBuf, String)> {
        let e = self.env(r)?;
        if e["status"] == "retired" {
            return Err(err("environment_retired", "请先恢复此环境登记再启动"));
        }
        let root = PathBuf::from(string(&e, "root")?);
        guard(&root)?;
        // Static discovery precedence is unchanged: PATH first, then this user's
        // native install. An environment's recorded executable reflects that same
        // resolution and is re-derived here, so a stale label cannot select a
        // different binary than discovery would.
        let executable = self.executable().ok_or_else(|| {
            err(
                "executable_missing",
                "没有找到 Claude Code，请安装后重新检查",
            )
        })?;
        Ok((e, root, executable))
    }

    fn client_version(&self, executable: &str) -> Value {
        policy::product(Some(executable))
            .get("version")
            .cloned()
            .unwrap_or(Value::Null)
    }

    /// The registered environment named by a frozen launch/resume plan must
    /// still exist, must not be retired, and its current root must equal the
    /// frozen config root. A change here invalidates the approved target.
    fn check_frozen_environment(&self, environment_id: &Value, frozen_root: &Value) -> Result<()> {
        let e = self
            .env(&json!({"environment_id": environment_id}))
            .map_err(|_| err("stale_plan", "批准的环境登记已变化，请重新预览"))?;
        if e["status"] == "retired" {
            return Err(err("stale_plan", "批准的环境已退役，请重新预览或恢复登记"));
        }
        if e["root"] != *frozen_root {
            return Err(err("stale_plan", "批准环境的配置 root 已变化，请重新预览"));
        }
        Ok(())
    }

    /// Plan-only finite launch request. Freezes the exact environment/root/
    /// executable/static version/project cwd/mode/input reference. The request
    /// ID is the plan ID, so one immutable ID resolves plan -> launch -> query.
    pub(crate) fn plan_launch(&self, r: &Value) -> Result<Value> {
        let mode = string(r, "mode")?;
        if mode != "interactive" {
            return Err(err("unsupported_mode", "本轮只支持 interactive 交互启动"));
        }
        let project = check_project_cwd(string(r, "project_cwd")?)?;
        let (e, root, executable) = self.launch_environment(r)?;
        let input_reference = normalize_input_reference(r.get("input_reference"))?;
        let proxy = self.check_proxy(r.get("proxy_url").and_then(Value::as_str))?;
        // Freeze the exact target identity at preview: the config root directory
        // object and the executable object. Execution rechecks both so a
        // same-path replacement (even same version string) is stale before any
        // Terminal attempt.
        let root_identity = dir_identity(&root)?;
        let executable_identity = exec_identity(Path::new(&executable))?;
        let project_identity = project_identity(&project)?;
        let product = policy::product(Some(&executable));
        let startup = crate::components::startup::freeze(&self.home, &root, &project)?;
        let request_id = id();
        let launch_request = json!({
            "id": request_id,
            "environment_id": e["id"],
            "project_cwd": project,
            "project_cwd_identity": project_identity,
            "config_root": root,
            "config_root_identity": root_identity,
            "executable": executable,
            "executable_identity": executable_identity,
            "product": product,
            "client_version": self.client_version(&executable),
            "mode": mode,
            "input_reference": input_reference,
            "proxy_url": proxy,
            "created_at": now(),
            "startup": startup.view,
        });
        let mut plan = self.plan(
            &e,
            "launch",
            "开始新会话",
            json!([]),
            vec!["配置文件与项目目录本身", "用户未选择的历史内容"],
            json!([{"id":"launch","label":"在准确的项目目录与配置环境中启动交互会话","reversible":false}]),
            json!({"launch_request":launch_request,"mode":mode,"startup_binding":startup.binding}),
        )?;
        // Make the frozen request ID identical to the plan ID, then re-hash so
        // approval still binds the exact frozen target.
        let plan_id = string(&plan, "id")?.to_string();
        let stored = load(&self.path("plans", &plan_id))?;
        let mut stored = stored;
        stored["extra"]["launch_request"]["id"] = json!(plan_id);
        stored.as_object_mut().unwrap().remove("hash");
        stored["hash"] = json!(digest(&serde_json::to_vec(&stored)?));
        save(&self.path("plans", &plan_id), &stored)?;
        plan["launch_request"]["id"] = json!(plan_id);
        plan["hash"] = stored["hash"].clone();
        Ok(plan)
    }

    /// Validate a loopback HTTP proxy URL and return its normalized form. The
    /// value is recorded in the frozen launch request and reused verbatim.
    fn check_proxy(&self, raw: Option<&str>) -> Result<Value> {
        let Some(url) = raw else {
            return Ok(Value::Null);
        };
        if url.is_empty() {
            return Ok(Value::Null);
        }
        let rest = url
            .strip_prefix("http://")
            .ok_or_else(|| err("invalid_proxy", "只接受 loopback HTTP 通道"))?;
        let addr: std::net::SocketAddr = rest
            .parse()
            .map_err(|_| err("invalid_proxy", "通道地址无效"))?;
        if !addr.ip().is_loopback() || addr.port() == 0 {
            return Err(err("invalid_proxy", "只接受已监听的 loopback 通道"));
        }
        Ok(json!(format!("http://{addr}")))
    }

    /// Execute a frozen launch plan. Persists durable intent and serializes
    /// same-ID concurrency before any Terminal attempt; an attempt whose outcome
    /// is uncertain is never repeated — the caller queries the original request.
    pub(crate) fn launch_request(&self, r: &Value) -> Result<Value> {
        let rid = safe_id(r, "request_id")?;
        private_dir(&self.state.join("launches"))?;
        let _held = lock(&self.state.join("launches").join(format!("{rid}.lock")))?;
        let result_path = self.state.join("launches").join(format!("{rid}.json"));
        if result_path.exists() {
            return self.launch_query(r);
        }
        let path = self.path("plans", &rid);
        if !path.exists() {
            return Err(err(
                "launch_not_found",
                "没有找到此启动请求；请先生成有限启动计划",
            ));
        }
        let plan = load(&path)?;
        if !plan.is_object() || plan["kind"] != "launch" {
            return Err(err("invalid_launch", "该 ID 不是启动请求"));
        }
        if r["approval"] != plan["hash"] {
            return Err(err("approval_mismatch", "授权与当前启动预览不一致"));
        }
        let mut unhashed = plan.clone();
        unhashed.as_object_mut().unwrap().remove("hash");
        if plan["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
            return Err(err("plan_changed", "启动请求已变化，需要重新预览"));
        }
        // Serialize same-ID launches under the launch lock; a second caller waits
        // and observes the recorded result instead of opening another session.
        let request = &plan["extra"]["launch_request"];
        self.check_frozen_environment(&plan["environment_id"], &request["config_root"])?;
        // Re-check the whole frozen target before any side effect: project dir,
        // config root identity, executable identity and product/version. Any
        // mismatch is a stale plan, not a new launch.
        let project = check_project_cwd(
            request["project_cwd"]
                .as_str()
                .ok_or_else(|| err("invalid_launch", "启动请求缺少项目目录"))?,
        )?;
        if project_identity(&project)? != request["project_cwd_identity"] {
            return Err(err(
                "stale_plan",
                "项目工作目录的实际对象在预览后改变，请重新预览",
            ));
        }
        let root = PathBuf::from(string(request, "config_root")?);
        guard(&root)?;
        if dir_identity(&root)? != request["config_root_identity"] {
            return Err(err(
                "stale_plan",
                "配置 root 的实际对象在预览后改变，请重新预览",
            ));
        }
        let executable = string(request, "executable")?;
        if Some(executable) != self.executable().as_deref() {
            return Err(err(
                "executable_changed",
                "启动来源在预览后改变，请重新预览",
            ));
        }
        if exec_identity(Path::new(executable))? != request["executable_identity"] {
            return Err(err(
                "stale_plan",
                "客户端可执行文件的实际对象在预览后改变，请重新预览",
            ));
        }
        if policy::product(Some(executable)) != request["product"] {
            return Err(err(
                "stale_product",
                "客户端产品/版本元数据在预览后改变，请重新预览",
            ));
        }
        let proxy = request["proxy_url"].as_str();
        crate::components::startup::recheck(
            &self.home,
            &root,
            &project,
            &plan["extra"]["startup_binding"],
        )?;
        // Durable intent BEFORE the Terminal attempt.
        let mut record = json!({
            "status": "launch_intent",
            "phase": "opening_terminal",
            "request_id": rid,
            "environment_id": plan["environment_id"],
            "project_cwd": project,
            "root": root,
            "executable": executable,
            "mode": request["mode"],
            "recorded_at": now(),
        });
        save(&result_path, &record)?;
        // The Terminal attempt is the sole side effect. Whether it succeeds or
        // fails, the outcome is recorded so a repeat can only query it and can
        // never open a second session.
        let outcome = match self.terminal_launch(&rid, &root, &project, executable, proxy) {
            Ok(outcome) => outcome,
            Err(failure) => {
                record["status"] = json!("launch_failed");
                record["error"] = json!({"code": failure.code, "message": failure.message});
                record["message"] = json!("启动请求未成功；请核对原启动请求，不自动重试。");
                save(&result_path, &record)?;
                return Err(failure);
            }
        };
        record["status"] = outcome["status"].clone();
        record["message"] = outcome["message"].clone();
        record["project_cwd"] = json!(project);
        record["root"] = json!(root);
        save(&result_path, &record)?;
        record["replayed_query"] = json!(false);
        Ok(record)
    }

    /// Native resume adapter plan (SPEC §8). Distinct from archive/import/new
    /// context: it prepares a private running copy of one archived transcript and
    /// reports finite support facts from static evidence only. The original
    /// archive is never handed to the client and never modified.
    pub(crate) fn plan_resume(&self, r: &Value) -> Result<Value> {
        let (e, root, executable) = self.launch_environment(r)?;
        let project = check_project_cwd(string(r, "project_cwd")?)?;
        let opened = self.open_archive(r)?;
        let name = string(r, "path")?;
        let entry = opened.entry(name)?;
        let transcript_digest = entry.digest.clone();
        let format = assess_resume_format(
            name,
            &entry.plain,
            entry.bytes,
            policy::product(Some(&executable))
                .get("version")
                .and_then(Value::as_str),
        );
        let archive_digest = opened.cipher_digest.clone();
        let source_path = opened.path.clone();
        let product = policy::product(Some(&executable));
        let root_identity = dir_identity(&root)?;
        let executable_identity = exec_identity(Path::new(&executable))?;
        let project_identity = project_identity(&project)?;
        let private_copy = self.state.join("resume").join(format!("{}.jsonl", id()));
        let startup = crate::components::startup::freeze(&self.home, &root, &project)?;
        let resume = json!({
            "supported": format.support.supported(),
            "reason": format.support.reason(),
            "mode": "transcript_path",
            "source": {"archive_path": source_path, "archive_digest": archive_digest},
            "transcript_path": name,
            "transcript_digest": transcript_digest,
            "private_copy_path": private_copy,
            "config_root": root,
            "config_root_identity": root_identity,
            "project_cwd": project,
            "project_cwd_identity": project_identity,
            "executable": executable,
            "executable_identity": executable_identity,
            "product": product,
            "client_version": self.client_version(&executable),
            "client_support": {
                "evidence": "static_metadata",
                "resume_entry": "transcript_path",
                "format": format.format,
                "inference": format.inference,
                "observed": format.observed,
                "declared_versions": SUPPORTED_RESUME_VERSIONS,
                "sources": [
                    {"url": "https://code.claude.com/docs/en/cli-reference", "checked": "2026-10-05"},
                    {"url": "https://code.claude.com/docs/en/sessions", "checked": "2026-10-05"}
                ],
                "notes": "官方资料说明 --resume 接受绝对 .jsonl 会话路径，--fork-session 生成新 ID；transcript 存放于 CLAUDE_CONFIG_DIR，恢复会重读当前设置。未 fork 的恢复可能与既有会话交错；客户端仍可能按内部路径读取原项目/subagent 会话，私有副本不构成隔离保证。本候选只声明上述静态已知版本，且不证明认证状态或原会话是否运行。"
            },
            "archive_unmodified": true,
            "auth_unverified": true,
            "attach_risk": "unverified",
            "policy_version": RESUME_POLICY_VERSION,
            "write_scope": {
                "config_root": root,
                "project_cwd": project,
                "private_copy_path": private_copy,
                "may_read_original_paths": true,
                "note": "客户端可能按自身规则更新所选配置环境中的运行状态，并可能按内部索引读取原项目/subagent 会话；私有副本不构成隔离保证，归档原件不交给客户端写入。",
            },
            "format": format.format,
            "startup": startup.view,
        });
        self.plan(
            &e,
            "resume",
            "原生续聊",
            json!([]),
            vec!["归档原件字节", "本机认证与登录状态"],
            json!([{"id":"resume","label":"用私有运行副本进行有限原生续聊；不修改归档原件","reversible":false}]),
            json!({"resume":resume,"archive_passphrase_required":true,"archive_path":source_path,"transcript_path":name,"startup_binding":startup.binding}),
        )
    }

    /// Execute a frozen resume plan: write the private running copy from the
    /// archived bytes (byte-recheck, 0600) and record the concrete finite facts.
    /// The archive original is never modified. Authentication stays unverified.
    pub(crate) fn resume_request(&self, r: &Value) -> Result<Value> {
        let rid = safe_id(r, "request_id")?;
        private_dir(&self.state.join("launches"))?;
        let _held = lock(&self.state.join("launches").join(format!("{rid}.lock")))?;
        let result_path = self.state.join("launches").join(format!("{rid}.json"));
        if result_path.exists() {
            return self.launch_query(r);
        }
        let path = self.path("plans", &rid);
        if !path.exists() {
            return Err(err(
                "launch_not_found",
                "没有找到此续聊请求；请先生成有限续聊计划",
            ));
        }
        let plan = load(&path)?;
        if plan["kind"] != "resume" {
            return Err(err("invalid_launch", "该 ID 不是续聊请求"));
        }
        if r["approval"] != plan["hash"] {
            return Err(err("approval_mismatch", "授权与当前续聊预览不一致"));
        }
        // Original-record-first: under the same-ID lock, an already-attempted
        // request is answered from its durable record with no approval re-run, no
        // passphrase, no live executable/source read and no Terminal. This is the
        // query-only path used after a lost reply or an App restart.
        // The frozen plan must still hash to its recorded approval: a tampered
        // stored plan cannot be executed even if the caller echoes the old hash.
        let mut unhashed = plan.clone();
        unhashed.as_object_mut().unwrap().remove("hash");
        if plan["hash"] != digest(&serde_json::to_vec(&unhashed)?) {
            return Err(err("plan_changed", "续聊预览已变化，请重新预览"));
        }
        let resume = &plan["extra"]["resume"];
        self.check_frozen_environment(&plan["environment_id"], &resume["config_root"])?;
        let name = string(resume, "transcript_path")?.to_string();
        // Recheck the frozen target identity + executable identity + product
        // metadata before any side effect, exactly like a plain launch.
        let project = check_project_cwd(
            resume["project_cwd"]
                .as_str()
                .ok_or_else(|| err("invalid_launch", "续聊请求缺少项目目录"))?,
        )?;
        if project_identity(&project)? != resume["project_cwd_identity"] {
            return Err(err(
                "stale_plan",
                "项目工作目录的实际对象在预览后改变，请重新预览",
            ));
        }
        let root = PathBuf::from(string(resume, "config_root")?);
        guard(&root)?;
        if dir_identity(&root)? != resume["config_root_identity"] {
            return Err(err(
                "stale_plan",
                "配置 root 的实际对象在预览后改变，请重新预览",
            ));
        }
        let executable = string(resume, "executable")?.to_string();
        if Some(executable.as_str()) != self.executable().as_deref() {
            return Err(err(
                "executable_changed",
                "启动来源在预览后改变，请重新预览",
            ));
        }
        if exec_identity(Path::new(&executable))? != resume["executable_identity"] {
            return Err(err(
                "stale_plan",
                "客户端可执行文件的实际对象在预览后改变，请重新预览",
            ));
        }
        if policy::product(Some(&executable)) != resume["product"] {
            return Err(err(
                "stale_product",
                "客户端产品/版本元数据在预览后改变，请重新预览",
            ));
        }
        // A plan frozen under a different adapter-policy revision is stale before
        // its first launch, so a code/policy change cannot silently reuse it.
        if resume["policy_version"] != RESUME_POLICY_VERSION {
            return Err(err(
                "stale_plan",
                "续聊适配器策略已更新，请重新预览后再启动",
            ));
        }
        let supported = resume["supported"] == true;
        if !supported {
            return Err(err(
                "resume_unsupported",
                "该组合暂不支持原生续聊；可改用阅读或提取上下文入口",
            ));
        }
        crate::components::startup::recheck(
            &self.home,
            &root,
            &project,
            &plan["extra"]["startup_binding"],
        )?;
        // Re-read and re-verify the exact transcript bytes from the frozen source.
        let source = json!({
            "archive_path": resume["source"]["archive_path"],
            "archive_passphrase": r["archive_passphrase"],
        });
        let opened = self.open_archive(&source)?;
        let digest_now = opened.cipher_digest.clone();
        if resume["source"]["archive_digest"].as_str() != Some(digest_now.as_str()) {
            return Err(err("stale_archive", "工作包在预览后改变，请重新预览"));
        }
        let entry = opened.entry(&name)?;
        if entry.digest != string(resume, "transcript_digest")? {
            return Err(err("stale_archive", "会话文件在预览后改变，请重新预览"));
        }
        let copy = Path::new(string(resume, "private_copy_path")?);
        guard(copy)?;
        if !copy.is_absolute() {
            return Err(err("invalid_path", "私有运行副本路径必须是绝对路径"));
        }
        if let Some(parent) = copy.parent() {
            private_dir(parent)?;
        }
        // Durable publication intent BEFORE the copy and the Terminal attempt.
        let mut record = json!({
            "status": "resume_intent",
            "phase": "preparing_private_copy",
            "request_id": rid,
            "environment_id": plan["environment_id"],
            "mode": "resume",
            "private_copy_path": copy,
            "root": root,
            "project_cwd": project,
            "executable": executable,
            "client_version": resume["client_version"],
            "archive_digest": digest_now,
            "recorded_at": now(),
        });
        save(&result_path, &record)?;
        let prepared = (|| -> Result<()> {
            // Stream-copy the verified plaintext into the private running copy
            // and read it back, hashing both, so a large transcript is never
            // loaded whole.
            crate::archive::copy_staged_verified(&entry.plain, copy, &entry.digest)
        })();
        if let Err(failure) = prepared {
            record["status"] = json!("launch_failed");
            record["error"] = json!({"code":failure.code,"message":failure.message});
            record["message"] = json!("私有运行副本未完成；保留原 ID 和确切路径，不启动或重试。");
            save(&result_path, &record)?;
            return Err(failure);
        }
        record["phase"] = json!("opening_terminal");
        record["status"] = json!("resume_prepared");
        save(&result_path, &record)?;
        // The Finite native entry: `--resume <abs-copy> --fork-session`. Only the
        // validated absolute copy path and fixed flags are passed; the archive
        // original is untouched and no ID/signature is rewritten.
        let extra_args = vec![
            "--resume".to_string(),
            copy.to_string_lossy().into_owned(),
            "--fork-session".to_string(),
        ];
        let outcome = match self.terminal_launch_args(
            &rid,
            &root,
            &project,
            &executable,
            None,
            &extra_args,
        ) {
            Ok(outcome) => outcome,
            Err(failure) => {
                record["status"] = json!("launch_failed");
                record["error"] = json!({"code": failure.code, "message": failure.message});
                save(&result_path, &record)?;
                return Err(failure);
            }
        };
        record["status"] = outcome["status"].clone();
        record["message"] = outcome["message"].clone();
        save(&result_path, &record)?;
        record["replayed_query"] = json!(false);
        Ok(record)
    }
}

struct ResumeFormat {
    support: ResumeSupport,
    format: &'static str,
    inference: Value,
    /// Fields observed in the transcript (sessionId / cwd / uuid presence) so a
    /// caller can see the concrete shape, not just a boolean.
    observed: Value,
}

/// Bounded, conservative assessment of whether one archived member is a Claude
/// session transcript we can hand to the finite native adapter, given the
/// declared client version. This is a *format + policy* check on the archived
/// bytes and static version metadata, not a claim of authentication or actual
/// client support.
/// Read at most `limit` bytes from the head of a file (bounded; fewer only at
/// EOF). Used for the resume-format prefix scan.
fn read_prefix(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buf = vec![0u8; limit.min(usize::MAX as u64) as usize];
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(ref e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(err("archive_read_failed", "无法读取会话文件")),
        }
    }
    buf.truncate(filled);
    Ok(buf)
}

fn assess_resume_format(
    name: &str,
    plain: &Path,
    size: u64,
    client_version: Option<&str>,
) -> ResumeFormat {
    if !name.ends_with(".jsonl") {
        return ResumeFormat {
            support: ResumeSupport::Unsupported(
                "不是 .jsonl 会话文件；此组合不受支持，可改用阅读或提取上下文入口",
            ),
            format: "unknown",
            inference: json!("文件扩展名不属于受支持的原生会话格式"),
            observed: json!({}),
        };
    }
    // Read at most `RESUME_SCAN_BYTES` from the head of the transcript: a huge
    // session is assessed from a bounded prefix, and the scan is marked
    // truncated rather than loading the whole file.
    let prefix = match read_prefix(plain, RESUME_SCAN_BYTES) {
        Ok(prefix) => prefix,
        Err(_) => {
            return ResumeFormat {
                support: ResumeSupport::Unsupported("无法读取会话文件内容"),
                format: "unknown",
                inference: json!("读取归档会话文件失败"),
                observed: json!({}),
            }
        }
    };
    let truncated_scan = size > prefix.len() as u64;
    let text = String::from_utf8_lossy(&prefix);
    let mut recognized = 0usize;
    let mut total = 0usize;
    let mut has_session_id = false;
    let mut has_cwd = false;
    let mut has_uuid = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        if let Ok(record) = serde_json::from_str::<Value>(line) {
            has_session_id |= record.get("sessionId").and_then(Value::as_str).is_some();
            has_cwd |= record.get("cwd").and_then(Value::as_str).is_some();
            has_uuid |= record.get("uuid").and_then(Value::as_str).is_some();
            if matches!(
                record.get("type").and_then(Value::as_str),
                Some("user" | "assistant" | "summary" | "system")
            ) {
                recognized += 1;
            }
        }
    }
    let observed = json!({
        "recognized_records": recognized,
        "total_records": total,
        "sessionId": has_session_id,
        "cwd": has_cwd,
        "uuid": has_uuid,
        "truncated_scan": truncated_scan,
        "scanned_bytes": prefix.len(),
        "total_bytes": size,
    });
    // A transcript larger than the bounded scan cannot be verified in full for
    // the fields resume relies on, so it is explicitly unsupported rather than
    // approved from a partial observation.
    if truncated_scan {
        return ResumeFormat {
            support: ResumeSupport::Unsupported(
                "会话文件超过可核对的原生续聊容量；本候选不从部分内容推断可恢复，请改用阅读入口",
            ),
            format: "claude-session-jsonl",
            inference: json!("超出有界核对范围；拒绝从截断内容推断原生续聊支持"),
            observed,
        };
    }
    if total == 0 {
        return ResumeFormat {
            support: ResumeSupport::Unsupported("会话文件为空，没有可恢复的记录"),
            format: "empty",
            inference: json!("未发现任何 JSONL 记录"),
            observed,
        };
    }
    if recognized == 0 {
        return ResumeFormat {
            support: ResumeSupport::Unsupported(
                "记录类型不在已知 Claude 会话格式内；未识别内容保持 unknown，不作恢复",
            ),
            format: "unknown-records",
            inference: json!("所有记录均无法按已知会话类型识别"),
            observed,
        };
    }
    // The declared version is required. Unknown/absent versions are never
    // assumed supported, so an unrecognized client cannot silently resume.
    let Some(version) = client_version else {
        return ResumeFormat {
            support: ResumeSupport::Unsupported(
                "无法从静态安装元数据确定客户端版本；不假设支持原生续聊",
            ),
            format: "claude-session-jsonl",
            inference: json!("缺少静态版本证据；版本未知时保守判为不支持"),
            observed,
        };
    };
    if !SUPPORTED_RESUME_VERSIONS.contains(&version) {
        return ResumeFormat {
            support: ResumeSupport::Unsupported("此客户端版本不在本候选声明支持的原生续聊范围内"),
            format: "claude-session-jsonl",
            inference: json!(format!(
                "版本 {version} 不在声明集合 {SUPPORTED_RESUME_VERSIONS:?}；不据此开启原生续聊"
            )),
            observed,
        };
    }
    // A recognized transcript for a declared version still needs the fields the
    // official resume path relies on; a malformed/foreign shape is unsupported.
    if !has_session_id || !has_cwd {
        return ResumeFormat {
            support: ResumeSupport::Unsupported(
                "会话记录缺少 sessionId 或 cwd；形状不匹配已知原生续聊输入",
            ),
            format: "claude-session-shape-mismatch",
            inference: json!("声明的版本可用，但会话缺少必需字段；保守判为不支持"),
            observed,
        };
    }
    ResumeFormat {
        support: ResumeSupport::Supported,
        format: "claude-session-jsonl",
        inference: json!(format!(
            "识别 {recognized}/{total} 条已知会话记录类型，版本 {version} 在声明集合内；仍不证明认证状态或原会话是否运行"
        )),
        observed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn directory_generation_distinguishes_reuse_without_freezing_project_contents() {
        let fixture = tempfile::tempdir().unwrap();
        let project = fixture.path().join("project");
        std::fs::create_dir(&project).unwrap();
        let original = project_identity(&project).unwrap();
        std::fs::write(project.join("draft.txt"), "new ordinary project file").unwrap();
        assert_eq!(project_identity(&project).unwrap(), original);
        assert_eq!(dir_identity(&project).unwrap(), original);
        std::fs::remove_file(project.join("draft.txt")).unwrap();
        std::fs::remove_dir(&project).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        std::fs::create_dir(&project).unwrap();
        let mut replacement = project_identity(&project).unwrap();
        // Model immediate inode reuse while retaining the real new birth time.
        replacement["inode"] = original["inode"].clone();
        assert_ne!(replacement, original);
    }
    #[test]
    fn project_cwd_is_never_created() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let missing = base.join("does-not-exist");
        let error = check_project_cwd(missing.to_str().unwrap()).unwrap_err();
        assert_eq!(error.code, "project_missing");
        assert!(!missing.exists(), "cwd check created the project directory");
    }

    #[test]
    fn project_cwd_rejects_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let real = base.join("real");
        fs::create_dir(&real).unwrap();
        let link = base.join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        assert_eq!(
            check_project_cwd(link.to_str().unwrap()).unwrap_err().code,
            "symlink_target"
        );
    }

    #[test]
    fn input_reference_is_bounded_metadata_only() {
        let ok = json!({"files":[{"path":"projects/x/s.jsonl","digest":"a".repeat(64),"package_digest":"b".repeat(64),"index":3}]});
        let normalized = normalize_input_reference(Some(&ok)).unwrap();
        assert_eq!(normalized["files"][0]["index"], 3);
        assert_eq!(normalized["files"][0]["package_digest"], "b".repeat(64));
        assert!(
            normalize_input_reference(Some(&json!({"files":[{"path":"p","digest":"short"}]})))
                .is_err()
        );
        assert!(normalize_input_reference(Some(&json!({"body":"secret text"}))).is_err());
        assert_eq!(normalize_input_reference(None).unwrap(), Value::Null);
    }

    #[test]
    fn reference_fields_cannot_carry_body_or_unbounded_values() {
        for entry in [
            json!({"path":"p","digest":"a".repeat(64),"body":"secret"}),
            json!({"path":"p","digest":"a".repeat(64),"package_digest":"secret"}),
            json!({"path":"p","digest":"a".repeat(64),"index":-1}),
            json!({"path":"p","digest":"a".repeat(64),"index":1000001}),
            json!({"path":"p\nbody","digest":"a".repeat(64)}),
        ] {
            assert!(normalize_input_reference(Some(&json!({"files":[entry]}))).is_err());
        }
        assert!(exec_identity(Path::new("/usr/bin/true")).is_ok());
    }
    #[test]
    fn resume_format_rejects_unknown_and_accepts_known_declared_version() {
        // A recognized transcript with the fields the resume path relies on and a
        // declared version is a supported candidate.
        let temp = tempfile::tempdir().unwrap();
        let write = |name: &str, bytes: &[u8]| {
            let path = temp.path().join(name);
            std::fs::write(&path, bytes).unwrap();
            (path, bytes.len() as u64)
        };
        let known = br#"{"type":"user","sessionId":"s","cwd":"/p","uuid":"u","message":{"content":[{"type":"text","text":"hi"}]}}"#;
        let (known_path, known_len) = write("s.jsonl", known);
        assert!(assess_resume_format(
            "projects/x/s.jsonl",
            &known_path,
            known_len,
            Some("2.1.285")
        )
        .support
        .supported());
        // A recognized shape but an unknown/absent version is never assumed.
        assert!(
            !assess_resume_format("projects/x/s.jsonl", &known_path, known_len, None)
                .support
                .supported()
        );
        assert!(
            !assess_resume_format("projects/x/s.jsonl", &known_path, known_len, Some("2.0.0"))
                .support
                .supported()
        );
        // Missing required fields (sessionId/cwd) is unsupported even for a
        // declared version.
        let no_fields = br#"{"type":"user","message":{"content":[{"type":"text","text":"hi"}]}}"#;
        let (no_fields_path, no_fields_len) = write("no-fields.jsonl", no_fields);
        assert!(!assess_resume_format(
            "projects/x/s.jsonl",
            &no_fields_path,
            no_fields_len,
            Some("2.1.285")
        )
        .support
        .supported());
        // Non-.jsonl, unknown record types and empty content are unsupported.
        let unknown = b"{\"type\":\"weird\"}\n";
        let (unknown_path, unknown_len) = write("unknown.jsonl", unknown);
        assert!(!assess_resume_format(
            "projects/x/s.jsonl",
            &unknown_path,
            unknown_len,
            Some("2.1.285")
        )
        .support
        .supported());
        assert!(
            !assess_resume_format("notes.txt", &known_path, known_len, Some("2.1.285"))
                .support
                .supported()
        );
        let (empty_path, _) = write("empty.jsonl", b"");
        assert!(
            !assess_resume_format("projects/x/s.jsonl", &empty_path, 0, Some("2.1.285"))
                .support
                .supported()
        );
        // A transcript larger than the bounded scan is explicitly unsupported.
        let (big_path, big_len) = write("big.jsonl", known);
        assert!(!assess_resume_format(
            "projects/x/s.jsonl",
            &big_path,
            big_len + RESUME_SCAN_BYTES,
            Some("2.1.285"),
        )
        .support
        .supported());
    }
}
