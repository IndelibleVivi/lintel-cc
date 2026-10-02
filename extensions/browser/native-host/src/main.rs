use lintel_browser_host::{control, manifest, native, random, read_frame, release, write_frame};
use serde_json::Value;
use std::io::{self, Read};
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("register") {
        if args.len() < 4 {
            return Err("usage: lintel-browser-host register chrome|edge|firefox EXTENSION_ID /absolute/host [--home /absolute/home] [--apply]".into());
        }
        let mut home =
            std::path::PathBuf::from(std::env::var_os("HOME").ok_or("HOME is required")?);
        let mut apply = false;
        let mut index = 4;
        while index < args.len() {
            match args[index].as_str() {
                "--apply" => {
                    apply = true;
                    index += 1;
                }
                "--home" if index + 1 < args.len() => {
                    home = std::path::PathBuf::from(&args[index + 1]);
                    index += 2;
                }
                _ => return Err("unknown_registration_argument".into()),
            }
        }
        let platform = std::env::consts::OS;
        let result = if apply {
            lintel_browser_host::installation::install(
                &args[1],
                &args[2],
                std::path::Path::new(&args[3]),
                &home,
                platform,
            )
        } else {
            lintel_browser_host::installation::plan(
                &args[1],
                &args[2],
                std::path::Path::new(&args[3]),
                &home,
                platform,
            )
        }?;
        println!("{}", serde_json::to_string_pretty(&result).unwrap());
        return Ok(());
    }
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
            return Err("usage: lintel-browser-host manifest chrome|edge|chromium|firefox EXTENSION_ID /absolute/path/to/host".into());
        }
        let m = manifest(&args[1], &args[2], std::path::Path::new(&args[3]))?;
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
