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

`mnpy` always runs code through the sandbox boundary:

```bash
mnpy --max-memory-bytes 134217728 -c "print(1 + 2)"
```

`mnpy` applies the fixed safe-stdlib allowlist, source-size, instruction,
call-depth, captured-output, VM-allocation, and child-process memory limits. It
always launches an internal worker; there is no public flag that disables the
sandbox. On macOS the parent monitors the worker's physical footprint; on other
Unix hosts it uses kernel process limits. File execution uses the script's
directory as the default sandbox module root; `-c` and stdin expose no host
module root unless `--root` is passed. Library APIs remain useful for focused
runtime and parity tests, but in-process calls are not the supported untrusted-
code boundary.

### Rust execution API

Embedders use the same worker boundary as the CLI. `Sandbox` starts the supplied
`mnpy` worker executable, sends a length-framed MessagePack request, and returns
a structured result without exposing VM objects to the host:

```rust
use minipython::{Sandbox, SandboxInputs, SandboxValue};

let sandbox = Sandbox::new("target/release/mnpy");
let mut inputs = SandboxInputs::new();
inputs.insert("price".into(), SandboxValue::from(40_i64));

let result = sandbox.eval_with_inputs("price + 2", inputs);
assert!(result.is_success());
assert_eq!(result.value, Some(SandboxValue::from(42_i64)));
```

`ExecutionResult` separates status, value, exact captured stdout/stderr,
exception phase/type/message, and resource usage. Inputs and returned values are
restricted to inert data (`None`, booleans, numbers, strings, bytes, bytearrays,
lists, tuples, and dictionaries). Unsupported runtime objects are output-only
`Opaque` descriptions and cannot be injected back into the sandbox. The worker
path is explicit so an embedding application controls which reviewed sidecar it
executes.

CLI execution is bounded to 1,000,000 VM instructions by default. Use
`--max-steps N` to select a smaller or larger budget. Library callers can use
`RuntimeOptions::with_max_instructions`; `SandboxPolicy` also applies the same
finite default to virtual and sandbox-directory modules. The budget is shared
across functions, generators, coroutines, dynamic execution, and imports.
The worker also has a 5-second wall-clock deadline by default, covering parsing,
compilation, VM execution, and shutdown. Use `--max-time-ms N` to configure it.
Nested VM frames are also bounded to 3 by default; use `--max-depth N` or
`RuntimeOptions::with_max_call_depth` to configure that guard.
Captured output is bounded to 1 MiB by default and shares one byte budget across
nested execution; use `--max-output-bytes N` or
`RuntimeOptions::with_max_output_bytes` to configure it.
Core VM value materialization has a shared monotonic 8 MiB default budget,
configurable with `--max-allocated-bytes N` or
`RuntimeOptions::with_max_allocated_bytes`. This complements the existing
64 MiB single-allocation guard. The `mnpy` child-process boundary covers
compiler and host allocations outside VM value accounting.

Runnable boundary examples live under `examples/sandbox/`. For example,
`mnpy examples/sandbox/blocked_host_capabilities.py` shows the host I/O,
network, process, and C-ABI capabilities that the sandbox intentionally blocks.

The current whole-project sandbox completion state is tracked in
`tests/README.md`. A green parity corpus alone is not a completion signal.
The exact core runtime stopping point, including the disposition of every
CPython coverage row that remains partial, is in `tests/README.md`.
Run `tests/run.sh --focused` while developing sandbox controls,
`tests/run.sh --discovery` for differential discovery only, and
`tests/run.sh` for the complete release gate. This is now the only test
entrypoint; the old sandbox and gap-sweep runners were removed.

## Testing

```bash
tests/run.sh --focused
tests/run.sh --discovery
tests/run.sh --discovery --seed 20260710 --generated-cases 1024
tests/run.sh
tests/run.sh --module json
tests/run.sh --root-cause json-loads-core
```

The unified pipeline runs driver unit tests, the checked-in corpus, and
deterministic generated programs through real CPython and the sandbox-default
`mnpy`. The fixed seed is balanced across syntax, runtime, stdlib, and security
layers. Unexpected generated differences are minimized and written under
`reports/differential-repros/`; one representative per root cause is minimized
while every original difference remains in the report. Reports preserve original
and minimized source, classification, root cause, seed, interpreter output, and
shrink attempts.
The release and discovery modes fail while a generated `must_fix` or
`should_fix` root cause remains open. The pipeline uses
`/opt/homebrew/bin/python3` as the fixed CPython oracle, checks it against the
pinned `.python-version`, and builds `mnpy`. Promoted behavior still needs focused
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
root causes that still have `needs_triage` cases. The runner enables
`--fail-on-open` so unexpected open root causes fail the batch while accepted
sandbox/compatibility gaps stay visible in the report. Open root-cause reports
also include the focused `tests/run.sh --root-cause ...`
command to rerun the grouped repair slice.

## Architecture

```
Host → Rust API / CLI → MessagePack worker → Lexer → Parser → Compiler → Register VM
```

A register-based VM with 80+ instructions and 60+ value types.

## License

MIT
