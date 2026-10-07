# Quickstart: a first local trial from public source

[中文](quickstart.md) · English

This page walks a new visitor through one small journey inside a **disposable
synthetic fixture**: register a Claude config environment, generate synthetic
instruction / memory / session originals (plus one file you will deliberately
**not** select), preview the scope with an exact selection, review the original
plan and its hash, submit an **archive-only** preservation, query the original
ID and the final result, then re-inspect and read from the explicit
`archive_path` in a **separate independent state** — and finally verify
byte-for-byte that the source originals, settings and placeholder credential
were never touched.

Audience: someone who has the Lintel source and wants to confirm the CLI's real
behavior on their own machine.

> This is **Lintel 0.1.0 Preview / development preview**. Real authentication and
> native resume, production Chrome/Edge/Firefox/AdsPower, production VPS and
> signed distribution are **not verified**. See
> [current state](current-state.md) for exact evidence and gaps.

---

## 0. Prerequisites

- Rust stable (to build the CLI and workspace).
- Python 3 (to parse JSON and read a passphrase without echo). **No `jq`.**
- No Node, Tauri, GUI or installed App is needed anywhere.

Get the source and build the CLI. `cargo build` writes to `target/` and may
fetch dependencies — that is a normal local build; it does **not** run Claude,
discover personal directories, or initialize Lintel state.

```sh
git clone https://github.com/IndelibleVivi/lintel-cc.git
cd lintel-cc
cargo build -p lintel-runner
```

The artifact is `target/debug/lintel`. Run every command below with the
**repository root as the current directory**.

## 1. Open a separate interactive shell

Run `sh` to open a separate interactive shell, then execute the blocks one at
a time. Each block runs immediately, so you can read the actual plan before
approving it. Finish with `exit` to return to your original shell. We define
a **disposable fixture** and a `lintel` function that injects the environment
only for each CLI call. Your original shell's `HOME` and `PATH` stay unchanged;
your installed Python 3 remains available.

```sh
sh
set -eu

# Disposable synthetic fixture: fully fake originals, not your real ~/.claude
ROOT="$(python3 -c 'import os,tempfile;print(os.path.realpath(tempfile.mkdtemp(prefix="lintel-quickstart.")))')"
echo "fixture = $ROOT"
mkdir -p "$ROOT/home" "$ROOT/state" "$ROOT/claude-root/projects/example/memory"

printf '%s\n' 'Synthetic instruction only.'                 > "$ROOT/claude-root/CLAUDE.md"
printf '%s\n' 'Synthetic memory only.'                      > "$ROOT/claude-root/projects/example/memory/MEMORY.md"
printf '%s\n' 'This synthetic memory file is NOT selected.' > "$ROOT/claude-root/projects/example/memory/NOT_SELECTED.md"
printf '%s\n' '{"type":"synthetic","message":"hello"}'      > "$ROOT/claude-root/projects/example/session.jsonl"
printf '%s\n' '{"env":{"SYNTHETIC_KEEP":"1"}}'              > "$ROOT/claude-root/settings.json"
printf '%s\n' 'SYNTHETIC_CREDENTIAL_DO_NOT_COPY'            > "$ROOT/claude-root/.credentials.json"

# Absolute CLI path (valid for your checkout); quoting keeps a path with spaces working
LINTEL="$PWD/target/debug/lintel"

# Inject the synthetic HOME/state only per call; never modify the current shell
lintel() { env HOME="$ROOT/home" LINTEL_TEST_HOME="$ROOT/home" LINTEL_STATE_DIR="$ROOT/state" "$LINTEL" "$@"; }
```

The remaining blocks **continue in this interactive shell**; step 8 closes it
with `exit`. If any step fails, keep the displayed original ID and fixture path,
stop the later steps and reconcile the result. Do not resubmit.

> This journey's operations target only this synthetic root and do not invoke
> Claude, authentication, browser or services. Separating HOME is **not** an OS
> sandbox and does not prove other processes or managed sources cannot reach
> your real environment; it only means this journey does not point at them.
> This is a **synthetic test**, not a production workflow.

## 2. Register the environment

`env register` records an existing config root in Lintel and returns an
environment `id` (UUID). Later commands use that id.

```sh
lintel env register --name 'Synthetic quickstart' --root "$ROOT/claude-root" > "$ROOT/register.json"
ID="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["data"]["id"])' "$ROOT/register.json")"
echo "environment id = $ID"
```

Registration only records inventory; it does not run Claude or write to the
target.

## 3. Read-only scan: see what is there

`work inventory` is a **read-only, paged, metadata-only** listing: it returns
the relative path, category and byte size of the selected categories, plus a
`digest` bound to that one scan. It does not read file bodies.

```sh
lintel work inventory --environment "$ID" --categories instructions,memory,sessions --json
```

You should see **four** synthetic originals:

| path |
| --- |
| `CLAUDE.md` |
| `projects/example/memory/MEMORY.md` |
| `projects/example/memory/NOT_SELECTED.md` |
| `projects/example/session.jsonl` |

plus a `digest`. Note that `NOT_SELECTED.md` appears in the listing but will
**not** be selected later.

The optional capacity preflight is metadata-only and flags over-limit files or
scan gaps. Here we use repeated `--path` to select exactly three:

```sh
lintel work preflight --environment "$ID" --categories instructions,memory,sessions \
  --path CLAUDE.md \
  --path projects/example/memory/MEMORY.md \
  --path projects/example/session.jsonl \
  --json
```

The response has `work_selection.mode` = `paths`, `totals.files` = 3 and
`eligible` = true. The current runner admission is **256 MiB per file, 1 GiB
total, 10,000 files**, with a separate **16 MiB** versioned persisted-record
limit. A passing preflight is only metadata admission; the real plan still
re-reads every selected original.

## 4. Generate and review the archive-only plan

`work archive plan` builds a **frozen plan**: it freezes the selected files and
their full content digests. `--output-path` may be omitted (then it is stored
in Lintel state); here we write to a new file in the fixture. It **creates no
new config root, logs out nothing, deletes nothing, and changes no settings**.

```sh
lintel work archive plan --environment "$ID" --categories instructions,memory,sessions \
  --path CLAUDE.md \
  --path projects/example/memory/MEMORY.md \
  --path projects/example/session.jsonl \
  --output-path "$ROOT/carried.age" --json > "$ROOT/plan.json"
```

Check the plan's `kind=archive`, `outcome=archive_only`, `file_count=3`, and
that `work_selection` lists exactly those three paths (**not**
`NOT_SELECTED.md`).

Pull out its id and hash, then read the frozen plan back:

```sh
PLAN_ID="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["data"]["id"])' "$ROOT/plan.json")"
PLAN_HASH="$(python3 -c 'import json,sys;print(json.load(open(sys.argv[1]))["data"]["hash"])' "$ROOT/plan.json")"
lintel plan show "$PLAN_ID" --json | python3 -m json.tool
```

`plan show` reads back only the public scope: `kind`, `hash`, `output_path`, the
selection — **no** file bodies or passphrase.

> **Approval must match this plan's hash.** If a selected original or frozen target
> identity changes after the plan is generated, the approval is invalidated and you must preview
> again. There is no blanket `--yes`.

## 5. Submit explicitly (passphrase via stdin only)

Submission must include the **original plan hash**. The archive passphrase goes
in via JSON stdin and **never** in argv, shell history, request files, the plan
or the receipt. The `getpass` call below reads the passphrase without echo from
the controlling terminal; `inspect` / `read` require at least 12 characters.

Type **your own test passphrase of at least 12 characters** (for example a
temporary string) and **repeat exactly the same one** in step 7. It is a
temporary passphrase for this synthetic fixture, not a pre-existing fixed
value; do not use it for real.

```sh
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' \
  | lintel job submit --plan "$PLAN_ID" --approval "$PLAN_HASH" > "$ROOT/receipt.json"
```

`job submit` returns a **durable ACK**, usually with `status` = `accepted`.

> **accepted is not completed.** The ACK only means the request was durably
> received; the task can still fail afterwards. `ok:true` means the request was
> handled, not that the task finished — check `data.status`, steps and coverage.

To wait for the result, use `job wait` (it **only polls, never resubmits**;
a timeout returns non-zero without resubmitting):

```sh
lintel job wait "$PLAN_ID" --timeout 30s --json | python3 -m json.tool
```

On completion `status` is `completed`, the `archive` step has status `completed`,
`archive_path` points at `$ROOT/carried.age`, and there is an `archive_digest`.
An archive-only plan does **not** produce a `new_root`.

At any time you can query by the original ID:

```sh
lintel job show "$PLAN_ID" --json | python3 -m json.tool
```

> **Query-only**: querying the original job only **verifies, never replays**. It
> does not auto-recover, migrate or launch, and needs no passphrase; but the
> query itself **can update a persisted interruption-reconciliation record**, so
> it is not purely read-only. After an interruption you only verify by the
> original ID.

## 6. Confirm the source is intact and no root was created

Archive-only preservation **does not touch** the source directory and creates
no new environment. Verify all four originals byte-for-byte (including the
unselected `NOT_SELECTED.md`), plus settings and the placeholder credential:

```sh
python3 - "$ROOT" <<'PYTHON'
from pathlib import Path
import sys
root = Path(sys.argv[1])
expected = {
    "CLAUDE.md": b"Synthetic instruction only.\n",
    "projects/example/memory/MEMORY.md": b"Synthetic memory only.\n",
    "projects/example/memory/NOT_SELECTED.md": b"This synthetic memory file is NOT selected.\n",
    "projects/example/session.jsonl": b'{"type":"synthetic","message":"hello"}\n',
    "settings.json": b'{"env":{"SYNTHETIC_KEEP":"1"}}\n',
    ".credentials.json": b"SYNTHETIC_CREDENTIAL_DO_NOT_COPY\n",
}
for relative, original in expected.items():
    assert (root / "claude-root" / relative).read_bytes() == original, relative
ciphertext = (root / "carried.age").read_bytes()
assert ciphertext.startswith(b"age-encryption.org/")
assert expected["CLAUDE.md"].strip() not in ciphertext
print("PASS: 6 source files unchanged; age ciphertext created")
PYTHON
```

Then confirm only one environment is registered, and the package is
recognizable as an age package with no plaintext:

```sh
lintel env list --json | python3 -c 'import sys,json;print("environments:",len(json.load(sys.stdin)["data"]["environments"]))'
head -c 19 "$ROOT/carried.age"; echo
```

The environment count should be 1 (only the synthetic root was registered), the
output starts with `age-encryption.org/`, and `Synthetic instruction` never
appears as plaintext inside the package.

## 7. Inspect / read in a separate independent state

This is the key step: carry the **ciphertext** to a **separate independent
HOME/state** (simulating another machine) and unlock it again with **no source
inventory and no source job**. Still use no-echo input; **never** put the
passphrase in shell history.

```sh
mkdir -p "$ROOT/independent/home" "$ROOT/independent/state"

lintel_dest() { env HOME="$ROOT/independent/home" LINTEL_TEST_HOME="$ROOT/independent/home" LINTEL_STATE_DIR="$ROOT/independent/state" "$LINTEL" "$@"; }

```

```sh
# inspect: read-only manifest, requires exactly one source (here --archive-path)
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' \
  | lintel_dest work archive inspect --archive-path "$ROOT/carried.age"

```

```sh
# read: read the body of one explicit path inside the package
python3 -c 'import getpass,json; print(json.dumps({"archive_passphrase":getpass.getpass("Archive passphrase: ")}))' \
  | lintel_dest work archive read --archive-path "$ROOT/carried.age" --path CLAUDE.md

```

```sh
# this independent state holds no work-package record of its own
lintel_dest work archive list --json
```

Use **exactly the same** test passphrase as step 5. `inspect` returns exactly
three files (with `sha256` digests, **not** including `NOT_SELECTED.md`);
`read` returns that file's body; and `work archive list` returns
`{"jobs":[]}`, proving that reading does **not** depend on the source job.

Ciphertext transfer between hosts is done by an **explicit system file
transfer** (SSH/SFTP/scp or removable storage). Lintel does not transfer across
hosts automatically, and does not upload the whole state.

## 8. Close the interactive shell

The journey is done; close the interactive shell opened in step 1 with `exit`.
Your original shell environment stays unchanged. The synthetic fixture is
retained in the temporary directory for inspection; no real environment is
installed or activated.

```sh
exit
```

## 9. Connect to the independent work package (optional)

To **import** selected content in the independent state, use `work import plan`
(also with `--archive-path` as the source) to preview the final destinations,
then separately approve execution. It freezes the ciphertext byte digest and
the target file state, and never overwrites a same-name file. Sessions and
memory are kept as data; there is **no claim that a conversation can be
resumed**. See the portable-work section of the
[Agent CLI guide](agents.md#portable-work).

---

## Want to use the App instead?

The desktop dev path is the local-build section of the repository
[README](../README.md). `npm run dev:synthetic` creates a separate temporary
home/state and calls the real CLI; the interface clearly labels it a "test
space" and never uses your Claude login or browser data. Plain `npm run dev`
does not provide a browser-to-host execution channel.

## Just want the CLI installed?

Public releases are empty; there is **no formal download link**. Install
independently from verified source, or use the candidate-package flow (verify
identity, platform and bytes, unpack into a new version directory, change no
PATH / shell rc / App / service). See
[obtaining a candidate](agents.md#用户安装候选包) and
[independent source install](agents.md#从源码独立安装).

## Three distinctions to keep straight

| Statement | Is not |
| --- | --- |
| `accepted` (durable ACK) | the task is complete |
| querying the original job (query-only, no replay) | auto recovery / migration / launch |
| fully preserved data | the original conversation can be resumed |

## Platforms and unverified boundaries

- **macOS desktop GUI**: Tauri, currently built as an unsigned arm64 candidate.
- **macOS arm64 CLI** and **Linux headless CLI** (x86_64 / aarch64): standalone
  CLI; Linux source is built and checked synthetically. Three-platform
  candidate package records exist, but that does **not** mean they are
  downloadable or that the Linux runtime fully passes.
- Real authentication, native resume, production browsers / AdsPower,
  production VPS and signed distribution are **not yet verified**.
- Every read first fully decrypts and privately stages the **entire
  authenticated package**; allow temporary disk and time.
- Independent package inspect returns file metadata; read outputs private
  bodies that **do not enter ordinary job / diagnostic / Agent metadata**.
  Control their terminal display and any later copying yourself.

## Related documentation

- [Current state](current-state.md): candidate / resource / evidence table and undelivered goals.
- [Human operator guide](operator-guide.md): pick an entry by goal in the GUI (Chinese).
- [Agent CLI guide](agents.md): interface discovery, plan / approval / query, portable work and candidate install.
- [Unified verification](verification.md): default synthetic entry point and independent runtime gates.
- [Preview feedback template](../.github/ISSUE_TEMPLATE/preview-feedback.yml): attach synthetic / redacted examples only.
