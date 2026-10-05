//! Interactive sessions use a terminal, separately from durable mutation jobs.
use super::*;
use std::os::unix::fs::PermissionsExt;
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
fn now_string() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| "0".into())
}
impl Controller {
    /// Freeze the exact runner used for this preview, before any Terminal attempt.
    pub(super) fn pin_launch_binding(
        &self,
        alias: &str,
        response: &Value,
        bound: Option<&Value>,
    ) -> Result<()> {
        if response["ok"] != true {
            return Ok(());
        }
        let plan = &response["data"];
        let request_id = field(plan, "id")?;
        valid_core_id(request_id)?;
        let resume = plan["kind"] == "resume";
        let target = if resume {
            &plan["resume"]
        } else {
            &plan["launch_request"]
        };
        let dir = self.state.join("launches").join(alias);
        private_dir(&dir)?;
        let path = dir.join(format!("{request_id}.json"));
        let _held = lock(&path.with_extension("lock"))?;
        if path.exists() {
            let existing = load(&path)?;
            if existing["request_id"] != request_id
                || existing["runner_digest"] != bound.cloned().unwrap_or(Value::Null)
            {
                return Err(failure(
                    "launch_context_conflict",
                    "原启动 ID 已有不同 runner 绑定，保留原记录",
                ));
            }
            return Ok(()); // never erase an earlier attempt/dedup record
        }
        save(
            &path,
            &json!({
                "request_id": request_id, "alias": alias,
                "runner_digest": bound.cloned().unwrap_or(Value::Null),
                "status":"planned", "attempted":false,
                "mode":if resume {"resume"} else {"interactive"},
                "environment_id":plan["environment_id"],
                "root":target["config_root"], "project_cwd":target["project_cwd"],
                "pinned_at":now_string(), "created_at":plan["created_at"],
            }),
        )
    }
    pub(super) fn resolve_launch_binding(
        &self,
        alias: &str,
        request_id: &str,
    ) -> Result<(Option<Value>, bool)> {
        valid_core_id(request_id)?;
        let path = self
            .state
            .join("launches")
            .join(alias)
            .join(format!("{request_id}.json"));
        if path.exists() {
            let record = load(&path)?;
            if record["request_id"] != request_id
                || (record["alias"].is_string() && record["alias"] != alias)
                || !(record["runner_digest"].is_null() || record["runner_digest"].is_string())
                || record.get("runner_digest").is_none()
            {
                return Err(failure(
                    "local_record_invalid",
                    "原启动记录的 ID 或 runner 绑定无效",
                ));
            }
            let bound = record["runner_digest"].as_str().map(|d| json!(d));
            installation::runner_program(bound.as_ref())?;
            return Ok((bound, true)); // null is a deliberate PATH pin
        }
        Ok((self.binding(alias)?, false))
    }
    fn launch_record_view(alias: &str, record: &Value) -> Value {
        let mut view = json!({
            "request_id":record["request_id"], "alias":alias,
            "status":record["status"].as_str().unwrap_or("planned"),
            "runner_digest":record["runner_digest"],
            "binding_resolution":if record["runner_digest"].is_string() {"pinned_digest"} else {"pinned_path"},
            "observed":"local_record", "query":"not_requested",
        });
        for key in [
            "mode",
            "environment_id",
            "root",
            "project_cwd",
            "created_at",
            "recorded_at",
            "phase",
            "attempted",
            "message",
        ] {
            if !record[key].is_null() {
                view[key] = record[key].clone();
            }
        }
        view
    }
    pub(super) fn launch_inventory(&self, selected_alias: Option<&str>) -> Result<Vec<Value>> {
        let root = self.state.join("launches");
        let mut out = vec![];
        if !root.exists() {
            return Ok(out);
        }
        for entry in fs::read_dir(root).map_err(storage)? {
            let entry = entry.map_err(storage)?;
            if !entry.file_type().map_err(storage)?.is_dir() {
                continue;
            }
            let Some(alias) = entry
                .file_name()
                .to_str()
                .map(str::to_owned)
                .filter(|s| valid_alias(s).is_ok())
            else {
                continue;
            };
            if selected_alias.is_some_and(|selected| selected != alias) {
                continue;
            }
            for item in fs::read_dir(entry.path()).map_err(storage)? {
                let path = item.map_err(storage)?.path();
                if path.extension().is_none_or(|ext| ext != "json") {
                    continue;
                }
                let Some(id) = path.file_stem().and_then(|v| v.to_str()) else {
                    continue;
                };
                if self.resolve_launch_binding(&alias, id).is_err() {
                    continue;
                }
                if let Ok(record) = load(&path) {
                    out.push(Self::launch_record_view(&alias, &record));
                }
            }
        }
        out.sort_by(|a, b| {
            b["recorded_at"]
                .as_str()
                .unwrap_or("")
                .cmp(a["recorded_at"].as_str().unwrap_or(""))
        });
        Ok(out)
    }
    pub(super) fn launch_query_response(&self, alias: &str, request_id: &str) -> Result<Value> {
        let (bound, pinned) = self.resolve_launch_binding(alias, request_id)?;
        let path = self
            .state
            .join("launches")
            .join(alias)
            .join(format!("{request_id}.json"));
        let record = if pinned { Some(load(&path)?) } else { None };
        let response = self.runner_call(
            alias,
            &json!({"command":"launch_query","request_id":request_id}),
            false,
            bound.as_ref(),
        );
        let mut view = match response {
            Ok(response)
                if response["ok"] == true
                    && response["data"]["request_id"] == request_id
                    && response["data"]["status"].is_string() =>
            {
                let mut data = response["data"].clone();
                data["query"] = json!("live");
                if record.as_ref().is_some_and(|r| r["attempted"] == true)
                    && data["observed"] == "plan"
                {
                    data["remote_status"] = data["status"].clone();
                    data["status"] = json!("uncertain");
                    data["message"] = json!("本机已有打开意图，远端尚只观察到计划。请核对原 Terminal；不再次打开或重发。");
                }
                data
            }
            other => {
                let Some(record) = &record else {
                    return match other {
                        Ok(response) if response["ok"] != true => Ok(response),
                        Ok(_) => Err(failure(
                            "protocol_invalid",
                            "远端启动查询的 ID 或状态不匹配",
                        )),
                        Err(error) => Err(error),
                    };
                };
                let mut local = Self::launch_record_view(alias, record);
                local["query"] = json!("unavailable");
                local["local_status"] = local["status"].clone();
                local["status"] = json!("uncertain");
                local["message"] = json!(
                    "原 runner 的实时结果未取得；已保留本机原启动记录和冻结绑定，只查询，不重试。"
                );
                local
            }
        };
        view["alias"] = json!(alias);
        view["runner_digest"] = bound.clone().unwrap_or(Value::Null);
        view["binding_resolution"] = json!(match (pinned, bound.is_some()) {
            (true, true) => "pinned_digest",
            (true, false) => "pinned_path",
            (false, true) => "current_binding",
            (false, false) => "current_path",
        });
        Ok(json!({"ok":true,"data":view}))
    }
    pub(super) fn launch_remote(&self, alias: &str, payload: &Value) -> Result<Value> {
        exact_operation_fields(payload)?;
        let environment_id = field(payload, "environment_id")?;
        let terminal = self.terminal.as_ref().ok_or_else(|| {
            failure(
                "platform_unsupported",
                "此远端交互启动需要 macOS Terminal；当前控制端平台不支持",
            )
        })?;
        let bound = self.binding(alias)?;
        // The runner checks retirement, the exact root and executable again when
        // Terminal connects. Remote-returned paths never enter the local script.
        let preflight = self.runner_call(
            alias,
            &json!({"command":"launch_context","environment_id":environment_id}),
            false,
            bound.as_ref(),
        )?;
        if preflight["ok"] != true {
            return Ok(preflight);
        }
        let program = installation::runner_program(bound.as_ref())?;
        let remote = format!("{program} launch {}", quote(environment_id));
        let args = ssh_options(true)
            .into_iter()
            .map(quote)
            .chain([quote(alias), quote(&remote)])
            .collect::<Vec<_>>()
            .join(" ");
        let content = format!(
            "#!/bin/sh\nexec {} {args}\n",
            quote(&self.transport.ssh.to_string_lossy())
        );
        let root = self.state.join("launches");
        private_dir(&root)?;
        let version = bound.as_ref().and_then(Value::as_str).unwrap_or("path");
        // One filename per immutable alias/root/runner command: opening the same
        // session twice cannot rewrite an earlier request to another version.
        let path = root.join(format!("{alias}-{environment_id}-{version}.command"));
        let _held = lock(&path.with_extension("lock"))?;
        let pending = path.with_extension("pending");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o700)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&pending)
            .map_err(storage)?;
        file.set_permissions(fs::Permissions::from_mode(0o700))
            .map_err(storage)?;
        file.write_all(content.as_bytes()).map_err(storage)?;
        file.sync_all().map_err(storage)?;
        fs::rename(&pending, &path).map_err(storage)?;
        let status = Command::new(terminal)
            .args(["-a", "Terminal"])
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|_| failure("launch_failed", "无法向 Terminal 提交远端会话"))?;
        if !status.success() {
            return Err(failure("launch_failed", "Terminal 未接受远端启动请求"));
        }
        Ok(
            json!({"ok":true,"data":{"status":"launch_requested","message":"已请求在 Terminal 打开此远端环境。登录与会话状态请在终端确认；会话随 SSH 连接结束。"}}),
        )
    }

    /// Finite frozen-request launch. Resolves the immutable request ID through the
    /// bound runner's frozen `plan_show` preflight, freezes that runner binding, and
    /// opens a PTY session that hands only the request ID + approval to the bound
    /// runner. No remote-returned path/executable ever enters the local script.
    pub(super) fn launch_request_remote(&self, alias: &str, payload: &Value) -> Result<Value> {
        self.launch_frozen_remote(alias, payload, "interactive")
    }
    pub(super) fn resume_request_remote(&self, alias: &str, payload: &Value) -> Result<Value> {
        self.launch_frozen_remote(alias, payload, "resume")
    }
    fn launch_frozen_remote(&self, alias: &str, payload: &Value, mode: &str) -> Result<Value> {
        exact_operation_fields(payload)?;
        let request_id = field(payload, "request_id")?;
        valid_core_id(request_id)?;
        let dir = self.state.join("launches").join(alias);
        private_dir(&dir)?;
        let record_path = dir.join(format!("{request_id}.json"));
        let _held = lock(&record_path.with_extension("lock"))?;
        if record_path.exists() && load(&record_path)?["attempted"] == true {
            // Once attempted, the sole recovery is a query of the original runner.
            // Missing/changed approval cannot turn it into a second Terminal.
            return self.launch_query_response(alias, request_id);
        }
        let approval = field(payload, "approval")?;
        if !lintel_operations::valid_plan_hash(approval) {
            return Err(failure(
                "invalid_approval",
                "请使用原启动预览返回的准确 64 位小写 hex hash",
            ));
        }
        let terminal = self.terminal.as_ref().ok_or_else(|| {
            failure(
                "platform_unsupported",
                "此远端交互启动需要 macOS Terminal；当前控制端平台不支持",
            )
        })?;
        let (bound, pinned) = self.resolve_launch_binding(alias, request_id)?;
        if !pinned {
            return Err(failure(
                "launch_context_missing",
                "请通过此 controller 准备并审阅启动预览，以固定原 runner",
            ));
        }
        // Re-read the approved plan through the original runner. Hash/mode are
        // checked before opening Terminal; remote paths are never interpolated.
        let preflight = self.runner_call(
            alias,
            &json!({"command":"plan_show","plan_id":request_id}),
            false,
            bound.as_ref(),
        )?;
        if preflight["ok"] != true {
            return Ok(preflight);
        }
        let plan = &preflight["data"];
        if plan["id"] != request_id || plan["hash"] != approval {
            return Err(failure(
                "approval_mismatch",
                "原启动计划与批准不一致；请重新核对原预览",
            ));
        }
        if (mode == "resume" && plan["kind"] != "resume")
            || (mode == "interactive" && plan["kind"] != "launch")
        {
            return Err(failure("invalid_launch", "原请求的启动模式不一致"));
        }
        if mode == "resume" && plan["resume"]["supported"] != true {
            return Err(failure(
                "resume_unsupported",
                "此组合不支持原生续聊；可继续阅读或整理新上下文",
            ));
        }
        let program = installation::runner_program(bound.as_ref())?;
        let command = if mode == "resume" {
            "resume_request"
        } else {
            "launch_request"
        };
        let remote = format!(
            "{program} {command} {} {}",
            quote(request_id),
            quote(approval)
        );
        let args = ssh_options(true)
            .into_iter()
            .map(quote)
            .chain([quote(alias), quote(&remote)])
            .collect::<Vec<_>>()
            .join(" ");
        let content = format!(
            "#!/bin/sh\nexec {} {args}\n",
            quote(&self.transport.ssh.to_string_lossy())
        );
        let version = bound.as_ref().and_then(Value::as_str).unwrap_or("path");
        let path = self
            .state
            .join("launches")
            .join(format!("{alias}-{request_id}-{version}-{command}.command"));
        let mut record = load(&record_path)?;
        record["attempted"] = json!(true);
        record["status"] = json!(if mode == "resume" {
            "resume_attempt"
        } else {
            "launch_attempt"
        });
        record["phase"] = json!("preparing_terminal_script");
        record["recorded_at"] = json!(now_string());
        save(&record_path, &record)?;
        let pending = path.with_extension("pending");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o700)
            .custom_flags(libc::O_NOFOLLOW)
            .open(&pending)
            .map_err(storage)?;
        file.write_all(content.as_bytes()).map_err(storage)?;
        file.sync_all().map_err(storage)?;
        fs::rename(&pending, &path).map_err(storage)?;
        File::open(path.parent().unwrap())
            .and_then(|f| f.sync_all())
            .map_err(storage)?;
        record["phase"] = json!("opening_terminal");
        save(&record_path, &record)?;
        let opened = Command::new(terminal)
            .args(["-a", "Terminal"])
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        if opened.is_ok_and(|status| status.success()) {
            record["status"] = json!("launch_requested");
            record["message"] = json!(if mode == "resume" {
                "已请求打开原远端续聊；包口令在 SSH Terminal 再次无回显输入。登录与恢复未观察，重复请求只查询。"
            } else {
                "已请求打开原远端启动。实际客户端运行未观察，重复请求只查询。"
            });
            save(&record_path, &record)?;
            return Ok(json!({"ok":true,"data":Self::launch_record_view(alias, &record)}));
        }
        record["status"] = json!("launch_failed");
        save(&record_path, &record)?;
        Err(failure(
            "launch_failed",
            "Terminal 打开结果未确认；保留原启动 ID，只查询，不自动重试",
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    const CONTEXT: &str = r#"printf '%s\n' '{"ok":true,"data":{"root":"REMOTE;touch /bad","executable":"REMOTE_EXEC"}}'"#;
    fn opener(controller: &Controller, body: &str) {
        let path = controller.terminal.as_ref().unwrap();
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
    }
    const ID: &str = "00000000-0000-4000-8000-0000000000ab";
    fn preview(hash: &str) -> Value {
        json!({"ok":true,"data":{"id":ID,"hash":hash,"kind":"launch","environment_id":"00000000-0000-4000-8000-000000000001","launch_request":{"config_root":"REMOTE;touch /bad","project_cwd":"REMOTE_CWD"}}})
    }
    #[test]
    fn frozen_launch_serializes_one_attempt_and_retains_original_binding_after_removal() {
        let hash = "c".repeat(64);
        let plan = preview(&hash);
        let query = json!({"ok":true,"data":{"request_id":ID,"status":"launch_intent","observed":"record"}});
        let body = format!("if grep -q '\"command\":\"plan_show\"' input; then printf '%s\\n' '{}'; else printf '%s\\n' '{}'; fi",plan,query);
        let (temp, c) = super::super::tests::fixture(&body);
        let original = json!("a".repeat(64));
        c.pin_launch_binding("synthetic-host", &plan, Some(&original))
            .unwrap();
        private_dir(&c.state.join("bindings")).unwrap();
        save(
            &c.state.join("bindings/synthetic-host.json"),
            &json!({"digest":"b".repeat(64)}),
        )
        .unwrap();
        opener(
            &c,
            &format!(
                "echo opened >> '{}'/opened\nprintf '%s\\n' \"$@\" > '{}'/open-args",
                temp.path().display(),
                temp.path().display()
            ),
        );
        let request =
            json!({"op":"launch_request","alias":"synthetic-host","request_id":ID,"approval":hash});
        std::thread::scope(|scope| {
            let first = scope.spawn(|| c.dispatch(request.clone()).unwrap());
            let second = scope.spawn(|| c.dispatch(request.clone()).unwrap());
            assert_eq!(first.join().unwrap()["ok"], true);
            assert_eq!(second.join().unwrap()["ok"], true);
        });
        assert_eq!(
            fs::read_to_string(temp.path().join("opened"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        let args = fs::read_to_string(temp.path().join("open-args")).unwrap();
        let script = fs::read_to_string(args.lines().last().unwrap()).unwrap();
        assert!(
            script.contains(ID)
                && script.contains(&hash)
                && script.contains(original.as_str().unwrap())
        );
        assert!(!script.contains("REMOTE") && !script.contains(&"b".repeat(64)));
        let record = load(
            &c.state
                .join("launches/synthetic-host")
                .join(format!("{ID}.json")),
        )
        .unwrap();
        assert_eq!(record["attempted"], true);
        assert_eq!(record["runner_digest"], original);
        assert!(!record.to_string().contains(&hash));
        c.dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
            .unwrap();
        let before = fs::read_to_string(temp.path().join("args")).unwrap();
        let hosts = c.dispatch(json!({"op":"hosts"})).unwrap();
        assert_eq!(hosts["data"]["launches"][0]["request_id"], ID);
        assert_eq!(
            fs::read_to_string(temp.path().join("args")).unwrap(),
            before
        );
        let repeat = c.dispatch(json!({"op":"launch_request","alias":"synthetic-host","request_id":ID,"approval":"wrong"})).unwrap();
        assert_eq!(repeat["data"]["request_id"], ID);
        assert_eq!(repeat["data"]["runner_digest"], original);
        assert_eq!(
            fs::read_to_string(temp.path().join("opened"))
                .unwrap()
                .lines()
                .count(),
            1
        );
        assert!(!fs::read_to_string(temp.path().join("args"))
            .unwrap()
            .contains(&"b".repeat(64)));
    }
    #[test]
    fn wrong_query_identity_preserves_original_and_unattempted_removed_alias_cannot_open() {
        let (temp, c) = super::super::tests::fixture(
            r#"printf '%s\n' '{"ok":true,"data":{"request_id":"other-id","status":"completed"}}'"#,
        );
        let plan = preview(&"c".repeat(64));
        c.pin_launch_binding("synthetic-host", &plan, None).unwrap();
        let response = c
            .dispatch(json!({"op":"launch_query","alias":"synthetic-host","request_id":ID}))
            .unwrap();
        assert_eq!(response["data"]["request_id"], ID);
        assert_eq!(response["data"]["status"], "uncertain");
        assert_eq!(response["data"]["query"], "unavailable");
        assert_eq!(response["data"]["binding_resolution"], "pinned_path");
        assert!(!response.to_string().contains("other-id"));
        c.dispatch(json!({"op":"remove_host","alias":"synthetic-host"}))
            .unwrap();
        let before = fs::read_to_string(temp.path().join("args")).unwrap();
        assert_eq!(c.dispatch(json!({"op":"launch_request","alias":"synthetic-host","request_id":ID,"approval":"c".repeat(64)})).unwrap_err().code,"host_not_registered");
        assert_eq!(
            fs::read_to_string(temp.path().join("args")).unwrap(),
            before
        );
        assert!(!temp.path().join("open").exists());
    }
    #[test]
    fn frozen_plan_mismatch_does_not_record_an_attempt_or_open() {
        let plan = preview(&"d".repeat(64));
        let (temp, c) = super::super::tests::fixture(&format!("printf '%s\\n' '{}'", plan));
        c.pin_launch_binding("synthetic-host", &plan, None).unwrap();
        assert_eq!(c.dispatch(json!({"op":"launch_request","alias":"synthetic-host","request_id":ID,"approval":"c".repeat(64)})).unwrap_err().code,"approval_mismatch");
        assert_eq!(
            load(
                &c.state
                    .join("launches/synthetic-host")
                    .join(format!("{ID}.json"))
            )
            .unwrap()["attempted"],
            false
        );
        assert!(!temp.path().join("open").exists());
    }
    #[test]
    fn remote_launch_pins_runner_uses_pty_and_ignores_returned_paths() {
        let (temp, controller) = super::super::tests::fixture(CONTEXT);
        let sha = "a".repeat(64);
        private_dir(&controller.state.join("bindings")).unwrap();
        save(
            &controller.state.join("bindings/synthetic-host.json"),
            &json!({"digest":sha}),
        )
        .unwrap();
        opener(
            &controller,
            &format!(
                "printf '%s\\n' \"$@\" > '{}'",
                temp.path().join("opened").display()
            ),
        );
        let response = controller
            .dispatch(json!({"op":"launch","alias":"synthetic-host","environment_id":"00000000-0000-4000-8000-000000000001"}))
            .unwrap();
        assert_eq!(response["data"]["status"], "launch_requested");
        let args = fs::read_to_string(temp.path().join("opened")).unwrap();
        let path = args.lines().last().unwrap();
        let script = fs::read_to_string(path).unwrap();
        for expected in [
            "'-tt'",
            "'-oRequestTTY=force'",
            "'-oProxyCommand=none'",
            "'-oRemoteCommand=none'",
            "'-oPermitLocalCommand=no'",
            "'-oClearAllForwardings=yes'",
            "'-oStrictHostKeyChecking=yes'",
            "launch",
            "00000000-0000-4000-8000-000000000001",
            &sha,
        ] {
            assert!(script.contains(expected), "{expected}");
        }
        assert!(!script.contains("REMOTE"));
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert!(fs::read_to_string(temp.path().join("input"))
            .unwrap()
            .contains("launch_context"));
    }
    #[test]
    fn failed_preflight_or_invalid_request_never_opens_terminal() {
        let (temp, controller) = super::super::tests::fixture(
            r#"printf '%s\n' '{"ok":false,"error":{"code":"environment_retired","message":"retired"}}'"#,
        );
        opener(
            &controller,
            &format!("touch '{}'", temp.path().join("opened").display()),
        );
        assert_eq!(
            controller
                .dispatch(json!({"op":"launch","alias":"synthetic-host","environment_id":"00000000-0000-4000-8000-000000000001"}))
                .unwrap()["error"]["code"],
            "environment_retired"
        );
        for request in [
            json!({"op":"launch","alias":"synthetic-host","environment_id":"x;bad"}),
            json!({"op":"launch","alias":"synthetic-host","environment_id":"00000000-0000-4000-8000-000000000001","proxy_url":"http://127.0.0.1:1"}),
        ] {
            assert!(controller.dispatch(request).is_err());
        }
        assert!(!temp.path().join("opened").exists());
    }
    #[test]
    fn terminal_rejection_and_unsupported_platform_are_specific() {
        let (_temp, mut controller) = super::super::tests::fixture(CONTEXT);
        opener(&controller, "exit 1");
        let request = json!({"op":"launch","alias":"synthetic-host","environment_id":"00000000-0000-4000-8000-000000000001"});
        assert_eq!(
            controller.dispatch(request.clone()).unwrap_err().code,
            "launch_failed"
        );
        controller.terminal = None;
        assert_eq!(
            controller.dispatch(request).unwrap_err().code,
            "platform_unsupported"
        );
    }
}
