"""Run the oracle corpora: generate programs, run them in CPython and mnpy, compare.

usage: run_oracles.py --cpython PY --mnpy MNPY --out DIR [--module NAME,...]
                      [--manifest tests/oracles/corpora.toml] [--jobs N]

Every corpus in the manifest names a generator (a script in this directory)
and how to compare the two interpreters' results. Generators are run with the
pinned CPython. Randomized generators use the fixed seed and size recorded in
the manifest. Extractors read CPython's own Lib/test from the pinned install
(or MINIPYTHON_CPYTHON_LIB) at run time and write their programs under DIR
only, so no extracted CPython test content is ever committed.

A corpus passes when every scenario is identical in both interpreters apart
from the exclusions listed for it in the manifest, each with its reason. An
exclusion that no longer differs is stale and fails the corpus too, so the
list cannot outlive the difference it documents.

A corpus may also list `platform_exclusions`: the platform C library decides
the result of some scenarios, so they differ between macOS and Linux. The
generator marks those scenarios in a `<program>.platform-keys.json` file next
to each program it writes. On the platforms the manifest lists for the corpus
the runner skips the marked scenarios and counts every one of them as a
documented exclusion with the manifest's reason, never as a pass: the marks
say which scenarios the platform decides, not whether they happen to differ
here. A scenario mnpy did not print is a mismatch, never an exclusion, so a
missing record cannot hide behind a platform mark. Every other scenario is
compared as usual.

Exit status: 0 when every selected corpus passes, 1 otherwise. A JSON summary
is written to DIR/oracles.json and a markdown one to DIR/oracles.md.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import fnmatch
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import time
import tomllib
from dataclasses import dataclass, field
from pathlib import Path

HERE = Path(__file__).resolve().parent


@dataclass
class ProgramResult:
    program: str
    scenarios: int = 0
    matched: int = 0
    excluded: int = 0
    mismatches: list[dict] = field(default_factory=list)
    stale_exclusions: list[str] = field(default_factory=list)
    platform_exclusions: list[tuple[str, str]] = field(default_factory=list)
    error: str = ""


def stdlib_dir(cpython: str) -> str:
    """The pinned CPython's standard library, or MINIPYTHON_CPYTHON_LIB, which
    names the Lib directory of the matching source tree on hosts whose CPython
    ships without Lib/test (the official Docker images)."""
    override = os.environ.get("MINIPYTHON_CPYTHON_LIB")
    if override:
        return override
    output = subprocess.run(
        [cpython, "-I", "-c", "import sysconfig; print(sysconfig.get_path('stdlib'))"],
        capture_output=True,
        text=True,
        check=True,
        stdin=subprocess.DEVNULL,
    )
    return output.stdout.strip()


def strip_addresses(text: str) -> str:
    return re.sub(r" at 0x[0-9a-fA-F]+", " at 0x?", text)


def platform_exclusions(corpus: dict) -> list[dict]:
    """The platform-scoped exclusions of a corpus that apply on this host."""
    system = platform.system()
    return [entry for entry in corpus.get("platform_exclusions", []) if entry.get("platform") == system]


def platform_keys(program: Path) -> set[str]:
    """The scenario keys a generator marked as decided by the platform."""
    path = program.with_suffix(".platform-keys.json")
    if not path.is_file():
        return set()
    try:
        return set(json.loads(path.read_text()))
    except (OSError, ValueError):
        return set()


def split_records(text: str, key_pattern: re.Pattern[str]) -> tuple[list[str], dict[str, str], list[str]]:
    """Split keyed output into (preamble lines, {key: record}, duplicate keys).

    A record starts at a line whose first token matches the key pattern;
    following lines that do not start a record belong to it.
    """
    preamble: list[str] = []
    records: dict[str, list[str]] = {}
    order: list[str] = []
    duplicates: list[str] = []
    current: list[str] | None = None
    for line in text.splitlines():
        token = line.split(" ", 1)[0]
        if key_pattern.fullmatch(token):
            if token in records:
                duplicates.append(token)
            current = [line]
            records[token] = current
            order.append(token)
        elif current is None:
            preamble.append(line)
        else:
            current.append(line)
    return preamble, {key: "\n".join(records[key]) for key in order}, duplicates


TRACEBACK_FRAME = re.compile(r'^(?P<prefix>[ |+\-]*?)  File "[^"]*", line \d+')


def normalize_traceback(stderr: str) -> str:
    """Drop CPython's `File "...", line N` frame lines and their source lines.

    MiniPython prints the traceback header, chained causes and contexts, and
    the final exception lines, but no frame lines (README: Usage). A frame's
    source and caret lines are indented deeper than its `File` line.
    """
    kept: list[str] = []
    frame_indent: int | None = None
    for line in stderr.splitlines():
        match = TRACEBACK_FRAME.match(line)
        if match:
            frame_indent = len(line) - len(line.lstrip(" |+-"))
            continue
        if frame_indent is not None:
            body = line.lstrip(" |+-")
            indent = len(line) - len(body)
            if body and indent > frame_indent:
                continue
            frame_indent = None
        kept.append(line)
    return "\n".join(kept)


def run(command: list[str], cwd: Path, env: dict[str, str], timeout: float) -> subprocess.CompletedProcess:
    try:
        return subprocess.run(
            command,
            cwd=cwd,
            env=env,
            capture_output=True,
            timeout=timeout,
            stdin=subprocess.DEVNULL,
        )
    except subprocess.TimeoutExpired as error:
        return subprocess.CompletedProcess(command, -999, error.stdout or b"", (error.stderr or b"") + b"\n<timeout>")


def decode(data: bytes) -> str:
    return data.decode("utf-8", errors="backslashreplace")


def compare_program(corpus: dict, program: Path, cpython_run, mnpy_run, exclusions: dict[str, str],
                    platform_scenarios: set[str] = frozenset(), platform_reason: str = "") -> ProgramResult:
    name = str(program.relative_to(corpus["_dir"]))
    result = ProgramResult(program=name)
    compare = corpus["compare"]
    c_out, m_out = strip_addresses(decode(cpython_run.stdout)), strip_addresses(decode(mnpy_run.stdout))
    c_err, m_err = decode(cpython_run.stderr), decode(mnpy_run.stderr)

    if compare == "keyed":
        pattern = re.compile(corpus.get("key_pattern", r"\d+"))
        c_pre, c_rec, c_dup = split_records(c_out, pattern)
        m_pre, m_rec, _ = split_records(m_out, pattern)
        if cpython_run.returncode != 0:
            result.error = f"CPython exited {cpython_run.returncode}: {c_err[-1500:]}"
            return result
        if c_dup:
            result.error = f"CPython printed duplicate scenario keys: {c_dup[:5]}"
            return result
        if c_pre != m_pre:
            result.mismatches.append({"key": "<preamble>", "cpython": "\n".join(c_pre)[:600], "mnpy": "\n".join(m_pre)[:600]})
        if cpython_run.returncode != mnpy_run.returncode:
            result.mismatches.append({"key": "<exit>", "cpython": str(cpython_run.returncode), "mnpy": f"{mnpy_run.returncode} {m_err[-600:]}"})
        for key, expected in c_rec.items():
            result.scenarios += 1
            actual = m_rec.get(key)
            excluded = f"{name}:{key}" in exclusions
            if excluded:
                result.excluded += 1
                if actual == expected:
                    result.stale_exclusions.append(f"{name}:{key}")
                continue
            if actual is None:
                # mnpy printed no record for this scenario. A missing record
                # is a mismatch, never a platform exclusion: the platform
                # cannot explain a scenario that did not run.
                result.mismatches.append({"key": key, "cpython": expected[:600], "mnpy": "<missing>"})
                continue
            if key in platform_scenarios:
                # The platform C library decides this scenario, so on this
                # platform it is a documented exclusion and never a pass
                # (tests/README.md, "Platform-dependent behavior").
                result.excluded += 1
                result.platform_exclusions.append((f"{name}:{key}", platform_reason))
                continue
            if actual == expected:
                result.matched += 1
            else:
                result.mismatches.append({"key": key, "cpython": expected[:600], "mnpy": actual[:600]})
        for key in m_rec:
            if key not in c_rec:
                result.mismatches.append({"key": key, "cpython": "<missing>", "mnpy": m_rec[key][:600]})
        return result

    # Whole-program comparisons: one scenario per program.
    result.scenarios = 1
    if compare == "exact":
        same = (cpython_run.returncode, c_out, c_err) == (mnpy_run.returncode, m_out, m_err)
    elif compare == "traceback":
        same = (cpython_run.returncode, c_out, normalize_traceback(c_err)) == (
            mnpy_run.returncode,
            m_out,
            normalize_traceback(m_err),
        )
    else:
        raise SystemExit(f"unknown compare mode {compare!r} in corpus {corpus['name']}")
    if name in exclusions:
        result.excluded = 1
        if same:
            result.stale_exclusions.append(name)
        return result
    if same:
        result.matched = 1
    else:
        result.mismatches.append(
            {
                "key": name,
                "cpython": f"exit={cpython_run.returncode}\n{c_out[-400:]}\n--- stderr\n{c_err[-400:]}",
                "mnpy": f"exit={mnpy_run.returncode}\n{m_out[-400:]}\n--- stderr\n{m_err[-400:]}",
            }
        )
    return result


def substitute(argument: str, values: dict[str, str]) -> str:
    for key, value in values.items():
        argument = argument.replace("{" + key + "}", value)
    return argument


def prepare_corpus(corpus: dict, defaults: dict, args: argparse.Namespace, stdlib: str) -> dict:
    """Generate (or copy) a corpus's programs; return its summary skeleton."""
    corpus_dir = Path(args.out) / corpus["module"] / corpus["name"]
    if corpus_dir.exists():
        shutil.rmtree(corpus_dir)
    corpus_dir.mkdir(parents=True)
    corpus["_dir"] = corpus_dir
    summary = {
        "module": corpus["module"],
        "name": corpus["name"],
        "kind": corpus["kind"],
        "seeds": corpus.get("seeds"),
        "count": corpus.get("count"),
        "compare": corpus["compare"],
        "programs": 0,
        "scenarios": 0,
        "matched": 0,
        "excluded": 0,
        "mismatched": 0,
        "stale_exclusions": [],
        "errors": [],
        "mismatches": [],
        "exclusions": [
            {"key": entry["key"], "reason": entry["reason"]} for entry in corpus.get("exclusions", [])
        ],
        "passed": False,
        "_programs": [],
        "_started": time.monotonic(),
    }
    if "generator" in corpus:
        # A corpus with several seeds runs its generator once per seed; each
        # run writes its own program set (`{out}` is a per-seed directory).
        for seed in corpus.get("seeds", [None]):
            out_dir = corpus_dir if seed is None else corpus_dir / f"seed-{seed}"
            out_dir.mkdir(parents=True, exist_ok=True)
            values = {
                "out": str(out_dir),
                "seed": "" if seed is None else str(seed),
                "count": str(corpus.get("count", "")),
                "per": str(corpus.get("per", "")),
                "stdlib": stdlib,
            }
            generator = [substitute(part, values) for part in corpus["generator"]]
            generated = subprocess.run(
                [args.cpython, "-E", "-s", "-B", str(HERE / generator[0]), *generator[1:]],
                capture_output=True,
                text=True,
                stdin=subprocess.DEVNULL,
                cwd=out_dir,
            )
            if generated.returncode != 0:
                summary["errors"].append(f"generator failed (seed {seed}): {generated.stderr[-1500:]}")
                return summary
    if "copy" in corpus:
        source = HERE / corpus["copy"]
        for path in sorted(source.rglob("*.py")):
            target = corpus_dir / path.relative_to(source)
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(path, target)
    summary["_programs"] = sorted(
        path
        for path in corpus_dir.rglob("*.py")
        if fnmatch.fnmatch(path.name, corpus.get("programs", "*.py"))
    )
    if not summary["_programs"]:
        summary["errors"].append("no programs were generated")
    return summary


def run_program(corpus: dict, defaults: dict, args: argparse.Namespace, program: Path) -> ProgramResult:
    cpython_env = {"PATH": "/usr/bin:/bin", **defaults.get("cpython_env", {}), **corpus.get("cpython_env", {})}
    cpython_flags = corpus.get("cpython_args", defaults["cpython_args"])
    mnpy_flags = corpus.get("mnpy_args", defaults["mnpy_args"])
    timeout = float(corpus.get("timeout", defaults["timeout"]))
    exclusions = {entry["key"]: entry["reason"] for entry in corpus.get("exclusions", [])}
    scoped = platform_exclusions(corpus)
    platform_reason = scoped[0]["reason"] if scoped else ""
    platform_scenarios = platform_keys(program) if scoped else set()
    cpy = run([args.cpython, *cpython_flags, program.name], program.parent, cpython_env, timeout)
    mnp = run([args.mnpy, *mnpy_flags, program.name], program.parent, {"PATH": "/usr/bin:/bin"}, timeout + 30)
    program.with_suffix(".cpy").write_bytes(cpy.stdout + b"\n--- stderr\n" + cpy.stderr + f"\n--- exit {cpy.returncode}\n".encode())
    program.with_suffix(".mnp").write_bytes(mnp.stdout + b"\n--- stderr\n" + mnp.stderr + f"\n--- exit {mnp.returncode}\n".encode())
    return compare_program(corpus, program, cpy, mnp, exclusions, platform_scenarios, platform_reason)


def finish_corpus(summary: dict, results: list[ProgramResult]) -> dict:
    platform_counts: dict[str, int] = {}
    for result in results:
        for key, reason in result.platform_exclusions:
            platform_counts[reason] = platform_counts.get(reason, 0) + 1
        summary["programs"] += 1
        summary["scenarios"] += result.scenarios
        summary["matched"] += result.matched
        summary["excluded"] += result.excluded
        summary["mismatched"] += len(result.mismatches)
        summary["stale_exclusions"].extend(result.stale_exclusions)
        if result.error:
            summary["errors"].append(f"{result.program}: {result.error}")
        for mismatch in result.mismatches:
            if len(summary["mismatches"]) < 40:
                summary["mismatches"].append({"program": result.program, **mismatch})
    for reason, count in sorted(platform_counts.items()):
        summary["exclusions"].append({"key": platform.system(), "reason": reason, "count": count})
    summary["passed"] = (
        not summary["errors"]
        and summary["mismatched"] == 0
        and not summary["stale_exclusions"]
        and summary["scenarios"] > 0
        and summary["matched"] + summary["excluded"] == summary["scenarios"]
    )
    summary["seconds"] = round(time.monotonic() - summary.pop("_started"), 2)
    summary.pop("_programs", None)
    return summary


def render_markdown(meta: dict, summaries: list[dict]) -> str:
    lines = ["# Oracle corpora", "", f"- CPython: `{meta['cpython']}` ({meta['cpython_version']})", f"- mnpy: `{meta['mnpy']}`", ""]
    lines += ["| Module | Corpus | Kind | Seeds | Programs | Scenarios | Matched | Excluded | Mismatched | Result |", "| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |"]
    for s in summaries:
        seeds = ", ".join(str(seed) for seed in s["seeds"]) if s.get("seeds") else "-"
        lines.append(
            f"| `{s['module']}` | `{s['name']}` | {s['kind']} | {seeds} | {s['programs']} | {s['scenarios']} | {s['matched']} | {s['excluded']} | {s['mismatched']} | {'PASS' if s['passed'] else 'FAIL'} |"
        )
    excluded = [s for s in summaries if s["exclusions"]]
    if excluded:
        lines += ["", "## Documented exclusions", ""]
        for s in excluded:
            for entry in s["exclusions"]:
                where = f"`{entry['key']}`" + (f" ({entry['count']} scenarios)" if entry.get("count") else "")
                lines.append(f"- `{s['module']}/{s['name']}` {where}: {entry['reason']}")
    for s in summaries:
        if s["passed"]:
            continue
        lines += ["", f"## `{s['module']}/{s['name']}` failures", ""]
        for error in s["errors"]:
            lines.append(f"- error: {error}")
        for stale in s["stale_exclusions"]:
            lines.append(f"- stale exclusion (now matches; remove it from the manifest): `{stale}`")
        for m in s["mismatches"]:
            lines += [f"- `{m['program']}` `{m['key']}`", "", "```text", "cpython: " + m["cpython"], "mnpy:    " + m["mnpy"], "```"]
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--cpython", required=True)
    parser.add_argument("--mnpy", required=True)
    parser.add_argument("--out", required=True)
    parser.add_argument("--manifest", default=str(HERE / "corpora.toml"))
    parser.add_argument("--module", default="*")
    parser.add_argument("--jobs", type=int, default=max(1, min(8, (os.cpu_count() or 2) - 2)))
    args = parser.parse_args()
    args.mnpy = str(Path(args.mnpy).resolve())

    manifest = tomllib.loads(Path(args.manifest).read_text())
    defaults = manifest["defaults"]
    modules = {m.strip() for m in args.module.split(",") if m.strip()}
    corpora = [c for c in manifest["corpus"] if "*" in modules or c["module"] in modules]
    if not corpora:
        print(f"no corpus matches --module {args.module}", file=sys.stderr)
        return 1
    version = subprocess.run(
        [args.cpython, "-I", "-c", "import platform; print(platform.python_version())"],
        capture_output=True, text=True, check=True, stdin=subprocess.DEVNULL,
    ).stdout.strip()
    stdlib = stdlib_dir(args.cpython)
    Path(args.out).mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    prepared = [(corpus, prepare_corpus(corpus, defaults, args, stdlib)) for corpus in corpora]
    # One pool runs every program of every corpus, longest-running corpora
    # (by program size) first, so the step's wall time stays near the
    # slowest single program.
    tasks = [
        (index, program)
        for index, (_, summary) in enumerate(prepared)
        for program in summary["_programs"]
    ]
    tasks.sort(key=lambda task: -task[1].stat().st_size)
    results: dict[int, list[ProgramResult]] = {index: [] for index in range(len(prepared))}
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as pool:
        futures = {
            pool.submit(run_program, prepared[index][0], defaults, args, program): index
            for index, program in tasks
        }
        for future in concurrent.futures.as_completed(futures):
            results[futures[future]].append(future.result())
    summaries = []
    for index, (corpus, summary) in enumerate(prepared):
        summary = finish_corpus(summary, sorted(results[index], key=lambda r: r.program))
        summaries.append(summary)
        print(
            f"{summary['module']}/{summary['name']}: {summary['matched']}/{summary['scenarios']} matched, "
            f"{summary['excluded']} excluded, {summary['mismatched']} mismatched"
            f"{', ' + str(len(summary['stale_exclusions'])) + ' stale exclusions' if summary['stale_exclusions'] else ''}"
            f"{', errors' if summary['errors'] else ''} -> {'PASS' if summary['passed'] else 'FAIL'}",
            flush=True,
        )
    print(f"oracle corpora finished in {time.monotonic() - started:.1f}s", flush=True)
    meta = {"cpython": args.cpython, "cpython_version": version, "mnpy": args.mnpy, "stdlib": stdlib}
    payload = {"meta": meta, "corpora": summaries, "passed": all(s["passed"] for s in summaries)}
    (Path(args.out) / "oracles.json").write_text(json.dumps(payload, indent=2, default=str) + "\n")
    (Path(args.out) / "oracles.md").write_text(render_markdown(meta, summaries))
    return 0 if payload["passed"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
