//! Interactive sessions use a terminal, separately from durable mutation jobs.
use super::*;
use std::os::unix::fs::PermissionsExt;
fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
impl Controller {
    pub(super) fn launch_remote(&self, alias: &str, payload: &Value) -> Result<Value> {
        exact_fields(payload, &["op", "alias", "environment_id"], &[])?;
        let environment_id = valid_id(field(payload, "environment_id")?)?;
        let terminal = self.terminal.as_ref().ok_or_else(|| {
            failure(
                "platform_unsupported",
                "桌面远端交互启动需要 macOS Terminal；Linux 终端请使用 lintel launch",
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
            .dispatch(json!({"op":"launch","alias":"synthetic-host","environment_id":"env-1"}))
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
            "env-1",
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
                .dispatch(json!({"op":"launch","alias":"synthetic-host","environment_id":"env-1"}))
                .unwrap()["error"]["code"],
            "environment_retired"
        );
        for request in [
            json!({"op":"launch","alias":"synthetic-host","environment_id":"x;bad"}),
            json!({"op":"launch","alias":"synthetic-host","environment_id":"env-1","proxy_url":"http://127.0.0.1:1"}),
        ] {
            assert!(controller.dispatch(request).is_err());
        }
        assert!(!temp.path().join("opened").exists());
    }
    #[test]
    fn terminal_rejection_and_unsupported_platform_are_specific() {
        let (_temp, mut controller) = super::super::tests::fixture(CONTEXT);
        opener(&controller, "exit 1");
        let request = json!({"op":"launch","alias":"synthetic-host","environment_id":"env-1"});
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
