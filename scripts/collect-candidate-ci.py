#!/usr/bin/env python3
"""Read authenticated exact-head canonical CI, then run the production gate.

Uses the installed official gh client for API and artifact downloads; never
handles redirect Authorization headers or accepts arbitrary download URLs.
"""
import argparse
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]
REPO = "IndelibleVivi/lintel-cc"
spec = importlib.util.spec_from_file_location("candidate_gate", ROOT / "scripts/prepare-candidate.py")
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


def api(endpoint):
    result = subprocess.run(["gh", "api", "--hostname", "github.com", f"repos/{REPO}/{endpoint}"],
                            capture_output=True, text=True, timeout=60)
    gate.require(result.returncode == 0 and len(result.stdout) <= 8 * 1024**2, "authenticated GitHub API read failed or exceeded budget")
    return gate.unique_json(result.stdout)


def select_run(runs, revision):
    selected = [run for run in runs if run.get("head_sha") == revision and run.get("event") == "push"
                and run.get("path") == ".github/workflows/verify.yml"]
    gate.require(len(selected) <= 1, "ambiguous canonical push runs for candidate SHA")
    return selected[0] if selected else None


def expected_groups(revision):
    ci = gate.ci_module()
    plan = ci.build_plan("workflow_dispatch", base=None, head=revision, checkout_head=revision)
    prefixes = {"frontend": "frontend", "native": "native rust", "heavy": "heavy work", "browser": "browser/UI", "runner": "runner/CLI"}
    groups = {"plan": ["plan"], "control": ["control (self-tests)"], "vm": ["linux vm"]}
    for group, prefix in prefixes.items():
        groups[group] = [f"{prefix} ({platform})" for platform in plan.job(group).platforms]
    return groups


def group_results(jobs, revision, run_id):
    groups = expected_groups(revision)
    expected = {"aggregate", *(name for names in groups.values() for name in names)}
    names = [job.get("name") for job in jobs]
    gate.require(len(names) == len(set(names)) and set(names) == expected, "canonical workflow jobs are missing, duplicate or unexpected")
    for job in jobs:
        gate.require(job.get("run_id") == run_id and job.get("head_sha") == revision,
                     "GitHub job belongs to another run or source")
        gate.require(job.get("status") == "completed" and job.get("conclusion") == "success",
                     f"canonical job did not pass: {job.get('name')}")
    # Every member is individually checked above; no missing matrix sibling
    # or skipped check can be hidden behind a synthetic aggregate success.
    by_name = {job["name"]: job for job in jobs}
    return {group: {"result": by_name[names[0]]["conclusion"]} for group, names in groups.items()}


def collect(receipt_path, revision, out, wait_seconds):
    gate.require(re.fullmatch(r"[a-f0-9]{40}", revision), "exact lowercase candidate SHA required")
    gate.require(0 <= wait_seconds <= 3600, "CI wait must be bounded to one hour")
    gate.require(not out.exists() and not out.is_symlink(), "CI gate output directory must be absent")
    receipt = gate.json_read(receipt_path)
    gate.require(receipt.get("source_revision") == revision, "candidate receipt revision mismatch")
    gate.clean_revision(ROOT, revision)
    out.mkdir()
    deadline, run = time.monotonic() + wait_seconds, None
    while True:
        if run is None:
            listing = api(f"actions/workflows/verify.yml/runs?event=push&head_sha={revision}&per_page=100")
            gate.require(listing.get("total_count", 0) <= 100, "too many exact-head workflow runs")
            run = select_run(listing.get("workflow_runs", []), revision)
        else:
            run = api(f"actions/runs/{int(run['id'])}")
        if run:
            gate.require(select_run([run], revision) is not None, "canonical run identity changed")
            if run.get("status") == "completed":
                gate.publish(out / "github-run.json", run)
                gate.require(run.get("conclusion") == "success", "exact-head canonical CI did not pass")
                break
        gate.require(time.monotonic() < deadline, "canonical exact-head CI missing or did not complete within bounded wait")
        print("Waiting for exact-head canonical push CI", flush=True)
        time.sleep(min(20, max(0, deadline - time.monotonic())))
    run_id, attempt = int(run["id"]), int(run["run_attempt"])
    jobs = api(f"actions/runs/{run_id}/attempts/{attempt}/jobs?per_page=100")
    gate.require(jobs.get("total_count") == len(jobs.get("jobs", [])) <= 100, "incomplete canonical job listing")
    gate.publish(out / "github-jobs.json", jobs)
    results = group_results(jobs["jobs"], revision, run_id)
    artifacts = api(f"actions/runs/{run_id}/artifacts?per_page=100")
    gate.require(artifacts.get("total_count") == len(artifacts.get("artifacts", [])) <= 100, "incomplete canonical artifact listing")
    selected = [item for item in artifacts["artifacts"] if item.get("name") == "ci-plan" or item.get("name", "").startswith("evidence-")]
    gate.require(len([item for item in selected if item["name"] == "ci-plan"]) == 1, "canonical plan artifact missing or duplicated")
    gate.require(all(not item.get("expired") and item.get("workflow_run", {}).get("head_sha") == revision for item in selected), "canonical artifact expired or source differs")
    gate.publish(out / "github-artifacts.json", artifacts)
    for selection, directory in [("--name", "plan"), ("--pattern", "evidence")]:
        destination = out / directory
        destination.mkdir()
        subprocess.run(["gh", "run", "download", str(run_id), "--repo", REPO, selection,
                        "ci-plan" if directory == "plan" else "evidence-*", "--dir", str(destination)], check=True, timeout=300)
    gate.publish(out / "results.json", results)
    # Invoke the real production command, with the exact clean candidate source
    # owning both package verification and canonical required-check policy.
    subprocess.run(["python3", ROOT / "scripts/prepare-candidate.py", "verify", "--receipt", receipt_path,
                    "--ci-plan", out / "plan/plan.json", "--ci-results", out / "results.json", "--ci-evidence", out / "evidence",
                    "--out", out / "candidate-verified.json"], check=True, timeout=300)
    gate.publish(out / "provenance.json", {"schema": "lintel.canonical-ci-observation/1", "source_revision": revision,
                 "repository": REPO, "workflow_run_id": run_id, "run_attempt": attempt, "event": "push",
                 "metadata_transport": "authenticated GitHub API through official gh client",
                 "artifact_transport": "official gh run download, no hand-written authorization redirects",
                 "scope": "exact-head production candidate consistency gate; no release authorization",
                 "gate_result": gate.digest(out / "candidate-verified.json")})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--receipt", required=True, type=Path)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--out", required=True, type=Path)
    parser.add_argument("--wait-seconds", type=int, default=2400)
    args = parser.parse_args()
    collect(args.receipt.absolute(), args.revision, args.out.absolute(), args.wait_seconds)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError, subprocess.SubprocessError) as error:
        raise SystemExit(f"candidate canonical CI gate rejected: {error}")
