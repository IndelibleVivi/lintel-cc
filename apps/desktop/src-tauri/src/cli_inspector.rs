//! Explicitly selected CLI; fixed static argv, bounded output/time, no shell.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    os::unix::fs::MetadataExt,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
const LIMIT: u64 = 1024 * 1024;
fn fail(code: &str, message: &str) -> Value {
    json!({"ok":false,"error":{"code":code,"message":message}})
}
fn static_call(path: &Path, args: &[&str]) -> Result<Value, &'static str> {
    use std::os::fd::AsRawFd;
    let mut child = Command::new(path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "cli_spawn_failed")?;
    let mut stdout = child.stdout.take().unwrap();
    let mut stderr = child.stderr.take().unwrap();
    for fd in [stdout.as_raw_fd(), stderr.as_raw_fd()] {
        let ready = unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            flags >= 0 && libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) >= 0
        };
        if !ready {
            let _ = child.kill();
            let _ = child.wait();
            return Err("cli_pipe_failed");
        }
    }
    let start = Instant::now();
    let mut bytes = Vec::new();
    let mut errors = Vec::new();
    fn drain(pipe: &mut impl Read, into: &mut Vec<u8>) -> Result<(), &'static str> {
        let mut buffer = [0u8; 8192];
        loop {
            match pipe.read(&mut buffer) {
                Ok(0) => return Ok(()),
                Ok(n) => {
                    into.extend_from_slice(&buffer[..n]);
                    if into.len() > LIMIT as usize {
                        return Err("cli_output_limit");
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => return Ok(()),
                Err(_) => return Err("cli_response_failed"),
            }
        }
    }
    let status = loop {
        if let Err(e) = drain(&mut stdout, &mut bytes).and_then(|_| drain(&mut stderr, &mut errors))
        {
            let _ = child.kill();
            let _ = child.wait();
            return Err(e);
        }
        match child.try_wait() {
            Ok(Some(s)) => {
                drain(&mut stdout, &mut bytes)?;
                drain(&mut stderr, &mut errors)?;
                break s;
            }
            Ok(None) if start.elapsed() < Duration::from_secs(5) => {
                thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("cli_timeout");
            }
        }
    };
    if !status.success() {
        return Err("cli_static_command_failed");
    }
    let response: Value = serde_json::from_slice(&bytes).map_err(|_| "cli_invalid_response")?;
    if response["ok"] != true {
        return Err("cli_interface_unavailable");
    }
    Ok(response["data"].clone())
}
fn identity(path: &Path) -> Result<(u64, u64, u64, i64, i64), &'static str> {
    let m = fs::metadata(path).map_err(|_| "cli_not_found")?;
    if !m.is_file() || m.mode() & 0o111 == 0 || m.mode() & 0o6000 != 0 {
        return Err("cli_not_executable");
    }
    Ok((m.dev(), m.ino(), m.len(), m.mtime(), m.mtime_nsec()))
}
fn canonical_architecture(value: &Value) -> Option<&'static str> {
    match value.as_str()? {
        "aarch64" | "arm64" => Some("aarch64"),
        "x86_64" => Some("x86_64"),
        _ => None,
    }
}
fn inspect(path: &Path) -> Result<Value, &'static str> {
    if !path.is_absolute() {
        return Err("cli_absolute_path_required");
    }
    let path = path.canonicalize().map_err(|_| "cli_not_found")?;
    let before = identity(&path)?;
    let mut header = [0u8; 20];
    fs::File::open(&path)
        .map_err(|_| "cli_not_found")?
        .read_exact(&mut header)
        .map_err(|_| "cli_wrong_format")?;
    #[cfg(target_os = "macos")]
    if &header[..4] != [0xcf, 0xfa, 0xed, 0xfe]
        || u32::from_le_bytes(header[4..8].try_into().unwrap())
            != if cfg!(target_arch = "aarch64") {
                0x0100000c
            } else {
                0x01000007
            }
    {
        return Err("cli_wrong_architecture");
    }
    #[cfg(target_os = "linux")]
    if &header[..4] != b"\x7fELF"
        || u16::from_le_bytes(header[18..20].try_into().unwrap())
            != if cfg!(target_arch = "aarch64") {
                183
            } else {
                62
            }
    {
        return Err("cli_wrong_architecture");
    }
    let version = static_call(&path, &["version", "--json"])?;
    if version["product"] != "Lintel" && version["product"] != "lintel" {
        return Err("cli_wrong_product");
    }
    if version["protocol"] != 1 {
        return Err("cli_protocol_mismatch");
    }
    let capabilities = static_call(&path, &["capabilities", "--json"])?;
    let context = static_call(&path, &["context", "--json"])?;
    if context["protocol"] != 1
        || !context["user"]["uid"].is_u64()
        || !context["user"]["euid"].is_u64()
        || !context["user"]["home"]
            .as_str()
            .is_some_and(|p| Path::new(p).is_absolute())
        || !context["state"]["path"]
            .as_str()
            .is_some_and(|p| Path::new(p).is_absolute())
    {
        return Err("cli_context_invalid");
    }
    let description = static_call(&path, &["describe", "job", "--json"])?;
    let schema = static_call(&path, &["schema", "job"])?;
    if identity(&path)? != before {
        return Err("cli_changed_during_check");
    }
    let parent = path.parent().ok_or("cli_invalid_path")?;
    let sidecar = parent.parent().unwrap_or(parent).join("candidate.json");
    let candidate = if sidecar.exists() {
        let m = fs::symlink_metadata(&sidecar).map_err(|_| "candidate_unreadable")?;
        if !m.is_file() || m.len() > LIMIT {
            return Err("candidate_invalid");
        }
        let v: Value =
            serde_json::from_slice(&fs::read(&sidecar).map_err(|_| "candidate_unreadable")?)
                .map_err(|_| "candidate_invalid")?;
        let digest = format!(
            "{:x}",
            Sha256::digest(fs::read(&path).map_err(|_| "cli_not_found")?)
        );
        let arch =
            canonical_architecture(&version["architecture"]).ok_or("cli_wrong_architecture")?;
        if v["schema"] != "lintel.cli-candidate/1"
            || v["protocol"] != version["protocol"]
            || v["version"] != version["version"]
            || v["platform"] != version["platform"]
            || canonical_architecture(&v["architecture"]) != Some(arch)
            || v["files"]
                .as_array()
                .and_then(|files| files.iter().find(|f| f["path"] == "bin/lintel"))
                .map(|f| f["sha256"].as_str())
                != Some(Some(digest.as_str()))
        {
            return Err("candidate_mismatch");
        }
        json!({"status":"bytes_match","identity":v["candidate_identity"],"source_revision":v["source_revision"],"signed":v["signed"],"source_authenticated":false})
    } else {
        json!({"status":"unknown","message":"独立构建／候选身份未知；静态接口已观察"})
    };
    if identity(&path)? != before {
        return Err("cli_changed_during_check");
    }
    Ok(
        json!({"executable":path,"version":version,"capabilities":capabilities,"context":context,"job_description":description,"job_schema":schema,"candidate":candidate,"checked_at":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs()}),
    )
}
#[tauri::command]
pub async fn inspect_cli(executable: String) -> Value {
    match tauri::async_runtime::spawn_blocking(move || inspect(Path::new(&executable))).await {
        Ok(Ok(data))=>json!({"ok":true,"data":data}),
        Ok(Err(code))=>fail(code,match code {
            "cli_absolute_path_required"=>"请输入 Lintel CLI 的完整绝对路径；可从候选新版本目录选择 bin/lintel。",
            "cli_not_found"=>"这个路径没有可读取的 CLI。先取得或构建当前平台产物，再选择实际 executable。",
            "cli_wrong_architecture" | "cli_wrong_format"=>"文件格式或 CPU 架构不适用于当前 App。选择当前平台的原生 Lintel CLI；脚本和其它平台产物不能用于此核对。",
            "cli_protocol_mismatch"=>"这份 CLI 的 protocol 与 App 不兼容。选择兼容的独立版本；保留原 state 和原任务 ID。",
            "candidate_mismatch" | "candidate_invalid"=>"候选声明与实际 executable 不一致。按安装指南重新核验该包，选择新的准确版本目录。",
            "cli_timeout" | "cli_output_limit"=>"CLI 的静态接口超过核对时限或输出限额，探测已停止。核对版本和来源，选择实现有限静态接口的 Lintel CLI。",
            _=>"CLI 静态核对未完成。核对路径、静态版本和候选文件；不会自动更改 state 或 PATH。",
        }),
        Err(_)=>fail("cli_check_failed","CLI 核对线程未完成"),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    #[test]
    fn wrong_path_or_format_never_executes() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path().join("wrong");
        fs::write(&p, b"#!/bin/sh\ntouch should-never-run\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(inspect(&p).unwrap_err(), "cli_wrong_architecture");
        assert_eq!(
            inspect(Path::new("relative")).unwrap_err(),
            "cli_absolute_path_required"
        );
        assert!(!t.path().join("should-never-run").exists());
        let mut header = [0u8; 20];
        #[cfg(target_os = "macos")]
        {
            header[..4].copy_from_slice(&[0xcf, 0xfa, 0xed, 0xfe]);
            header[4..8].copy_from_slice(
                &(if cfg!(target_arch = "aarch64") {
                    0x01000007u32
                } else {
                    0x0100000cu32
                })
                .to_le_bytes(),
            );
        }
        #[cfg(target_os = "linux")]
        {
            header[..4].copy_from_slice(b"\x7fELF");
            header[18..20].copy_from_slice(
                &(if cfg!(target_arch = "aarch64") {
                    62u16
                } else {
                    183u16
                })
                .to_le_bytes(),
            );
        }
        fs::write(&p, header).unwrap();
        assert_eq!(inspect(&p).unwrap_err(), "cli_wrong_architecture");
    }
    // Native inert executables exercise the actual process/header boundary. No
    // shell script is admitted and no test modifies process-wide HOME/state.
    fn native(t: &Path, protocol: u32, behavior: &str) -> std::path::PathBuf {
        native_architecture(t, protocol, behavior, Some(json!(std::env::consts::ARCH)))
    }
    fn native_architecture(
        t: &Path,
        protocol: u32,
        behavior: &str,
        arch: Option<Value>,
    ) -> std::path::PathBuf {
        let bin = t.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let executable = bin.join("lintel");
        let platform = std::env::consts::OS;
        let mut version = json!({"ok":true,"data":{"product":"Lintel","version":"0.1.0","protocol":protocol,"catalog_version":1,"platform":platform}});
        if let Some(arch) = arch {
            version["data"]["architecture"] = arch;
        }
        let version = version.to_string();
        let context=json!({"ok":true,"data":{"protocol":1,"user":{"uid":unsafe{libc::getuid()},"euid":unsafe{libc::geteuid()},"home":t.join("isolated-home")},"state":{"path":t.join("not-created-state"),"source":"explicit"}}}).to_string();
        let source=format!("#include <stdio.h>\n#include <string.h>\n#include <unistd.h>\nint main(int argc,char**argv){{{} if(argc>1 && !strcmp(argv[1],\"version\")) puts({});else if(argc>1 && !strcmp(argv[1],\"context\")) puts({});else puts(\"{{\\\"ok\\\":true,\\\"data\\\":{{}}}}\");return 0;}}",behavior,serde_json::to_string(&version).unwrap(),serde_json::to_string(&context).unwrap());
        let c = t.join("inert.c");
        fs::write(&c, source).unwrap();
        let output = Command::new("cc")
            .arg(&c)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }
    fn matching_sidecar(bin: &Path, architecture: Value) -> Value {
        json!({"schema":"lintel.cli-candidate/1","protocol":1,"version":"0.1.0","platform":std::env::consts::OS,"architecture":architecture,"candidate_identity":"synthetic-candidate","signed":false,"files":[{"path":"bin/lintel","sha256":format!("{:x}", Sha256::digest(fs::read(bin).unwrap()))}]})
    }
    #[test]
    fn finite_architecture_aliases_are_symmetric() {
        for left in ["arm64", "aarch64"] {
            for right in ["arm64", "aarch64"] {
                assert_eq!(
                    canonical_architecture(&json!(left)),
                    canonical_architecture(&json!(right))
                );
            }
            assert_ne!(
                canonical_architecture(&json!(left)),
                canonical_architecture(&json!("x86_64"))
            );
        }
        assert_eq!(canonical_architecture(&json!("x86_64")), Some("x86_64"));
        for invalid in [
            Value::Null,
            json!(""),
            json!("ARM64"),
            json!("amd64"),
            json!("riscv64"),
            json!(64),
            json!(true),
            json!(["aarch64"]),
        ] {
            assert_eq!(canonical_architecture(&invalid), None, "{invalid}");
        }
    }
    #[cfg(target_arch = "aarch64")]
    #[test]
    fn native_arm_aliases_match_on_both_sides_of_candidate_comparison() {
        for version_arch in ["arm64", "aarch64"] {
            let t = tempfile::tempdir().unwrap();
            let bin = native_architecture(t.path(), 1, "", Some(json!(version_arch)));
            for candidate_arch in ["arm64", "aarch64"] {
                let sidecar = matching_sidecar(&bin, json!(candidate_arch));
                fs::write(t.path().join("candidate.json"), sidecar.to_string()).unwrap();
                let result = inspect(&bin).unwrap();
                assert_eq!(result["candidate"]["status"], "bytes_match");
                assert_eq!(result["candidate"]["source_authenticated"], false);
            }
        }
    }
    #[test]
    fn candidate_architecture_and_checksum_remain_required() {
        let t = tempfile::tempdir().unwrap();
        let bin = native(t.path(), 1, "");
        let original = matching_sidecar(&bin, json!(std::env::consts::ARCH));
        let sidecar = t.path().join("candidate.json");
        fs::write(&sidecar, original.to_string()).unwrap();
        assert_eq!(inspect(&bin).unwrap()["candidate"]["status"], "bytes_match");
        let mismatching = if cfg!(target_arch = "aarch64") {
            "x86_64"
        } else {
            "aarch64"
        };
        for architecture in [
            Value::Null,
            json!(""),
            json!("mips64"),
            json!(false),
            json!(64),
            json!(["aarch64"]),
            json!(mismatching),
        ] {
            let mut changed = original.clone();
            changed["architecture"] = architecture;
            fs::write(&sidecar, changed.to_string()).unwrap();
            assert_eq!(inspect(&bin).unwrap_err(), "candidate_mismatch");
        }
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove("architecture");
        fs::write(&sidecar, missing.to_string()).unwrap();
        assert_eq!(inspect(&bin).unwrap_err(), "candidate_mismatch");
        for (field, value) in [
            ("schema", json!("lintel.cli-candidate/2")),
            ("version", json!("0.2.0")),
            ("protocol", json!(2)),
            ("platform", json!("wrong-platform")),
        ] {
            let mut changed = original.clone();
            changed[field] = value;
            fs::write(&sidecar, changed.to_string()).unwrap();
            assert_eq!(inspect(&bin).unwrap_err(), "candidate_mismatch");
        }
        let mut corrupt = original;
        corrupt["files"][0]["sha256"] = json!("0".repeat(64));
        fs::write(&sidecar, corrupt.to_string()).unwrap();
        assert_eq!(inspect(&bin).unwrap_err(), "candidate_mismatch");
        assert!(!t.path().join("not-created-state").exists());
    }
    #[test]
    fn missing_or_malformed_static_architecture_is_not_a_matching_candidate() {
        for architecture in [
            None,
            Some(Value::Null),
            Some(json!("")),
            Some(json!("mips64")),
            Some(json!(64)),
            Some(json!(["aarch64"])),
        ] {
            let t = tempfile::tempdir().unwrap();
            let bin = native_architecture(t.path(), 1, "", architecture.clone());
            let sidecar = matching_sidecar(&bin, architecture.unwrap_or(Value::Null));
            fs::write(t.path().join("candidate.json"), sidecar.to_string()).unwrap();
            assert_eq!(inspect(&bin).unwrap_err(), "cli_wrong_architecture");
        }
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn canonical_packaged_native_candidate_is_inspectable() {
        let t = tempfile::tempdir().unwrap();
        let bin = native(t.path(), 1, "");
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        // Reuse the existing explicitly synthetic static ELF fixture mechanism.
        // The selected macOS executable, packager, archive metadata and inspector
        // are real; no remote runner is executed and no personal state is used.
        let script = r#"
import importlib.util, json, pathlib, subprocess, sys
root, native, base = map(pathlib.Path, sys.argv[1:])
spec = importlib.util.spec_from_file_location('package_fixture', root / 'tests/cli_package_journey.py')
fixture = importlib.util.module_from_spec(spec)
spec.loader.exec_module(fixture)
runners = base / 'synthetic-runners'
entries = []
inputs = []
for triple, machine, flag in [('x86_64-unknown-linux-musl', 62, '--linux-x86_64'), ('aarch64-unknown-linux-musl', 183, '--linux-aarch64')]:
    payload = fixture.elf(machine)
    target = runners / triple
    target.mkdir(parents=True)
    binary = target / 'lintel'
    binary.write_bytes(payload)
    entries.append({'target':triple, 'version':'0.1.0', 'protocol':1, 'bytes':len(payload), 'sha256':fixture.sha256_bytes(payload)})
    inputs += [flag, str(binary)]
(runners / 'manifest.json').write_text(json.dumps({'runners':entries}))
out = base / 'candidate-output'
result = subprocess.run(['node', str(root / 'scripts/package-cli.mjs'), '--macos-arm64', str(native), *inputs, '--linux-runners', str(runners), '--revision', '0' * 40, '--out', str(out)], capture_output=True, text=True, timeout=30)
if result.returncode:
    sys.stderr.write(result.stdout + result.stderr)
    sys.exit(result.returncode)
summary = json.loads(result.stdout)
archive = next(entry for entry in summary['archives'] if entry['target'] == 'macos-arm64')
dest = base / 'selected-candidate'
dest.mkdir()
subprocess.run(['tar', '-xzf', str(out / archive['archive']), '-C', str(dest)], check=True, capture_output=True)
print(dest / 'bin/lintel')
"#;
        let output = Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .args(["-c", script])
            .arg(&root)
            .arg(&bin)
            .arg(t.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let selected = std::path::PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
        let metadata: Value = serde_json::from_slice(
            &fs::read(
                selected
                    .parent()
                    .unwrap()
                    .parent()
                    .unwrap()
                    .join("candidate.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(metadata["architecture"], "aarch64");
        let result = inspect(&selected).unwrap();
        assert_eq!(result["candidate"]["status"], "bytes_match");
        assert_eq!(result["candidate"]["signed"], false);
        assert_eq!(result["candidate"]["source_authenticated"], false);
        assert!(!t.path().join("not-created-state").exists());
    }
    #[test]
    fn native_static_unknown_identity_and_candidate_mismatch() {
        let t = tempfile::tempdir().unwrap();
        let bin = native(t.path(), 1, "");
        let result = inspect(&bin).unwrap();
        assert_eq!(result["candidate"]["status"], "unknown");
        assert!(!t.path().join("not-created-state").exists());
        let sidecar = json!({"schema":"lintel.cli-candidate/1","protocol":1,"version":"0.1.0","platform":std::env::consts::OS,"architecture":if cfg!(target_arch="aarch64"){"arm64"}else{"x86_64"},"candidate_identity":"synthetic-candidate","files":[{"path":"bin/lintel","sha256":"0".repeat(64)}]});
        fs::write(t.path().join("candidate.json"), sidecar.to_string()).unwrap();
        assert_eq!(inspect(&bin).unwrap_err(), "candidate_mismatch");
        let mut correct = sidecar;
        correct["files"][0]["sha256"] =
            json!(format!("{:x}", Sha256::digest(fs::read(&bin).unwrap())));
        fs::write(t.path().join("candidate.json"), correct.to_string()).unwrap();
        assert_eq!(
            inspect(&bin).unwrap()["candidate"]["source_authenticated"],
            false
        );
        assert!(!t.path().join("not-created-state").exists());
    }
    #[test]
    fn native_wrong_protocol_and_bounded_process() {
        let t = tempfile::tempdir().unwrap();
        let bin = native(t.path(), 2, "");
        assert_eq!(inspect(&bin).unwrap_err(), "cli_protocol_mismatch");
        let overflow = tempfile::tempdir().unwrap();
        let bin = native(
            overflow.path(),
            1,
            "for(int i=0;i<1100000;i++) putchar('x');",
        );
        assert_eq!(inspect(&bin).unwrap_err(), "cli_output_limit");
        let slow = tempfile::tempdir().unwrap();
        let bin = native(slow.path(), 1, "sleep(20);");
        let start = Instant::now();
        assert_eq!(inspect(&bin).unwrap_err(), "cli_timeout");
        assert!(start.elapsed() < Duration::from_secs(10));
    }
}
