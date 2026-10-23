#!/usr/bin/env python3
"""Local candidate construction and consistency gate (never publication).

build --revision SHA --out ABSENT_DIR [--app] uses a fresh isolated source clone/target,
fixed locked builds and existing packagers. verify --receipt ... --ci-plan ...
--ci-results ... --ci-evidence ... --out ABSENT_JSON checks full CI and final
packages. Receipts/CI JSON remain local, unauthenticated observations. No keys,
release, upload, updater configuration, toolchain installation or activation.
"""
import argparse
from contextlib import contextmanager
import hashlib
import importlib.util
import json
import os
from pathlib import Path, PurePosixPath
import platform
import plistlib
import tomllib
import re
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile

ROOT = Path(__file__).resolve().parents[1]
SCHEMA = "lintel.local-candidate-build/1"
TARGETS = {"macos-arm64": "aarch64-apple-darwin", "linux-x86_64": "x86_64-unknown-linux-musl", "linux-aarch64": "aarch64-unknown-linux-musl"}
TERMS = ("LICENSE", "LICENSE-DOCUMENTATION", "LICENSING.md")
MAX_FILES, MAX_BYTES = 20000, 8 * 1024**3


def require(condition, message):
    if not condition:
        raise ValueError(message)


def identity(info):
    return (info.st_dev, info.st_ino, info.st_mode, info.st_size, info.st_mtime_ns, info.st_ctime_ns)


@contextmanager
def stable_regular(path):
    descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
    with os.fdopen(descriptor, "rb") as stream:
        before = os.fstat(stream.fileno())
        require(stat.S_ISREG(before.st_mode) and before.st_size <= MAX_BYTES, "bounded regular file required")
        require(identity(path.lstat()) == identity(before), "file replaced before read")
        yield stream
        after = os.fstat(stream.fileno())
        require(identity(before) == identity(after) == identity(path.lstat()), "file changed during read")


def stream_digest(stream):
    h, count = hashlib.sha256(), 0
    for block in iter(lambda: stream.read(1024 * 1024), b""):
        count += len(block)
        require(count <= MAX_BYTES, "file byte budget exceeded")
        h.update(block)
    return {"sha256": h.hexdigest(), "bytes": count}


def digest(path):
    with stable_regular(path) as stream:
        result = stream_digest(stream)
        require(result["bytes"] == os.fstat(stream.fileno()).st_size, "incomplete file read")
        return result


def unique_json(content):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result
    return json.loads(content, object_pairs_hook=unique)


def json_read(path):
    require(path.is_file() and not path.is_symlink() and path.stat().st_size <= 16 * 1024**2, "bounded regular JSON required")
    with stable_regular(path) as stream:
        content = stream.read(16 * 1024**2 + 1)
        require(len(content) <= 16 * 1024**2, "JSON byte budget exceeded")
        return unique_json(content)


def publish(path, value):
    # Publish only complete fsynced bytes, atomically without replacement.
    descriptor, temporary = tempfile.mkstemp(prefix=".lintel-candidate-", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w") as stream:
            json.dump(value, stream, ensure_ascii=False, indent=2)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        os.link(temporary, path, follow_symlinks=False)
        parent = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    finally:
        os.unlink(temporary)


def safe_name(name):
    require(isinstance(name, str) and name and "\\" not in name and "\x00" not in name, "invalid member name")
    p = PurePosixPath(name)
    require(not p.is_absolute() and all(x not in (".", "..", "") for x in name.split("/")), "unsafe member path")
    return name


def inventory(root):
    require(root.is_dir() and not root.is_symlink(), "regular payload directory required")
    result, total = {}, 0
    for path in sorted(root.rglob("*")):
        name = safe_name(path.relative_to(root).as_posix())
        if path.is_symlink():
            require(path.resolve().is_relative_to(root.resolve()), "App link escapes bundle")
            result[name] = {"link": os.readlink(path)}
        elif path.is_file():
            result[name] = digest(path)
            total += result[name]["bytes"]
        else:
            require(path.is_dir(), "unsupported payload object")
        require(len(result) <= MAX_FILES and total <= MAX_BYTES, "payload budget exceeded")
    return result


def tar_inventory(path, destination=None, expected_artifact=None, metadata=None):
    """Read actual tar member bytes; never trust archive checksum declarations."""
    with stable_regular(path) as raw:
        if expected_artifact is not None:
            require(stream_digest(raw) == expected_artifact, "archive changed before extraction")
            raw.seek(0)
        with tarfile.open(fileobj=raw, mode="r:gz") as archive:
            return read_tar_members(archive, destination, metadata)


def read_tar_members(archive, destination, metadata):
    result, seen, total = {}, set(), 0
    for member in archive:
        name = safe_name(member.name)
        require(name not in seen, "duplicate archive member")
        seen.add(name)
        require(member.isfile(), "archive links/directories/special objects are not payload members")
        require(0 <= member.size <= MAX_BYTES, "archive member budget exceeded")
        total += member.size
        require(len(seen) <= MAX_FILES and total <= MAX_BYTES, "archive budget exceeded")
        h, count = hashlib.sha256(), 0
        capture = metadata is not None and name in ("candidate.json", "bin/remote-runners/manifest.json")
        require(not capture or member.size < 1024**2, "bounded package manifest required")
        content = bytearray() if capture else None
        output = None
        if destination is not None:
            target = destination / name
            target.parent.mkdir(parents=True, exist_ok=True)
            output = target.open("xb")
        try:
            stream = archive.extractfile(member)
            require(stream is not None, "missing archive body")
            with stream:
                for block in iter(lambda: stream.read(1024 * 1024), b""):
                    h.update(block)
                    count += len(block)
                    if content is not None:
                        content.extend(block)
                    if output:
                        output.write(block)
            require(count == member.size, "truncated archive body")
        finally:
            if output:
                output.close()
        result[name] = {"sha256": h.hexdigest(), "bytes": count}
        if content is not None:
            metadata[name] = unique_json(content)
    return result


def run(args, cwd, env=None, capture=False):
    result = subprocess.run([str(a) for a in args], cwd=cwd, env=env, check=True,
                            stdout=subprocess.PIPE if capture else None, text=capture)
    return result.stdout.strip() if capture else None


def clean_revision(repo, revision):
    require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None, "exact SHA required")
    require(run(["git", "rev-parse", "HEAD"], repo, capture=True) == revision, "wrong source SHA")
    require(not run(["git", "status", "--porcelain", "--untracked-files=normal"], repo, capture=True), "clean source required")


def source_export(repo, revision, output):
    """Local isolated clone: real Git metadata, no ignored outputs or fetch."""
    source = output / "source"
    # --no-local prevents hardlink/object sharing; no-checkout ensures no hook
    # runs before the explicit hook-disabled detached checkout. Local path only.
    run(["git", "clone", "--no-local", "--no-hardlinks", "--no-checkout", "--", repo.resolve(), source], repo)
    run(["git", "-c", "core.hooksPath=/dev/null", "checkout", "--detach", revision], source)
    clean_revision(source, revision)
    return source


def tracked_inventory(source):
    names = run(["git", "ls-files", "-z"], source, capture=True).split("\0")
    result = {}
    for name in names:
        if name:
            safe_name(name)
            path = source / name
            require(path.is_file() and not path.is_symlink(), "tracked build inputs must be regular files")
            result[name] = digest(path)
    require(result, "empty build source")
    return result


def protected_cli(source, target, cli_runners):
    resources = source / "apps/desktop/src-tauri"
    result = {"bin/lintel": digest(source / "target" / TARGETS[target] / "release/lintel")}
    for directory, prefix in [(cli_runners, "bin/remote-runners"),
                              (resources / "third-party-notices", "third-party-notices")]:
        files = inventory(directory)
        require(files, "missing component inventory")
        result.update({f"{prefix}/{name}": value for name, value in files.items()})
    result.update({f"project-licenses/{name}": digest(source / name) for name in TERMS})
    return result


def dmg_inventory(path, source, app_version, expected_artifact):
    require(sys.platform == "darwin", "DMG subgate requires actual macOS; unverified elsewhere")
    directory = Path(tempfile.mkdtemp(prefix="lintel-candidate-mount-"))
    mount = directory / "volume"
    mount.mkdir()
    attached = False
    try:
        # hdiutil opens a pathname itself. First freeze the approved bytes into
        # our private directory through one stable no-follow source handle.
        image = directory / "final.dmg"
        with stable_regular(path) as src, image.open("xb") as dst:
            shutil.copyfileobj(src, dst, 1024 * 1024)
        require(digest(image) == expected_artifact, "DMG changed before mounting")
        image.chmod(0o400)
        attached = True
        subprocess.run(["hdiutil", "attach", "-readonly", "-nobrowse", "-noautoopen", "-mountpoint", str(mount), str(image)], check=True, stdout=subprocess.DEVNULL)
        apps = list(mount.glob("*.app"))
        require(len(apps) == 1 and apps[0].name == "Lintel.app", "ambiguous DMG App")
        run(["node", source / "apps/desktop/scripts/verify-macos-bundle.mjs", apps[0]], source)
        with (apps[0] / "Contents/Info.plist").open("rb") as stream:
            require(plistlib.load(stream).get("CFBundleShortVersionString") == app_version, "extracted App version mismatch")
        runners = json_read(apps[0] / "Contents/Resources/remote-runners/manifest.json")
        require(len(runners.get("runners", [])) == 2 and all(item.get("version") == app_version for item in runners["runners"]), "App runner projection version mismatch")
        return inventory(apps[0])
    finally:
        if attached:
            detached = subprocess.run(["hdiutil", "detach", str(mount)], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            # Do not recurse through a mount whose detach is uncertain.
            require(detached.returncode == 0, f"managed DMG detach unconfirmed; retained exact mountpoint {mount}")
        shutil.rmtree(directory)


TOOL_COMMANDS = {
    "rustc": ["rustc", "-vV"], "cargo": ["cargo", "-V"],
    "cargo_zigbuild": ["cargo-zigbuild", "--version"], "zig": ["zig", "version"],
    "node": ["node", "--version"], "npm": ["npm", "--version"],
    "rust_targets": ["rustup", "target", "list", "--installed"],
}


def preflight(repo, env):
    for tool in ("rustc", "cargo", "cargo-zigbuild", "zig", "node", "npm", "rustup"):
        require(shutil.which(tool, path=env.get("PATH")) is not None, f"missing required preinstalled tool: {tool}; configure PATH explicitly, no automatic installation")
    tools = {name: run(args, repo, env, capture=True) for name, args in TOOL_COMMANDS.items()}
    require(tools["zig"] == "0.16.0", "Zig 0.16.0 required by reviewed native-runtime notice profile")
    require(tools["cargo_zigbuild"], "cargo-zigbuild did not report its version")
    require(set(TARGETS.values()) <= set(tools["rust_targets"].splitlines()), "required preinstalled Rust targets missing; prepare native and both musl targets separately")
    sysroot = Path(run(["rustc", "--print", "sysroot"], repo, env, capture=True))
    docs = sysroot / "share/doc/rust"
    require(all((docs / name).is_file() for name in ("COPYRIGHT-library.html", "licenses/MIT.txt", "licenses/Apache-2.0.txt")), "rust-docs notice texts missing for active toolchain; prepare rust-docs separately, no automatic installation")
    names = ["COPYRIGHT-library.html", *["licenses/" + path.name for path in sorted((docs / "licenses").glob("*.txt"))]]
    tools["rust_docs"] = {name: digest(docs / name) for name in names}
    require(all(item["bytes"] > 0 for item in tools["rust_docs"].values()), "rust-docs notice texts are empty; prepare complete rust-docs separately")
    return tools


def build(repo, revision, output, app=False):
    clean_revision(repo, revision)
    require(sys.platform == "darwin" and platform.machine() == "arm64", "canonical matrix build requires native macOS arm64 and preinstalled Linux musl toolchains")
    require(not output.exists() and not output.is_symlink(), "output must be absent")
    require(not output.resolve().is_relative_to(repo.resolve()), "build output must be outside source checkout")
    allowed = ("PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME", "TMPDIR", "SDKROOT", "DEVELOPER_DIR")
    env = {key: os.environ[key] for key in allowed if key in os.environ}
    tools = preflight(repo, env)
    output.mkdir(parents=False)
    source = source_export(repo, revision, output)
    inputs = tracked_inventory(source)
    for ancestor in source.parents:
        require(not any((ancestor / ".cargo" / name).exists() for name in ("config", "config.toml")), "unfrozen ancestor Cargo configuration; choose an isolated output parent")
    # No caller runner/output path and no inherited RUSTFLAGS/compiler wrapper.
    env["CARGO_TARGET_DIR"] = str(source / "target")
    # Reuse downloads, never caller Cargo config/rustflags/wrappers. Linker/SDK
    # prerequisites must already work through the tracked source/default tools.
    cache = Path(env.get("CARGO_HOME", str(Path.home() / ".cargo")))
    cargo_home = output / "cargo-home"
    cargo_home.mkdir()
    for folder in ("registry", "git"):
        if (cache / folder).is_dir():
            (cargo_home / folder).symlink_to((cache / folder).resolve(), target_is_directory=True)
    env["CARGO_HOME"] = str(cargo_home)
    # Compilation/tool caches belong to this candidate too. In particular Zig
    # must not silently fill the operator's system volume during a fresh build.
    env["ZIG_GLOBAL_CACHE_DIR"] = str(output / "zig-cache-global")
    env["ZIG_LOCAL_CACHE_DIR"] = str(output / "zig-cache-local")
    env["XDG_CACHE_HOME"] = str(output / "cache")
    env["NPM_CONFIG_CACHE"] = str(output / "npm-cache")
    env["NPM_CONFIG_USERCONFIG"] = os.devnull
    env["NPM_CONFIG_GLOBALCONFIG"] = os.devnull
    commands = []
    def execute(command, cwd=source):
        commands.append({"argv": [str(a).replace(str(source), "<source>").replace(str(output), "<output>") for a in command], "cwd": str(cwd.relative_to(source))})
        run(command, cwd, env)
    for manifest in ("Cargo.toml", "apps/desktop/src-tauri/Cargo.toml", "extensions/browser/native-host/Cargo.toml"):
        execute(["cargo", "fetch", "--locked", "--manifest-path", manifest])
    # All three binaries and both remote runners originate in this fresh target.
    execute(["cargo", "build", "--locked", "--release", "-p", "lintel-runner", "--bin", "lintel", "--target", TARGETS["macos-arm64"]])
    execute(["cargo", "zigbuild", "--locked", "--release", "-p", "lintel-runner", "--bin", "lintel", "--target", TARGETS["linux-x86_64"], "--target", TARGETS["linux-aarch64"]])
    execute(["node", "apps/desktop/scripts/prepare-remote-runners.mjs"])
    execute(["npm", "ci", "--ignore-scripts"], source / "apps/desktop")
    execute(["node", "apps/desktop/scripts/prepare-browser-host.mjs"])
    execute(["node", "apps/desktop/scripts/prepare-browser-extension.mjs"])
    execute(["npm", "run", "build"], source / "apps/desktop")
    execute(["node", "scripts/prepare-rust-notices.mjs"])
    resources = source / "apps/desktop/src-tauri"
    cli_version = tomllib.loads((source / "apps/runner/Cargo.toml").read_text())["package"]["version"]
    app_version = json_read(source / "apps/desktop/package.json")["version"]
    # The existing App projection labels its runner resource set by App version;
    # CLI packaging correctly requires the runner's own version. Project only
    # that finite manifest identity from the SAME freshly built runner bytes.
    cli_runners = output / "cli-runners"
    shutil.copytree(resources / "runner-bundles", cli_runners)
    cli_manifest = json_read(cli_runners / "manifest.json")
    require(len(cli_manifest.get("runners", [])) == 2, "canonical runner manifest required")
    for runner in cli_manifest["runners"]:
        runner["version"] = cli_version
    (cli_runners / "manifest.json").write_text(json.dumps(cli_manifest, indent=2) + "\n")
    components = {name: {"source_revision": revision, "files": inventory(resources / folder)} for name, folder in {
        "runners": "runner-bundles", "host": "browser-host-bundle", "extensions": "browser-extension-bundle", "notices": "third-party-notices"}.items()}
    components["runners"]["version"] = app_version
    components["cli_runners"] = {"source_revision": revision, "version": cli_version, "files": inventory(cli_runners)}
    components["terms"] = {"source_revision": revision, "files": {name: digest(source / name) for name in TERMS}}
    packages = output / "packages"
    command = ["node", "scripts/package-cli.mjs", "--revision", revision, "--out", str(packages), "--linux-runners", str(cli_runners), "--notices", str(resources / "third-party-notices")]
    for target, triple in TARGETS.items():
        command += ["--" + target, str(source / "target" / triple / "release/lintel")]
    execute(command)
    artifacts = []
    for target in TARGETS:
        matches = list(packages.glob(f"*-{target}.tar.gz"))
        require(len(matches) == 1, "missing/ambiguous CLI package")
        payload = tar_inventory(matches[0])
        protected = protected_cli(source, target, cli_runners)
        require(all(payload.get(name) == info for name, info in protected.items()), "packager changed built component bytes")
        require(set(payload) == set(protected) | {"candidate.json", "README.txt", "SHA256SUMS"}, "unknown CLI payload")
        artifacts.append({"kind": "cli", "target": target, "path": str(matches[0].relative_to(output)), "artifact": digest(matches[0]), "files": payload})
    if app:
        app_output = output / "app"
        execute(["node", source / "apps/desktop/scripts/build-macos-app.mjs", "--out", str(app_output), "--bundles", "app,dmg", "--ci"], source / "apps/desktop")
        bundle = app_output / "Lintel.app"
        with (bundle / "Contents/Info.plist").open("rb") as stream:
            require(plistlib.load(stream).get("CFBundleShortVersionString") == app_version, "App version differs from frozen source")
        files = inventory(bundle)
        prefixes = {"runners": "remote-runners", "host": "browser-host", "extensions": "browser-extensions", "notices": "third-party-notices", "terms": "project-licenses"}
        for name, prefix in prefixes.items():
            for relative, info in components[name]["files"].items():
                require(files.get(f"Contents/Resources/{prefix}/{relative}") == info, f"App component not from this build: {name}/{relative}")
        dmgs = list(app_output.glob("*.dmg"))
        require(len(dmgs) == 1, "one final DMG required")
        require(dmg_inventory(dmgs[0], source, app_version, digest(dmgs[0])) == files, "final DMG differs from canonical App")
        artifacts.append({"kind": "app", "target": "macos-arm64", "path": str(dmgs[0].relative_to(output)), "artifact": digest(dmgs[0]), "files": files})
    require(preflight(source, env) == tools, "build toolchain/targets/notice inputs changed during candidate construction")
    clean_revision(repo, revision)
    clean_revision(source, revision)
    # Recheck every cloned tracked source after build; generators may only
    # modify ignored outputs, never silently alter approved build inputs.
    require(all((source / name).is_file() and digest(source / name) == info for name, info in inputs.items()), "build altered frozen source inputs")
    receipt = {"schema": SCHEMA, "source_revision": revision, "cli_version": cli_version, "app_version": app_version, "origin_authenticated": False,
               "build_kind": "fixed-canonical-fresh-clone", "tools": tools, "commands": commands,
               "inputs": inputs, "components": components, "artifacts": artifacts,
               "limitations": ["local process observation, not independently authenticated source provenance", "isolated local Git clone retains exact clean source metadata", "runtime platform/clean-Mac/signing/notarization acceptance remains independent"]}
    publish(output / "build-receipt.json", receipt)
    return receipt


def ci_module():
    spec = importlib.util.spec_from_file_location("lintel_candidate_ci", ROOT / "tests/ci_plan.py")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def verify(receipt, base, plan_doc, results, docs):
    require(set(receipt) == {"schema", "source_revision", "cli_version", "app_version", "origin_authenticated", "build_kind", "tools", "commands", "inputs", "components", "artifacts", "limitations"}, "invalid build receipt fields")
    require(receipt["schema"] == SCHEMA and receipt["origin_authenticated"] is False and receipt["build_kind"] == "fixed-canonical-fresh-clone", "untrusted receipt kind")
    revision = receipt["source_revision"]
    require(re.fullmatch(r"[0-9a-f]{40}", revision) is not None, "invalid revision")
    ci = ci_module()
    plan = ci._plan_from_dict(plan_doc)
    require(plan.profile == "full" and plan.head == revision, "exact full CI SHA required")
    # Rebuild the expected finite full selection: a caller cannot rename a
    # reduced plan to full and omit required checks.
    expected = ci.build_plan("workflow_dispatch", base=None, head=revision, checkout_head=revision)
    require(plan.jobs == expected.jobs, "full plan selection differs from canonical required checks")
    problems = ci.aggregate(plan, results, docs)
    require(not problems, "CI rejected: " + "; ".join(problems))
    for job in plan.jobs:
        for platform_name in job.platforms:
            for filename, checks in ci.canonical_evidence(job.id, platform_name, job.checks):
                document = ci.fetch_evidence(docs, filename)
                require(document["git"].get("dirty") is False, "CI evidence checkout cleanliness unknown/dirty")
    require(set(receipt["components"]) == {"runners", "cli_runners", "host", "extensions", "notices", "terms"}, "missing component provenance")
    for item in receipt["components"].values():
        require(item["source_revision"] == revision and item["files"], "unknown component origin")
    minimums = {
        "runners": {"manifest.json", "x86_64-unknown-linux-musl/lintel", "aarch64-unknown-linux-musl/lintel"},
        "cli_runners": {"manifest.json", "x86_64-unknown-linux-musl/lintel", "aarch64-unknown-linux-musl/lintel"},
        "host": {"manifest.json", "lintel-browser-host"},
        "extensions": {"manifest.json", "chromium/manifest.json", "firefox/manifest.json"},
        "notices": {"inventory.json", "README.md", "frontend.md", "rust.md", "rust-standard-library.html", "rust-native-runtime.md"},
        "terms": set(TERMS),
    }
    require(receipt["components"]["runners"].get("version") == receipt["app_version"] and receipt["components"]["cli_runners"].get("version") == receipt["cli_version"], "crossed App/CLI runner projection identity")
    for component, names in minimums.items():
        require(names <= set(receipt["components"][component]["files"]), "missing required component files")
    require(any(name.startswith("licenses/") for name in receipt["components"]["notices"]["files"]), "missing toolchain licenses")
    require(receipt["inputs"] and receipt["commands"] and receipt["tools"], "missing observed build inputs")
    require(receipt["artifacts"], "no final artifacts")
    require(isinstance(receipt["cli_version"], str) and re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:[-+][0-9A-Za-z.+-]+)?", receipt["cli_version"]), "invalid candidate version")
    require({item.get("target") for item in receipt["artifacts"] if item.get("kind") == "cli"} == set(TARGETS), "complete three-target CLI matrix required")
    checked, seen, selected = [], set(), set()
    for item in receipt["artifacts"]:
        require(set(item) == {"kind", "target", "path", "artifact", "files"}, "invalid artifact fields")
        name = safe_name(item["path"])
        require(name not in seen, "duplicate final artifact")
        seen.add(name)
        identity = (item["kind"], item["target"])
        require(identity not in selected, "duplicate platform artifact")
        selected.add(identity)
        path = base / name
        require(path.resolve().is_relative_to(base.resolve()) and path.is_file() and not path.is_symlink(), "artifact escapes receipt root")
        require(path.stat().st_size <= MAX_BYTES, "final artifact budget exceeded")
        require(digest(path) == item["artifact"], "final artifact bytes changed")
        if item["kind"] == "cli":
            require(item["target"] in TARGETS, "unknown CLI target")
            metadata = {}
            actual = tar_inventory(path, expected_artifact=item["artifact"], metadata=metadata)
            require(actual == item["files"], "extracted CLI bytes differ from build receipt")
            manifest = metadata.get("candidate.json", {})
            require(manifest.get("schema") == "lintel.cli-candidate/1" and manifest.get("source_revision") == revision and manifest.get("target") == item["target"] and manifest.get("version") == receipt["cli_version"] and manifest.get("notices_included") is True, "package metadata differs from build identity")
            runners = metadata.get("bin/remote-runners/manifest.json", {})
            require(len(runners.get("runners", [])) == 2 and all(runner.get("version") == receipt["cli_version"] for runner in runners["runners"]), "CLI runner projection version mismatch")
            require("bin/lintel" in actual and all(f"project-licenses/{term}" in actual for term in TERMS), "missing binary/project licenses")
            prefixes = {"cli_runners": "bin/remote-runners", "notices": "third-party-notices", "terms": "project-licenses"}
        else:
            require(item["kind"] == "app" and item["target"] == "macos-arm64", "unknown artifact kind")
            actual = dmg_inventory(path, ROOT, receipt["app_version"], item["artifact"])
            require(actual == item["files"], "extracted DMG bytes differ from build receipt")
            prefixes = {"runners": "Contents/Resources/remote-runners", "host": "Contents/Resources/browser-host", "extensions": "Contents/Resources/browser-extensions", "notices": "Contents/Resources/third-party-notices", "terms": "Contents/Resources/project-licenses"}
        if item["kind"] == "cli":
            allowed = {"bin/lintel", "candidate.json", "README.txt", "SHA256SUMS"}
            for component, prefix in prefixes.items():
                allowed.update(f"{prefix}/{safe_name(name)}" for name in receipt["components"][component]["files"])
            require(set(actual) == allowed, "unknown/missing finite CLI member")
            if item["target"] != "macos-arm64":
                require(actual["bin/lintel"] == receipt["components"]["cli_runners"]["files"][TARGETS[item["target"]] + "/lintel"], "Linux CLI differs from its same-build runner")
        for component, prefix in prefixes.items():
            for relative, info in receipt["components"][component]["files"].items():
                require(actual.get(f"{prefix}/{safe_name(relative)}") == info, "component bytes missing or not from recorded build")
        checked.append({"path": name, "kind": item["kind"], "target": item["target"], "sha256": item["artifact"]["sha256"]})
    ci_inputs = {"plan": plan_doc, "results": results, "evidence": docs}
    ci_identity = {name: hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest() for name, value in ci_inputs.items()}
    return {"schema": "lintel.local-candidate-gate/1", "source_revision": revision,
            "cli_version": receipt["cli_version"], "app_version": receipt["app_version"],
            "ci_profile": "full", "ci_semantic_json_sha256": ci_identity, "consistency_passed": True,
            "build_origin_authenticated": False, "ci_origin_authenticated": False, "release_authorized": False,
            "artifacts": checked, "limitations": ["local receipts and CI JSON were not independently authenticated", "no signing/notarization, clean-Mac download, upgrade or target-runtime acceptance is inferred"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    subs = parser.add_subparsers(dest="mode", required=True)
    build_args = subs.add_parser("build")
    build_args.add_argument("--revision", required=True)
    build_args.add_argument("--out", type=Path, required=True)
    build_args.add_argument("--app", action="store_true")
    check = subs.add_parser("verify")
    for name in ("receipt", "ci-plan", "ci-results", "ci-evidence", "out"):
        check.add_argument("--" + name, type=Path, required=True)
    args = parser.parse_args()
    if args.mode == "build":
        build(ROOT, args.revision, args.out.absolute(), args.app)
    else:
        ci = ci_module()
        receipt = json_read(args.receipt)
        # The gate/required-check policy itself must be the candidate source,
        # rather than whichever older verifier happens to be on PATH.
        clean_revision(ROOT, receipt["source_revision"])
        result = verify(receipt, args.receipt.absolute().parent,
                        json_read(args.ci_plan), json_read(args.ci_results), ci._collect_evidence(args.ci_evidence))
        publish(args.out, result)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (ValueError, OSError, subprocess.CalledProcessError, tarfile.TarError, KeyError, TypeError) as error:
        print(f"candidate gate rejected: {error}", file=sys.stderr)
        sys.exit(1)
