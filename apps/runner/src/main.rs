use serde_json::json;
use std::io::{self, Read, Write};
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "help" {
        println!("Lintel 0.1.0 — Claude environment control\n\n  lintel request                One JSON request from stdin, one JSON response\n  lintel discover               List local registered environments\n  lintel inspect <id>           Inspect one environment\n  lintel jobs                   List durable receipts\n  lintel job <plan-id>           Query after a lost response\n  lintel launch <id>            Open Claude with this root (interactive terminal)\n  lintel tui                    Interactive terminal interface\n\nMutations use JSON plan + exact approval digest. Test roots: LINTEL_TEST_HOME and LINTEL_STATE_DIR. No blanket --yes.\nExample: printf '%s' '{{\"command\":\"discover\"}}' | lintel request");
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
            lintel_core::parse_request(&bytes)
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
    let _ = io::stdin().read_line(&mut line);
    line.trim().to_string()
}
fn tui() {
    loop {
        println!("\nLintel · 终端环境控制\n1 查看环境  2 添加环境  3 新建环境  4 应用方案  5 恢复配置  6 查看任务  7 检查漂移  q 退出");
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
            "6" => json!({"command":"jobs"}),
            "7" => json!({"command":"drift","environment_id":read_line("环境 ID: ")}),
            _ => continue,
        };
        let response = lintel_core::handle_request(request);
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
        if response["ok"] == true
            && ["4", "5"].contains(&choice.as_str())
            && read_line("按以上具体范围执行? 输入 apply: ") == "apply"
        {
            let p = &response["data"];
            let result = lintel_core::handle_request(
                json!({"command":"execute","plan_id":p["id"],"approval":p["hash"]}),
            );
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
    }
}
