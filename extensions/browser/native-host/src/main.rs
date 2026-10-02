use lintel_browser_host::{control, native, random, read_frame, release, write_frame};
use serde_json::{json, Value};
use std::io::{self, Read};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("control") {
        let mut input = String::new();
        io::stdin()
            .take(65537)
            .read_to_string(&mut input)
            .map_err(|e| e.to_string())?;
        if input.len() > 65536 {
            return Err("request_too_large".into());
        }
        let result = control(serde_json::from_str(&input).map_err(|e| e.to_string())?);
        println!("{result}");
        return Ok(());
    }
    if args.first().map(String::as_str) == Some("manifest") {
        if args.len() != 4 {
            return Err("usage: lintel-browser-host manifest chromium|firefox EXTENSION_ID /absolute/path/to/host".into());
        }
        if !std::path::Path::new(&args[3]).is_absolute() {
            return Err("host path must be absolute".into());
        }
        let mut m = json!({"name":"app.lintel.browser","description":"Lintel paired browser bridge","path":args[3],"type":"stdio"});
        match args[1].as_str() {
            "chromium" => {
                m["allowed_origins"] = json!([format!("chrome-extension://{}/", args[2])])
            }
            "firefox" => m["allowed_extensions"] = json!([args[2]]),
            _ => return Err("unknown browser".into()),
        }
        println!("{}", serde_json::to_string_pretty(&m).unwrap());
        return Ok(());
    }
    // Chromium passes an origin; Firefox passes manifest path then add-on ID.
    let extension = args
        .iter()
        .find_map(|a| {
            a.strip_prefix("chrome-extension://")
                .and_then(|s| s.strip_suffix('/'))
                .map(str::to_owned)
        })
        .or_else(|| {
            args.iter()
                .find(|a| a.as_str() == "lintel@lintel.local")
                .cloned()
        })
        .ok_or("browser caller identity missing")?;
    let connection = random();
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    let result = (|| {
        while let Some(request) = read_frame(&mut input)? {
            let response: Value = native(request, &extension, &connection);
            write_frame(&mut output, &response)?;
        }
        Ok(())
    })();
    release(&connection);
    result
}
