# CPython Gap Corpus

This corpus feeds `tools/cpython_gap_sweep.py`. Prefer
`tools/run_cpython_gap_sweep.sh` for local runs because it builds `mnpy`, uses
`/opt/homebrew/bin/python3` as the fixed CPython oracle, and checks it against
the pinned `.python-version`.

The sweep is a discovery tool, not a release gate. It compares a fixed CPython
oracle against MiniPython, classifies differences, and writes structured
reports. Differences found here should be triaged into:

- `must_fix`: syntax or core sandbox runtime behavior.
- `should_fix`: supported pure-memory stdlib behavior.
- `nice_to_have`: metadata, exact wording, or low-impact introspection.
- `wont_fix`: intentional sandbox exclusions or full-CPython surfaces.

Each case also carries a root-cause `category` so the sweep can be grouped
before implementation:

- `syntax`: parser, tokenizer, AST, or compile-lowering gaps.
- `runtime-semantic`: object model, containers, descriptors, values, or safe
  stdlib behavior that should be implemented.
- `exception-shape`: exception type, catchability, or public message shape.
- `stdlib-missing`: CPython stdlib behavior that is not in the sandbox
  allowlist or has not been promoted yet.
- `sandbox-excluded`: deliberate host I/O, network, process, or C ABI stop
  lines.
- `cpython-internal`: CPython implementation-only modules or contracts.

Each case must also carry `modules`, or the singular `module`, so the same
corpus can be filtered by affected surface. Use concrete stdlib names such as
`json`, `collections.abc`, or `math.integer` for stdlib cases, and component
names such as `syntax`, `core-runtime`, or `exceptions` for non-stdlib cases.
Run a focused slice with `tools/run_cpython_gap_sweep.sh --module json`, or use
the `--module json` option directly when invoking the driver. Comma-separated
values such as `--module json,collections.abc` can cover related surfaces.

Each case must carry a concrete `root_cause` id. `category` is the broad failure
class, `modules` describes affected surface area, and `root_cause` is the repair
unit used for grouped work. Use
`tools/run_cpython_gap_sweep.sh --root-cause json-loads-core` before
implementing a fix so one commit can cover all cases that share the same root
cause. The `--root-cause` filter is the preferred entry point when moving from
gap discovery to grouped repair. JSON reports include both a full
`root_cause_summary` and `open_root_causes`; use `open_root_causes` as the
machine-readable repair queue for root causes that still contain
`needs_triage` cases. Use `--fail-on-open` when the batch should fail on that
repair queue while continuing to accept explicitly classified sandbox,
compatibility, and CPython-internal gaps. The open-root-cause report also
includes the focused `tools/run_cpython_gap_sweep.sh --root-cause ...` command
to rerun one grouped repair slice.

Use `expected = "intentional_sandbox_block"` for deliberate sandbox rejections
and `expected = "unsupported_out_of_scope"` for public CPython behavior that is
outside MiniPython's sandbox target. Use `expected = "stdlib_missing"` and
`expected = "cpython_internal"` for known non-goal gaps that should stay visible
without failing the smoke sweep. Use `expected = "cpython_missing_compat"` for
MiniPython compatibility surfaces such as `math.integer` that are in the
sandbox allowlist but absent from the fixed CPython oracle.

Once a difference is promoted for implementation, add focused `cpython_subset`,
`cpython_diff`, manifest, coverage, and migration evidence before considering it
part of the supported surface.
