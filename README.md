# minipython

A Python interpreter written in Rust.

## Goal

Build a maintainable, sandbox-focused Rust Python rather than a full CPython
clone. MiniPython should fully cover the Python syntax frontend where practical,
incrementally implement core runtime semantics and a safe pure-memory standard
library subset, and migrate CPython tests by public behavior. CPython internal
implementation tests should be classified and tagged rather than copied as
runtime requirements.

CPython is the behavior oracle, not an implementation source. MiniPython must
not wholesale port CPython `Lib/`; standard-library behavior is accepted only
when the supported and excluded sandbox surfaces are documented and backed by
direct differential evidence.

## Scope

In scope:

- CPython-compatible syntax frontend coverage: tokenizer, parser, AST,
  compiler lowering, and user-visible syntax/error classification.
- Core runtime semantics: object model, descriptors, MRO, functions,
  closures, generators, async constructs, exceptions, containers, numbers,
  strings, bytes, bytearray, array, and memoryview behavior.
- Safe pure-memory standard library modules: `builtins`, `sys`, `types`,
  `collections`, `collections.abc`, `math`, `math.integer`, `array`, `copy`,
  `io.BytesIO`, `operator`, `functools`, `itertools`, and `json`.
  Additional pure-memory compatibility shims may exist to support migrated
  CPython tests, but they do not expand the default product scope unless they
  are added to the migration manifest with explicit supported and excluded
  surfaces.
- CPython public behavior migration through executable differential tests.
  Every bundled stdlib module must have a matching `cpython_diff` case before
  its supported surface is considered complete. Partial modules must document
  their supported API and excluded API in the migration and coverage notes.

Out of scope by default:

- Full CPython standard library coverage.
- Host I/O integration such as real `open()`, file descriptors, TTY behavior,
  `input()`, and `pty`.
- Network and process integration such as `socket`, `subprocess`, and
  `signal`.
- C ABI and C extension compatibility, including modules such as `_ssl`,
  `_socket`, `_ctypes`, and `_testcapi`.
- CPython implementation internals such as refcounts, GC tracking,
  bytecode/opcode identity, interpreter specialization, and exact
  `co_stacksize`.
- Default `pdb` integration and full `breakpoint()` environment-variable
  behavior.
- locale-sensitive behavior unless it is explicitly promoted into the sandbox
  runtime requirements.

## Install

```bash
cargo build --release
```

## Usage

```bash
mnpy script.py          # run a file
mnpy -c "print(1+2)"    # execute a string
mnpy -e "1 + 2 * 3"     # evaluate an expression
echo "print(1)" | mnpy  # pipe input
```

## Testing

```bash
/opt/homebrew/bin/python3 tools/test_cpython_gap_sweep.py
tools/run_cpython_gap_sweep.sh
tools/run_cpython_gap_sweep.sh --module json
tools/run_cpython_gap_sweep.sh --root-cause json-loads-core
```

The first command runs fast unit tests for the gap-sweep driver. The gap sweep
then uses `/opt/homebrew/bin/python3` as the fixed CPython oracle, checks it
against the pinned `.python-version`, builds `mnpy`, and compares the bounded
corpus. It is a discovery loop; promoted behavior still needs focused
`cpython_subset`, `cpython_diff`, manifest, coverage, and migration evidence.
Gap reports record both the required pinned CPython version and the actual
oracle/driver interpreter paths so a stale oracle cannot hide in the results.
Use `--module` to focus a batch run on one affected surface, for example
`json`, `collections.abc`, or `math.integer`. The report keeps interpreter
`status` separate from workflow `triage_status`: passing cases, accepted
sandbox/compatibility gaps, and unexpected diffs that need root-cause work are
machine-readable in the JSON output.
Use `--root-cause` when moving from discovery to repair so a commit can address
one grouped cause while covering all affected cases in the report. JSON reports
also include `open_root_causes`, the current machine-readable repair queue for
root causes that still have `needs_triage` cases.

## Architecture

```
Source → Lexer → Parser → AST → Compiler → Bytecode → VM → Output
```

A register-based VM with 80+ instructions and 60+ value types.

## License

MIT
