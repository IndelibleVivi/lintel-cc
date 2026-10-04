//! Exact systemd service holds. The original durable quiesce plan owns restoration.
//! No sudo, arbitrary shell, global supervisor stop, or login/linger changes.
use crate::{err, now, safe_id, storage::*, string, Engine, Result};
use serde_json::{json, Value};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::{MetadataExt, OpenOptionsExt},
    path::{Path, PathBuf},
    process::Command,
};

// Only properties whose systemctl --all printer emits one line even when empty.
// v255 omits empty EnvironmentFiles and Exec* arrays entirely; query those on D-Bus.
const SHOW_PROPERTIES: &[&str] = &[
    "Id",
    "LoadState",
    "ActiveState",
    "SubState",
    "UnitFileState",
    "FragmentPath",
    "NeedDaemonReload",
    "Transient",
    "MainPID",
    "ControlGroup",
    "InvocationID",
    "Restart",
    "KillMode",
    "Delegate",
    "User",
    "PassEnvironment",
    "UnsetEnvironment",
    "RefuseManualStop",
    "RefuseManualStart",
    "RequiredBy",
    "BoundBy",
    "ConsistsOf",
    "PropagatesStopTo",
    "TriggeredBy",
    "OnFailure",
    "OnSuccess",
    "SuccessAction",
    "FailureAction",
    "StartLimitAction",
    "Job",
    "Type",
    "SourcePath",
    "Requires",
    "Requisite",
    "BindsTo",
    "Wants",
    "Upholds",
    "Conflicts",
];
// Exactly the queried v255 Unit properties using property_get_dependencies:
// each is an unordered Hashmap of units, not an ordered configuration list.
const DEPENDENCY_PROPERTIES: &[&str] = &[
    "RequiredBy",
    "BoundBy",
    "ConsistsOf",
    "PropagatesStopTo",
    "TriggeredBy",
    "OnFailure",
    "OnSuccess",
    "Requires",
    "Requisite",
    "BindsTo",
    "Wants",
    "Upholds",
    "Conflicts",
];
const COMPLEX_PROPERTIES: &[(&str, &str)] = &[
    ("EnvironmentFiles", "a(sb)"),
    ("ExecStop", "a(sasbttttuii)"),
    ("ExecStopPost", "a(sasbttttuii)"),
];
const VOLATILE: &[&str] = &[
    "ActiveState",
    "SubState",
    "MainPID",
    "ControlGroup",
    "InvocationID",
    "Job",
];

fn unit_name(unit: &str) -> Result<()> {
    if unit.len() > 240
        || !unit.ends_with(".service")
        || unit.ends_with("@.service")
        || !unit
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
        || !unit
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.@-".contains(&c))
    {
        return Err(err(
            "invalid_service_unit",
            "只接受单一完整 service unit 名称；不接受 alias、路径、template、glob 或命令",
        ));
    }
    Ok(())
}
fn manager_name(manager: &str) -> Result<()> {
    if !["user", "system"].contains(&manager) {
        return Err(err(
            "service_manager_unsupported",
            "仅支持当前 UID 的 systemd user manager，或当前 root 的 system manager",
        ));
    }
    Ok(())
}
fn bus_path(unit: &str) -> String {
    let mut out = String::from("/org/freedesktop/systemd1/unit/");
    for (i, c) in unit.bytes().enumerate() {
        if c.is_ascii_alphabetic() || (i > 0 && c.is_ascii_digit()) {
            out.push(c as char);
        } else {
            out.push_str(&format!("_{c:02x}"));
        }
    }
    out
}
fn config_snapshot(path: &Path) -> Result<Value> {
    guard(path)?;
    let m = fs::metadata(path).map_err(|_| {
        err(
            "service_restore_conflict",
            "unit 配置或 owned blocker 缺失；保留现状，查询原任务",
        )
    })?;
    if !m.is_file()
        || m.nlink() != 1
        || ![0, unsafe { libc::geteuid() }].contains(&m.uid())
        || m.mode() & 0o022 != 0
    {
        return Err(err(
            "service_config_owner",
            "unit 配置文件必须是 root 或当前用户拥有的非共享可写普通文件",
        ));
    }
    Ok(
        json!({"path":path,"digest":digest(&read(path, 1024*1024)?),"device":m.dev(),"inode":m.ino(),"mode":m.mode(),"owner":m.uid()}),
    )
}
fn config_only(properties: &Value) -> Value {
    let mut p = properties.clone();
    for key in VOLATILE {
        p.as_object_mut().unwrap().remove(*key);
    }
    p
}
fn normalize_dependencies(p: &mut Value) -> Result<()> {
    for key in DEPENDENCY_PROPERTIES {
        let text = p[key].as_str().ok_or_else(|| {
            err(
                "service_schema_unsupported",
                "systemd unit 关系属性格式未知",
            )
        })?;
        // v255's array printer separates shell_maybe_quote(unit->id) tokens
        // with an ASCII space. Valid unit IDs contain no literal whitespace;
        // keep each printed token's quoting/escapes and multiplicity intact.
        let mut members: Vec<_> = text.split(' ').collect();
        members.sort_unstable();
        p[key] = json!(members.join(" "));
    }
    Ok(())
}
fn diff_keys(expected: &Value, current: &Value) -> Vec<String> {
    let keys: std::collections::BTreeSet<_> = expected
        .as_object()
        .into_iter()
        .chain(current.as_object())
        .flat_map(|object| object.keys())
        .collect();
    keys.into_iter()
        .filter(|key| expected.get(*key) != current.get(*key))
        .cloned()
        .collect()
}
fn diagnostic_property(value: &Value) -> Value {
    match value.as_str() {
        Some(text) if text.len() > 2048 => {
            json!({"prefix":text.chars().take(2048).collect::<String>(),"bytes":text.len(),"truncated":true})
        }
        Some(_) => value.clone(),
        None if value.is_null() => Value::Null,
        None => json!({"value_withheld":"non-string property"}),
    }
}
fn diagnostic_file(file: Option<&Value>) -> Value {
    file.map(|file| {
        json!({"digest":file["digest"],"device":file["device"],"inode":file["inode"],"mode":file["mode"],"owner":file["owner"]})
    })
    .unwrap_or(Value::Null)
}
// Observability for the strict comparison only; never normalize or relax it.
// Do not export environment assignments, identities, condition arguments,
// source paths or source bytes. File indices retain their frozen source order.
fn configuration_conflict(expected: &Value, current: &Value) -> Value {
    let property_diff_keys = diff_keys(&expected["properties"], &current["properties"]);
    let mut property_values = json!({});
    let mut withheld_property_diff_keys = vec![];
    for key in &property_diff_keys {
        if (SHOW_PROPERTIES.contains(&key.as_str())
            && ![
                "User",
                "PassEnvironment",
                "UnsetEnvironment",
                "FragmentPath",
                "SourcePath",
            ]
            .contains(&key.as_str()))
            || COMPLEX_PROPERTIES.iter().any(|(name, _)| name == key)
        {
            property_values[key] = json!({"expected":diagnostic_property(&expected["properties"][key]),"current":diagnostic_property(&current["properties"][key])});
        } else {
            withheld_property_diff_keys.push(key);
        }
    }
    let expected_files = expected["files"].as_array().unwrap();
    let current_files = current["files"].as_array().unwrap();
    let mut differences = vec![];
    let mut difference_count = 0;
    for index in 0..expected_files.len().max(current_files.len()) {
        let before = expected_files.get(index);
        let after = current_files.get(index);
        if before != after {
            difference_count += 1;
            if differences.len() < 32 {
                differences.push(json!({"index":index,
                    "diff_keys":diff_keys(before.unwrap_or(&Value::Null),after.unwrap_or(&Value::Null)),
                    "expected":diagnostic_file(before),"current":diagnostic_file(after)}));
            }
        }
    }
    json!({"snapshot_diff_keys":diff_keys(expected,current),"property_diff_keys":property_diff_keys,
        "property_values":property_values,"withheld_property_diff_keys":withheld_property_diff_keys,
        "files":{"expected_count":expected_files.len(),"current_count":current_files.len(),
            "difference_count":difference_count,"differences":differences,"differences_truncated":difference_count>differences.len()}})
}
fn view(properties: &Value) -> Value {
    json!({"active_state":properties["ActiveState"],"sub_state":properties["SubState"],
        "unit_file_state":properties["UnitFileState"],"restart":properties["Restart"]})
}

fn parse_service_property(bytes: &[u8], signature: &str) -> Result<Value> {
    let v = parse(bytes).map_err(|_| {
        err(
            "service_schema_unsupported",
            "systemd D-Bus 属性格式不受支持；没有据此批准变更",
        )
    })?;
    // busctl get-property prints a variant's value directly in data. A method
    // reply has a different outer argument array and must not be unwrapped here.
    let data = v["data"]
        .as_array()
        .filter(|a| signature != "as" || a.iter().all(Value::is_string));
    if v["type"] != signature || data.is_none() {
        return Err(err(
            "service_schema_unsupported",
            "systemd D-Bus 属性格式不受支持；没有据此批准变更",
        ));
    }
    Ok(v["data"].clone())
}

fn read_service_properties(
    bytes: &[u8],
    mut property: impl FnMut(&str, &str, &str) -> Result<Value>,
) -> Result<Value> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| err("service_schema_unsupported", "systemd 返回了未知属性格式"))?;
    let mut p = json!({});
    for line in text.lines() {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| err("service_schema_unsupported", "systemd 返回了未知属性格式"))?;
        if !SHOW_PROPERTIES.contains(&key) || p.get(key).is_some() {
            return Err(err("service_schema_unsupported", "systemd 属性缺失或重复"));
        }
        p[key] = json!(value);
    }
    for key in SHOW_PROPERTIES {
        if p.get(key).is_none() {
            return Err(err(
                "service_schema_unsupported",
                &format!("systemd 未提供 {key}；此版本不支持精确服务维护"),
            ));
        }
    }
    for (key, signature) in COMPLEX_PROPERTIES {
        let value = property("org.freedesktop.systemd1.Service", key, signature)?;
        let array = value.as_array().ok_or_else(|| {
            err(
                "service_schema_unsupported",
                "systemd D-Bus 数组属性格式不受支持",
            )
        })?;
        // Freeze only presence: every nonempty array is unsupported, and raw
        // environment-file paths or stop argv must not enter a durable plan.
        // Keep the existing snapshot representation for supported empty arrays.
        p[key] = json!(if array.is_empty() { "" } else { "[configured]" });
    }
    p["Environment"] = property("org.freedesktop.systemd1.Service", "Environment", "as")?;
    retain_root_environment(&mut p)?;
    p["DropInPaths"] = property("org.freedesktop.systemd1.Unit", "DropInPaths", "as")?;
    p["Conditions"] = property("org.freedesktop.systemd1.Unit", "Conditions", "a(sbbsi)")?;
    // Condition status is runtime state, not part of the frozen configuration.
    normalize_conditions(&mut p)?;
    normalize_dependencies(&mut p)?;
    Ok(p)
}

fn condition_content(job_id: &str, path: &Path) -> Result<String> {
    let argument = path
        .to_str()
        .filter(|s| s.starts_with('/') && !s.chars().any(char::is_control))
        .ok_or_else(|| {
            err(
                "service_condition_path_unsupported",
                "服务 hold 的 state 路径必须是无控制字符的 UTF-8 绝对路径；未生成 blocker",
            )
        })?
        .replace('%', "%%");
    // v255 config_parse_unit_condition_path does not unquote or C-unescape.
    // Quotes, spaces and backslashes inside the absolute path are literal;
    // only percent specifiers need escaping. The fixed suffix prevents a
    // trailing backslash from becoming a config-file line continuation.
    Ok(format!("# Lintel owned hold; restore through original job {job_id}\n[Unit]\nConditionPathExists={argument}\n"))
}

impl Engine {
    fn service_platform(&self) -> Result<()> {
        #[cfg(test)]
        {
            // Unit tests never inspect the host manager, including on Linux CI.
            // Service lifecycle tests explicitly supply their synthetic adapter.
            if self.service_fixture.is_some() {
                Ok(())
            } else {
                Err(err(
                    "service_test_fixture_required",
                    "unit-test Engine 没有 synthetic service fixture；不访问真实 systemd manager",
                ))
            }
        }
        #[cfg(not(test))]
        {
            if cfg!(target_os = "linux") {
                Ok(())
            } else {
                Err(err(
                    "service_platform_unsupported",
                    "systemd 服务操作仅在 Linux runner 上可用；此主机未执行服务操作",
                ))
            }
        }
    }
    fn service_write_authority(&self, manager: &str) -> Result<()> {
        if manager == "system" && unsafe { libc::geteuid() } != 0 {
            #[cfg(test)]
            if self.service_fixture.is_some() {
                return Ok(());
            }
            return Err(err(
                "service_permission_required",
                "system manager 变更需要当前进程已是 root；Lintel 不会执行 sudo 或申请提权",
            ));
        }
        Ok(())
    }
    fn service_command(&self, tool: &str, manager: &str, args: &[&str]) -> Result<Vec<u8>> {
        self.service_platform()?;
        manager_name(manager)?;
        let executable = match tool {
            "systemctl" => "/usr/bin/systemctl",
            "busctl" => "/usr/bin/busctl",
            _ => unreachable!(),
        };
        let mut command = Command::new(executable);
        command
            .arg(format!("--{manager}"))
            .args(["--no-pager", "--no-ask-password"]);
        if tool == "busctl" {
            // busctl has a separate finite no-interactive-authorization option.
            command = Command::new(executable);
            command.arg(format!("--{manager}")).args([
                "--no-pager",
                "--allow-interactive-authorization=no",
                "--json=short",
            ]);
        }
        command
            .args(args)
            .env("LC_ALL", "C")
            .env("SYSTEMD_COLORS", "0");
        // Do not let caller-controlled bus addresses redirect the authority to another manager.
        command.env_remove("DBUS_SYSTEM_BUS_ADDRESS");
        if manager == "user" {
            command.env_remove("DBUS_SESSION_BUS_ADDRESS");
            command.env(
                "XDG_RUNTIME_DIR",
                format!("/run/user/{}", unsafe { libc::geteuid() }),
            );
        }
        let (code, bytes) = crate::cleanup::bounded(command).map_err(|_| {
            err(
                "service_manager_unavailable",
                "systemd 命令不可用、超时或返回过大；查询现状，不自动重发变更",
            )
        })?;
        if code != 0 {
            return Err(err(
                "service_command_failed",
                &format!(
                    "systemd {tool} {} 未完成；确认 manager、权限和原任务状态，不自动重试",
                    args.first().copied().unwrap_or("query")
                ),
            ));
        }
        Ok(bytes)
    }
    fn service_property(
        &self,
        manager: &str,
        unit: &str,
        interface: &str,
        property: &str,
        signature: &str,
    ) -> Result<Value> {
        let path = if unit.is_empty() {
            "/org/freedesktop/systemd1".to_string()
        } else {
            bus_path(unit)
        };
        let bytes = self.service_command(
            "busctl",
            manager,
            &[
                "get-property",
                "org.freedesktop.systemd1",
                &path,
                interface,
                property,
            ],
        )?;
        parse_service_property(&bytes, signature)
    }
    fn service_properties(&self, manager: &str, unit: &str) -> Result<Value> {
        self.service_platform()?;
        manager_name(manager)?;
        unit_name(unit)?;
        #[cfg(test)]
        if let Some(fixture) = &self.service_fixture {
            let mut properties = load(&fixture.join(format!("{manager}-{unit}.json")))?;
            retain_root_environment(&mut properties)?;
            normalize_dependencies(&mut properties)?;
            return Ok(properties);
        }
        let selection = format!("--property={}", SHOW_PROPERTIES.join(","));
        let bytes = self.service_command(
            "systemctl",
            manager,
            &["show", "--all", &selection, "--", unit],
        )?;
        read_service_properties(&bytes, |interface, property, signature| {
            self.service_property(manager, unit, interface, property, signature)
        })
    }
    fn service_directory(&self, manager: &str, unit: &str) -> Result<PathBuf> {
        #[cfg(test)]
        if let Some(fixture) = &self.service_fixture {
            return Ok(fixture.join("units").join(format!("{unit}.d")));
        }
        let base = if manager == "user" {
            self.home.join(".config/systemd/user")
        } else {
            PathBuf::from("/etc/systemd/system")
        };
        let paths = self.service_property(
            manager,
            "",
            "org.freedesktop.systemd1.Manager",
            "UnitPath",
            "as",
        )?;
        if !paths
            .as_array()
            .is_some_and(|p| p.iter().any(|v| v == base.to_string_lossy().as_ref()))
        {
            return Err(err(
                "service_config_location_unsupported",
                "manager 使用不同的 unit 配置目录；此版本不会猜测或写入自定义搜索路径",
            ));
        }
        guard(&base)?;
        Ok(base.join(format!("{unit}.d")))
    }
    fn service_snapshot(&self, manager: &str, unit: &str, root: &str) -> Result<Value> {
        let p = self.service_properties(manager, unit)?;
        validate_properties(&p, unit, root)?;
        let mut paths = vec![PathBuf::from(string(&p, "FragmentPath")?)];
        for path in p["DropInPaths"]
            .as_array()
            .ok_or_else(|| err("service_schema_unsupported", "缺少 unit drop-in 清单"))?
        {
            paths.push(PathBuf::from(path.as_str().ok_or_else(|| {
                err("service_schema_unsupported", "drop-in 路径格式不受支持")
            })?));
        }
        let files = paths
            .iter()
            .map(|p| config_snapshot(p))
            .collect::<Result<Vec<_>>>()?;
        self.service_processes(&p, root, true)?;
        Ok(json!({"properties":p,"files":files}))
    }
    fn service_processes(&self, p: &Value, root: &str, require_binding: bool) -> Result<Vec<u32>> {
        #[cfg(test)]
        if self.service_fixture.is_some() {
            if require_binding && p["ActiveState"] == "active" && p["fixture_process_root"] != root
            {
                return Err(err(
                    "service_process_unbound",
                    "目标 service 进程没有可核验的精确 root 绑定",
                ));
            }
            return Ok(if p["MainPID"] == "0" {
                vec![]
            } else {
                vec![p["MainPID"].as_str().unwrap().parse().unwrap()]
            });
        }
        let cg = p["ControlGroup"].as_str().unwrap_or("");
        let main: u32 = string(p, "MainPID")?
            .parse()
            .map_err(|_| err("service_schema_unsupported", "MainPID 格式未知"))?;
        if cg.is_empty() {
            if main != 0 || p["ActiveState"] == "active" {
                return Err(err(
                    "service_process_unbound",
                    "无法定位目标 service 的 cgroup",
                ));
            }
            return Ok(vec![]);
        }
        if !cg.starts_with('/') || cg.contains("..") {
            return Err(err("service_cgroup_unsupported", "目标 cgroup 路径未知"));
        }
        let base = PathBuf::from("/sys/fs/cgroup").join(cg.trim_start_matches('/'));
        if !Path::new("/sys/fs/cgroup/cgroup.controllers").exists() {
            return Err(err(
                "service_cgroup_unsupported",
                "此版本需要可核验的统一 cgroup v2",
            ));
        }
        let mut pids = vec![];
        if base.exists() {
            collect_pids(&base, &mut pids)?;
        }
        pids.sort_unstable();
        pids.dedup();
        if p["ActiveState"] == "active" && (main == 0 || !pids.contains(&main)) {
            return Err(err(
                "service_process_unbound",
                "active service 的 MainPID 不在声明的 cgroup 中",
            ));
        }
        if require_binding {
            for pid in &pids {
                if fs::metadata(format!("/proc/{pid}"))?.uid() != unsafe { libc::geteuid() } {
                    return Err(err(
                        "service_process_owner",
                        "service cgroup 包含其他用户进程；未停止服务",
                    ));
                }
                let bytes = fs::read(format!("/proc/{pid}/environ")).map_err(|_| {
                    err(
                        "service_process_unbound",
                        "无法核验 service cgroup 内进程的 root；未停止服务",
                    )
                })?;
                let expected = format!("CLAUDE_CONFIG_DIR={root}");
                if !bytes.split(|b| *b == 0).any(|v| v == expected.as_bytes()) {
                    return Err(err(
                        "service_process_unbound",
                        "service cgroup 包含未绑定或不同环境的进程；不会全局停止共享 service",
                    ));
                }
            }
        }
        Ok(pids)
    }
    fn service_candidates(&self, manager: &str) -> Result<Vec<String>> {
        #[cfg(test)]
        if let Some(fixture) = &self.service_fixture {
            let prefix = format!("{manager}-");
            return Ok(fs::read_dir(fixture)?
                .filter_map(|entry| {
                    let name = entry.ok()?.file_name().to_string_lossy().into_owned();
                    name.strip_prefix(&prefix)?
                        .strip_suffix(".json")
                        .map(str::to_owned)
                })
                .collect());
        }
        let mut out = vec![];
        for args in [
            vec![
                "list-units",
                "--all",
                "--type=service",
                "--plain",
                "--no-legend",
            ],
            vec!["list-unit-files", "--type=service", "--no-legend"],
        ] {
            let bytes = self.service_command("systemctl", manager, &args)?;
            for line in String::from_utf8_lossy(&bytes).lines() {
                if let Some(name) = line
                    .split_whitespace()
                    .next()
                    .filter(|n| unit_name(n).is_ok())
                {
                    out.push(name.to_string());
                }
            }
        }
        out.sort();
        out.dedup();
        Ok(out)
    }
    fn service_holds(&self, e: &Value) -> Result<Vec<(Value, Value)>> {
        let mut out = vec![];
        for entry in fs::read_dir(self.state.join("jobs"))? {
            let path = entry?.path();
            if path.extension().is_none_or(|x| x != "json") {
                continue;
            }
            let job = load(&path)?;
            if job["environment_id"] == e["id"] && job["service_restorable"] == true {
                let plan = self.service_original(string(&job, "id")?)?;
                out.push((job, plan));
            }
        }
        Ok(out)
    }
    fn service_original(&self, job_id: &str) -> Result<Value> {
        let plan = load(&self.path("plans", job_id))?;
        let mut unhashed = plan.clone();
        unhashed
            .as_object_mut()
            .ok_or_else(|| err("invalid_plan", "原服务计划损坏"))?
            .remove("hash");
        if plan["kind"] != "service_quiesce"
            || plan["hash"] != digest(&serde_json::to_vec(&unhashed)?)
        {
            return Err(err("plan_changed", "原服务计划已变化；不会据此恢复服务"));
        }
        Ok(plan)
    }
    pub(crate) fn service_inspect(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let manager = string(r, "manager")?;
        let unit = string(r, "unit")?;
        let root = string(&e, "root")?;
        let snapshot = self.service_snapshot(manager, unit, root)?;
        let p = &snapshot["properties"];
        let mut hold = Value::Null;
        let mut original = Value::Null;
        let mut quiesced = false;
        for (job, plan) in self.service_holds(&e)? {
            let s = &plan["extra"]["service"];
            if s["manager"] == manager && s["unit"] == unit {
                hold = json!({"path":s["hold"]["path"],"persistent":true});
                original = job["id"].clone();
                quiesced = self.check_service_hold(&plan, &snapshot, false).is_ok();
            }
        }
        Ok(
            json!({"environment_id":e["id"],"manager":manager,"unit":unit,"root":root,
            "active_state":p["ActiveState"],"sub_state":p["SubState"],"unit_file_state":p["UnitFileState"],"restart":p["Restart"],
            "main_pid":p["MainPID"].as_str().and_then(|x| x.parse::<u32>().ok()),"control_group":p["ControlGroup"],
            "triggered_by":p["TriggeredBy"].as_str().unwrap_or("").split_whitespace().collect::<Vec<_>>(),
            "bound":true,"quiesced":quiesced,"quiesce_job_id":original,"hold":hold,
            "limitations":["只覆盖明确配置并核验为此 root 的单一 systemd service；其他 supervisor、IDE、手工进程需独立处理。", "持久 blocker 不证明 user manager 在 logout 后存活；主机重启后必须重新查询。"]}),
        )
    }
    pub(crate) fn plan_service_quiesce(&self, r: &Value) -> Result<Value> {
        let e = self.env(r)?;
        let manager = string(r, "manager")?;
        let unit = string(r, "unit")?;
        let root = string(&e, "root")?;
        if fs::metadata(root)?.uid() != unsafe { libc::geteuid() } {
            return Err(err(
                "service_root_owner",
                "此 root 不属于当前 runner 用户；未生成服务暂停计划",
            ));
        }
        let before = self.service_snapshot(manager, unit, root)?;
        self.service_write_authority(manager)?;
        for (_, old) in self.service_holds(&e)? {
            if old["extra"]["service"]["manager"] == manager
                && old["extra"]["service"]["unit"] == unit
            {
                return Err(err(
                    "service_already_held",
                    "此目标已有未恢复的暂停任务；查询原任务并独立预览恢复，不重复暂停",
                ));
            }
        }
        let directory = self.service_directory(manager, unit)?;
        guard(&directory)?;
        self.plan(&e, "service_quiesce", "暂停精确 systemd 服务", json!([]), vec!["相邻 service 与 SSH", "原 unit 文件、启动 enablement 与其他 drop-in", "登录、凭据及工作内容"],
            json!([{"id":"service_hold","label":"写入此 unit 专属的持久启动 blocker 并核验加载","reversible":true},{"id":"service_stop","label":"仅停止已批准 unit，核验 cgroup 无进程","reversible":true}]),
            json!({"service":{"manager":manager,"unit":unit,"root":root,"before":before,"directory":directory,"original_job":null}}))
    }
    pub(crate) fn plan_service_resume(&self, r: &Value) -> Result<Value> {
        let jid = safe_id(r, "job_id")?;
        let job = load(&self.path("jobs", &jid))?;
        if job["service_restorable"] != true {
            return Err(err("service_not_restorable", "此任务没有待恢复的服务 hold"));
        }
        let old = self.service_original(&jid)?;
        let e = self.env(&json!({"environment_id":old["environment_id"]}))?;
        let s = &old["extra"]["service"];
        let current_root = fs::metadata(string(&e, "root")?)?;
        if e["root"] != s["root"]
            || json!([current_root.dev(), current_root.ino()]) != old["root_identity"]
        {
            return Err(err(
                "service_restore_conflict",
                "登记 root 已变化；保留当前 unit 与 blocker",
            ));
        }
        let manager = string(s, "manager")?;
        let unit = string(s, "unit")?;
        let current = self.service_snapshot(manager, unit, string(&e, "root")?)?;
        self.check_service_hold(&old, &current, true)?;
        self.service_write_authority(manager)?;
        let mut service = s.clone();
        service["original_job"] = json!(jid);
        service["held"] = current;
        self.plan(&e, "service_resume", "恢复原 systemd 服务启动状态", json!([]), vec!["相邻 service 与 SSH", "unit 配置与外部编辑", "原 enablement、登录与工作内容"],
            json!([{"id":"service_unhold","label":"仅移除原任务仍拥有的 blocker 并核验配置","reversible":true},{"id":"service_resume","label":"原先运行才启动此 unit；原先停止则保持停止","reversible":true}]), json!({"service":service}))
    }
    pub(crate) fn freeze_service_hold(&self, p: &mut Value) -> Result<()> {
        let pid = string(p, "id")?;
        let s = &p["extra"]["service"];
        let path = PathBuf::from(string(s, "directory")?).join(format!("90-lintel-{pid}.conf"));
        guard(&path)?;
        if path.exists() {
            return Err(err(
                "service_hold_conflict",
                "预览 blocker 路径已存在；不会覆盖现有文件",
            ));
        }
        let journal = self
            .state
            .join("jobs")
            .join(format!("{pid}.service-resume-permit"));
        if journal.exists() {
            return Err(err(
                "service_hold_conflict",
                "启动 permit 路径已存在，未生成 hold",
            ));
        }
        let content = condition_content(pid, &journal)?;
        p["extra"]["service"]["hold"] = json!({"path":path,"persistent":true,"condition_path":journal,
            "content":content});
        Ok(())
    }
    fn check_service_hold(
        &self,
        original: &Value,
        current: &Value,
        allow_active_restore: bool,
    ) -> Result<()> {
        let s = &original["extra"]["service"];
        let hold = &s["hold"];
        let path = Path::new(string(hold, "path")?);
        if Path::new(string(hold, "condition_path")?).exists() {
            return Err(err(
                "service_hold_changed",
                "启动 permit 路径被外部创建，不能认定仍阻止启动；保留外部文件",
            ));
        }
        if read(path, 64 * 1024).ok().as_deref() != Some(string(hold, "content")?.as_bytes()) {
            return Err(err(
                "service_restore_conflict",
                "原任务 blocker 被移除或编辑；保留外部修改，暂停状态需核对",
            ));
        }
        let mut unheld = current.clone();
        let p = &mut unheld["properties"];
        let paths = p["DropInPaths"]
            .as_array_mut()
            .ok_or_else(|| err("service_schema_unsupported", "drop-in 清单格式未知"))?;
        if !paths.iter().any(|v| v == &hold["path"]) {
            return Err(err(
                "service_hold_not_loaded",
                "原任务 blocker 未被 manager 加载；不能认定目标仍暂停",
            ));
        }
        paths.retain(|v| v != &hold["path"]);
        let conditions = p["Conditions"]
            .as_array_mut()
            .ok_or_else(|| err("service_schema_unsupported", "conditions 格式未知"))?;
        let expected = json!([
            "ConditionPathExists",
            false,
            false,
            hold["condition_path"],
            0
        ]);
        if !conditions.contains(&expected) {
            return Err(err(
                "service_hold_not_loaded",
                "manager 没有加载原任务的精确启动 condition；不能认定目标仍暂停",
            ));
        }
        conditions.retain(|v| v != &expected);
        unheld["files"]
            .as_array_mut()
            .unwrap()
            .retain(|v| v["path"] != hold["path"]);
        if config_only(&unheld["properties"]) != config_only(&s["before"]["properties"])
            || unheld["files"] != s["before"]["files"]
        {
            return Err(err(
                "service_restore_conflict",
                "unit 配置、启动绑定或 drop-in 在暂停后改变；保留后续编辑与 blocker",
            ));
        }
        let actual = &current["properties"];
        let stopped = actual["ActiveState"] == "inactive"
            && actual["MainPID"] == "0"
            && self
                .service_processes(actual, string(s, "root")?, false)?
                .is_empty();
        let original_running = allow_active_restore
            && s["before"]["properties"]["ActiveState"] == "active"
            && actual["ActiveState"] == "active";
        if !stopped && !original_running {
            return Err(err(
                "service_not_quiescent",
                "目标 service 未处于无进程的 inactive 状态；先核对原任务，不自动重跑 stop",
            ));
        }
        Ok(())
    }
    pub(crate) fn check_service_plan(&self, p: &Value) -> Result<()> {
        let s = &p["extra"]["service"];
        let manager = string(s, "manager")?;
        let unit = string(s, "unit")?;
        self.service_write_authority(manager)?;
        if self.service_directory(manager, unit)? != PathBuf::from(string(s, "directory")?) {
            return Err(err(
                "stale_service_plan",
                "manager 配置目录变化；请重新预览",
            ));
        }
        let current = self.service_snapshot(manager, unit, string(s, "root")?)?;
        if p["kind"] == "service_quiesce" {
            let pid = string(p, "id")?;
            let condition = self
                .state
                .join("jobs")
                .join(format!("{pid}.service-resume-permit"));
            if s["hold"]["condition_path"] != json!(condition)
                || s["hold"]["content"] != condition_content(pid, &condition)?
            {
                return Err(err(
                    "stale_service_plan",
                    "服务 blocker 生成规则已变化；原批准不再有效，请重新预览",
                ));
            }
            if current != s["before"] {
                return Err(err(
                    "stale_service_plan",
                    "预览后 unit、进程或配置变化；请重新预览",
                ));
            }
            if Path::new(string(&s["hold"], "path")?).exists() {
                return Err(err(
                    "service_hold_conflict",
                    "预览后 blocker 路径出现文件；未覆盖",
                ));
            }
        } else {
            let original = self.service_original(string(s, "original_job")?)?;
            if current != s["held"] {
                return Err(err(
                    "service_restore_conflict",
                    "恢复预览后 unit 或运行状态变化；保留当前配置与 blocker",
                ));
            }
            self.check_service_hold(&original, &current, true)?;
        }
        Ok(())
    }
    fn service_mutation(
        &self,
        manager: &str,
        unit: &str,
        operation: &str,
        _hold: Option<&Value>,
    ) -> Result<()> {
        #[cfg(test)]
        if let Some(fixture) = &self.service_fixture {
            let path = fixture.join(format!("{manager}-{unit}.json"));
            let mut p = load(&path)?;
            let log = fixture.join("mutations.log");
            writeln!(
                OpenOptions::new().create(true).append(true).open(log)?,
                "{operation}:{unit}"
            )?;
            if operation == "daemon-reload" {
                if let Some(hold) = _hold {
                    if Path::new(string(hold, "path")?).exists() {
                        if !p["DropInPaths"].as_array().unwrap().contains(&hold["path"]) {
                            p["DropInPaths"]
                                .as_array_mut()
                                .unwrap()
                                .push(hold["path"].clone());
                        }
                        // Read the actual owned text instead of manufacturing
                        // the expected tuple from plan metadata. Like v255's
                        // condition parser, quotes are literal, not delimiters.
                        p["Conditions"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|v| v[3] != hold["condition_path"]);
                        let content = fs::read_to_string(string(hold, "path")?)?;
                        if let Some(argument) = content.lines().find_map(|line| {
                            line.strip_prefix("ConditionPathExists=")
                                .filter(|value| value.starts_with('/'))
                        }) {
                            let condition = json!([
                                "ConditionPathExists",
                                false,
                                false,
                                argument.replace("%%", "%"),
                                0
                            ]);
                            if !p["Conditions"].as_array().unwrap().contains(&condition) {
                                p["Conditions"].as_array_mut().unwrap().push(condition);
                            }
                        }
                    } else {
                        p["DropInPaths"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|v| v != &hold["path"]);
                        p["Conditions"]
                            .as_array_mut()
                            .unwrap()
                            .retain(|v| v[3] != hold["condition_path"]);
                    }
                }
                // Synthetic-only seam for configuration changes in the reload
                // window, after execute has accepted the original preview.
                if let Some(requires) = p["fixture_reload_requires"].as_str().map(str::to_owned) {
                    p["Requires"] = json!(requires);
                }
                if p["fixture_reload_source_edit"] == true {
                    fs::write(
                        string(&p, "FragmentPath")?,
                        "synthetic reload-time source edit",
                    )?;
                }
            } else if operation == "stop" {
                if p["fixture_fail_stop"] == true {
                    return Err(err("service_command_failed", "synthetic interrupted stop"));
                }
                p["ActiveState"] = json!("inactive");
                p["SubState"] = json!("dead");
                p["MainPID"] = json!("0");
            } else if operation == "start" {
                p["ActiveState"] = json!("active");
                p["SubState"] = json!("running");
                p["MainPID"] = json!("123");
            }
            save(&path, &p)?;
            return Ok(());
        }
        if operation == "daemon-reload" {
            self.service_command("systemctl", manager, &[operation])?;
        } else {
            self.service_command("systemctl", manager, &[operation, "--", unit])?;
        }
        Ok(())
    }
    pub(crate) fn execute_service(&self, p: &Value, j: &mut Value, journal: &Path) -> Result<()> {
        let s = &p["extra"]["service"];
        let hold = &s["hold"];
        let path = Path::new(string(hold, "path")?);
        let manager = string(s, "manager")?;
        let unit = string(s, "unit")?;
        let root = string(s, "root")?;
        j["service"] = public_service(s, p["kind"] == "service_resume");
        if p["kind"] == "service_quiesce" {
            j["service_restorable"] = json!(true);
            j["steps"] = json!([{"id":"service_hold","label":"写入并核验服务启动阻止","status":"executing","message":"持久 blocker 写入意图已保存；中断后只查询原任务"}]);
            save(journal, j)?;
            let directory = path.parent().unwrap();
            guard(directory)?;
            fs::create_dir_all(directory)?;
            let owner = fs::metadata(directory)?;
            if owner.uid() != unsafe { libc::geteuid() } || owner.mode() & 0o022 != 0 {
                return Err(err(
                    "service_config_owner",
                    "blocker 目录不是当前用户拥有的非共享可写目录；未写入",
                ));
            }
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)?;
            f.write_all(string(hold, "content")?.as_bytes())?;
            f.sync_all()?;
            fs::File::open(directory)?.sync_all()?;
            self.service_mutation(manager, unit, "daemon-reload", Some(hold))?;
            let loaded = self.service_snapshot(manager, unit, root)?;
            // Check blocker before issuing stop, including resets from later drop-ins.
            ensure_condition(&loaded["properties"], hold)?;
            let current = config_without_hold(&loaded, hold)?;
            let expected = config_without_hold(&s["before"], hold)?;
            if current != expected {
                j["service"]["configuration_conflict"] =
                    configuration_conflict(&expected, &current);
                return Err(err(
                    "service_restore_conflict",
                    "写入 blocker 时 unit 配置变化；保留当前文件并查询原任务",
                ));
            }
            j["steps"][0]["status"] = json!("completed");
            j["steps"].as_array_mut().unwrap().push(json!({"id":"service_stop","label":"停止精确目标服务","status":"executing","message":"只提交原计划 unit 的 stop，不重复提交"}));
            save(journal, j)?;
            self.service_mutation(manager, unit, "stop", Some(hold))?;
            let after = self.service_snapshot(manager, unit, root)?;
            self.check_service_hold(p, &after, false)?;
            j["steps"][1]["status"] = json!("completed");
            j["service"]["observed"] = json!({"active_state":after["properties"]["ActiveState"],"main_pid":0,"quiesced":true});
        } else {
            let jid = string(s, "original_job")?;
            let original = self.service_original(jid)?;
            self.check_service_hold(
                &original,
                &self.service_snapshot(manager, unit, root)?,
                true,
            )?;
            j["steps"] = json!([{"id":"service_unhold","label":"移除原任务启动阻止","status":"executing","message":"只移除原任务仍拥有的 blocker"}]);
            save(journal, j)?;
            let held_file = s["held"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["path"] == hold["path"])
                .ok_or_else(|| err("invalid_plan", "恢复计划没有冻结 owned blocker"))?;
            let expected = json!({"digest":held_file["digest"],"device":held_file["device"],"inode":held_file["inode"],"mode":held_file["mode"],"owner":held_file["owner"]});
            let quarantine = path
                .parent()
                .unwrap()
                .join(format!(".lintel-service-restore-{}", string(p, "id")?));
            j["steps"][0]["quarantine_path"] = json!(quarantine);
            save(journal, j)?;
            crate::cleanup::quarantine_remove(path, &expected, &quarantine)?;
            self.service_mutation(manager, unit, "daemon-reload", Some(hold))?;
            let unheld = self.service_snapshot(manager, unit, root)?;
            if config_only(&unheld["properties"]) != config_only(&s["before"]["properties"])
                || unheld["files"] != s["before"]["files"]
            {
                return Err(err(
                    "service_restore_conflict",
                    "移除 blocker 后 unit 配置不同；未启动服务，保留外部编辑",
                ));
            }
            j["steps"][0]["status"] = json!("completed");
            save(journal, j)?;
            let was_active = s["before"]["properties"]["ActiveState"] == "active";
            j["steps"].as_array_mut().unwrap().push(json!({"id":"service_resume","label":"恢复原服务启动状态","status":"executing","message":if was_active{"按原状态仅启动此 unit"}else{"原先停止，保持 inactive"}}));
            save(journal, j)?;
            if was_active && unheld["properties"]["ActiveState"] != "active" {
                self.service_mutation(manager, unit, "start", None)?;
            }
            let after = self.service_snapshot(manager, unit, root)?;
            if after["properties"]["ActiveState"] != s["before"]["properties"]["ActiveState"] {
                return Err(err(
                    "service_resume_failed",
                    "原启动状态未恢复；只查询原任务，不自动重启",
                ));
            }
            j["steps"][1]["status"] = json!("completed");
            j["service"]["observed"] = json!({"active_state":after["properties"]["ActiveState"],"main_pid":after["properties"]["MainPID"].as_str().and_then(|v|v.parse::<u32>().ok()),"quiesced":false});
            let job_path = self.path("jobs", jid);
            let mut original_job = load(&job_path)?;
            original_job["service_restorable"] = json!(false);
            original_job["service_resumed_by"] = p["id"].clone();
            save(&job_path, &original_job)?;
        }
        j["status"] = json!("completed");
        j["service"]["verified_at"] = json!(now());
        Ok(())
    }
    /// This supplements manual confirmation with live systemd evidence. A stopped
    /// but unheld Restart/timer/socket target cannot be approved for cleanup.
    pub(crate) fn check_cleanup_services(&self, e: &Value) -> Result<Vec<Value>> {
        let holds = self.service_holds(e)?;
        if self.service_platform().is_err() {
            return Ok(vec![]);
        }
        let root = string(e, "root")?;
        let mut out = vec![];
        for (_, plan) in &holds {
            let s = &plan["extra"]["service"];
            let current = self.service_snapshot(string(s, "manager")?, string(s, "unit")?, root)?;
            self.check_service_hold(plan, &current, false)?;
            out.push(json!({"manager":s["manager"],"unit":s["unit"],"quiesced":true,"quiesce_job_id":plan["id"]}));
        }
        for manager in ["user", "system"] {
            let Ok(units) = self.service_candidates(manager) else {
                continue;
            };
            for unit in units {
                #[cfg(test)]
                let environment = self.service_properties(manager, &unit)?["Environment"].clone();
                #[cfg(not(test))]
                let environment = match (|| {
                    // show loads an inactive unit without activating it. A bus-only
                    // lookup would miss restart-capable unit files not yet loaded.
                    self.service_command(
                        "systemctl",
                        manager,
                        &["show", "--property=LoadState", "--value", "--", &unit],
                    )?;
                    self.service_property(
                        manager,
                        &unit,
                        "org.freedesktop.systemd1.Service",
                        "Environment",
                        "as",
                    )
                })() {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if !root_binding(&environment, root) {
                    continue;
                }
                if !holds.iter().any(|(_, p)| {
                    p["extra"]["service"]["manager"] == manager
                        && p["extra"]["service"]["unit"] == unit
                }) {
                    return Err(err("service_quiescence_required", &format!("发现绑定此 root 的 {manager} service {unit}；必须先独立预览并批准精确暂停，stopped 确认不能代替 blocker 证据")));
                }
            }
        }
        Ok(out)
    }
}
fn normalize_conditions(p: &mut Value) -> Result<()> {
    for c in p["Conditions"]
        .as_array_mut()
        .ok_or_else(|| err("service_schema_unsupported", "systemd Conditions 格式未知"))?
    {
        let parts = c
            .as_array_mut()
            .filter(|c| c.len() == 5)
            .ok_or_else(|| err("service_schema_unsupported", "systemd condition 格式未知"))?;
        if !parts[0].is_string()
            || !parts[1].is_boolean()
            || !parts[2].is_boolean()
            || !parts[3].is_string()
            || !parts[4].is_i64()
        {
            return Err(err(
                "service_schema_unsupported",
                "systemd condition 类型未知",
            ));
        }
        parts[4] = json!(0);
    }
    Ok(())
}
fn retain_root_environment(p: &mut Value) -> Result<()> {
    let env = p["Environment"]
        .as_array()
        .ok_or_else(|| err("service_schema_unsupported", "systemd Environment 格式未知"))?;
    if !env.iter().all(Value::is_string) {
        return Err(err(
            "service_schema_unsupported",
            "systemd Environment 类型未知",
        ));
    }
    // Freeze only the root binding. Original source byte snapshots detect external
    // unit edits without persisting API keys or exporting a whole environment.
    p["Environment"] = json!(env
        .iter()
        .filter(|v| v.as_str().unwrap().starts_with("CLAUDE_CONFIG_DIR="))
        .cloned()
        .collect::<Vec<_>>());
    Ok(())
}
fn root_binding(environment: &Value, root: &str) -> bool {
    let Some(values) = environment.as_array() else {
        return false;
    };
    let expected = format!("CLAUDE_CONFIG_DIR={root}");
    let bindings: Vec<_> = values
        .iter()
        .filter_map(Value::as_str)
        .filter(|v| v.starts_with("CLAUDE_CONFIG_DIR="))
        .collect();
    bindings == vec![expected.as_str()]
}
fn validate_properties(p: &Value, unit: &str, root: &str) -> Result<()> {
    if p["Id"] != unit
        || p["LoadState"] != "loaded"
        || p["Transient"] != "no"
        || p["SourcePath"] != ""
    {
        return Err(err(
            "service_unit_unsupported",
            "只支持直接 loaded 的持久 service；alias、masked、generated 与 transient unit 不受支持",
        ));
    }
    if p["NeedDaemonReload"] != "no" {
        return Err(err(
            "service_configuration_unloaded",
            "unit 配置已改变但 manager 未 reload；先由操作者核对，不批准过期配置",
        ));
    }
    if ![json!("active"), json!("inactive")].contains(&p["ActiveState"])
        || ![json!("running"), json!("dead")].contains(&p["SubState"])
        || p["Job"] != ""
    {
        return Err(err(
            "service_state_unsupported",
            "service 正在转换、失败或有 pending job；先查询稳定状态",
        ));
    }
    if ![json!("simple"), json!("exec"), json!("notify")].contains(&p["Type"])
        || p["KillMode"] != "control-group"
        || p["Delegate"] != "no"
    {
        return Err(err(
            "service_cgroup_unsupported",
            "仅支持无 delegation、KillMode=control-group 的 simple/exec/notify service",
        ));
    }
    if !root_binding(&p["Environment"], root)
        || ["EnvironmentFiles", "PassEnvironment", "UnsetEnvironment"]
            .iter()
            .any(|key| p[key] != "")
    {
        return Err(err("service_root_unbound", "unit 必须直接且唯一配置 CLAUDE_CONFIG_DIR=此登记 root；EnvironmentFile、继承、unset 或 wrapper 的未知绑定不受支持"));
    }
    if [
        "RequiredBy",
        "BoundBy",
        "ConsistsOf",
        "PropagatesStopTo",
        "OnFailure",
        "OnSuccess",
        "ExecStop",
        "ExecStopPost",
    ]
    .iter()
    .any(|key| p[key] != "")
    {
        return Err(err("service_shared_stop_scope", "unit 有停止传播、依赖它的其他 unit 或自定义 stop/success/failure 动作；不能保证邻居不受影响，未生成暂停操作"));
    }
    for key in [
        "Requires",
        "Requisite",
        "BindsTo",
        "Wants",
        "Upholds",
        "Conflicts",
    ] {
        if p[key]
            .as_str()
            .unwrap_or("")
            .split_whitespace()
            .any(|unit| {
                ![
                    "sysinit.target",
                    "basic.target",
                    "shutdown.target",
                    "system.slice",
                    "app.slice",
                    "-.slice",
                ]
                .contains(&unit)
            })
        {
            return Err(err("service_shared_start_scope", "unit 启动可能启动、保持或停止其他 unit；本次只支持独立 service，不扩大批准影响范围"));
        }
    }
    if ["SuccessAction", "FailureAction", "StartLimitAction"]
        .iter()
        .any(|key| p[key] != "none")
        || p["RefuseManualStop"] != "no"
        || p["RefuseManualStart"] != "no"
    {
        return Err(err(
            "service_action_unsupported",
            "unit 含主机/服务特殊动作或拒绝手动启动停止；此适配器不修改这些控制",
        ));
    }
    Ok(())
}
fn collect_pids(directory: &Path, out: &mut Vec<u32>) -> Result<()> {
    let text = fs::read_to_string(directory.join("cgroup.procs")).map_err(|_| {
        err(
            "service_cgroup_unreadable",
            "无法读取目标 cgroup 的进程清单",
        )
    })?;
    for line in text.lines() {
        out.push(
            line.parse()
                .map_err(|_| err("service_cgroup_unsupported", "cgroup 进程清单格式未知"))?,
        );
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        if entry.file_type()?.is_dir() {
            collect_pids(&entry.path(), out)?;
        }
    }
    Ok(())
}
fn ensure_condition(p: &Value, hold: &Value) -> Result<()> {
    if Path::new(string(hold, "condition_path")?).exists() {
        return Err(err(
            "service_hold_changed",
            "启动 permit 路径已存在；未停止服务",
        ));
    }
    let expected = json!([
        "ConditionPathExists",
        false,
        false,
        hold["condition_path"],
        0
    ]);
    if !p["DropInPaths"]
        .as_array()
        .is_some_and(|a| a.contains(&hold["path"]))
        || !p["Conditions"]
            .as_array()
            .is_some_and(|a| a.contains(&expected))
    {
        return Err(err(
            "service_hold_not_loaded",
            "manager 未加载精确 owned blocker；未提交 stop，保留文件并查询原任务",
        ));
    }
    Ok(())
}
fn config_without_hold(snapshot: &Value, hold: &Value) -> Result<Value> {
    let mut snapshot = snapshot.clone();
    let p = &mut snapshot["properties"];
    p["DropInPaths"]
        .as_array_mut()
        .ok_or_else(|| err("service_schema_unsupported", "drop-in 清单格式未知"))?
        .retain(|v| v != &hold["path"]);
    // Remove only the owned tuple, preserve every external condition.
    p["Conditions"].as_array_mut().unwrap().retain(|v| {
        v != &json!([
            "ConditionPathExists",
            false,
            false,
            hold["condition_path"],
            0
        ])
    });
    snapshot["files"]
        .as_array_mut()
        .unwrap()
        .retain(|v| v["path"] != hold["path"]);
    snapshot["properties"] = config_only(&snapshot["properties"]);
    Ok(snapshot)
}
pub(crate) fn public_service(s: &Value, resume: bool) -> Value {
    let before = view(&s["before"]["properties"]);
    json!({"manager":s["manager"],"unit":s["unit"],"root":s["root"],"before":before,
        "after":{"active_state":if resume {s["before"]["properties"]["ActiveState"].clone()} else {json!("inactive")},"hold":!resume},
        "hold":{"path":s["hold"]["path"],"persistent":true},"original_job":s["original_job"],
        "limitations":["需要独立验证 logout 后 manager 与任务存活、以及主机重启后现状；旧 receipt 不证明当前服务状态。"]})
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    fn fixture(active: bool) -> (TempDir, Engine, Value, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let base = temp.path().canonicalize().unwrap();
        let home = base.join("home");
        fs::create_dir(&home).unwrap();
        let root = home.join("synthetic-root");
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("settings.json"),
            "intentionally non-JSON: service plans do not parse settings",
        )
        .unwrap();
        let mut engine = Engine::new(home, base.join("state")).unwrap();
        let adapter = base.join("adapter");
        fs::create_dir(&adapter).unwrap();
        engine.service_fixture = Some(adapter.clone());
        let e = engine.register("synthetic", &root, false).unwrap();
        for (unit, binding) in [
            ("target.service", root.clone()),
            ("neighbor.service", base.join("neighbor-root")),
        ] {
            fs::create_dir_all(&binding).unwrap();
            let source = adapter.join(format!("{unit}.unit"));
            fs::write(&source, "[Service]\nRestart=always\n").unwrap();
            let mut p = json!({});
            for key in SHOW_PROPERTIES
                .iter()
                .copied()
                .chain(COMPLEX_PROPERTIES.iter().map(|(key, _)| *key))
            {
                p[key] = json!("");
            }
            for (key, value) in [
                ("Id", unit),
                ("LoadState", "loaded"),
                ("ActiveState", if active { "active" } else { "inactive" }),
                ("SubState", if active { "running" } else { "dead" }),
                ("UnitFileState", "enabled"),
                ("NeedDaemonReload", "no"),
                ("Transient", "no"),
                ("MainPID", if active { "123" } else { "0" }),
                ("ControlGroup", "/synthetic/target.service"),
                ("Restart", "always"),
                ("KillMode", "control-group"),
                ("Delegate", "no"),
                ("RefuseManualStop", "no"),
                ("RefuseManualStart", "no"),
                ("SuccessAction", "none"),
                ("FailureAction", "none"),
                ("StartLimitAction", "none"),
                ("Type", "simple"),
            ] {
                p[key] = json!(value);
            }
            p["FragmentPath"] = json!(source);
            p["Environment"] = json!([format!("CLAUDE_CONFIG_DIR={}", binding.display())]);
            p["DropInPaths"] = json!([]);
            p["Conditions"] = json!([]);
            p["fixture_process_root"] = json!(binding);
            save(&adapter.join(format!("user-{unit}.json")), &p).unwrap();
        }
        (temp, engine, e, root)
    }
    fn request(engine: &Engine, e: &Value, command: &str) -> Value {
        engine.request(json!({"command":command,"environment_id":e["id"],"manager":"user","unit":"target.service"}))
    }
    fn execute(engine: &Engine, p: &Value) -> Value {
        engine.request(json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}))
    }
    fn stored(engine: &Engine) -> PathBuf {
        engine
            .service_fixture
            .as_ref()
            .unwrap()
            .join("user-target.service.json")
    }
    fn log(engine: &Engine) -> String {
        fs::read_to_string(
            engine
                .service_fixture
                .as_ref()
                .unwrap()
                .join("mutations.log"),
        )
        .unwrap_or_default()
    }
    fn wire_outputs(p: &Value) -> (String, Value) {
        // Match systemd v255's printers: empty complex arrays have no show line,
        // and get-property JSON data is the value, without a reply-argument layer.
        let show = SHOW_PROPERTIES
            .iter()
            .map(|key| format!("{key}={}\n", p[key].as_str().unwrap()))
            .collect();
        let bus = json!({
            "EnvironmentFiles":{"type":"a(sb)","data":[]},
            "ExecStop":{"type":"a(sasbttttuii)","data":[]},
            "ExecStopPost":{"type":"a(sasbttttuii)","data":[]},
            "Environment":{"type":"as","data":p["Environment"]},
            "DropInPaths":{"type":"as","data":p["DropInPaths"]},
            "Conditions":{"type":"a(sbbsi)","data":p["Conditions"]}
        });
        (show, bus)
    }
    fn read_wire(show: &str, bus: &Value) -> Result<Value> {
        read_service_properties(show.as_bytes(), |interface, key, signature| {
            let expected = if ["DropInPaths", "Conditions"].contains(&key) {
                "org.freedesktop.systemd1.Unit"
            } else {
                "org.freedesktop.systemd1.Service"
            };
            assert_eq!(interface, expected);
            parse_service_property(&serde_json::to_vec(&bus[key]).unwrap(), signature)
        })
    }
    #[test]
    fn wire_normalizes_only_unit_dependency_order_without_interpreting_tokens() {
        let (_temp, engine, _e, _root) = fixture(true);
        let mut properties = load(&stored(&engine)).unwrap();
        let dependencies = [
            "RequiredBy",
            "BoundBy",
            "ConsistsOf",
            "PropagatesStopTo",
            "TriggeredBy",
            "OnFailure",
            "OnSuccess",
            "Requires",
            "Requisite",
            "BindsTo",
            "Wants",
            "Upholds",
            "Conflicts",
        ];
        for key in dependencies {
            properties[key] = json!(r#"z.timer "a\\x20b.socket" a.service"#);
        }
        properties["User"] = json!("z a");
        properties["FragmentPath"] = json!("/z /a");
        properties["DropInPaths"] = json!(["/z/source.conf", "/a/source.conf"]);
        properties["Conditions"] = json!([
            ["ConditionPathExists", false, false, "/z", 0],
            ["ConditionPathExists", false, false, "/a", 0]
        ]);
        let (show, bus) = wire_outputs(&properties);
        let observed = read_wire(&show, &bus).unwrap();
        for key in dependencies {
            assert_eq!(
                observed[key], r#""a\\x20b.socket" a.service z.timer"#,
                "{key}"
            );
        }
        for key in ["User", "FragmentPath", "DropInPaths", "Conditions"] {
            assert_eq!(observed[key], properties[key], "{key}");
        }
        // Preserve an unexpected duplicate rather than silently deleting it.
        properties["Requires"] = json!("system.slice sysinit.target system.slice");
        let (show, bus) = wire_outputs(&properties);
        assert_eq!(
            read_wire(&show, &bus).unwrap()["Requires"],
            "sysinit.target system.slice system.slice"
        );
        let mut invalid = observed;
        invalid["Requires"] = json!([]);
        assert_eq!(
            normalize_dependencies(&mut invalid).unwrap_err().code,
            "service_schema_unsupported"
        );
    }
    #[test]
    fn old_noncanonical_dependency_snapshot_requires_new_preview_before_mutation() {
        let (_temp, engine, e, _root) = fixture(true);
        let mut properties = load(&stored(&engine)).unwrap();
        properties["Requires"] = json!("system.slice sysinit.target");
        save(&stored(&engine), &properties).unwrap();
        let preview = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let path = engine.path("plans", preview["id"].as_str().unwrap());
        let mut old = load(&path).unwrap();
        assert_eq!(
            old["extra"]["service"]["before"]["properties"]["Requires"],
            "sysinit.target system.slice"
        );
        // A formerly frozen raw permutation remains intact and validly hashed.
        // The existing full snapshot check rejects it; no legacy write path.
        old["extra"]["service"]["before"]["properties"]["Requires"] =
            json!("system.slice sysinit.target");
        old.as_object_mut().unwrap().remove("hash");
        old["hash"] = json!(digest(&serde_json::to_vec(&old).unwrap()));
        save(&path, &old).unwrap();
        let rejected = execute(&engine, &old);
        assert_eq!(
            rejected["error"]["code"], "stale_service_plan",
            "{rejected}"
        );
        assert_eq!(log(&engine), "");
        assert!(!Path::new(preview["service"]["hold"]["path"].as_str().unwrap()).exists());
    }
    #[test]
    fn condition_path_is_literal_unquoted_and_old_approved_text_is_stale() {
        let (_temp, mut engine, e, _root) = fixture(true);
        let plan = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let pid = plan["id"].as_str().unwrap();
        let plan_path = engine.path("plans", pid);
        let mut stored_plan = load(&plan_path).unwrap();
        let hold = &stored_plan["extra"]["service"]["hold"];
        let condition = hold["condition_path"].as_str().unwrap();
        assert_eq!(hold["content"], format!("# Lintel owned hold; restore through original job {pid}\n[Unit]\nConditionPathExists={condition}\n"));
        let special = Path::new(
            "/synthetic/state space%literal/quote\"and\\slash/jobs/job.service-resume-permit",
        );
        assert_eq!(condition_content("job", special).unwrap(), "# Lintel owned hold; restore through original job job\n[Unit]\nConditionPathExists=/synthetic/state space%%literal/quote\"and\\slash/jobs/job.service-resume-permit\n");
        for path in [
            "/synthetic/state\nConditionPathExists=/tmp",
            "/synthetic/state\r",
            "/synthetic/state\t",
            "relative/path",
        ] {
            assert_eq!(
                condition_content("job", Path::new(path)).unwrap_err().code,
                "service_condition_path_unsupported"
            );
        }
        // Reproduce a validly hashed plan approved under the old generator.
        // Integrity is intact; the changed blocker semantics must reject it.
        stored_plan["extra"]["service"]["hold"]["content"] = json!(format!("# Lintel owned hold; restore through original job {pid}\n[Unit]\nConditionPathExists={}\n", serde_json::to_string(condition).unwrap()));
        stored_plan.as_object_mut().unwrap().remove("hash");
        stored_plan["hash"] = json!(digest(&serde_json::to_vec(&stored_plan).unwrap()));
        save(&plan_path, &stored_plan).unwrap();
        let result = execute(&engine, &stored_plan);
        assert_eq!(result["error"]["code"], "stale_service_plan", "{result}");
        assert_eq!(log(&engine), "");
        assert!(!Path::new(
            stored_plan["extra"]["service"]["hold"]["path"]
                .as_str()
                .unwrap()
        )
        .exists());
        assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "active");
        // Exercise full hold/restore with literal spaces, percent, quote and
        // backslash in state; the loaded tuple must remain the exact path.
        let special_state = engine
            .state
            .parent()
            .unwrap()
            .join("state space%literal\"and\\slash");
        fs::rename(&engine.state, &special_state).unwrap();
        engine.state = special_state;
        let fresh = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let paused = execute(&engine, &fresh);
        assert_eq!(paused["data"]["status"], "completed", "{paused}");
        assert_eq!(
            request(&engine, &e, "service_inspect")["data"]["quiesced"],
            true
        );
        let resume =
            engine.request(json!({"command":"plan_service_resume","job_id":paused["data"]["id"]}));
        assert_eq!(
            execute(&engine, &resume["data"])["data"]["status"],
            "completed"
        );
    }
    #[test]
    fn quoted_or_wrong_loaded_condition_never_submits_stop() {
        let (_temp, engine, e, _root) = fixture(true);
        let plan = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let stored_plan = load(&engine.path("plans", plan["id"].as_str().unwrap())).unwrap();
        let hold = &stored_plan["extra"]["service"]["hold"];
        let path = Path::new(hold["path"].as_str().unwrap());
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // A real text parser ignores the old quoted absolute path. The
        // synthetic manager must not invent its expected tuple from metadata.
        fs::write(
            path,
            format!(
                "[Unit]\nConditionPathExists={}\n",
                serde_json::to_string(&hold["condition_path"]).unwrap()
            ),
        )
        .unwrap();
        engine
            .service_mutation("user", "target.service", "daemon-reload", Some(hold))
            .unwrap();
        let p = engine.service_properties("user", "target.service").unwrap();
        assert!(p["DropInPaths"].as_array().unwrap().contains(&hold["path"]));
        assert!(p["Conditions"].as_array().unwrap().is_empty());
        assert_eq!(
            ensure_condition(&p, hold).unwrap_err().code,
            "service_hold_not_loaded"
        );
        let mut wrong = p.clone();
        wrong["Conditions"] = json!([[
            "ConditionPathExists",
            false,
            false,
            "/synthetic/wrong-permit",
            0
        ]]);
        assert_eq!(
            ensure_condition(&wrong, hold).unwrap_err().code,
            "service_hold_not_loaded"
        );
        assert_eq!(p["ActiveState"], "active");
        assert!(!log(&engine).contains("stop:"));
        // The exact raw absolute value now loads; retain the strict readback.
        fs::write(path, hold["content"].as_str().unwrap()).unwrap();
        engine
            .service_mutation("user", "target.service", "daemon-reload", Some(hold))
            .unwrap();
        ensure_condition(
            &engine.service_properties("user", "target.service").unwrap(),
            hold,
        )
        .unwrap();
    }
    #[test]
    fn systemd255_empty_complex_arrays_and_direct_property_values_are_supported() {
        let (_temp, engine, _e, root) = fixture(true);
        let mut p = load(&stored(&engine)).unwrap();
        p["Environment"]
            .as_array_mut()
            .unwrap()
            .push(json!("SYNTHETIC_API_KEY=do-not-export"));
        p["DropInPaths"] = json!(["/synthetic/external.conf", "/synthetic/owned.conf"]);
        p["Conditions"] = json!([["ConditionPathExists", false, false, "/synthetic/permit", -1]]);
        let (show, bus) = wire_outputs(&p);
        for key in ["EnvironmentFiles", "ExecStop", "ExecStopPost"] {
            assert!(!show.contains(&format!("{key}=")));
        }
        let observed = read_wire(&show, &bus).unwrap();
        validate_properties(&observed, "target.service", root.to_str().unwrap()).unwrap();
        assert_eq!(
            observed["Environment"],
            json!([format!("CLAUDE_CONFIG_DIR={}", root.display())])
        );
        assert_eq!(observed["DropInPaths"], p["DropInPaths"]);
        assert_eq!(
            observed["Conditions"],
            json!([["ConditionPathExists", false, false, "/synthetic/permit", 0]])
        );
        assert!(!observed.to_string().contains("do-not-export"));
        assert_eq!(
            parse_service_property(br#"{"type":"as","data":["/etc/systemd/system"]}"#, "as")
                .unwrap(),
            json!(["/etc/systemd/system"])
        );
        // No fixture-independent Engine or real manager access is involved.
        assert_eq!(log(&engine), "");
    }
    #[test]
    fn systemd_wire_missing_unknown_or_configured_properties_never_allow_mutation() {
        let (_temp, engine, _e, root) = fixture(true);
        let p = load(&stored(&engine)).unwrap();
        let (show, bus) = wire_outputs(&p);
        for bad_show in [
            show.replace("Id=target.service\n", ""),
            format!("{show}Id=target.service\n"),
            format!("{show}EnvironmentFiles=\n"),
        ] {
            assert_eq!(
                read_wire(&bad_show, &bus).unwrap_err().code,
                "service_schema_unsupported"
            );
        }
        for (key, value) in [
            ("EnvironmentFiles", json!({"type":"as","data":[]})),
            ("EnvironmentFiles", json!({"type":"a(sb)"})),
            ("EnvironmentFiles", json!({"type":"a(sb)","data":""})),
            ("ExecStop", json!({"type":"a(sasbttttuii)","data":null})),
            ("ExecStopPost", json!({"type":"a(sasbttttuii)","data":{}})),
            (
                "Environment",
                json!({"type":"as","data":[p["Environment"]]}),
            ),
            ("DropInPaths", json!({"type":"as","data":[[]]})),
            ("Conditions", json!({"type":"a(sbbsi)","data":[[]]})),
        ] {
            let mut bad = bus.clone();
            bad[key] = value;
            assert_eq!(
                read_wire(&show, &bad).unwrap_err().code,
                "service_schema_unsupported",
                "{key}"
            );
        }
        for (key, data, code) in [
            (
                "EnvironmentFiles",
                json!([
                    ["/synthetic/override.env", false],
                    ["/synthetic/other.env", true]
                ]),
                "service_root_unbound",
            ),
            (
                "ExecStop",
                json!([[
                    "/synthetic/stop",
                    ["/synthetic/stop", "SYNTHETIC_SECRET"],
                    false,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0
                ]]),
                "service_shared_stop_scope",
            ),
            (
                "ExecStopPost",
                json!([[
                    "/synthetic/after-stop",
                    ["/synthetic/after-stop"],
                    false,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0,
                    0
                ]]),
                "service_shared_stop_scope",
            ),
        ] {
            let mut configured = bus.clone();
            configured[key]["data"] = data;
            let observed = read_wire(&show, &configured).unwrap();
            assert_eq!(
                validate_properties(&observed, "target.service", root.to_str().unwrap())
                    .unwrap_err()
                    .code,
                code,
                "{key}"
            );
            assert!(!observed.to_string().contains("SYNTHETIC_SECRET"));
            assert!(!observed.to_string().contains("override.env"));
        }
        assert_eq!(log(&engine), "");
    }
    #[test]
    fn exact_hold_resume_and_replay_leave_neighbor_and_settings_untouched() {
        let (_temp, engine, e, root) = fixture(true);
        let neighbor = fs::read(
            engine
                .service_fixture
                .as_ref()
                .unwrap()
                .join("user-neighbor.service.json"),
        )
        .unwrap();
        let settings = fs::read(root.join("settings.json")).unwrap();
        let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        assert!(p["id"].is_string(), "{p}");
        let hold = PathBuf::from(p["service"]["hold"]["path"].as_str().unwrap());
        assert!(!hold.exists());
        let bad = engine.request(json!({"command":"execute","plan_id":p["id"],"approval":"wrong"}));
        assert_eq!(bad["error"]["code"], "approval_mismatch");
        assert_eq!(log(&engine), "");
        let j = execute(&engine, &p);
        assert_eq!(j["data"]["status"], "completed", "{j}");
        assert!(hold.exists());
        let inspected = request(&engine, &e, "service_inspect");
        assert_eq!(inspected["data"]["quiesced"], true, "{inspected}");
        let before = log(&engine);
        assert_eq!(execute(&engine, &p)["data"]["id"], j["data"]["id"]);
        assert_eq!(log(&engine), before);
        let restored =
            engine.request(json!({"command":"plan_service_resume","job_id":j["data"]["id"]}));
        assert_eq!(restored["ok"], true, "{restored}");
        assert!(hold.exists(), "resume preview mutated service");
        let final_job = execute(&engine, &restored["data"]);
        assert_eq!(final_job["data"]["status"], "completed", "{final_job}");
        assert!(!hold.exists());
        assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "active");
        let log_before = log(&engine);
        execute(&engine, &restored["data"]);
        assert_eq!(log(&engine), log_before);
        assert_eq!(
            fs::read(
                engine
                    .service_fixture
                    .as_ref()
                    .unwrap()
                    .join("user-neighbor.service.json")
            )
            .unwrap(),
            neighbor
        );
        assert_eq!(fs::read(root.join("settings.json")).unwrap(), settings);
        assert_eq!(log_before.matches("stop:target.service").count(), 1);
        assert_eq!(log_before.matches("start:target.service").count(), 1);
    }
    #[test]
    fn reload_configuration_conflict_records_evidence_without_stopping_or_replaying() {
        for (reload_requires, source_edit) in [
            ("sysinit.target", false),
            ("sysinit.target system.slice basic.target", false),
            ("", true),
        ] {
            let (_temp, engine, e, _root) = fixture(true);
            let mut properties = load(&stored(&engine)).unwrap();
            properties["Requires"] = json!("sysinit.target system.slice");
            if source_edit {
                properties["fixture_reload_source_edit"] = json!(true);
            } else {
                properties["fixture_reload_requires"] = json!(reload_requires);
            }
            save(&stored(&engine), &properties).unwrap();
            let plan = request(&engine, &e, "plan_service_quiesce")["data"].clone();
            let receipt = execute(&engine, &plan);
            assert_eq!(
                receipt["data"]["status"], "needs_reconciliation",
                "{receipt}"
            );
            let conflict = &receipt["data"]["service"]["configuration_conflict"];
            assert!(conflict.is_object(), "{receipt}");
            if source_edit {
                assert_eq!(conflict["property_diff_keys"], json!([]));
                assert_eq!(
                    conflict["files"]["differences"][0]["diff_keys"],
                    json!(["digest"])
                );
            } else {
                assert_eq!(conflict["property_diff_keys"], json!(["Requires"]));
                assert_eq!(
                    conflict["property_values"]["Requires"]["expected"],
                    "sysinit.target system.slice"
                );
                assert_eq!(
                    conflict["property_values"]["Requires"]["current"],
                    if reload_requires.contains("basic.target") {
                        "basic.target sysinit.target system.slice"
                    } else {
                        reload_requires
                    }
                );
                assert_eq!(conflict["files"]["differences"], json!([]));
            }
            assert_eq!(receipt["data"]["steps"][0]["status"], "executing");
            assert_eq!(receipt["data"]["steps"].as_array().unwrap().len(), 1);
            assert!(Path::new(plan["service"]["hold"]["path"].as_str().unwrap()).exists());
            assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "active");
            let before = log(&engine);
            assert_eq!(before, "daemon-reload:target.service\n");
            assert_eq!(execute(&engine, &plan), receipt);
            let query = engine.request(json!({"command":"job","job_id":receipt["data"]["id"]}));
            assert_eq!(query["data"], receipt["data"]);
            assert_eq!(log(&engine), before);
        }
    }
    #[test]
    fn dependency_permutations_allow_exact_hold_resume_and_remain_query_only() {
        let (_temp, engine, e, _root) = fixture(true);
        let neighbor = engine
            .service_fixture
            .as_ref()
            .unwrap()
            .join("user-neighbor.service.json");
        let neighbor_before = fs::read(&neighbor).unwrap();
        let mut properties = load(&stored(&engine)).unwrap();
        properties["Requires"] = json!("sysinit.target system.slice");
        properties["fixture_reload_requires"] = json!("system.slice sysinit.target");
        properties["TriggeredBy"] = json!("z-trigger.timer a-trigger.socket");
        save(&stored(&engine), &properties).unwrap();
        let plan = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let paused = execute(&engine, &plan);
        assert_eq!(paused["data"]["status"], "completed", "{paused}");
        assert_eq!(
            request(&engine, &e, "service_inspect")["data"]["quiesced"],
            true
        );
        let before = log(&engine);
        assert_eq!(execute(&engine, &plan), paused);
        assert_eq!(log(&engine), before);
        let original = engine
            .service_original(plan["id"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            original["extra"]["service"]["before"]["properties"]["TriggeredBy"],
            "a-trigger.socket z-trigger.timer"
        );
        // The manager can return either permutation during resume preview,
        // approval recheck and the daemon-reload after removing the blocker.
        properties = load(&stored(&engine)).unwrap();
        properties["TriggeredBy"] = json!("a-trigger.socket z-trigger.timer");
        save(&stored(&engine), &properties).unwrap();
        let resume =
            engine.request(json!({"command":"plan_service_resume","job_id":paused["data"]["id"]}));
        assert_eq!(resume["ok"], true, "{resume}");
        properties["Requires"] = json!("sysinit.target system.slice");
        properties["TriggeredBy"] = json!("z-trigger.timer a-trigger.socket");
        save(&stored(&engine), &properties).unwrap();
        let restored = execute(&engine, &resume["data"]);
        assert_eq!(restored["data"]["status"], "completed", "{restored}");
        assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "active");
        assert!(!Path::new(plan["service"]["hold"]["path"].as_str().unwrap()).exists());
        let before = log(&engine);
        assert_eq!(before.matches("stop:target.service").count(), 1);
        assert_eq!(before.matches("start:target.service").count(), 1);
        assert_eq!(execute(&engine, &resume["data"]), restored);
        assert_eq!(log(&engine), before);
        assert_eq!(fs::read(&neighbor).unwrap(), neighbor_before);
    }
    #[test]
    fn configuration_diagnostics_withhold_sensitive_values_and_bound_output() {
        let mut expected =
            json!({"properties":{"Requires":"sysinit.target system.slice"},"files":[]});
        let mut current =
            json!({"properties":{"Requires":"system.slice sysinit.target"},"files":[]});
        for key in [
            "Environment",
            "User",
            "PassEnvironment",
            "UnsetEnvironment",
            "FragmentPath",
            "SourcePath",
            "DropInPaths",
            "Conditions",
        ] {
            expected["properties"][key] = json!("SYNTHETIC_PRIVATE_EXPECTED");
            current["properties"][key] = json!("SYNTHETIC_PRIVATE_CURRENT");
        }
        for index in 0..33 {
            expected["files"].as_array_mut().unwrap().push(json!({"path":format!("/SYNTHETIC_PRIVATE_EXPECTED/{index}"),"digest":"before","device":1,"inode":index,"mode":0o100600,"owner":0}));
            current["files"].as_array_mut().unwrap().push(json!({"path":format!("/SYNTHETIC_PRIVATE_CURRENT/{index}"),"digest":"after","device":1,"inode":index+1,"mode":0o100600,"owner":0}));
        }
        let evidence = configuration_conflict(&expected, &current);
        assert_eq!(
            evidence["snapshot_diff_keys"],
            json!(["files", "properties"])
        );
        assert_eq!(
            evidence["property_values"],
            json!({"Requires":{"expected":"sysinit.target system.slice","current":"system.slice sysinit.target"}})
        );
        assert_eq!(
            evidence["withheld_property_diff_keys"]
                .as_array()
                .unwrap()
                .len(),
            8
        );
        assert!(!evidence.to_string().contains("SYNTHETIC_PRIVATE"));
        assert_eq!(evidence["files"]["difference_count"], 33);
        assert_eq!(
            evidence["files"]["differences"].as_array().unwrap().len(),
            32
        );
        assert_eq!(evidence["files"]["differences_truncated"], true);
        assert_eq!(
            evidence["files"]["differences"][0]["diff_keys"],
            json!(["digest", "inode", "path"])
        );
        assert_eq!(
            evidence["files"]["differences"][0]["expected"]["digest"],
            "before"
        );
        current["properties"]["Requires"] = json!("a".repeat(2049));
        let evidence = configuration_conflict(&expected, &current);
        assert_eq!(
            evidence["property_values"]["Requires"]["current"]["truncated"],
            true
        );
        assert_eq!(
            evidence["property_values"]["Requires"]["current"]["bytes"],
            2049
        );
        assert_eq!(
            evidence["property_values"]["Requires"]["current"]["prefix"]
                .as_str()
                .unwrap()
                .len(),
            2048
        );
    }
    #[test]
    fn original_inactive_state_stays_inactive_on_resume() {
        let (_temp, engine, e, _root) = fixture(false);
        let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let j = execute(&engine, &p);
        assert_eq!(j["data"]["status"], "completed", "{j}");
        let p = engine.request(json!({"command":"plan_service_resume","job_id":j["data"]["id"]}));
        assert_eq!(execute(&engine, &p["data"])["data"]["status"], "completed");
        assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "inactive");
        assert!(!log(&engine).contains("start:"));
    }
    #[test]
    fn pending_or_failed_target_refuses_resume_before_mutation() {
        for failed in [false, true] {
            let (_temp, engine, e, _root) = fixture(true);
            let plan = request(&engine, &e, "plan_service_quiesce")["data"].clone();
            let paused = execute(&engine, &plan);
            assert_eq!(paused["data"]["status"], "completed", "{paused}");
            let mut properties = load(&stored(&engine)).unwrap();
            if failed {
                properties["ActiveState"] = json!("failed");
                properties["SubState"] = json!("failed");
            } else {
                properties["Job"] = json!("7 /org/freedesktop/systemd1/job/7");
            }
            save(&stored(&engine), &properties).unwrap();
            let before = log(&engine);
            let resume = engine
                .request(json!({"command":"plan_service_resume","job_id":paused["data"]["id"]}));
            assert_eq!(
                resume["error"]["code"], "service_state_unsupported",
                "{resume}"
            );
            assert!(Path::new(plan["service"]["hold"]["path"].as_str().unwrap()).exists());
            assert_eq!(log(&engine), before);
            assert_eq!(
                engine.request(json!({"command":"job","job_id":paused["data"]["id"]}))["data"],
                paused["data"]
            );
        }
    }
    #[test]
    fn root_alias_shared_stop_and_live_process_binding_are_rejected_before_mutation() {
        for (key, value, code) in [
            ("Id", json!("alias.service"), "service_unit_unsupported"),
            (
                "RequiredBy",
                json!("neighbor.service"),
                "service_shared_stop_scope",
            ),
            (
                "PropagatesStopTo",
                json!("neighbor.service"),
                "service_shared_stop_scope",
            ),
            ("KillMode", json!("process"), "service_cgroup_unsupported"),
            (
                "EnvironmentFiles",
                json!("/some-file"),
                "service_root_unbound",
            ),
            (
                "fixture_process_root",
                json!("/different-root"),
                "service_process_unbound",
            ),
        ] {
            let (_temp, engine, e, _root) = fixture(true);
            let mut p = load(&stored(&engine)).unwrap();
            p[key] = value;
            save(&stored(&engine), &p).unwrap();
            let result = request(&engine, &e, "plan_service_quiesce");
            assert_eq!(result["error"]["code"], code, "{key}: {result}");
            assert_eq!(log(&engine), "");
        }
    }
    #[test]
    fn stale_unit_or_running_generation_rejects_approval() {
        for key in ["Restart", "InvocationID"] {
            let (_temp, engine, e, _root) = fixture(true);
            let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
            let mut properties = load(&stored(&engine)).unwrap();
            properties[key] = json!("changed");
            save(&stored(&engine), &properties).unwrap();
            let j = execute(&engine, &p);
            assert_eq!(j["error"]["code"], "stale_service_plan", "{j}");
            assert_eq!(log(&engine), "");
            assert!(!Path::new(p["service"]["hold"]["path"].as_str().unwrap()).exists());
        }
    }
    #[test]
    fn external_unit_or_owned_file_edit_survives_resume_conflict() {
        for edit_hold in [true, false] {
            let (_temp, engine, e, _root) = fixture(true);
            let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
            let j = execute(&engine, &p);
            assert_eq!(j["data"]["status"], "completed", "{j}");
            let hold = PathBuf::from(p["service"]["hold"]["path"].as_str().unwrap());
            let path = if edit_hold {
                hold.clone()
            } else {
                PathBuf::from(
                    load(&stored(&engine)).unwrap()["FragmentPath"]
                        .as_str()
                        .unwrap(),
                )
            };
            fs::write(&path, "external edit retained").unwrap();
            let before = log(&engine);
            let result =
                engine.request(json!({"command":"plan_service_resume","job_id":j["data"]["id"]}));
            assert_eq!(
                result["error"]["code"], "service_restore_conflict",
                "{result}"
            );
            assert_eq!(fs::read_to_string(&path).unwrap(), "external edit retained");
            assert!(hold.exists());
            assert_eq!(log(&engine), before);
        }
    }
    #[test]
    fn cleanup_requires_live_hold_even_when_writer_checkbox_is_true() {
        {
            let (_temp, mut engine, e, _root) = fixture(false);
            engine.service_fixture = None;
            assert!(engine.check_cleanup_services(&e).unwrap().is_empty());
            assert_eq!(
                engine
                    .service_properties("system", "target.service")
                    .unwrap_err()
                    .code,
                "service_test_fixture_required"
            );
        }
        let (_temp, engine, e, root) = fixture(false);
        fs::remove_file(root.join("settings.json")).unwrap();
        let request = json!({"command":"plan_cleanup","environment_id":e["id"],"recipe":"repair_login","writers_confirmed_stopped":true,"official_logout":false,"categories":[]});
        assert_eq!(
            engine.request(request.clone())["error"]["code"],
            "service_quiescence_required"
        );
        let p = super::tests::request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let j = execute(&engine, &p);
        assert_eq!(j["data"]["status"], "completed", "{j}");
        let cleanup = engine.request(request);
        assert_eq!(cleanup["ok"], true, "{cleanup}");
        fs::remove_file(Path::new(p["service"]["hold"]["path"].as_str().unwrap())).unwrap();
        let j = execute(&engine, &cleanup["data"]);
        assert_eq!(j["error"]["code"], "service_restore_conflict", "{j}");
    }
    #[test]
    fn external_permit_creation_and_condition_reset_cannot_report_a_hold() {
        for permit in [true, false] {
            let (_temp, engine, e, _root) = fixture(true);
            let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
            let j = execute(&engine, &p);
            assert_eq!(j["data"]["status"], "completed", "{j}");
            if permit {
                let original = engine
                    .service_original(j["data"]["id"].as_str().unwrap())
                    .unwrap();
                fs::write(
                    original["extra"]["service"]["hold"]["condition_path"]
                        .as_str()
                        .unwrap(),
                    "external permit",
                )
                .unwrap();
            } else {
                let mut properties = load(&stored(&engine)).unwrap();
                properties["Conditions"] = json!([]);
                save(&stored(&engine), &properties).unwrap();
            }
            assert_eq!(
                request(&engine, &e, "service_inspect")["data"]["quiesced"],
                false
            );
            let r =
                engine.request(json!({"command":"plan_service_resume","job_id":j["data"]["id"]}));
            assert_eq!(r["ok"], false, "{r}");
        }
    }
    #[test]
    fn interrupted_stop_is_query_only_and_independent_restore_keeps_original_running_state() {
        let (_temp, engine, e, _root) = fixture(true);
        let mut properties = load(&stored(&engine)).unwrap();
        properties["fixture_fail_stop"] = json!(true);
        save(&stored(&engine), &properties).unwrap();
        let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let j = execute(&engine, &p);
        assert_eq!(j["data"]["status"], "needs_reconciliation", "{j}");
        assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "active");
        let before = log(&engine);
        execute(&engine, &p);
        assert_eq!(log(&engine), before);
        let restore =
            engine.request(json!({"command":"plan_service_resume","job_id":j["data"]["id"]}));
        assert_eq!(restore["ok"], true, "{restore}");
        let resumed = execute(&engine, &restore["data"]);
        assert_eq!(resumed["data"]["status"], "completed", "{resumed}");
        assert_eq!(load(&stored(&engine)).unwrap()["ActiveState"], "active");
        assert_eq!(log(&engine).matches("stop:").count(), 1);
        assert!(!log(&engine).contains("start:"));
    }
    #[test]
    fn plans_keep_root_binding_without_exporting_service_secrets_and_reject_neighbor_start_scope() {
        let (_temp, engine, e, _root) = fixture(true);
        let mut properties = load(&stored(&engine)).unwrap();
        properties["Environment"]
            .as_array_mut()
            .unwrap()
            .push(json!("ANTHROPIC_API_KEY=SYNTHETIC_SECRET_NEVER_PERSIST"));
        properties["Requires"] = json!("sysinit.target system.slice");
        save(&stored(&engine), &properties).unwrap();
        let p = request(&engine, &e, "plan_service_quiesce");
        assert_eq!(p["ok"], true, "{p}");
        assert!(!p.to_string().contains("SYNTHETIC_SECRET_NEVER_PERSIST"));
        let original = engine
            .service_original(p["data"]["id"].as_str().unwrap())
            .unwrap();
        assert!(!original
            .to_string()
            .contains("SYNTHETIC_SECRET_NEVER_PERSIST"));
        properties["Wants"] = json!("neighbor.service");
        save(&stored(&engine), &properties).unwrap();
        assert_eq!(
            request(&engine, &e, "plan_service_quiesce")["error"]["code"],
            "service_shared_start_scope"
        );
    }
    #[test]
    fn replacing_environment_directory_invalidates_service_restoration() {
        let (_temp, engine, e, root) = fixture(true);
        let p = request(&engine, &e, "plan_service_quiesce")["data"].clone();
        let j = execute(&engine, &p);
        assert_eq!(j["data"]["status"], "completed", "{j}");
        fs::rename(&root, root.with_file_name("original-synthetic-root")).unwrap();
        fs::create_dir(&root).unwrap();
        let r = engine.request(json!({"command":"plan_service_resume","job_id":j["data"]["id"]}));
        assert_eq!(r["error"]["code"], "service_restore_conflict", "{r}");
        assert!(Path::new(p["service"]["hold"]["path"].as_str().unwrap()).exists());
    }
    #[test]
    fn unit_validation_has_no_shell_or_pattern_escape() {
        for unit in [
            "*.service",
            "--all.service",
            "a.service;touch /tmp/x",
            "../a.service",
            "a@.service",
            "ssh",
            "a\\x20b.service",
        ] {
            assert!(unit_name(unit).is_err(), "{unit}");
        }
        assert!(unit_name("lintel-example@first.service").is_ok());
        assert_eq!(
            bus_path("3_a.service"),
            "/org/freedesktop/systemd1/unit/_33_5fa_2eservice"
        );
    }
}
