#!/usr/bin/env python3
"""Compare a fixed CPython oracle against MiniPython over a bounded corpus."""

from __future__ import annotations

import argparse
import json
import os
import platform
import re
import subprocess
import sys
import textwrap
import time
import tomllib
from collections import Counter
from dataclasses import asdict, dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


DEFAULT_CPYTHON_ORACLE = "/opt/homebrew/bin/python3"

STATUSES = [
    "MATCH",
    "OUTPUT_DIFF",
    "EXCEPTION_CLASS_DIFF",
    "EXCEPTION_MESSAGE_DIFF",
    "MINIPYTHON_REJECTS",
    "CPYTHON_REJECTS",
    "INTENTIONAL_SANDBOX_BLOCK",
    "UNSUPPORTED_OUT_OF_SCOPE",
    "STDLIB_MISSING",
    "CPYTHON_INTERNAL",
    "TIMEOUT",
    "CRASH",
]

EXPECTED_STATUS_BY_MARKER = {
    "intentional_sandbox_block": "INTENTIONAL_SANDBOX_BLOCK",
    "unsupported_out_of_scope": "UNSUPPORTED_OUT_OF_SCOPE",
    "stdlib_missing": "STDLIB_MISSING",
    "cpython_internal": "CPYTHON_INTERNAL",
}

VALID_CATEGORIES = {
    "syntax",
    "runtime-semantic",
    "exception-shape",
    "stdlib-missing",
    "sandbox-excluded",
    "cpython-internal",
}

DEFAULT_CATEGORY_BY_SCOPE = {
    "syntax": "syntax",
    "core-runtime": "runtime-semantic",
    "stdlib-sandbox": "runtime-semantic",
    "intentional-sandbox": "sandbox-excluded",
}

NON_FAILING_STATUSES = {
    "MATCH",
    "INTENTIONAL_SANDBOX_BLOCK",
    "UNSUPPORTED_OUT_OF_SCOPE",
    "STDLIB_MISSING",
    "CPYTHON_INTERNAL",
}


@dataclass
class RunResult:
    exit_code: int | None
    stdout: str
    stderr: str
    timeout: bool
    exception_class: str | None
    exception_message: str | None


@dataclass
class SweepResult:
    name: str
    scope: str
    category: str
    priority: str
    status: str
    expected: str | None
    diff: str
    cpython: RunResult
    minipython: RunResult


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Run bounded CPython-vs-MiniPython gap discovery cases.",
    )
    parser.add_argument(
        "--cpython",
        default=DEFAULT_CPYTHON_ORACLE,
        help=f"CPython oracle executable. Defaults to {DEFAULT_CPYTHON_ORACLE}.",
    )
    parser.add_argument(
        "--require-version",
        required=True,
        help="Exact CPython version required for the oracle, for example 3.14.6.",
    )
    parser.add_argument(
        "--minipython",
        default="target/debug/mnpy",
        help="MiniPython executable to compare.",
    )
    parser.add_argument(
        "--corpus",
        default="tests/gap_corpus",
        help="TOML corpus file or directory containing TOML corpus files.",
    )
    parser.add_argument(
        "--scope",
        default="syntax,core-runtime,stdlib-sandbox,intentional-sandbox",
        help="Comma-separated scopes to run.",
    )
    parser.add_argument(
        "--category",
        default=",".join(sorted(VALID_CATEGORIES)),
        help="Comma-separated root-cause categories to run.",
    )
    parser.add_argument(
        "--out",
        default="reports/cpython-gap-sweep",
        help="Output path prefix. Writes .json and .md files.",
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=5.0,
        help="Per-case timeout in seconds for each interpreter.",
    )
    parser.add_argument(
        "--fail-on-diff",
        action="store_true",
        help="Exit nonzero when non-intentional differences are found.",
    )
    return parser.parse_args()


def run_command(command: list[str], source: str, timeout: float) -> RunResult:
    try:
        completed = subprocess.run(
            command,
            input=source,
            text=True,
            capture_output=True,
            timeout=timeout,
            check=False,
        )
    except subprocess.TimeoutExpired as error:
        return RunResult(
            exit_code=None,
            stdout=error.stdout or "",
            stderr=error.stderr or "",
            timeout=True,
            exception_class=None,
            exception_message=None,
        )
    except OSError as error:
        return RunResult(
            exit_code=None,
            stdout="",
            stderr=str(error),
            timeout=False,
            exception_class=error.__class__.__name__,
            exception_message=str(error),
        )
    exception_class = extract_exception_class(completed.stderr)
    return RunResult(
        exit_code=completed.returncode,
        stdout=completed.stdout,
        stderr=completed.stderr,
        timeout=False,
        exception_class=exception_class,
        exception_message=extract_exception_message(completed.stderr, exception_class),
    )


def extract_exception_class(stderr: str) -> str | None:
    lines = [line.strip() for line in stderr.splitlines() if line.strip()]
    if not lines:
        return None
    for line in reversed(lines):
        match = re.search(
            r"(?:runtime error:\s*)?([A-Za-z_][A-Za-z0-9_]*(?:Error|Exception|Warning|Exit|Interrupt))\b",
            line,
        )
        if match:
            return match.group(1)
    return None


def normalized_message(result: RunResult) -> str:
    if result.exception_message is not None:
        return result.exception_message
    if result.exception_class is None:
        return result.stderr.strip()
    return extract_exception_message(result.stderr, result.exception_class) or result.stderr.strip()


def extract_exception_message(stderr: str, exception_class: str | None) -> str | None:
    if exception_class is None:
        return None
    lines = [line.strip() for line in stderr.splitlines() if line.strip()]
    for line in reversed(lines):
        if exception_class in line:
            return line.replace("runtime error: ", "", 1)
    return None


def normalized_diff(cpython: RunResult, minipython: RunResult) -> str:
    parts: list[str] = []
    if cpython.exit_code != minipython.exit_code:
        parts.append(f"exit_code {cpython.exit_code!r} != {minipython.exit_code!r}")
    if cpython.timeout != minipython.timeout:
        parts.append(f"timeout {cpython.timeout!r} != {minipython.timeout!r}")
    if cpython.exception_class != minipython.exception_class:
        parts.append(
            f"exception_class {cpython.exception_class!r} != {minipython.exception_class!r}"
        )
    if normalized_message(cpython) != normalized_message(minipython):
        parts.append(
            f"exception_message {normalized_message(cpython)!r} != {normalized_message(minipython)!r}"
        )
    if cpython.stdout != minipython.stdout:
        parts.append("stdout differs")
    elif cpython.stderr != minipython.stderr:
        parts.append("stderr differs")
    return "; ".join(parts)


def classify(cpython: RunResult, minipython: RunResult, expected: str | None) -> str:
    if cpython.timeout or minipython.timeout:
        return "TIMEOUT"
    if cpython.exit_code is None or minipython.exit_code is None:
        return "CRASH"
    same_exit = cpython.exit_code == minipython.exit_code
    same_stdout = cpython.stdout == minipython.stdout
    same_stderr = cpython.stderr == minipython.stderr
    if same_exit and same_stdout and same_stderr:
        return "MATCH"
    if expected in EXPECTED_STATUS_BY_MARKER:
        return EXPECTED_STATUS_BY_MARKER[expected]
    cpython_ok = cpython.exit_code == 0
    minipython_ok = minipython.exit_code == 0
    if cpython_ok and not minipython_ok:
        return "MINIPYTHON_REJECTS"
    if minipython_ok and not cpython_ok:
        return "CPYTHON_REJECTS"
    if cpython_ok and minipython_ok:
        return "OUTPUT_DIFF"
    if cpython.exception_class != minipython.exception_class:
        return "EXCEPTION_CLASS_DIFF"
    if normalized_message(cpython) != normalized_message(minipython):
        return "EXCEPTION_MESSAGE_DIFF"
    return "OUTPUT_DIFF"


def corpus_paths(corpus: Path) -> list[Path]:
    if corpus.is_file():
        return [corpus]
    return sorted(corpus.glob("*.toml"))


def load_cases(corpus: Path) -> list[dict[str, Any]]:
    cases: list[dict[str, Any]] = []
    for path in corpus_paths(corpus):
        data = tomllib.loads(path.read_text())
        for case in data.get("case", []):
            case = dict(case)
            case.setdefault("scope", "unspecified")
            case.setdefault(
                "category",
                DEFAULT_CATEGORY_BY_SCOPE.get(case["scope"], "runtime-semantic"),
            )
            case.setdefault("priority", "unspecified")
            case["_path"] = str(path)
            if "name" not in case or "source" not in case:
                raise ValueError(f"{path}: every case needs name and source")
            if case["category"] not in VALID_CATEGORIES:
                raise ValueError(
                    f"{path}: unknown category `{case['category']}`; "
                    f"expected one of {sorted(VALID_CATEGORIES)}"
                )
            if (
                "expected" in case
                and case["expected"] not in EXPECTED_STATUS_BY_MARKER
            ):
                raise ValueError(
                    f"{path}: unknown expected marker `{case['expected']}`"
                )
            cases.append(case)
    return cases


def oracle_version(cpython: str) -> str:
    completed = subprocess.run(
        [cpython, "-c", "import platform; print(platform.python_version())"],
        text=True,
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.strip() or "failed to run CPython oracle")
    return completed.stdout.strip()


def run_sweep(args: argparse.Namespace) -> tuple[dict[str, Any], list[SweepResult]]:
    version = oracle_version(args.cpython)
    if version != args.require_version:
        raise SystemExit(
            f"CPython oracle version mismatch: expected {args.require_version}, got {version}"
        )

    minipython = str(Path(args.minipython))
    if not Path(minipython).exists():
        raise SystemExit(f"MiniPython executable not found: {minipython}")

    scopes = {scope.strip() for scope in args.scope.split(",") if scope.strip()}
    categories = {
        category.strip() for category in args.category.split(",") if category.strip()
    }
    unknown_categories = sorted(categories - VALID_CATEGORIES)
    if unknown_categories:
        raise SystemExit(f"Unknown gap sweep categories: {', '.join(unknown_categories)}")
    cases = [
        case
        for case in load_cases(Path(args.corpus))
        if case["scope"] in scopes and case["category"] in categories
    ]
    started = time.time()
    results: list[SweepResult] = []
    for case in cases:
        source = textwrap.dedent(case["source"]).lstrip("\n")
        cpython = run_command([args.cpython, "-I", "-B", "-"], source, args.timeout)
        mini = run_command([minipython], source, args.timeout)
        status = classify(cpython, mini, case.get("expected"))
        diff = "" if status == "MATCH" else normalized_diff(cpython, mini)
        results.append(
            SweepResult(
                name=case["name"],
                scope=case["scope"],
                category=case["category"],
                priority=case["priority"],
                status=status,
                expected=case.get("expected"),
                diff=diff,
                cpython=cpython,
                minipython=mini,
            )
        )

    meta = {
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "duration_seconds": round(time.time() - started, 3),
        "cwd": os.getcwd(),
        "required_cpython_version": args.require_version,
        "cpython_executable": str(Path(args.cpython).resolve()),
        "cpython_version": version,
        "driver_executable": str(Path(sys.executable).resolve()),
        "driver_python": platform.python_version(),
        "minipython_executable": str(Path(minipython).resolve()),
        "corpus": str(Path(args.corpus)),
        "scope": sorted(scopes),
        "category": sorted(categories),
    }
    return meta, results


def write_reports(prefix: Path, meta: dict[str, Any], results: list[SweepResult]) -> None:
    prefix.parent.mkdir(parents=True, exist_ok=True)
    json_payload = {
        "meta": meta,
        "summary": dict(Counter(result.status for result in results)),
        "categories": dict(Counter(result.category for result in results)),
        "results": [asdict(result) for result in results],
    }
    (prefix.with_suffix(".json")).write_text(json.dumps(json_payload, indent=2) + "\n")
    (prefix.with_suffix(".md")).write_text(render_markdown(meta, results))


def render_markdown(meta: dict[str, Any], results: list[SweepResult]) -> str:
    summary = Counter(result.status for result in results)
    lines = [
        "# CPython Gap Sweep",
        "",
        f"- Generated: `{meta['generated_at']}`",
        f"- Required CPython: `{meta['required_cpython_version']}`",
        f"- CPython: `{meta['cpython_version']}` at `{meta['cpython_executable']}`",
        f"- Driver Python: `{meta['driver_python']}` at `{meta['driver_executable']}`",
        f"- MiniPython: `{meta['minipython_executable']}`",
        f"- Corpus: `{meta['corpus']}`",
        f"- Scopes: `{', '.join(meta['scope'])}`",
        f"- Categories: `{', '.join(meta['category'])}`",
        f"- Duration: `{meta['duration_seconds']}s`",
        "",
        "## Summary",
        "",
        "| Status | Count |",
        "| --- | ---: |",
    ]
    for status in STATUSES:
        if summary[status]:
            lines.append(f"| `{status}` | {summary[status]} |")
    lines.extend(
        [
            "",
            "## Categories",
            "",
            "| Category | Count |",
            "| --- | ---: |",
        ]
    )
    for category, count in sorted(Counter(result.category for result in results).items()):
        lines.append(f"| `{category}` | {count} |")
    lines.extend(
        [
            "",
            "## Cases",
            "",
            "| Case | Scope | Category | Priority | Status |",
            "| --- | --- | --- | --- | --- |",
        ]
    )
    for result in results:
        lines.append(
            f"| `{result.name}` | `{result.scope}` | `{result.category}` | `{result.priority}` | `{result.status}` |"
        )
    differing = [result for result in results if result.status != "MATCH"]
    if differing:
        lines.extend(["", "## Differences", ""])
        for result in differing:
            lines.extend(
                [
                    f"### `{result.name}`",
                    "",
                    f"- Status: `{result.status}`",
                    f"- Scope: `{result.scope}`",
                    f"- Category: `{result.category}`",
                    f"- Priority: `{result.priority}`",
                    f"- Diff: `{result.diff}`",
                    "",
                    "```text",
                    "CPython stdout:",
                    result.cpython.stdout.rstrip(),
                    "CPython stderr:",
                    result.cpython.stderr.rstrip(),
                    "MiniPython stdout:",
                    result.minipython.stdout.rstrip(),
                    "MiniPython stderr:",
                    result.minipython.stderr.rstrip(),
                    "```",
                    "",
                ]
            )
    return "\n".join(lines).rstrip() + "\n"


def main() -> int:
    args = parse_args()
    meta, results = run_sweep(args)
    write_reports(Path(args.out), meta, results)
    summary = Counter(result.status for result in results)
    print(
        "gap sweep:",
        ", ".join(f"{status}={summary[status]}" for status in STATUSES if summary[status]),
    )
    if args.fail_on_diff:
        bad = [result for result in results if result.status not in NON_FAILING_STATUSES]
        if bad:
            return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
