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
    "service_inspect": {"environment_id": str, "manager": str, "unit": str},
    "plan_service_quiesce": {"environment_id": str, "manager": str, "unit": str},
    "plan_service_resume": {"job_id": str},
    "drift": {"environment_id": str}, "export_support": {},
}
OPTIONAL_FIELDS = {
    "inspect": {"trusted_devices": str},
    "plan_policy": {"trusted_devices": str, "release_settings": list, "custom_settings": dict,
                    "custom_values": dict},
}
REQUEST_FIELDS.update({
    'residue_inventory': {'environment_id': str}, 'identity_inspect': {'environment_id': str},
    'snapshot_save': {'environment_id': str}, 'snapshot_list': {'environment_id': str},
    'snapshot_compare': {'environment_id': str, 'old_snapshot_id': str, 'new_snapshot_id': str},
    'snapshot_accept': {'environment_id': str, 'old_snapshot_id': str, 'new_snapshot_id': str, 'selected_changes': list},
    'residue_archive_inspect': {'archive_passphrase': str},
    'plan_residue_restore': {'environment_id': str, 'archive_passphrase': str, 'selected_paths': list},
    'env_compare': {'environment_id': str, 'other_environment_id': str},
    'config_inspect': {'environment_id': str}, 'config_read': {'environment_id': str, 'config_path': str},
    'plan_config_edit': {'environment_id': str, 'config_path': str, 'document': str, 'expected_digest': str},
    'plan_config_apply': {'environment_id': str, 'source_environment_id': str, 'selected_keys': list},
    'config_restore_inspect': {'environment_id': str, 'job_id': str},
    'plan_config_restore': {'environment_id': str, 'job_id': str, 'selected_keys': list},
    'plan_identity_reset': {'environment_id': str, 'writers_confirmed_stopped': bool},
    'plan_client_workflow': {'environment_id': str, 'writers_confirmed_stopped': bool},
    'plan_cleanup': {'environment_id': str, 'recipe': str, 'writers_confirmed_stopped': bool, 'official_logout': bool},
    'accept_drift': {'environment_id': str},
})
OPTIONAL_FIELDS.update({
    'residue_inventory': {'extra_roots': list, 'expected_digest': str, 'offset': int, 'limit': int, 'category': str, 'class': str, 'project': str, 'modified_after': int, 'since': int, 'until': int, 'min_bytes': int, 'max_bytes': int},
    'identity_inspect': {'reveal': bool}, 'snapshot_save': {'project_cwd': str},
    'env_compare': {'project_cwd': str, 'other_project_cwd': str}, 'config_inspect': {'project_cwd': str},
    'plan_config_apply': {'config_path': str}, 'plan_identity_reset': {'identity_backup_paths': list},
    'plan_client_workflow': {'policy': dict, 'residue_paths': list, 'archive_before_clear': bool, 'reset_identity': bool, 'identity_backup_paths': list, 'extra_roots': list, 'delete_work': bool},
    'plan_cleanup': {'categories': list, 'activate': dict, 'residue_paths': list, 'archive_before_clear': bool, 'archive_only': bool, 'extra_roots': list, 'delete_work': bool},
    'residue_archive_inspect': {'job_id': str, 'archive_path': str},
    'plan_residue_restore': {'job_id': str, 'archive_path': str},
    'drift': {'lightweight': bool},
    'accept_drift': {'selected_keys': list},
})
OPTIONAL_FIELDS['plan_policy']['config_path'] = str

# The v5 recognized env keys, in the core policy catalog order. Kept as a literal
# set here because this controller never reads the runner's home or state.
POLICY_KEYS = {"DISABLE_TELEMETRY", "DISABLE_ERROR_REPORTING", "DISABLE_FEEDBACK_COMMAND",
               "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY", "DO_NOT_TRACK", "DISABLE_GROWTHBOOK",
               "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "CLAUDE_CODE_ENABLE_TELEMETRY",
               "OTEL_LOG_USER_PROMPTS", "OTEL_LOG_ASSISTANT_RESPONSES",
               "OTEL_LOG_TOOL_CONTENT", "OTEL_LOG_TOOL_DETAILS",
               "OTEL_LOG_RAW_API_BODIES", "OTEL_LOG_MANAGED_SETTINGS",
               "CLAUDE_CODE_OTEL_CONTENT_MAX_LENGTH", "OTEL_LOGS_EXPORTER",
               "OTEL_TRACES_EXPORTER", "OTEL_METRICS_EXPORTER",
               "OTEL_EXPORTER_OTLP_ENDPOINT", "OTEL_EXPORTER_OTLP_HEADERS",
               "OTEL_METRICS_INCLUDE_ACCOUNT_UUID", "OTEL_METRICS_INCLUDE_SESSION_ID",
               "OTEL_METRICS_INCLUDE_VERSION", "OTEL_METRICS_INCLUDE_ENTRYPOINT",
               "OTEL_METRICS_INCLUDE_REPOSITORY",
               "CLAUDE_CODE_ENABLE_FEEDBACK_SURVEY_FOR_OTEL", "CLAUDE_CODE_DISABLE_WEB_FETCH"}
# The verified privacy switches keep their historical action set; every other
# recognized key admits the full five. The masked key cannot carry a value, so
# `set`/`enable` are never admissible for it.
POLICY_SWITCH_ACTIONS = {"keep", "disable", "remove"}
POLICY_ACTIONS = {"keep", "disable", "enable", "remove", "set"}
POLICY_SWITCHES = {"DISABLE_TELEMETRY", "DISABLE_ERROR_REPORTING",
                   "DISABLE_FEEDBACK_COMMAND", "CLAUDE_CODE_DISABLE_FEEDBACK_SURVEY",
                   "DO_NOT_TRACK", "DISABLE_GROWTHBOOK",
                   "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"}
MASKED_KEY = "OTEL_EXPORTER_OTLP_HEADERS"
REMOTE_KEYS = {"DISABLE_TELEMETRY", "DO_NOT_TRACK", "DISABLE_GROWTHBOOK",
               "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"}


def policy_actions(key: str) -> set:
    """The admissible actions for one recognized key, or an empty set for a key
    the v4 catalog does not model. The core re-derives this, including the
    per-field value syntax, so this stays a syntax-level guard."""
    if key not in POLICY_KEYS:
        return set()
    if key == MASKED_KEY:
        return POLICY_SWITCH_ACTIONS
    return POLICY_ACTIONS


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
    optional = OPTIONAL_FIELDS.get(payload["command"], {})
    if not {"command", *fields} <= set(payload) or not set(payload) <= {"command", *fields, *optional}:
        raise ControllerError("invalid_request", "The request fields do not match this command's schema.")
    for key, expected in (fields | {k: v for k, v in optional.items() if k in payload}).items():
        if type(payload[key]) is not expected:
            raise ControllerError("invalid_request", "A request field has the wrong type.")
        if expected is str and (not payload[key] or len(payload[key]) > (1024 * 1024 if key == "document" else 4096)):
            raise ControllerError("invalid_request", "A request field has an invalid length.")
    for key in ('residue_paths', 'identity_backup_paths', 'extra_roots', 'selected_paths'):
        if key in payload and (len(payload[key]) > (8 if key == 'extra_roots' else 1000) or len(set(map(str,payload[key]))) != len(payload[key]) or not all(type(p) is str and p.startswith('/') and len(p)<=4096 and not any(ord(c)<32 for c in p) and '..' not in p.split('/') for p in payload[key])):
            raise ControllerError('invalid_request', 'Explicit paths must be unique, bounded absolute paths.')
    for key in ('selected_keys', 'selected_changes'):
        if key in payload and (not payload[key] or len(payload[key])>4096 or not all(type(k) is str and k and len(k)<=4096 for k in payload[key]) or len(set(payload[key]))!=len(payload[key])):
            raise ControllerError('invalid_request', 'Choose a finite nonempty set of exact configuration keys or changes.')
    if payload['command'] in {'residue_archive_inspect', 'plan_residue_restore'}:
        if ('job_id' in payload) == ('archive_path' in payload) or len(payload['archive_passphrase']) < 12:
            raise ControllerError('invalid_request', 'Choose exactly one original job or archive path and a passphrase of at least 12 characters.')
        if payload['command'] == 'plan_residue_restore' and not payload['selected_paths']:
            raise ControllerError('invalid_request', 'Choose at least one exact original archive entry.')
    if payload['command'] == 'residue_inventory':
        for key in ('offset', 'modified_after', 'since', 'until', 'min_bytes', 'max_bytes'):
            if key in payload and not 0 <= payload[key] <= 9007199254740991:
                raise ControllerError('invalid_request', 'Inventory numbers must be finite nonnegative integers.')
        if 'limit' in payload and not 1 <= payload['limit'] <= 200:
            raise ControllerError('invalid_request', 'Inventory pages contain between 1 and 200 rows.')
        if payload.get('offset', 0) > 0 and 'expected_digest' not in payload:
            raise ControllerError('invalid_request', 'Later pages need the original metadata digest.')
        if 'class' in payload and payload['class'] not in {'work', 'residue', 'identity', 'config', 'unclassified'}:
            raise ControllerError('invalid_request', 'Unknown inventory class.')
    if 'expected_digest' in payload and not re.fullmatch(r'[a-f0-9]{64}', payload['expected_digest']):
        raise ControllerError('invalid_request', 'Use the original lowercase metadata digest.')
    for key in ('config_path', 'project_cwd', 'other_project_cwd', 'archive_path'):
        if key in payload and (not payload[key].startswith('/') or '..' in payload[key].split('/') or any(ord(c)<32 for c in payload[key])):
            raise ControllerError('invalid_request', 'Choose an exact absolute target-host path.')
    if payload['command'] in {'plan_identity_reset','plan_client_workflow','plan_cleanup'} and payload['writers_confirmed_stopped'] is not True:
        raise ControllerError('invalid_request', 'Confirm that target writers are stopped before planning.')
    if payload['command']=='plan_client_workflow' and 'policy' in payload:
        validate_request(dict(payload['policy'], command='plan_policy', environment_id=payload['environment_id']))
    if payload['command']=='plan_cleanup':
        if payload['recipe'] not in {'repair_login','reset_client','retire','clear_residue'}:
            raise ControllerError('invalid_request', 'Unsupported cleanup recipe.')
        if payload['recipe']=='clear_residue' and (not payload.get('residue_paths') or payload['official_logout']):
            raise ControllerError('invalid_request', 'Residue clear needs exact paths and never logs out.')
    if payload.get("trusted_devices", "unknown") not in {"unknown", "required", "not_required"}:
        raise ControllerError("invalid_request", "The Trusted Devices condition is invalid.")
    if payload["command"] == "plan_policy" and payload["preset"] not in {"preserve", "reduce", "custom"}:
        raise ControllerError("invalid_request", "The policy preset is not supported.")
    if "release_settings" in payload and not all(type(v) is str and v in REMOTE_KEYS for v in payload["release_settings"]):
        raise ControllerError("invalid_request", "Only known Remote Control fields can be released.")
    if "custom_settings" in payload:
        if payload["preset"] != "custom" or not all(
                type(v) is str and v in policy_actions(k) for k, v in payload["custom_settings"].items()):
            raise ControllerError("invalid_request", "Custom choices must use known policy keys and their documented actions.")
    # `custom_values` is the only value-carrying channel and needs its `set`
    # action; the masked key is never writable. The runner re-derives both rules.
    choices = payload.get("custom_settings") or {}
    if "custom_values" in payload:
        values = payload["custom_values"]
        if payload["preset"] != "custom" or not isinstance(values, dict) or not values or not all(
                type(v) is str and v and k in POLICY_KEYS and k != MASKED_KEY
                and choices.get(k) == "set" for k, v in values.items()):
            raise ControllerError("invalid_request", "Custom values need a known key, a nonempty string and the set action.")
    if any(v == "set" and k not in (payload.get("custom_values") or {})
           for k, v in choices.items()):
        raise ControllerError("invalid_request", "A set action needs its value in custom_values.")
    if payload.get("preset") == "custom" and payload.get("release_settings"):
        raise ControllerError("invalid_request", "Use explicit remove choices in a custom policy.")
    if payload["command"] == "plan_reset":
        if payload["recipe"] != "rebuild" or not all(type(v) is str and len(v) <= 128 for v in payload["categories"]):
            raise ControllerError("invalid_request", "The rebuild recipe/categories are invalid.")
    if payload["command"] in {"service_inspect", "plan_service_quiesce"}:
        if payload["manager"] not in {"user", "system"} or len(payload["unit"]) > 240 or payload["unit"].endswith("@.service") or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.@-]*\.service", payload["unit"]):
            raise ControllerError("invalid_request", "Choose an exact service unit and user/system manager; no shell or patterns.")
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
