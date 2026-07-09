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

Use `expected = "intentional_sandbox_block"` for deliberate sandbox rejections
and `expected = "unsupported_out_of_scope"` for public CPython behavior that is
outside MiniPython's sandbox target. Use `expected = "stdlib_missing"` and
`expected = "cpython_internal"` for known non-goal gaps that should stay visible
without failing the smoke sweep.

Once a difference is promoted for implementation, add focused `cpython_subset`,
`cpython_diff`, manifest, coverage, and migration evidence before considering it
part of the supported surface.
