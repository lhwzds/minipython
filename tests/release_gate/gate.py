"""MiniPython release gate: `tests/run.sh --release-gate` (see RELEASING.md).

Runs every automated release check against the committed tree and writes
reports/release-gate/report.md and report.json. It stops at the first
failing step unless --keep-going is given; either way it exits nonzero when
any step fails. The manual checklist (step f) is written into the report with
blanks to fill in; a release needs both a passing gate and a completed
checklist.

Steps:
  preflight  clean tree, pinned CPython, grammar source checkout, versions
  a          cargo fmt, then the full cargo test (all targets, release profile,
             MINIPYTHON_REQUIRE_CPYTHON_SRC=1); any SKIPPED line or
             unexpected ignored test fails
  b          pipeline unit tests and the 1024-case discovery sweep, which must
             match tests/release_gate/discovery-baseline.json exactly
  c          oracle corpora (tests/oracles/corpora.toml), 100% identical apart
             from documented exclusions
  conformance  the layer-zero conformance corpus: the comparison against the
             recorded CPython 3.14.6 observations, the conformance status block
             of README.md, the oracle check of tests/conformance/cases, and the
             unit tests of the corpus
  d          leak scan of tracked files and commit messages since the last tag
  licensing  licensing: SPDX headers, NOTICE, the licences of the Cargo
             dependencies, and the provenance of the MIT-only files
  e          packaging smoke: release archive, npm pack + install, Homebrew
             formula install and test from a throwaway local tap
  f          manual checklist (written into the report)
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]

# CPython 3.14.6 files the suites and the oracle extractors read from the
# pinned standard library (tests/README.md, "CPython Oracle Files"). Official
# CPython Docker images strip Lib/test; such hosts point MINIPYTHON_CPYTHON_LIB
# at the Lib directory of the 3.14.6 source tree instead.
ORACLE_LIB_FILES = [
    "test/test_re.py",
    "test/re_tests.py",
    "test/test_csv.py",
    "test/test_memoryio.py",
    "test/test_compile.py",
    "test/test_ast/snippets.py",
    "test/test_ast/data/ast_repr.txt",
    "test/mathdata/formatfloat_testcases.txt",
    "test/mathdata/floating_points.txt",
]
# Files compared between MINIPYTHON_CPYTHON_LIB and the oracle's own
# standard library, so an override is the same CPython release.
LIB_IDENTITY_FILES = ["_strptime.py", "datetime.py", "re/_parser.py", "csv.py", "ast.py"]
REPORT_DIR = REPO / "reports" / "release-gate"
BASELINE = REPO / "tests" / "release_gate" / "discovery-baseline.json"

# Tests that `cargo test` may report as ignored. Each needs far more memory
# than a gate host should spend and runs only on request.
ALLOWED_IGNORED_TESTS = {
    "million_level_structures_are_freed_without_native_recursion":
        "needs about 2 GB per structure; run with `cargo test --test runtime -- --ignored million_level`",
}

NPM_TARGETS = {
    ("Darwin", "arm64"): "aarch64-apple-darwin",
    ("Darwin", "x86_64"): "x86_64-apple-darwin",
    ("Linux", "x86_64"): "x86_64-unknown-linux-musl",
    ("Linux", "aarch64"): "aarch64-unknown-linux-musl",
    ("Linux", "arm64"): "aarch64-unknown-linux-musl",
}

MANUAL_CHECKLIST = [
    (
        "Real-program corpus rerun",
        "Rerun the maintainers' corpus of real agent-skill programs on this candidate's mnpy and "
        "compare it with the previous release's run: no case that passed before may fail now. "
        "For the first release, record the run as the baseline.",
        ["Candidate binary SHA-256:", "Previous release run (date/version, or 'baseline'):",
         "Cases run / passed / failed:", "Regressions vs previous release (must be none):", "Run by / date:"],
    ),
    (
        "Dual review of the release diff",
        "Two independent reviews of `git diff <previous tag>..<candidate>` (for the first release: the "
        "whole tree) covering bugs, sandbox escapes, leaks and provenance. Verify every finding; fix every confirmed blocker before tagging.",
        ["Diff range:", "First review (reviewer, file):", "Second review (reviewer, file):", "Findings verified (count):",
         "Blockers found / fixed (commits):", "Reviewed by / date:"],
    ),
    (
        "Linux gate",
        "Run the same gate on Linux in Docker (RELEASING.md, Linux gate) for this candidate SHA and "
        "attach its report.",
        ["Docker host / architecture:", "Candidate SHA in the report:", "Result (all steps PASS?):",
         "Report path:", "Run by / date:"],
    ),
]


class StepFailed(Exception):
    pass


class Gate:
    def __init__(self, args: argparse.Namespace) -> None:
        self.args = args
        self.cpython = args.cpython
        self.target_dir = Path(os.environ.get("CARGO_TARGET_DIR", "/tmp/minipython-target"))
        self.steps: list[dict] = []
        self.started = time.monotonic()
        self.meta: dict = {}
        self.logs = REPORT_DIR / "logs"

    # -- helpers ---------------------------------------------------------

    def run(self, command: list[str], log: str, env: dict | None = None, cwd: Path = REPO,
            timeout: float = 3600) -> subprocess.CompletedProcess:
        """Run a command with stdin closed, its output saved to logs/<log>."""
        full_env = {**os.environ, **(env or {})}
        path = self.logs / log
        print(f"    $ {' '.join(command)}  (log: {path.relative_to(REPO)})", flush=True)
        with path.open("ab") as handle:
            handle.write(f"\n$ {' '.join(command)}\n".encode())
            handle.flush()
            try:
                completed = subprocess.run(command, cwd=cwd, env=full_env, stdin=subprocess.DEVNULL,
                                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
            except subprocess.TimeoutExpired as error:
                handle.write(error.stdout or b"")
                raise StepFailed(f"`{' '.join(command)}` timed out after {timeout}s") from error
            handle.write(completed.stdout)
        completed.stdout = completed.stdout.decode("utf-8", errors="replace")
        return completed

    def check(self, command: list[str], log: str, **kwargs) -> str:
        completed = self.run(command, log, **kwargs)
        if completed.returncode != 0:
            tail = "\n".join(completed.stdout.splitlines()[-25:])
            raise StepFailed(f"`{' '.join(command)}` exited {completed.returncode}:\n{tail}")
        return completed.stdout

    def step(self, key: str, title: str, function) -> bool:
        print(f"==> [{key}] {title}", flush=True)
        record = {"key": key, "title": title, "status": "running", "details": {}, "failures": []}
        self.steps.append(record)
        started = time.monotonic()
        try:
            function(record)
            if record["failures"]:
                raise StepFailed("; ".join(record["failures"][:3]))
            record["status"] = "PASS"
        except StepFailed as error:
            record["status"] = "FAIL"
            if not record["failures"]:
                record["failures"].append(str(error))
        except Exception as error:  # a gate bug must not look like a pass
            record["status"] = "FAIL"
            record["failures"].append(f"gate error: {type(error).__name__}: {error}")
        record["seconds"] = round(time.monotonic() - started, 1)
        print(f"<== [{key}] {record['status']} ({record['seconds']}s)", flush=True)
        for failure in record["failures"]:
            print("    FAIL: " + failure.replace("\n", "\n          "), flush=True)
        return record["status"] == "PASS"

    # -- preflight -------------------------------------------------------

    def preflight(self, record: dict) -> None:
        details = record["details"]
        head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPO, capture_output=True, text=True, check=True)
        details["revision"] = head.stdout.strip()
        status = subprocess.run(["git", "status", "--porcelain"], cwd=REPO, capture_output=True, text=True, check=True)
        dirty = [line for line in status.stdout.splitlines() if line.strip()]
        details["dirty_files"] = dirty[:50]
        if dirty and not self.args.allow_dirty:
            record["failures"].append(
                f"the working tree has {len(dirty)} uncommitted change(s); a release candidate must be committed "
                "(--allow-dirty is for development runs only)"
            )
        details["allow_dirty"] = self.args.allow_dirty
        describe = subprocess.run(["git", "describe", "--tags", "--abbrev=0", "--match", "v*"], cwd=REPO,
                                  capture_output=True, text=True)
        details["previous_tag"] = describe.stdout.strip() or None

        version = subprocess.run([self.cpython, "-I", "-c", "import platform, sys; print(platform.python_version()); print(sys.executable)"],
                                 capture_output=True, text=True, check=True, stdin=subprocess.DEVNULL).stdout.split("\n")
        details["cpython"] = {"path": self.cpython, "executable": version[1], "version": version[0]}
        if version[0] != self.args.require_version:
            record["failures"].append(f"CPython oracle is {version[0]}, the pin is {self.args.require_version}")

        source = os.environ.get("MINIPYTHON_CPYTHON_SRC")
        details["cpython_src"] = source
        if not source:
            record["failures"].append("MINIPYTHON_CPYTHON_SRC is not set; the gate needs a CPython 3.14.6 source checkout")
        else:
            gram = Path(source) / "Grammar" / "python.gram"
            patchlevel = Path(source) / "Include" / "patchlevel.h"
            if not gram.is_file():
                record["failures"].append(f"{gram} is missing")
            if patchlevel.is_file():
                match = re.search(r'#define PY_VERSION\s+"([^"]+)"', patchlevel.read_text())
                details["cpython_src_version"] = match.group(1) if match else None
                if not match or match.group(1) != self.args.require_version:
                    record["failures"].append(
                        f"MINIPYTHON_CPYTHON_SRC is CPython {match.group(1) if match else '?'}, not {self.args.require_version}"
                    )
            else:
                record["failures"].append(f"{patchlevel} is missing; cannot confirm the checkout's version")

        self.check_oracle_lib(record)

        versions = self.versions()
        details["versions"] = versions
        if len(set(versions.values())) != 1:
            record["failures"].append(f"versions disagree: {versions}")
        self.meta["version"] = versions.get("Cargo.toml")
        changelog = (REPO / "CHANGELOG.md")
        if not changelog.is_file() or not re.search(rf"^## \[?{re.escape(versions.get('Cargo.toml', ''))}\]?", changelog.read_text(), re.M):
            record["failures"].append(f"CHANGELOG.md has no entry for {versions.get('Cargo.toml')}")

        tools = {}
        needed = ["cargo", "rustc", "node", "npm", "tar", "git"]
        if platform.system() == "Darwin":
            needed.append("brew")
        for tool in needed:
            tools[tool] = shutil.which(tool)
            if not tools[tool]:
                record["failures"].append(f"{tool} is not on PATH")
        details["tools"] = tools
        details["host"] = f"{platform.system()} {platform.machine()}"
        self.meta.update({"revision": details["revision"], "host": details["host"]})

    def versions(self) -> dict[str, str]:
        cargo = tomllib.loads((REPO / "Cargo.toml").read_text())["package"]["version"]
        npm = json.loads((REPO / "npm" / "package.json").read_text())["version"]
        pyproject = tomllib.loads((REPO / "python" / "pyproject.toml").read_text())["project"]["version"]
        stdlib = (REPO / "src" / "stdlib.rs").read_text()
        parts = [re.search(rf"MINIPYTHON_VERSION_{name}: i64 = (\d+);", stdlib) for name in ("MAJOR", "MINOR", "MICRO")]
        runtime = ".".join(part.group(1) for part in parts) if all(parts) else "?"
        return {"Cargo.toml": cargo, "npm/package.json": npm, "python/pyproject.toml": pyproject,
                "src/stdlib.rs (sys.implementation.version)": runtime}

    # -- step a ----------------------------------------------------------

    def cargo(self, record: dict) -> None:
        details = record["details"]
        self.check(["cargo", "fmt", "--check"], "a-cargo-fmt.log")
        completed = self.run(
            ["cargo", "test", "--release", "--all-targets", "--locked", "--no-fail-fast"],
            "a-cargo-test.log",
            env={"MINIPYTHON_REQUIRE_CPYTHON_SRC": "1", "MINIPYTHON_CPYTHON": self.cpython,
                 "RUST_MIN_STACK": os.environ.get("RUST_MIN_STACK", "8388608")},
            timeout=5400,
        )
        output = completed.stdout
        totals = {"passed": 0, "failed": 0, "ignored": 0, "measured": 0, "filtered out": 0}
        binaries = 0
        for match in re.finditer(r"^test result: \w+\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out", output, re.M):
            binaries += 1
            for key, value in zip(totals, match.groups()):
                totals[key] += int(value)
        details["test_binaries"] = binaries
        details.update(totals)
        failed = re.findall(r"^test (\S+) \.\.\. FAILED$", output, re.M)
        details["failed_tests"] = failed
        ignored = re.findall(r"^test (\S+) \.\.\. ignored(?:, .*)?$", output, re.M)
        details["ignored_tests"] = ignored
        skipped = [line.strip() for line in output.splitlines() if "SKIPPED" in line]
        details["skipped_lines"] = skipped
        for name in failed:
            record["failures"].append(f"test failed: {name}")
        if skipped:
            record["failures"].append(f"{len(skipped)} SKIPPED line(s): skipped evidence is not a pass: {skipped[:3]}")
        unexpected = [name for name in ignored if name.rsplit("::", 1)[-1] not in ALLOWED_IGNORED_TESTS]
        if unexpected:
            record["failures"].append(f"unexpected ignored tests: {unexpected}")
        if completed.returncode != 0 and not failed:
            tail = "\n".join(output.splitlines()[-25:])
            record["failures"].append(f"cargo test exited {completed.returncode}:\n{tail}")
        if binaries == 0:
            record["failures"].append("no test results were reported")
        details["allowed_ignored"] = ALLOWED_IGNORED_TESTS

    # -- step b ----------------------------------------------------------

    def pipeline(self, record: dict) -> None:
        details = record["details"]
        output = self.check([self.cpython, "tests/test_pipeline.py"], "b-test-pipeline.log")
        details["pipeline_unit_tests"] = (re.findall(r"^Ran (\d+) tests?", output, re.M) or ["?"])[-1]
        baseline = json.loads(BASELINE.read_text())
        mnpy = self.release_binary()
        out = REPORT_DIR / "discovery" / "pipeline"
        completed = self.run(
            [self.cpython, "tests/pipeline.py", "--cpython", self.cpython, "--require-version", self.args.require_version,
             "--minipython", str(mnpy), "--corpus", baseline["corpus"], "--out", str(out),
             "--repro-dir", str(REPORT_DIR / "discovery" / "repros"),
             "--generated-cases", str(baseline["generated_cases"]), "--seed", str(baseline["seed"]),
             "--shrink", "--fail-priority", "must_fix,should_fix", "--module", "*"],
            "b-discovery.log",
        )
        report = json.loads(out.with_suffix(".json").read_text()) if out.with_suffix(".json").exists() else {}
        summary = dict(sorted(report.get("summary", {}).items()))
        triage = dict(sorted(report.get("triage", {}).items()))
        details.update({"seed": baseline["seed"], "generated_cases": baseline["generated_cases"], "summary": summary,
                        "triage": triage, "baseline_summary": baseline["summary"], "baseline_triage": baseline["triage"],
                        "report": str(out.with_suffix(".md").relative_to(REPO))})
        if completed.returncode != 0:
            tail = "\n".join(completed.stdout.splitlines()[-10:])
            record["failures"].append(f"discovery sweep failed (open must_fix/should_fix root causes or crashes):\n{tail}")
        if summary != baseline["summary"] or triage != baseline["triage"]:
            record["failures"].append(
                f"discovery counts differ from the baseline: got {summary} / {triage}, "
                f"baseline {baseline['summary']} / {baseline['triage']}"
            )
            if self.args.update_discovery_baseline and summary:
                baseline["summary"], baseline["triage"] = summary, triage
                BASELINE.write_text(json.dumps(baseline, indent=2) + "\n")
                record["failures"].append(
                    "rewrote tests/release_gate/discovery-baseline.json from this run: commit it on its own with the "
                    "reason for every changed count, then rerun the gate"
                )

    def check_oracle_lib(self, record: dict) -> None:
        """The CPython files read as oracle input exist and are 3.14.6's."""
        details = record["details"]
        installed = Path(subprocess.run(
            [self.cpython, "-I", "-c", "import sysconfig; print(sysconfig.get_path('stdlib'))"],
            capture_output=True, text=True, check=True, stdin=subprocess.DEVNULL).stdout.strip())
        override = os.environ.get("MINIPYTHON_CPYTHON_LIB")
        lib = Path(override) if override else installed
        details["cpython_lib"] = str(lib)
        missing = [name for name in ORACLE_LIB_FILES if not (lib / name).is_file()]
        if missing:
            hint = ("" if override else "; the pinned CPython has no Lib/test (official Docker images strip it), "
                    "so set MINIPYTHON_CPYTHON_LIB to the Lib directory of the CPython 3.14.6 source tree")
            record["failures"].append(f"CPython oracle files missing under {lib}: {', '.join(missing)}{hint}")
        if override:
            differing = [name for name in LIB_IDENTITY_FILES
                         if not (lib / name).is_file() or not (installed / name).is_file()
                         or (lib / name).read_bytes() != (installed / name).read_bytes()]
            if differing:
                record["failures"].append(
                    f"MINIPYTHON_CPYTHON_LIB={lib} is not the oracle's CPython release: "
                    f"{', '.join(differing)} differ from {installed}"
                )

    # -- step c ----------------------------------------------------------

    def oracles(self, record: dict) -> None:
        out = REPORT_DIR / "oracles"
        completed = self.run(
            [self.cpython, "tests/oracles/run_oracles.py", "--cpython", self.cpython, "--mnpy", str(self.release_binary()),
             "--out", str(out)],
            "c-oracles.log",
            timeout=3600,
        )
        summary_path = out / "oracles.json"
        if not summary_path.exists():
            raise StepFailed(f"the oracle runner wrote no summary (exit {completed.returncode})")
        payload = json.loads(summary_path.read_text())
        corpora = []
        for corpus in payload["corpora"]:
            corpora.append({key: corpus.get(key) for key in (
                "module", "name", "kind", "seeds", "count", "programs", "scenarios", "matched", "excluded",
                "mismatched", "stale_exclusions", "errors", "passed", "exclusions")})
            if not corpus["passed"]:
                problems = []
                if corpus["mismatched"]:
                    problems.append(f"{corpus['mismatched']} mismatched")
                if corpus["stale_exclusions"]:
                    problems.append(f"stale exclusions {corpus['stale_exclusions']}")
                if corpus["errors"]:
                    problems.append(f"errors {corpus['errors'][:2]}")
                first = corpus["mismatches"][:2]
                examples = "; ".join(f"{m['program']} {m['key']}: cpython={m['cpython'][:160]!r} mnpy={m['mnpy'][:160]!r}" for m in first)
                record["failures"].append(f"{corpus['module']}/{corpus['name']}: {', '.join(problems)}. {examples}")
        record["details"]["corpora"] = corpora
        record["details"]["scenarios"] = sum(c["scenarios"] for c in payload["corpora"])
        record["details"]["report"] = str((out / "oracles.md").relative_to(REPO))

    # -- the conformance step ---------------------------------------------

    def conformance(self, record: dict) -> None:
        """The layer-zero conformance corpus: compare, status, oracle check."""
        details = record["details"]
        mnpy = self.release_binary()
        out = REPORT_DIR / "conformance"
        out.mkdir(parents=True, exist_ok=True)
        report = out / "report.json"
        compared = self.run(
            [self.cpython, "tests/conformance/run.py", "--mnpy", str(mnpy), "--out", str(report)],
            "conformance-compare.log",
        )
        details["report"] = str(report.relative_to(REPO))
        details["comparison_exit"] = compared.returncode
        if compared.returncode != 0:
            record["failures"].append(
                f"the conformance comparison failed (exit {compared.returncode}); the log lists the "
                "cases that differ from the recorded CPython observations"
            )
        checked = self.run(
            [self.cpython, "tests/conformance/status.py", "--report", str(report), "--check"],
            "conformance-status.log",
        )
        details["status_exit"] = checked.returncode
        if checked.returncode == 1:
            record["failures"].append(
                "the conformance status block of README.md is stale; regenerate it with "
                "tests/conformance/status.py --report " + str(report.relative_to(REPO))
            )
        elif checked.returncode != 0:
            record["failures"].append(
                f"the conformance status check failed (exit {checked.returncode}); the log says why"
            )
        # The oracle check writes no oracle or report; the cases it executes are
        # trusted, tracked code. Exit 1 means that the recorded observations
        # differ from the oracle on this host, any other exit means that the check
        # itself did not run.
        oracle = self.run(
            [self.cpython, "tests/conformance/run.py", "--check-oracle", "--cpython", self.cpython],
            "conformance-oracle.log",
        )
        details["oracle_check_exit"] = oracle.returncode
        details["oracle_drift"] = oracle.returncode == 1
        if oracle.returncode == 1:
            record["failures"].append(
                "oracle drift: the recorded CPython observations of tests/conformance/cases differ from "
                "the oracle on this host; regenerate them and commit them, or fix the case that differs "
                "(see tests/conformance/README.md)"
            )
        elif oracle.returncode != 0:
            record["failures"].append(
                f"the oracle check failed (exit {oracle.returncode}); the log says why"
            )
        self.check([self.cpython, "-I", "-m", "unittest", "discover", "-s", "tests/conformance",
                    "-p", "test_*.py"], "conformance-tests.log")

    # -- step d ----------------------------------------------------------

    def leaks(self, record: dict) -> None:
        self.check([self.cpython, "tests/release_gate/test_leak_scan.py"], "d-leak-scan-tests.log")
        out = REPORT_DIR / "leak-scan.json"
        completed = self.run([self.cpython, "tests/release_gate/leak_scan.py", "--json", str(out)], "d-leak-scan.log")
        payload = json.loads(out.read_text()) if out.exists() else {}
        record["details"] = {key: payload.get(key) for key in (
            "files_scanned", "commits_scanned", "commit_base", "findings", "allowed", "unallowed", "unused_allowlist_entries")}
        record["details"]["security_suites_in_step_a"] = [
            "tests/sandbox.rs capability_contract", "tests/sandbox.rs host_capability_lint",
            "tests/sandbox.rs worker_processes", "tests/runtime.rs sandbox_policy and budgets"]
        if completed.returncode != 0:
            for finding in payload.get("unallowed", [])[:20]:
                record["failures"].append(f"{finding['rule']} in {finding['where']} {finding['path']}:{finding['line']}: {finding['match']!r}")
            for entry in payload.get("unused_allowlist_entries", []):
                record["failures"].append(f"unused allowlist entry: {entry}")
            if not record["failures"]:
                record["failures"].append("leak scan failed")

    # -- the licensing step -----------------------------------------------

    def licensing(self, record: dict) -> None:
        """The dual licence: SPDX headers, NOTICE, dependencies, provenance."""
        self.check([self.cpython, "tests/release_gate/test_licensing.py"], "licensing-tests.log")
        out = REPORT_DIR / "licensing.json"
        completed = self.run(
            [self.cpython, "tests/release_gate/licensing.py", "--repo", str(REPO), "--json", str(out)],
            "licensing.log",
        )
        payload = json.loads(out.read_text()) if out.exists() else {}
        report = payload.get("details", {})
        provenance = report.get("provenance", {})
        record["details"].update({
            "tracked_source_files": report.get("tracked_source_files"),
            "reuse_rules": report.get("reuse_rules", []),
            "psf_files": report.get("psf_files", []),
            "mit_only_rust_files": len(report.get("mit_only_rust_files", [])),
            "notice_entries": report.get("notice_entries", []),
            "dependency_offenders": report.get("dependency_offenders", []),
            "provenance": {key: provenance.get(key) for key in ("scanned", "cpython_files")},
        })
        if completed.returncode != 0:
            for failure in payload.get("failures", [])[:40]:
                record["failures"].append(failure)
            if not record["failures"]:
                record["failures"].append("the licensing check failed")

    # -- step e ----------------------------------------------------------

    def release_binary(self) -> Path:
        """The host release `mnpy`, built if a failed earlier step left none."""
        binary = self.target_dir / "release" / "mnpy"
        if not getattr(self, "_built", False):
            self.check(["cargo", "build", "--release", "--locked", "--bin", "mnpy"], "cargo-build.log")
            self._built = True
        return binary

    def packaging(self, record: dict) -> None:
        details = record["details"]
        version = self.meta.get("version") or self.versions()["Cargo.toml"]
        target = NPM_TARGETS.get((platform.system(), platform.machine()))
        if not target:
            raise StepFailed(f"no release target for {platform.system()} {platform.machine()}")
        host = re.search(r"^host: (\S+)$", self.check(["rustc", "-vV"], "e-rustc.log"), re.M).group(1)
        build = ["cargo", "build", "--release", "--locked", "--bin", "mnpy"]
        if host != target:
            build += ["--target", target]
            binary = self.target_dir / target / "release" / "mnpy"
        else:
            binary = self.target_dir / "release" / "mnpy"
        self.check(build, "e-cargo-build.log")
        details.update({"target": target, "rustc_host": host, "binary": str(binary),
                        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest()})

        work = Path(tempfile.mkdtemp(prefix="minipython-gate-"))
        details["workdir"] = str(work)
        dist = work / "dist"
        archive = Path(self.check(["bash", "ci/package-release.sh", target, str(binary), str(dist)], "e-package.log").strip().splitlines()[-1])
        listing = self.check(["tar", "-tzf", str(archive)], "e-package.log").split()
        if sorted(listing) != ["LICENSE", "LICENSE-PSF", "NOTICE", "mnpy"]:
            record["failures"].append(
                f"archive holds {listing}, expected mnpy, LICENSE, LICENSE-PSF and NOTICE"
            )
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        (dist / "checksums.txt").write_text(f"{digest}  {archive.name}\n")
        details["archive"] = {"name": archive.name, "sha256": digest, "bytes": archive.stat().st_size}

        # npm: unit tests, pack, then install the tarball into a throwaway
        # prefix; postinstall reads the local archive and verifies its checksum.
        npm_env = {"npm_config_cache": str(work / "npm-cache"), "npm_config_update_notifier": "false",
                   "npm_config_fund": "false", "npm_config_audit": "false"}
        self.check(["npm", "test", "--prefix", "npm"], "e-npm.log", env=npm_env)
        packed = self.check(["npm", "pack", "--pack-destination", str(work)], "e-npm.log", cwd=REPO / "npm",
                            env={**npm_env, "MINIPYTHON_SKIP_INSTALL": "1"})
        tarball = work / packed.strip().splitlines()[-1]
        details["npm_tarball"] = tarball.name
        if tarball.name != f"minipython-{version}.tgz":
            record["failures"].append(f"npm pack produced {tarball.name}, expected minipython-{version}.tgz")
        prefix = work / "npm-prefix"
        self.check(["npm", "install", "--global", "--prefix", str(prefix), str(tarball)], "e-npm.log",
                   env={**npm_env, "MINIPYTHON_NPM_ARCHIVE_DIR": str(dist)})
        npm_mnpy = prefix / "bin" / "mnpy"
        details["npm"] = self.smoke(npm_mnpy, version, record, "npm")

        # Homebrew (macOS): render the template for the local archive, install
        # it from a throwaway local tap, run its test block, audit it, and
        # remove everything again. The real tap is never touched.
        if platform.system() != "Darwin":
            details["homebrew"] = "not applicable on this host (no Homebrew); covered by the macOS gate"
            if shutil.which("ruby"):
                formula = self.render_formula(version, archive, digest, work)
                self.check(["ruby", "-c", str(formula)], "e-brew.log")
                details["homebrew"] += "; rendered formula passes `ruby -c`"
        else:
            details["homebrew"] = self.homebrew(version, archive, digest, work, record)
        shutil.rmtree(work, ignore_errors=True)

    def smoke(self, mnpy: Path, version: str, record: dict, label: str) -> dict:
        result = {}
        completed = self.run([str(mnpy), "--version"], "e-smoke.log")
        result["--version"] = completed.stdout.strip()
        if completed.returncode != 0 or completed.stdout != f"mnpy {version}\n":
            record["failures"].append(f"{label}: `mnpy --version` printed {completed.stdout!r} (exit {completed.returncode})")
        example = REPO / "examples" / "sandbox" / "argparse_cli.py"
        for args in (["stats", "[3, 1, 4, 1, 5]", "--round", "2"], ["pick", '{"a": 1}', "z", "--missing", "error"]):
            mine = subprocess.run([str(mnpy), str(example), *args], capture_output=True, text=True, stdin=subprocess.DEVNULL)
            theirs = subprocess.run([self.cpython, "-I", "-B", str(example), *args], capture_output=True, text=True,
                                    stdin=subprocess.DEVNULL, env={"PATH": "/usr/bin:/bin"})
            same = (mine.returncode, mine.stdout, mine.stderr) == (theirs.returncode, theirs.stdout, theirs.stderr)
            result[" ".join(["argparse_cli.py", *args])] = {"exit": mine.returncode, "matches_cpython": same}
            if not same:
                record["failures"].append(
                    f"{label}: argparse_cli.py {' '.join(args)} differs from CPython: "
                    f"mnpy=({mine.returncode}, {mine.stdout!r}, {mine.stderr!r}) cpython=({theirs.returncode}, {theirs.stdout!r}, {theirs.stderr!r})"
                )
        result["math"] = self.math_probe(mnpy, record, label)
        return result

    def math_probe(self, mnpy: Path, record: dict, label: str) -> dict:
        """The shipped binary's math: cbrt against CPython with the cargo test's
        exclusions, and the C library functions against the tested build
        (tests/release_gate/math_probe.py)."""
        completed = self.run(
            [self.cpython, "tests/release_gate/math_probe.py", "--cpython", self.cpython, "--mnpy", str(mnpy),
             "--reference", str(self.release_binary())],
            "e-math.log",
        )
        text = completed.stdout.decode() if isinstance(completed.stdout, bytes) else completed.stdout
        try:
            summary = json.loads(text[text.index("{"):])
        except ValueError:
            record["failures"].append(f"{label}: the math probe failed (exit {completed.returncode}); see logs/e-math.log")
            return {"exit": completed.returncode}
        for check in ("cbrt", "libm"):
            problems = summary[check]["problems"]
            if problems:
                more = f" (+{len(problems) - 5} more)" if len(problems) > 5 else ""
                record["failures"].append(f"{label}: math probe {check}: {'; '.join(problems[:5])}{more}")
        return summary

    def render_formula(self, version: str, archive: Path, digest: str, work: Path) -> Path:
        template = (REPO / "homebrew-tap-template" / "Formula" / "minipython.rb").read_text()
        release_url = "https://github.com/lhwzds/minipython/releases/download/v#{version}/"
        if release_url not in template:
            raise StepFailed("the formula template no longer uses the GitHub release URL the tap expects")
        text = re.sub(r'version "[^"]+"', f'version "{version}"', template, count=1)
        text = text.replace(release_url, f"file://{archive.parent}/")
        # The host's archive gets its real hash; the other targets (never
        # fetched here) get distinct stand-ins so the audit sees four hashes.
        host_placeholder = {
            "aarch64-apple-darwin": "ARM_MAC", "x86_64-apple-darwin": "X86_MAC",
            "aarch64-unknown-linux-musl": "ARM_LINUX", "x86_64-unknown-linux-musl": "X86_LINUX",
        }[archive.name.removeprefix("minipython-").removesuffix(".tar.gz")]
        for name in ("ARM_MAC", "X86_MAC", "ARM_LINUX", "X86_LINUX"):
            value = digest if name == host_placeholder else hashlib.sha256(f"not-built-{name}".encode()).hexdigest()
            if f'"PLACEHOLDER_SHA256_{name}"' not in text:
                raise StepFailed(f"the formula template lacks PLACEHOLDER_SHA256_{name}")
            text = text.replace(f'"PLACEHOLDER_SHA256_{name}"', f'"{value}"')
        formula = work / "minipython.rb"
        formula.write_text(text)
        return formula

    def homebrew(self, version: str, archive: Path, digest: str, work: Path, record: dict) -> dict:
        formula = self.render_formula(version, archive, digest, work)
        env = {"HOMEBREW_NO_AUTO_UPDATE": "1", "HOMEBREW_NO_INSTALL_FROM_API": "1", "HOMEBREW_NO_ANALYTICS": "1",
               "HOMEBREW_NO_ENV_HINTS": "1", "HOMEBREW_NO_INSTALL_CLEANUP": "1"}
        installed = subprocess.run(["brew", "list", "--formula", "--versions", "minipython"], capture_output=True,
                                   text=True, env={**os.environ, **env}, stdin=subprocess.DEVNULL)
        if installed.returncode == 0 and installed.stdout.strip():
            raise StepFailed(f"a `minipython` formula is already installed ({installed.stdout.strip()}); "
                             "uninstall it first, the gate will not replace it")
        tap = f"minipython-gate/local{os.getpid()}"
        name = f"{tap}/minipython"
        result = {"tap": tap}
        self.check(["brew", "tap-new", "--no-git", tap], "e-brew.log", env=env)
        try:
            repository = Path(self.check(["brew", "--repository", tap], "e-brew.log", env=env).strip().splitlines()[-1])
            shutil.copyfile(formula, repository / "Formula" / "minipython.rb")
            self.check(["brew", "install", "--formula", name], "e-brew.log", env=env)
            prefix = Path(self.check(["brew", "--prefix", name], "e-brew.log", env=env).strip().splitlines()[-1])
            result["install"] = "ok"
            self.check(["brew", "test", name], "e-brew.log", env=env)
            result["test"] = "ok"
            result["smoke"] = self.smoke(prefix / "bin" / "mnpy", version, record, "homebrew")
            audit = self.run(["brew", "audit", "--strict", "--formula", name], "e-brew.log", env=env)
            result["audit"] = "ok" if audit.returncode == 0 else f"exit {audit.returncode}"
            if audit.returncode != 0:
                problems = [line.strip() for line in audit.stdout.splitlines() if line.strip().startswith("*")]
                record["failures"].append(f"brew audit --strict exited {audit.returncode}: {problems or audit.stdout[-400:]}")
        finally:
            self.run(["brew", "uninstall", "--formula", "--force", name], "e-brew.log", env=env)
            self.run(["brew", "untrust", "--formula", name], "e-brew.log", env=env)
            self.run(["brew", "untap", "--force", tap], "e-brew.log", env=env)
        return result

    # -- report ------------------------------------------------------------

    def write_report(self) -> None:
        failed = [step for step in self.steps if step["status"] == "FAIL"]
        payload = {
            "gate": "minipython release gate",
            "generated": dt.datetime.now(dt.timezone.utc).isoformat(timespec="seconds"),
            "meta": self.meta,
            "passed": not failed and len(self.steps) == len(STEPS),
            "seconds": round(time.monotonic() - self.started, 1),
            "keep_going": self.args.keep_going,
            "steps": self.steps,
            "manual_checklist": [{"item": title, "instructions": text, "fields": {field: "" for field in fields}}
                                 for title, text, fields in MANUAL_CHECKLIST],
        }
        (REPORT_DIR / "report.json").write_text(json.dumps(payload, indent=2, default=str) + "\n")
        lines = [
            "# MiniPython release gate report",
            "",
            f"- Result: **{'PASS' if payload['passed'] else 'FAIL'}** (automated steps; the manual checklist below must also be complete)",
            f"- Version: `{self.meta.get('version')}`",
            f"- Revision: `{self.meta.get('revision')}`",
            f"- Host: `{self.meta.get('host')}`",
            f"- Generated: `{payload['generated']}`, {payload['seconds']}s",
            f"- Mode: {'--keep-going (every step ran)' if self.args.keep_going else 'stop at the first failure'}",
            "",
            "| Step | Title | Result | Seconds |",
            "| --- | --- | --- | ---: |",
        ]
        ran = {step["key"] for step in self.steps}
        for step in self.steps:
            lines.append(f"| {step['key']} | {step['title']} | {step['status']} | {step['seconds']} |")
        for key, title in STEPS:
            if key not in ran:
                lines.append(f"| {key} | {title} | NOT RUN | |")
        for step in self.steps:
            lines += ["", f"## Step {step['key']}: {step['title']} — {step['status']}", ""]
            for failure in step["failures"]:
                lines += ["```text", f"FAIL: {failure}", "```"]
            lines += ["```json", json.dumps(step["details"], indent=2, default=str), "```"]
        lines += ["", "## Step f: manual checklist", "",
                  "Fill in every blank before tagging; attach this report to the release PR or notes.", ""]
        for title, text, fields in MANUAL_CHECKLIST:
            lines += [f"### [ ] {title}", "", text, ""]
            lines += [f"- {field} ____________" for field in fields]
            lines.append("")
        (REPORT_DIR / "report.md").write_text("\n".join(lines) + "\n")
        print(f"report: {(REPORT_DIR / 'report.md').relative_to(REPO)} and report.json", flush=True)


STEPS = [
    ("preflight", "clean tree, pinned CPython, grammar source, versions"),
    ("a", "cargo fmt and the full cargo test (release, all targets, no skips)"),
    ("b", "pipeline unit tests and the 1024-case discovery sweep vs the baseline"),
    ("c", "oracle corpora"),
    ("conformance", "layer-zero conformance corpus: comparison, status block, oracle drift"),
    ("d", "leak scan of tracked files and commit messages"),
    ("licensing", "licensing: SPDX headers, NOTICE, dependency licences, provenance"),
    ("e", "packaging smoke: archive, npm, Homebrew"),
]


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--cpython", required=True)
    parser.add_argument("--require-version", required=True)
    parser.add_argument("--keep-going", action="store_true", help="Run every step even after a failure.")
    parser.add_argument("--allow-dirty", action="store_true", help="Accept uncommitted changes (development only).")
    parser.add_argument("--update-discovery-baseline", action="store_true",
                        help="Rewrite the discovery baseline from this run; the run still fails.")
    parser.add_argument("--only", default="",
                        help="Comma-separated step keys to run while developing the gate; such a run never passes.")
    args = parser.parse_args()
    only = {key.strip() for key in args.only.split(",") if key.strip()}

    if REPORT_DIR.exists():
        shutil.rmtree(REPORT_DIR)
    (REPORT_DIR / "logs").mkdir(parents=True)
    gate = Gate(args)
    functions = {"preflight": gate.preflight, "a": gate.cargo, "b": gate.pipeline, "c": gate.oracles,
                 "conformance": gate.conformance, "d": gate.leaks, "licensing": gate.licensing,
                 "e": gate.packaging}
    for key, title in STEPS:
        if only and key not in only:
            continue
        if not gate.step(key, title, functions[key]) and not args.keep_going:
            break
    gate.write_report()
    passed = all(step["status"] == "PASS" for step in gate.steps) and len(gate.steps) == len(STEPS)
    print(f"release gate: {'PASS' if passed else 'FAIL'}", flush=True)
    return 0 if passed else 1


if __name__ == "__main__":
    raise SystemExit(main())
