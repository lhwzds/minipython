# Sandbox Examples

Every Python file in this directory is both runnable documentation and the
exact source consumed by `tests/sandbox.rs`.

```bash
mnpy examples/sandbox/blocked_host_capabilities.py
mnpy examples/sandbox/safe_stdlib.py
mnpy --max-steps 100 examples/sandbox/instruction_budget.py
mnpy --max-source-bytes 8 examples/sandbox/source_size_budget.py
mnpy --max-depth 3 examples/sandbox/call_depth_budget.py
mnpy examples/sandbox/recursion_limit.py
mnpy --max-output-bytes 16 examples/sandbox/output_budget.py
mnpy --max-allocated-bytes 256 examples/sandbox/allocation_budget.py
mnpy --max-time-ms 1 --max-steps 100000000 examples/sandbox/wall_clock_budget.py
python3 examples/sandbox/compiler_memory_pressure_generator.py | \
  mnpy --max-memory-bytes 67108864 --max-source-bytes 524288
mnpy examples/sandbox/import_root/main.py
mnpy examples/sandbox/blocked_import_root/main.py
mnpy examples/sandbox/cache_injection.py
mnpy examples/sandbox/dynamic_imports.py
mnpy examples/sandbox/argparse_cli.py --help
mnpy examples/sandbox/argparse_cli.py stats '[3, 1, 4, 1, 5]' --round 2
mnpy examples/sandbox/re_text_tool.py extract "Mail ann@example.com by 2026-10-29"
mnpy examples/sandbox/re_text_tool.py grep "err(or)?" "ok" "Error: disk" -i
mnpy examples/sandbox/csv_report_tool.py summary "$(printf 'team,points\na,3\nb,4\na,5\n')" --by team --sum points
mnpy examples/sandbox/csv_report_tool.py sniff "$(printf 'x;y\n1;2\n')"
```

The host-capability example completes and reports intentional blocks.
`safe_stdlib.py` exercises every module in the positive allowlist. Eighteen
modules produce byte-for-byte identical output, including PEP 563 string
annotations from `from __future__ import annotations`; the example makes the one
versioned difference explicit because CPython 3.14.6 does not provide
`math.integer` while MiniPython does. Each
resource example completes under CPython and normal MiniPython defaults, but
the deliberately low limit shown above terminates it with a sandbox error. The
wall-clock example demonstrates the parent-process deadline, which also covers
time spent before VM instruction accounting begins.
The safe import-root example prints `7`. The blocked import-root and cache
injection examples show that policy is applied after loading local modules and
before accepting cached modules. `symlink_escape_main.py` and
`symlink_escape_target.py` are setup sources for the Unix E2E that places the
main file inside a temporary root and symlinks `escape.py` to the external
target.
The compiler-memory generator keeps the checked-in example readable while
producing the same finite 120,000-element literal used by the process-memory
containment E2E.
`dynamic_imports.py` proves that `eval`, `exec`, and compiled code reuse the
same import policy instead of creating an unrestricted nested VM.
`argparse_cli.py` is a skill-shaped command-line tool (argparse, json, and sys
only); its E2E runs it with sixteen argument lists in real CPython and real
`mnpy` and requires identical stdout, stderr, and exit status, covering
`--help`, usage errors with exit status 2, subcommands, aliases, mutually
exclusive options, and custom type errors.
`re_text_tool.py` is a skill-shaped text tool built on `re` (with argparse,
json, and sys): extracting emails, URLs, and dates, redacting numbers with a
callable replacement, grepping with a pattern given on the command line, and
parsing `key = value` lines with a verbose pattern. Its E2E runs it with ten
argument lists in real CPython and real `mnpy` and requires identical stdout,
stderr, and exit status, including the tool's report of an invalid pattern.
`csv_report_tool.py` is a skill-shaped data tool built on `csv` and
`io.StringIO` (with argparse, json, and sys): it reads CSV text given on the
command line with `csv.DictReader`, totals a column per group and writes the
report to `sys.stdout` with `csv.writer`, converts rows to JSON or to
tab-separated values with `csv.DictWriter`, sniffs a sample's dialect and
header, and parses strictly to report malformed input. Its E2E runs it with
thirteen argument lists in real CPython and real `mnpy` and requires identical
stdout, stderr, and exit status.
