mod submission;
use serde_json::json;
use std::io::{self, Read, Write};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        println!("Lintel 0.1.0 — Claude environment control\n\n  lintel request                One JSON request from stdin, one JSON response\n  lintel submit                 Durable execute acknowledgement; detached worker\n  lintel discover               List local registered environments\n  lintel inspect <id>           Inspect one environment\n  lintel jobs                   List durable receipts\n  lintel job <plan-id>           Query after a lost response\n  lintel launch <id>            Open Claude with this root (interactive terminal)\n  lintel tui                    Interactive terminal interface\n\nMutations use JSON plan + exact approval digest. Test roots: LINTEL_TEST_HOME and LINTEL_STATE_DIR. No blanket --yes.\nExample: printf '%s' '{{\"command\":\"discover\"}}' | lintel request");
        return;
    }
    if ["submit", "__worker"].contains(&args[0].as_str()) {
        submission::run(&args[0]);
        return;
    }
    if args[0] == "tui" {
        tui();
        return;
    }
    if args[0] == "launch" {
        let response = lintel_core::handle_request(
            json!({"command":"launch_context","environment_id":args.get(1)}),
        );
        if response["ok"] != true {
            eprintln!("{}", response);
            std::process::exit(1)
        }
        let data = &response["data"];
        let mut command = std::process::Command::new(data["executable"].as_str().unwrap());
        command
            .current_dir(data["root"].as_str().unwrap())
            .env("CLAUDE_CONFIG_DIR", data["root"].as_str().unwrap());
        use std::os::unix::process::CommandExt;
        eprintln!("Launch failed: {}", command.exec());
        std::process::exit(1);
    }
    let response = if args[0] == "request" {
        let mut bytes = vec![];
        if io::stdin()
            .take(1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .is_err()
            || bytes.len() > 1024 * 1024
        {
            json!({"ok":false,"error":{"code":"request_limit","message":"Request exceeds 1 MiB or stdin failed"}})
        } else {
            let mut response = lintel_core::parse_request(&bytes);
            if serde_json::from_slice::<serde_json::Value>(&bytes)
                .is_ok_and(|r| r["command"] == "discover")
            {
                submission::capability(&mut response);
            }
            response
        }
    } else {
        let request = match args[0].as_str() {
            "discover" => json!({"command":"discover"}),
            "jobs" => json!({"command":"jobs"}),
            "inspect" => json!({"command":"inspect","environment_id":args.get(1)}),
            "job" => json!({"command":"job","plan_id":args.get(1)}),
            _ => json!({"command":"unknown"}),
        };
        lintel_core::handle_request(request)
    };
    println!("{}", response);
    if response["ok"] != true {
        std::process::exit(1)
    }
}
fn read_line(prompt: &str) -> String {
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut line = String::new();
    match io::stdin().read_line(&mut line) {
        Ok(0) | Err(_) => std::process::exit(0),
        Ok(_) => line.trim().to_string(),
    }
}
fn secret(prompt: &str) -> Option<String> {
    // The TUI never accepts an archive password with terminal echo enabled.
    unsafe {
        let mut settings = std::mem::MaybeUninit::<libc::termios>::uninit();
        if libc::tcgetattr(libc::STDIN_FILENO, settings.as_mut_ptr()) != 0 {
            eprintln!("口令输入需要交互终端；自动化请使用 JSON stdin。");
            return None;
        }
        let original = settings.assume_init();
        let mut hidden = original;
        hidden.c_lflag &= !libc::ECHO;
        if libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &hidden) != 0 {
            return None;
        }
        print!("{prompt}");
        let _ = io::stdout().flush();
        let mut input = String::new();
        let result = io::stdin().read_line(&mut input);
        libc::tcsetattr(libc::STDIN_FILENO, libc::TCSANOW, &original);
        println!();
        if result.is_err() || matches!(result, Ok(0)) {
            return None;
        }
        Some(input.trim_end_matches(['\r', '\n']).to_owned())
    }
}
fn show(response: &serde_json::Value) {
    println!("{}", serde_json::to_string_pretty(response).unwrap());
}
fn approve_plan(response: serde_json::Value, password: Option<String>) {
    show(&response);
    if response["ok"] != true || read_line("按以上具体范围执行? 输入 apply: ") != "apply"
    {
        return;
    }
    let plan = &response["data"];
    let mut request = json!({"command":"execute","plan_id":plan["id"],"approval":plan["hash"]});
    if plan["archive_passphrase_required"] == true {
        let password = if let Some(password) = password {
            password
        } else {
            let Some(password) = secret("归档口令（至少 12 字符，不显示）: ") else {
                return;
            };
            let Some(confirmation) = secret("再次输入: ") else {
                return;
            };
            if password != confirmation {
                eprintln!("口令不一致，没有提交执行。");
                return;
            }
            password
        };
        request["archive_passphrase"] = json!(password);
    }
    show(&lintel_core::handle_request(request));
}
fn categories() -> Vec<String> {
    read_line("保留类别（逗号分隔 instructions,memory,sessions；空=无）: ")
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect()
}
fn archive_tui() {
    let job = read_line("归档原任务 ID: ");
    let Some(password) = secret("解锁口令（不显示）: ") else {
        return;
    };
    let result = lintel_core::handle_request(
        json!({"command":"archive_inspect","job_id":job,"archive_passphrase":password}),
    );
    show(&result);
    if result["ok"] != true {
        return;
    }
    match read_line("read 阅读文件 / import 迁入环境 / 回车返回: ").as_str() {
        "read" => show(&lintel_core::handle_request(
            json!({"command":"archive_read","job_id":job,"archive_passphrase":password,"path":read_line("清单中的完整相对路径: ")}),
        )),
        "import" => {
            let plan = lintel_core::handle_request(
                json!({"command":"plan_import","job_id":job,"archive_passphrase":password,"environment_id":read_line("目标环境 ID: "),"categories":categories()}),
            );
            approve_plan(plan, Some(password));
        }
        _ => (),
    }
}
fn tui() {
    println!("  ▐▛███▜▌  Lintel\n ▝▜█████▛▘ 环境整理，先预览再执行。\n   ▘▘ ▝▝");
    loop {
        println!("\n1 环境清单   2 登记环境   3 新建环境   4 应用方案\n5 恢复配置   6 任务记录   7 检查漂移   8 清理与重建\n9 工作归档   i 检查环境   a 认证检查   r 重新启用   q 退出");
        let choice = read_line("> ");
        let request = match choice.as_str() {
            "q" => return,
            "1" => json!({"command":"discover"}),
            "2" => {
                json!({"command":"register","name":read_line("环境名: "),"root":read_line("绝对配置目录: ")})
            }
            "3" => json!({"command":"create_environment","name":read_line("环境名: ")}),
            "4" => {
                json!({"command":"plan_policy","environment_id":read_line("环境 ID: "),"preset":if read_line("减少外发? [y/N]: ")=="y"{"reduce"}else{"preserve"},"keep_remote_control":read_line("保留 Remote Control? [Y/n]: ")!="n"})
            }
            "5" => json!({"command":"plan_restore","job_id":read_line("原任务 ID: ")}),
            "6" => {
                let id = read_line("原任务 ID（空=全部记录）: ");
                if id.is_empty() {
                    json!({"command":"jobs"})
                } else {
                    json!({"command":"job","job_id":id})
                }
            }
            "7" => json!({"command":"drift","environment_id":read_line("环境 ID: ")}),
            "8" => {
                let id = read_line("环境 ID: ");
                let inspection = lintel_core::handle_request(
                    json!({"command":"cleanup_inspect","environment_id":id}),
                );
                show(&inspection);
                if inspection["ok"] != true {
                    continue;
                }
                let recipe = read_line("配方 repair_login / reset_client / rebuild / retire: ");
                if recipe == "rebuild" {
                    json!({"command":"plan_reset","environment_id":id,"recipe":recipe,"categories":categories()})
                } else {
                    json!({"command":"plan_cleanup","environment_id":id,"recipe":recipe,"writers_confirmed_stopped":read_line("已停止 Claude、IDE 与服务写入者? 输入 stopped: ")=="stopped","official_logout":read_line("调用此配置根的官方 auth logout（可能联网）? [y/N]: ")=="y","categories":categories()})
                }
            }
            "9" => {
                archive_tui();
                continue;
            }
            "i" => json!({"command":"inspect","environment_id":read_line("环境 ID: ")}),
            "a" => {
                println!("将明确调用此配置根的 Claude auth status，不执行登录。");
                json!({"command":"auth_probe","environment_id":read_line("环境 ID: ")})
            }
            "r" => {
                json!({"command":"reactivate_environment","environment_id":read_line("环境 ID（不恢复已删凭据）: ")})
            }
            _ => continue,
        };
        let response = lintel_core::handle_request(request);
        if ["4", "5", "8"].contains(&choice.as_str()) {
            approve_plan(response, None);
        } else {
            show(&response);
        }
    }
}
