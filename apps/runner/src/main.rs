mod cli;
mod submission;
mod supervisor;
use serde_json::json;
use std::io::{self, IsTerminal, Write};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        print!("{}", cli::HELP);
        return;
    }
    if ["submit", "__worker"].contains(&args[0].as_str()) {
        if !submission::run(&args[0], &args[1..]) {
            std::process::exit(1);
        }
        return;
    }
    if args[0] == "tui" {
        tui();
        return;
    }
    if args[0] == "launch" {
        if args.len() != 2 {
            eprintln!("Usage: lintel launch <environment-id>");
            std::process::exit(1);
        }
        let mut command = match launch_command(&args[1]) {
            Ok(command) => command,
            Err(message) => {
                eprintln!("{message}");
                std::process::exit(1);
            }
        };
        use std::os::unix::process::CommandExt;
        eprintln!("Launch failed: {}", command.exec());
        std::process::exit(1);
    }
    let response = cli::run(&args);
    println!("{}", response);
    if response["ok"] != true {
        std::process::exit(1)
    }
}
fn launch_command(environment_id: &str) -> Result<std::process::Command, String> {
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        return Err(
            "terminal_required: 请在交互终端使用 lintel launch；不会在隐藏管道中启动 Claude。"
                .into(),
        );
    }
    let request = json!({"command":"launch_context","environment_id":environment_id});
    lintel_operations::validate(&request)
        .map_err(|e| cli::error("invalid_request", e).to_string())?;
    let response = lintel_core::handle_request(request);
    if response["ok"] != true {
        return Err(response.to_string());
    }
    let data = &response["data"];
    let mut command = std::process::Command::new(data["executable"].as_str().unwrap());
    command
        .current_dir(data["root"].as_str().unwrap())
        .env("CLAUDE_CONFIG_DIR", data["root"].as_str().unwrap());
    Ok(command)
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
        println!("\n1 环境清单   2 登记环境   3 新建环境   4 应用方案\n5 恢复配置   6 任务记录   7 检查漂移   8 清理与重建\n9 工作归档   i 检查环境   a 认证检查   r 重新启用   o 打开 Claude   s 精确服务暂停/恢复   q 退出");
        let choice = read_line("> ");
        let request = match choice.as_str() {
            "q" => return,
            "1" => json!({"command":"discover"}),
            "2" => {
                json!({"command":"register","name":read_line("环境名: "),"root":read_line("绝对配置目录: ")})
            }
            "3" => json!({"command":"create_environment","name":read_line("环境名: ")}),
            "4" => {
                let id = read_line("环境 ID: ");
                let preset = loop {
                    let selection =
                        read_line("方案 reduce 减少外发 / preserve 保持功能 / custom 自定义: ");
                    if ["reduce", "preserve", "custom"].contains(&selection.as_str()) {
                        break selection;
                    }
                    println!("请输入一个方案名称；不自动替你选择。");
                };
                let mut request = json!({"command":"plan_policy","environment_id":id,"preset":preset,"keep_remote_control":read_line("保留 Remote Control? [Y/n]: ")!="n"});
                request["trusted_devices"] = json!(loop {
                    let condition = read_line(
                        "组织 Trusted Devices 条件 unknown / required / not_required [unknown]: ",
                    );
                    if condition.is_empty() {
                        break "unknown".to_string();
                    }
                    if ["unknown", "required", "not_required"].contains(&condition.as_str()) {
                        break condition;
                    }
                    println!("请输入一个有效条件，仅记录你的声明。");
                });
                if preset == "custom" {
                    let inspected = lintel_core::handle_request(
                        json!({"command":"inspect","environment_id":id}),
                    );
                    if inspected["ok"] != true {
                        show(&inspected);
                        continue;
                    }
                    println!("只选择当前配置根的字段：keep 保持原值，disable 关闭，remove 移除覆盖（可能重新开放流量）。需新启动；稍后显示准确 diff 并再次批准。");
                    let mut choices = serde_json::Map::new();
                    for rule in inspected["data"]["policy"]["rules"].as_array().unwrap() {
                        println!(
                            "{} · {} · 当前 {}",
                            rule["label"].as_str().unwrap(),
                            rule["key"].as_str().unwrap(),
                            rule["value"]
                        );
                        let action = loop {
                            let action = read_line("keep / disable / remove [keep]: ");
                            if action.is_empty() {
                                break "keep".to_string();
                            }
                            if ["keep", "disable", "remove"].contains(&action.as_str()) {
                                break action;
                            }
                            println!("请输入 keep / disable / remove。");
                        };
                        choices.insert(rule["key"].as_str().unwrap().to_string(), json!(action));
                    }
                    request["custom_settings"] = choices.into();
                }
                request
            }
            "s" => {
                let operation = read_line("服务操作 inspect / quiesce / resume: ");
                if operation == "resume" {
                    json!({"command":"plan_service_resume","job_id":read_line("原暂停任务 ID: ")})
                } else if ["inspect", "quiesce"].contains(&operation.as_str()) {
                    json!({"command":if operation=="inspect" {"service_inspect"}else{"plan_service_quiesce"},"environment_id":read_line("环境 ID: "),"manager":read_line("systemd manager user / system（不会 sudo）: "),"unit":read_line("精确完整 unit 名称（例如 example.service）: ")})
                } else {
                    continue;
                }
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
            "o" => {
                match launch_command(&read_line("环境 ID: ")) {
                    Ok(mut command) => {
                        if let Err(error) = command.status() {
                            eprintln!("启动失败: {error}");
                        }
                    }
                    Err(message) => eprintln!("{message}"),
                }
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
        if ["4", "5", "8", "s"].contains(&choice.as_str()) && response["data"]["hash"].is_string() {
            approve_plan(response, None);
        } else {
            show(&response);
        }
    }
}
