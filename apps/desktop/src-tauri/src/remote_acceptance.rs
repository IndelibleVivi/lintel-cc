//! Real Linux OpenSSH acceptance. Selected only by the synthetic Python launcher.
use super::*;
use sha2::{Digest, Sha256};
use std::os::unix::fs::PermissionsExt;

const ALIAS: &str = "lintel-acceptance";

fn controller(base: &Path) -> Controller {
    Controller {
        state: base.join("controller-state/remote"),
        config: base.join("ssh_config"),
        bundles: base.join("bundles"),
        terminal: Some(base.join("terminal-opener")),
        transport: Transport {
            // This shim adds only the isolated OpenSSH config and execs ssh.
            // Failure injection belongs to sshd's synthetic ForceCommand.
            ssh: base.join("ssh"),
            interpreter: None,
            deadline: Duration::from_secs(20),
        },
    }
}

fn data(c: &Controller, payload: Value) -> Value {
    let response = c.dispatch(payload).expect("native controller operation");
    assert_eq!(response["ok"], true, "{response}");
    response["data"].clone()
}

fn request(c: &Controller, request: Value) -> Value {
    data(c, json!({"op":"request","alias":ALIAS,"request":request}))
}

fn commands(base: &Path) -> String {
    fs::read_to_string(base.join("commands")).unwrap()
}

fn submissions(base: &Path) -> usize {
    commands(base)
        .lines()
        .filter(|line| line.ends_with(" submit"))
        .count()
}

fn uploads(base: &Path) -> usize {
    commands(base).matches("\"uploaded\":true").count()
}

fn completed(c: &Controller, plan: &Value) -> Value {
    let started = Instant::now();
    loop {
        let receipt = data(
            c,
            json!({"op":"reconnect","alias":ALIAS,"plan_id":plan["id"]}),
        );
        assert_eq!(receipt["id"], plan["id"], "{receipt}");
        assert_eq!(receipt["plan_id"], plan["id"], "{receipt}");
        if !["accepted", "executing", "verifying"].contains(&receipt["status"].as_str().unwrap()) {
            assert_eq!(receipt["status"], "completed", "{receipt}");
            return receipt;
        }
        assert!(started.elapsed() < Duration::from_secs(20), "{receipt}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

#[test]
#[ignore = "requires real Linux sshd and the synthetic tests/remote_linux_ssh_journey.py launcher"]
fn linux_openssh_runtime_journey() {
    assert!(cfg!(target_os = "linux") && cfg!(target_arch = "x86_64"));
    assert_eq!(std::env::var("LINTEL_SSH_ACCEPTANCE").as_deref(), Ok("1"));
    let base = PathBuf::from(std::env::var_os("LINTEL_ACCEPTANCE_FIXTURE").unwrap());
    let runner = PathBuf::from(std::env::var_os("LINTEL_ACCEPTANCE_RUNNER").unwrap());
    let home = base.join("home");
    let root = home.join("synthetic-claude");
    let settings = root.join("settings.json");
    let original = json!({"env":{"UNRELATED_SYNTHETIC_FLAG":"keep","DISABLE_TELEMETRY":"false","DISABLE_FEEDBACK_COMMAND":"false","DISABLE_GROWTHBOOK":"true"},"permissions":{"allow":["Read"]}});
    fs::write(&settings, serde_json::to_vec(&original).unwrap()).unwrap();

    let c = controller(&base);
    let target = "x86_64-unknown-linux-musl";
    let bytes = fs::read(&runner).expect("real musl runner built by the Linux acceptance job");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    fs::create_dir_all(c.bundles.join(target)).unwrap();
    fs::write(c.bundles.join(target).join("lintel"), &bytes).unwrap();
    fs::write(
        c.bundles.join("manifest.json"),
        serde_json::to_vec(&json!({"runners":[{"target":target,"version":env!("CARGO_PKG_VERSION"),"protocol":1,"bytes":bytes.len(),"sha256":digest}]})).unwrap(),
    ).unwrap();

    data(&c, json!({"op":"add_host","alias":ALIAS}));
    let preview = data(&c, json!({"op":"prepare_runner","alias":ALIAS}));
    assert_eq!(preview["status"], "previewed");
    assert_eq!(preview["probe"]["os"], "Linux");
    assert_eq!(preview["probe"]["architecture"], "x86_64");
    assert_eq!(preview["probe"]["path_runner_present"], false);
    assert_eq!(preview["bundle"]["sha256"], digest);
    let installed = home
        .join(".local/share/lintel/runners")
        .join(&digest)
        .join("lintel");
    assert!(!installed.exists(), "preview must not upload");
    let invalid = c.dispatch(json!({"op":"install_runner","alias":ALIAS,"install_id":preview["install_id"],"approval":"incorrect approval"})).unwrap_err();
    assert_eq!(invalid.code, "approval_required");
    assert!(!installed.exists(), "wrong approval must not upload");
    assert_eq!(uploads(&base), 0);

    // The real upload succeeds, but sshd deliberately discards its response.
    fs::write(base.join("lose-install-ack"), b"synthetic ACK loss").unwrap();
    let approved = json!({"op":"install_runner","alias":ALIAS,"install_id":preview["install_id"],"approval":preview["approval"]});
    let lost = c.dispatch(approved.clone()).unwrap_err();
    assert_eq!(lost.code, "transport_unknown", "{lost:?}");
    let remote_ack: Value =
        serde_json::from_slice(&fs::read(base.join("lost-install-ack.json")).unwrap()).unwrap();
    assert_eq!(remote_ack["data"]["uploaded"], true);
    let inventory = data(&c, json!({"op":"hosts"}));
    assert_eq!(
        inventory["installations"][0]["status"],
        "needs_reconciliation"
    );
    assert_eq!(uploads(&base), 1);
    let reopened = controller(&base);
    let verified = data(
        &reopened,
        json!({"op":"query_install","alias":ALIAS,"install_id":preview["install_id"]}),
    );
    assert_eq!(verified["status"], "ready", "{verified}");
    assert_eq!(verified["activated"], true);
    assert_eq!(
        fs::read(&installed).unwrap(),
        bytes,
        "uploaded runner identity"
    );
    assert_eq!(
        fs::metadata(&installed).unwrap().permissions().mode() & 0o777,
        0o700
    );
    assert_eq!(data(&reopened, approved)["status"], "ready");
    assert_eq!(
        uploads(&base),
        1,
        "installation recovery must not upload again"
    );

    let connected = data(&reopened, json!({"op":"connect","alias":ALIAS}));
    assert!(connected["capabilities"]
        .as_array()
        .unwrap()
        .iter()
        .any(|cap| cap["name"] == "detached_submission" && cap["status"] == "available"));
    let environment = request(
        &reopened,
        json!({"command":"register","name":"Synthetic OpenSSH Claude","root":root}),
    );
    let eid = &environment["id"];
    assert_eq!(environment["product_version"], "2.1.283");
    assert!(
        !base.join("launch.json").exists(),
        "version identification must not execute Claude"
    );
    let policy = request(
        &reopened,
        json!({"command":"plan_policy","environment_id":eid,"preset":"custom","keep_remote_control":true,"trusted_devices":"not_required","custom_settings":{"DISABLE_TELEMETRY":"keep","DISABLE_ERROR_REPORTING":"disable","DISABLE_FEEDBACK_COMMAND":"disable","DISABLE_GROWTHBOOK":"remove"}}),
    );
    assert_eq!(policy["policy"]["preset"], "custom");
    assert_eq!(
        policy["policy"]["remote_control"]["status"],
        "configuration_compatible"
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(&settings).unwrap()).unwrap(),
        original,
        "policy preview mutated synthetic target"
    );

    fs::write(base.join("lose-submit-ack"), b"synthetic durable ACK loss").unwrap();
    let execute =
        json!({"op":"execute","alias":ALIAS,"plan_id":policy["id"],"approval":policy["hash"]});
    let lost = reopened.dispatch(execute.clone()).unwrap_err();
    assert_eq!(lost.code, "transport_unknown", "{lost:?}");
    let remote_ack: Value =
        serde_json::from_slice(&fs::read(base.join("lost-submit-ack.json")).unwrap()).unwrap();
    assert_eq!(remote_ack["ok"], true, "{remote_ack}");
    assert_eq!(
        remote_ack["data"]["status"], "accepted",
        "ACK must follow durable journal acceptance"
    );
    assert_eq!(remote_ack["data"]["plan_id"], policy["id"]);
    assert!(base
        .join("runner-state/jobs")
        .join(format!("{}.json", policy["id"].as_str().unwrap()))
        .exists());
    assert_eq!(submissions(&base), 1);
    let after_disconnect = controller(&base);
    let receipt = completed(&after_disconnect, &policy);
    assert_eq!(data(&after_disconnect, execute)["id"], receipt["id"]);
    assert_eq!(
        submissions(&base),
        1,
        "reconnect and repeated execute must only query original job"
    );
    let jobs = request(&after_disconnect, json!({"command":"jobs"}));
    assert_eq!(
        jobs["jobs"].as_array().unwrap().len(),
        1,
        "ACK loss must not create another job"
    );
    let written: Value = serde_json::from_slice(&fs::read(&settings).unwrap()).unwrap();
    for key in ["DISABLE_ERROR_REPORTING", "DISABLE_FEEDBACK_COMMAND"] {
        assert_eq!(written["env"][key], "1", "{key}");
    }
    assert_eq!(
        written["env"]["DISABLE_TELEMETRY"], "false",
        "custom keep preserves nonempty external value"
    );
    assert!(written["env"].get("DISABLE_GROWTHBOOK").is_none());
    assert!(written["env"]
        .get("CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY")
        .is_none());
    assert_eq!(
        receipt["policy"], policy["policy"],
        "custom choices freeze through lost ACK"
    );
    assert_eq!(written["env"]["UNRELATED_SYNTHETIC_FLAG"], "keep");
    assert_eq!(written["permissions"], original["permissions"]);
    let local_record = fs::read_to_string(
        after_disconnect
            .state
            .join("tasks")
            .join(ALIAS)
            .join(format!("{}.json", policy["id"].as_str().unwrap())),
    )
    .unwrap();
    let record: Value = serde_json::from_str(&local_record).unwrap();
    assert_eq!(record["runner_digest"], digest);
    assert!(
        !local_record.contains(policy["hash"].as_str().unwrap()),
        "approval must not persist in controller task record"
    );
    assert!(
        commands(&base).contains(&format!("runners/{digest}/lintel\" submit")),
        "submission must use installed pinned runner"
    );

    let launch = data(
        &after_disconnect,
        json!({"op":"launch","alias":ALIAS,"environment_id":eid}),
    );
    assert_eq!(launch["status"], "launch_requested", "{launch}");
    let launched: Value = serde_json::from_slice(
        &fs::read(base.join("launch.json")).expect("fake Claude launch evidence"),
    )
    .unwrap();
    assert_eq!(launched["root"], root.to_string_lossy().as_ref());
    assert_eq!(launched["config_root"], root.to_string_lossy().as_ref());
    assert_eq!(launched["home"], home.to_string_lossy().as_ref());
    assert_eq!(
        launched["stdin_tty"], true,
        "remote CLI launch requires a real PTY"
    );
    assert_eq!(launched["stdout_tty"], true);
    assert_eq!(
        launched["argc"], 0,
        "launch must not feed a prompt or model request"
    );
    assert_eq!(submissions(&base), 1);
    assert_eq!(uploads(&base), 1);
    println!("PASS: native preview/exact approval/upload/lost install ACK/verify/connect, durable musl submit/lost ACK/query-only recovery, interactive PTY launch on the selected synthetic root");
}
