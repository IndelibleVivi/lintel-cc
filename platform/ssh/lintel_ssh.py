#!/usr/bin/env python3
"""Explicit SSH controller for the Lintel JSON runner (Python stdlib only)."""
from __future__ import annotations

import argparse
import contextlib
import fcntl
import getpass
import json
import os
from pathlib import Path
import re
import selectors
import shlex
import subprocess
import sys
import tempfile
import time
import warnings

ALIAS = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,127}\Z")
IDENTIFIER = re.compile(r"[A-Za-z0-9][A-Za-z0-9_-]{0,159}\Z")
MAX_JSON = 2 * 1024 * 1024
REMOTE_COMMAND = ("lintel", "request")
SUBMIT_COMMAND = ("lintel", "submit")
# Deliberately no shell, generic file, arbitrary-exec, or installation endpoint.
REQUEST_FIELDS = {
    "discover": {}, "inspect": {"environment_id": str},
    "plan_policy": {"environment_id": str, "preset": str, "keep_remote_control": bool},
    "plan_reset": {"environment_id": str, "recipe": str, "categories": list},
    "plan_restore": {"job_id": str}, "jobs": {}, "job": {"job_id": str},
    "drift": {"environment_id": str}, "export_support": {},
}


class ControllerError(Exception):
    def __init__(self, code: str, message: str):
        self.code, self.message = code, message
        super().__init__(message)


def valid_alias(alias: str) -> str:
    if not ALIAS.fullmatch(alias):
        raise ControllerError("invalid_alias", "Choose a literal SSH alias without shell syntax or wildcards.")
    return alias


def valid_id(value: str) -> str:
    if not IDENTIFIER.fullmatch(value):
        raise ControllerError("invalid_id", "The runner plan/job identifier is invalid.")
    return value


def list_aliases(path: Path) -> dict:
    """Read literal Host entries only; never invoke ssh -G or follow Include."""
    try:
        with path.open("rb") as source:
            raw = source.read(MAX_JSON + 1)
        if len(raw) > MAX_JSON:
            raise ControllerError("config_too_large", "The explicit SSH config exceeds the inspection budget.")
        lines = raw.decode("utf-8").splitlines()
    except (OSError, UnicodeError):
        raise ControllerError("config_unreadable", "The explicit SSH config could not be read.") from None
    aliases, ignored = set(), set()
    for line in lines:
        try:
            words = shlex.split(line, comments=True)
        except ValueError:
            ignored.add("unparsed_line")
            continue
        if not words:
            continue
        # OpenSSH also permits Keyword=value. No value is ever executed here.
        first = words[0].split("=", 1)
        key = first[0].lower()
        values = ([first[1]] if len(first) == 2 else []) + words[1:]
        if key == "host":
            aliases.update(v for v in values if ALIAS.fullmatch(v))
            if any(not ALIAS.fullmatch(v) for v in values):
                ignored.add("Host patterns")
        elif key in {"include", "match"}:
            ignored.add(key.capitalize())
    return {"aliases": sorted(aliases), "ignored": sorted(ignored),
            "coverage": "literal Host entries in this file only; Include and Match are not evaluated"}


def validate_request(payload: dict) -> dict:
    if not isinstance(payload, dict) or not isinstance(payload.get("command"), str) or payload["command"] not in REQUEST_FIELDS:
        raise ControllerError("unsupported_command", "Use an allowed read/plan request, or the separate execute operation.")
    fields = REQUEST_FIELDS[payload["command"]]
    if set(payload) != {"command", *fields}:
        raise ControllerError("invalid_request", "The request fields do not match this command's schema.")
    for key, expected in fields.items():
        if type(payload[key]) is not expected:
            raise ControllerError("invalid_request", "A request field has the wrong type.")
        if expected is str and (not payload[key] or len(payload[key]) > 1024):
            raise ControllerError("invalid_request", "A request field has an invalid length.")
    if payload["command"] == "plan_policy" and payload["preset"] not in {"preserve", "reduce"}:
        raise ControllerError("invalid_request", "The policy preset is not supported.")
    if payload["command"] == "plan_reset":
        if payload["recipe"] != "rebuild" or not all(type(v) is str and len(v) <= 128 for v in payload["categories"]):
            raise ControllerError("invalid_request", "The rebuild recipe/categories are invalid.")
    return payload


class Transport:
    """One request, one process, no retry. ssh executable injection exists for tests only."""
    def __init__(self, ssh: str = "/usr/bin/ssh", deadline: float = 60):
        self.ssh, self.deadline = ssh, deadline

    def call(self, alias: str, payload: dict, *, submit: bool = False) -> dict:
        args = [self.ssh, "-T", "-oBatchMode=yes", "-oStrictHostKeyChecking=yes",
                "-oUpdateHostKeys=no", "-oPermitLocalCommand=no", "-oClearAllForwardings=yes", "-oRequestTTY=no",
                "-oConnectTimeout=10", "-oServerAliveInterval=15", "-oServerAliveCountMax=2",
                valid_alias(alias), *(SUBMIT_COMMAND if submit else REMOTE_COMMAND)]
        encoded = (json.dumps(payload, ensure_ascii=True, separators=(",", ":")) + "\n").encode()
        if len(encoded) > MAX_JSON:
            raise ControllerError("request_too_large", "The request exceeds the transport budget.")
        try:
            proc = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                    stderr=subprocess.PIPE, shell=False)
        except OSError:
            raise ControllerError("ssh_unavailable", "The system SSH process could not start.") from None
        # Bounded nonblocking I/O prevents hostile stdout/stderr or blocked stdin from
        # consuming unbounded memory or defeating the transport deadline.
        output = bytearray()
        with selectors.DefaultSelector() as selector:
            for stream in (proc.stdin, proc.stdout, proc.stderr):
                os.set_blocking(stream.fileno(), False)
            selector.register(proc.stdin, selectors.EVENT_WRITE, "stdin")
            selector.register(proc.stdout, selectors.EVENT_READ, "stdout")
            selector.register(proc.stderr, selectors.EVENT_READ, "stderr")
            sent = 0
            deadline = time.monotonic() + self.deadline
            try:
                while selector.get_map():
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise ControllerError("transport_unknown", "SSH timed out; query the existing job before another action.")
                    for key, _ in selector.select(min(remaining, 0.5)):
                        stream = key.fileobj
                        if key.data == "stdin":
                            try:
                                sent += os.write(stream.fileno(), encoded[sent:])
                            except BrokenPipeError:
                                selector.unregister(stream); stream.close(); continue
                            if sent == len(encoded):
                                selector.unregister(stream); stream.close()
                        else:
                            chunk = os.read(stream.fileno(), 65536)
                            if not chunk:
                                selector.unregister(stream); stream.close(); continue
                            if key.data == "stdout":
                                output.extend(chunk)
                                if len(output) > MAX_JSON:
                                    raise ControllerError("transport_unknown", "The runner response exceeds the transport budget; query the existing job.")
                            # stderr may contain paths/account data; do not store or echo it.
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    raise ControllerError("transport_unknown", "SSH timed out; query the existing job.")
                code = proc.wait(timeout=remaining)
            except (ControllerError, OSError, subprocess.TimeoutExpired) as error:
                proc.kill(); proc.wait()
                if isinstance(error, ControllerError):
                    raise
                raise ControllerError("transport_unknown", "SSH was interrupted; query the existing job before another action.") from None
            finally:
                for stream in (proc.stdin, proc.stdout, proc.stderr):
                    if not stream.closed:
                        stream.close()
        if code == 255:
            raise ControllerError("transport_unknown", "SSH or runner failed. Verify host trust/connectivity; query the existing job. No host key was accepted automatically.")
        try:
            response = json.loads(output)
        except (ValueError, UnicodeError):
            raise ControllerError("transport_unknown", "The runner response was incomplete or invalid; query the existing job.") from None
        if not isinstance(response, dict) or type(response.get("ok")) is not bool or (
            response["ok"] and "data" not in response
        ) or (not response["ok"] and not isinstance(response.get("error"), dict)):
            raise ControllerError("transport_unknown", "The runner response envelope is invalid; query the existing job.")
        if code != 0 and response["ok"]:
            raise ControllerError("transport_unknown", "The runner exited unexpectedly; query the existing job.")
        return response


class Controller:
    def __init__(self, state_dir: Path, transport: Transport | None = None):
        self.state_dir = state_dir
        self.transport = transport or Transport()

    @contextlib.contextmanager
    def _record(self, alias: str, plan_id: str):
        root = self.state_dir / valid_alias(alias)
        # fsync each new directory entry as well as the eventual record: durable
        # submission intent must survive a crash when this is the first task.
        missing = []
        directory = root
        while not directory.exists():
            missing.append(directory)
            directory = directory.parent
        for directory in reversed(missing):
            directory.mkdir(exist_ok=True, mode=0o700)
            parent_fd = os.open(directory.parent, os.O_RDONLY)
            try:
                os.fsync(parent_fd)
            finally:
                os.close(parent_fd)
        path = root / (valid_id(plan_id) + ".json")
        # Lock stays held through the single SSH call so concurrent UI/CLI callers
        # cannot both submit the same local task.
        fd = os.open(root / (plan_id + ".lock"), os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        try:
            fcntl.flock(fd, fcntl.LOCK_EX)
            yield path
        finally:
            os.close(fd)

    @staticmethod
    def _save(path: Path, record: dict):
        fd, temporary = tempfile.mkstemp(prefix=".pending-", dir=path.parent)
        try:
            with os.fdopen(fd, "w") as output:
                json.dump(record, output, separators=(",", ":")); output.write("\n")
                output.flush(); os.fsync(output.fileno())
            os.replace(temporary, path)
            directory = os.open(path.parent, os.O_RDONLY)
            try:
                os.fsync(directory)
            finally:
                os.close(directory)
        finally:
            if os.path.exists(temporary):
                os.unlink(temporary)

    @staticmethod
    def _load(path: Path):
        try:
            with path.open("rb") as source:
                raw = source.read(MAX_JSON + 1)
            record = json.loads(raw) if len(raw) <= MAX_JSON else None
            if not isinstance(record, dict) or not all(isinstance(record.get(k), str) for k in ("plan_id", "lookup_id", "status")):
                raise ValueError()
            valid_id(record["plan_id"]); valid_id(record["lookup_id"])
            return record
        except (ValueError, OSError, ControllerError):
            raise ControllerError("local_record_invalid", "The local task record needs inspection; no execution was attempted.") from None

    def request(self, alias: str, payload: dict) -> dict:
        return self.transport.call(valid_alias(alias), validate_request(payload))

    def execute(self, alias: str, plan_id: str, approval: str, *, archive_passphrase: str | None = None) -> dict:
        valid_id(plan_id)
        if not isinstance(approval, str) or not approval or len(approval) > 512:
            raise ControllerError("invalid_approval", "Supply the exact approval hash returned by the remote plan.")
        with self._record(alias, plan_id) as path:
            if path.exists():
                # Includes crashes before/after submission and ACK loss: never replay.
                record = self._load(path)
                return self._query(alias, record, path)
            payload = {"command": "execute", "plan_id": plan_id, "approval": approval}
            if archive_passphrase is not None:
                if not isinstance(archive_passphrase, str) or len(archive_passphrase) < 12:
                    raise ControllerError("invalid_archive_passphrase", "The archive passphrase must contain at least 12 characters.")
                payload["archive_passphrase"] = archive_passphrase
            record = {"plan_id": plan_id, "status": "submission_unknown", "lookup_id": plan_id}
            self._save(path, record)  # durable local intent before starting ssh
            # No payload/approval/passphrase/account/path is retained in the local journal.
            response = self.transport.call(alias, payload, submit=True)
            record["status"] = "response_received"
            if response["ok"]:
                receipt = response["data"]
                if not isinstance(receipt, dict) or not isinstance(receipt.get("id"), str) or receipt.get("plan_id") != plan_id:
                    raise ControllerError("transport_unknown", "The receipt identity was invalid; query the existing plan's job.")
                record["lookup_id"] = valid_id(receipt["id"])
                record["status"] = receipt.get("status", "response_received")
            self._save(path, record)
            return response

    def _query(self, alias: str, record: dict, path: Path) -> dict:
        response = self.transport.call(alias, {"command": "job", "job_id": record["lookup_id"]})
        if response["ok"]:
            receipt = response["data"]
            if not isinstance(receipt, dict) or receipt.get("plan_id") != record["plan_id"]:
                raise ControllerError("receipt_mismatch", "The remote receipt does not match this task; execution remains disabled.")
            record["status"] = receipt.get("status", "observed")
            self._save(path, record)
            return response
        # An absent job does not prove an operation did not run or is safe to replay.
        return {"ok": False, "error": {"code": "reconciliation_required", "message":
                "The original job is not available. Inspect the remote journal and target state; this controller will not replay the operation."}}

    def reconnect(self, alias: str, plan_id: str) -> dict:
        with self._record(alias, plan_id) as path:
            record = self._load(path) if path.exists() else {"plan_id": plan_id, "lookup_id": plan_id, "status": "query_only"}
            return self._query(alias, record, path)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--state-dir", type=Path, default=Path.home() / ".local/state/lintel/ssh-controller")
    commands = parser.add_subparsers(dest="operation", required=True)
    aliases = commands.add_parser("aliases", help="List literal Host entries without executing SSH config")
    aliases.add_argument("--config", type=Path, default=Path.home() / ".ssh/config")
    request = commands.add_parser("request", help="Read an allowed runner request from stdin")
    request.add_argument("alias")
    execute = commands.add_parser("execute", help="Explicitly submit one approved remote plan; repeats query only")
    execute.add_argument("alias"); execute.add_argument("--plan-id", required=True); execute.add_argument("--approval", required=True)
    execute.add_argument("--ask-archive-passphrase", action="store_true", help="Read the rebuild archive passphrase without echoing it; never put the value in argv")
    reconnect = commands.add_parser("reconnect", help="Query an existing job by its original plan ID")
    reconnect.add_argument("alias"); reconnect.add_argument("--plan-id", required=True)
    args = parser.parse_args()
    try:
        controller = Controller(args.state_dir)
        if args.operation == "aliases":
            response = {"ok": True, "data": list_aliases(args.config)}
        elif args.operation == "request":
            raw = sys.stdin.buffer.read(MAX_JSON + 1)
            if len(raw) > MAX_JSON:
                raise ControllerError("request_too_large", "The stdin request exceeds the transport budget.")
            try:
                payload = json.loads(raw)
            except (ValueError, UnicodeError):
                raise ControllerError("invalid_json", "stdin must contain one JSON request.") from None
            response = controller.request(args.alias, payload)
        elif args.operation == "execute":
            archive_passphrase = None
            if args.ask_archive_passphrase:
                try:
                    with warnings.catch_warnings():
                        # getpass must not fall back to an input mode that can echo secrets.
                        warnings.simplefilter("error", getpass.GetPassWarning)
                        archive_passphrase = getpass.getpass("Archive passphrase (at least 12 characters): ")
                except (getpass.GetPassWarning, EOFError, KeyboardInterrupt):
                    raise ControllerError("archive_passphrase_input_unavailable", "A private interactive terminal is required to read the archive passphrase; no request was submitted.") from None
            response = controller.execute(args.alias, args.plan_id, args.approval, archive_passphrase=archive_passphrase)
        else:
            response = controller.reconnect(args.alias, args.plan_id)
    except ControllerError as error:
        response = {"ok": False, "error": {"code": error.code, "message": error.message}}
    except OSError:
        response = {"ok": False, "error": {"code": "local_state_unavailable", "message": "The local task state could not be read or durably saved; inspect it before any retry."}}
    print(json.dumps(response, ensure_ascii=False))
    return 0 if response["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
