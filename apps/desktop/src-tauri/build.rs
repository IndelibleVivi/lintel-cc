fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8(o.stdout).ok())
    };
    let revision = git(&["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap_or_default();
    let clean = git(&["status", "--porcelain", "--untracked-files=normal"])
        .is_some_and(|s| s.trim().is_empty());
    let revision =
        if clean && revision.len() == 40 && revision.chars().all(|c| c.is_ascii_hexdigit()) {
            revision
        } else {
            "unknown".into()
        };
    println!("cargo:rustc-env=LINTEL_DOCS_REVISION={revision}");
    println!(
        "cargo:rerun-if-changed={}",
        root.join(".git/HEAD").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join(".git/index").display()
    );
    if let Some(files) = git(&["ls-files"]) {
        for file in files.lines() {
            println!("cargo:rerun-if-changed={}", root.join(file).display())
        }
    }
    println!(
        "cargo:rerun-if-changed={}",
        root.join(".git/refs/heads/main").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        root.join("contracts/documentation-resources.json")
            .display()
    );
    tauri_build::build()
}
