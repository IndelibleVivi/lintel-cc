#!/usr/bin/env python3
"""Read a bounded candidate App archive; never extract or execute it."""
import json
import plistlib
import struct
import sys
import tarfile
from pathlib import PurePosixPath

def inspect(file, version, platform):
    files, total, info, executable = 0, 0, None, None
    expected_cpu = {"darwin-aarch64": 0x0100000C, "darwin-x86_64": 0x01000007}[platform]
    seen = {}
    with tarfile.open(file, "r|gz") as archive:
        for entry in archive:
            files += 1
            if files > 10000 or entry.size < 0 or entry.size > 512 * 1024 * 1024:
                raise ValueError("App entry budget exceeded")
            p = PurePosixPath(entry.name)
            if p.is_absolute() or ".." in p.parts or not p.parts or p.parts[0] != "Lintel.app":
                raise ValueError("App archive path escapes the single bundle")
            normalized = str(p)
            if normalized in seen:
                raise ValueError("Duplicate App archive path")
            seen[normalized] = "directory" if entry.isdir() else "symlink" if entry.issym() else "file"
            if not (entry.isfile() or entry.isdir() or entry.issym()):
                raise ValueError("Unsupported App archive entry type")
            if entry.issym():
                target = PurePosixPath(entry.linkname)
                if target.is_absolute():
                    raise ValueError("Absolute App symlink")
                stack = list(p.parent.parts)
                for part in target.parts:
                    if part == "..":
                        if len(stack) <= 1:
                            raise ValueError("App symlink escapes bundle")
                        stack.pop()
                    elif part != ".":
                        stack.append(part)
            total += entry.size
            if total > 1024 * 1024 * 1024:
                raise ValueError("App unpacked budget exceeded")
            if entry.isfile():
                if normalized == "Lintel.app/Contents/Info.plist":
                    if entry.size > 1024 * 1024:
                        raise ValueError("Info.plist budget exceeded")
                    info = plistlib.loads(archive.extractfile(entry).read())
                elif normalized == "Lintel.app/Contents/MacOS/lintel-desktop":
                    if not entry.mode & 0o111:
                        raise ValueError("App executable has no executable mode")
                    header = archive.extractfile(entry).read(8)
                    if len(header) != 8 or header[:4] != bytes.fromhex("cffaedfe") or struct.unpack("<I", header[4:])[0] != expected_cpu:
                        raise ValueError("Expected selected native Mach-O architecture")
                    executable = normalized
    for name in seen:
        for parent in PurePosixPath(name).parents:
            if seen.get(str(parent)) in ("symlink", "file"):
                raise ValueError("App entry descends through a symlink or file")
    if not info or info.get("CFBundleIdentifier") != "app.lintel.desktop" or info.get("CFBundleShortVersionString") != version or info.get("CFBundleExecutable") != "lintel-desktop" or not executable:
        raise ValueError("App bundle identity/version differs from release record")
    return {"files": files, "plain_bytes": total, "version": version, "platform": platform, "scope": "format-and-declared-identity-only"}

if __name__ == "__main__":
    try:
        print(json.dumps(inspect(*sys.argv[1:])))
    except Exception as error:
        print(str(error), file=sys.stderr)
        sys.exit(1)
