"""Math probe of a shipped `mnpy` binary (release gate step e).

usage: math_probe.py --cpython PY --mnpy MNPY [--reference MNPY] [--json OUT]

The release binary is not the binary `cargo test` exercises: on Linux the
suites run the gnu build and the release ships a static musl build, whose C
library supplies different math functions. Two checks close that gap:

- cbrt: tests/oracles/math/cbrt_case.py in CPython and MNPY, compared with
  exactly the rules of `cpython_math_cbrt_diff_subset` (tests/parity.rs): the
  platform's entries of tests/oracles/math/cbrt-exclusions.txt may differ, and
  only from the listed oracle result, within two units in the last place;
  every other line must match; an entry that matches or is not exercised is
  stale.
- C library functions: tests/oracles/math/libm_probe.py in MNPY and in the
  REFERENCE binary the test suites ran (the host build). The shipped binary
  must produce the same results, apart from the entries of
  tests/oracles/math/libm-shipped-differences.txt for this platform, each of
  which must still differ (and from the listed shipped result). Differences
  from CPython are counted per function for the report; the suites hold
  those to their own (rounded) expectations.

Exit status 0 when both checks pass. A JSON summary goes to OUT or stdout.
"""

from __future__ import annotations

import argparse
import collections
import json
import platform
import struct
import subprocess
import sys
from pathlib import Path

MATH = Path(__file__).resolve().parents[1] / "oracles" / "math"
LIMITS = ["--max-steps", "100000000", "--max-time-ms", "60000"]


def platform_key() -> str:
    """`OS-ARCH` as Rust's std::env::consts names them."""
    system = {"Darwin": "macos", "Linux": "linux"}.get(platform.system(), platform.system().lower())
    machine = {"arm64": "aarch64", "amd64": "x86_64"}.get(platform.machine(), platform.machine())
    return f"{system}-{machine}"


def listed(path: Path, key: str) -> dict[str, str]:
    entries = {}
    for line in path.read_text().splitlines():
        if line.startswith("#") or not line.strip():
            continue
        fields = line.split()
        if fields[0] == key:
            entries[fields[1]] = fields[2]
    return entries


def run(command: list[str]) -> list[str]:
    completed = subprocess.run(command, capture_output=True, text=True, stdin=subprocess.DEVNULL)
    if completed.returncode != 0:
        raise RuntimeError(f"{command[0]} failed ({completed.returncode}): {completed.stderr.strip()}")
    return completed.stdout.splitlines()


def ulps(a: str, b: str) -> int:
    def bits(text: str) -> int:
        return struct.unpack("<q", struct.pack("<d", float.fromhex(text)))[0]
    return abs(bits(a) - bits(b))


def check_cbrt(cpython: str, mnpy: str, key: str) -> dict:
    case = str(MATH / "cbrt_case.py")
    theirs, ours = run([cpython, "-I", "-B", case]), run([mnpy, *LIMITS, case])
    exclusions = listed(MATH / "cbrt-exclusions.txt", key)
    problems, used = [], set()
    if len(theirs) != len(ours):
        problems.append(f"line count: cpython {len(theirs)}, mnpy {len(ours)}")
    for cp_line, mn_line in zip(theirs, ours):
        fields = cp_line.split()
        value = fields[2] if fields[0] == "cbrt" and len(fields) == 4 else None
        if value in exclusions:
            used.add(value)
            cp_result, mn_result = cp_line.rsplit(" ", 1)[1], mn_line.rsplit(" ", 1)[1]
            if cp_line == mn_line:
                problems.append(f"stale exclusion {value}: CPython now matches ({cp_line})")
            elif cp_result != exclusions[value] or cp_line.rsplit(" ", 1)[0] != mn_line.rsplit(" ", 1)[0]:
                problems.append(f"{value}: expected CPython {exclusions[value]}, got {cp_line!r} vs mnpy {mn_line!r}")
            elif ulps(cp_result, mn_result) > 2:
                problems.append(f"{value}: mnpy {mn_result} is not within 2 ulp of {cp_result}")
        elif cp_line != mn_line:
            problems.append(f"CPython {cp_line!r} vs mnpy {mn_line!r}")
    problems += [f"exclusion {value} is not an input of the case" for value in exclusions if value not in used]
    return {"lines": len(theirs), "excluded": len(used), "problems": problems}


def check_libm(cpython: str, mnpy: str, reference: str | None, key: str) -> dict:
    probe = str(MATH / "libm_probe.py")
    shipped = run([mnpy, *LIMITS, probe])
    oracle = run([cpython, "-I", "-B", probe])
    versus_cpython = collections.Counter(
        a.split(" ", 1)[0] for a, b in zip(oracle, shipped) if a != b)
    result = {"lines": len(shipped), "differs_from_cpython_by_function": dict(sorted(versus_cpython.items())),
              "problems": []}
    if reference is None or Path(reference).resolve() == Path(mnpy).resolve():
        result["reference"] = "the shipped binary is the tested build"
        return result
    tested = run([reference, *LIMITS, probe])
    known = listed(MATH / "libm-shipped-differences.txt", key)
    used = set()
    if len(tested) != len(shipped):
        result["problems"].append(f"line count: tested {len(tested)}, shipped {len(shipped)}")
    for tested_line, shipped_line in zip(tested, shipped):
        label = tested_line.split(" -> ")[0].split(" !! ")[0]
        entry = label.replace(" ", ":")
        if entry in known:
            used.add(entry)
            if tested_line == shipped_line:
                result["problems"].append(f"stale shipped difference {entry}: the builds now agree")
            elif shipped_line.rsplit(" ", 1)[1] != known[entry]:
                result["problems"].append(f"{entry}: expected shipped {known[entry]}, got {shipped_line!r}")
        elif tested_line != shipped_line:
            result["problems"].append(f"shipped {shipped_line!r} vs tested {tested_line!r}")
    result["problems"] += [f"shipped difference {entry} is not a probe input" for entry in known if entry not in used]
    result["shipped_differences_listed"] = len(used)
    return result


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--cpython", required=True)
    parser.add_argument("--mnpy", required=True)
    parser.add_argument("--reference")
    parser.add_argument("--json")
    args = parser.parse_args()
    key = platform_key()
    summary = {"platform": key, "cbrt": check_cbrt(args.cpython, args.mnpy, key),
               "libm": check_libm(args.cpython, args.mnpy, args.reference, key)}
    summary["passed"] = not summary["cbrt"]["problems"] and not summary["libm"]["problems"]
    text = json.dumps(summary, indent=2) + "\n"
    if args.json:
        Path(args.json).write_text(text)
    sys.stdout.write(text)
    return 0 if summary["passed"] else 1


if __name__ == "__main__":
    sys.exit(main())
