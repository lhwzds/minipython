# CPython Gap Corpus

This corpus feeds `tools/cpython_gap_sweep.py`. Prefer
`tools/run_cpython_gap_sweep.sh` for local runs because it builds `mnpy`, reads
the pinned `.python-version`, and launches the oracle through
`uv run --python`.

The sweep is a discovery tool, not a release gate. It compares a fixed CPython
oracle against MiniPython, classifies differences, and writes structured
reports. Differences found here should be triaged into:

- `must_fix`: syntax or core sandbox runtime behavior.
- `should_fix`: supported pure-memory stdlib behavior.
- `nice_to_have`: metadata, exact wording, or low-impact introspection.
- `wont_fix`: intentional sandbox exclusions or full-CPython surfaces.

Once a difference is promoted for implementation, add focused `cpython_subset`,
`cpython_diff`, manifest, coverage, and migration evidence before considering it
part of the supported surface.
