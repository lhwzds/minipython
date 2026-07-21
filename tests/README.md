# MiniPython Tests

`tests/run.sh` is the only test entrypoint. Test implementation, curated cases, Rust suites, and test documentation all live in this directory.

---

## Test Strategy

# Sandbox Test Strategy

This document separates CPython conformance evidence from sandbox security
evidence. A passing conformance corpus does not prove containment, and a passing
containment suite does not imply complete CPython behavior.

The requirement-to-example mapping is maintained in
`tests/README.md`.

## Test Sets

| Test set | Purpose | Authoritative entrypoint | Failure found means |
| --- | --- | --- | --- |
| Grammar inventory | Keep the pinned CPython grammar rule names synchronized | `cargo test --test cpython_inventory` | Syntax inventory drift or missing named evidence |
| Runtime subset | Exercise the supported MiniPython behavior directly | `cargo test --test cpython_subset` | A supported runtime or stdlib behavior regressed |
| Differential parity | Run a real CPython 3.14.6 process against the MiniPython in-process API for focused behavior slices | `cargo test --test cpython_diff` | Observable parity regressed inside the declared surface |
| Manifest guards | Keep implementation, tests, allowlist, coverage, and docs synchronized | `cargo test --test parity manifest::` | Evidence or scope documentation drifted |
| Full-process gap corpus | Run the same program in real CPython and the real sandbox-default `mnpy` executable, then compare exit status, stdout, stderr, exception shape, timeout, and crash state | `tests/run.sh --discovery` | An end-to-end root cause needs triage; corpus success alone is not completion |
| In-process sandbox policy | Test shared budgets, import policy, cache checks, and virtual/root modules | `cargo test --test runtime sandbox_policy` plus budget filters | A VM or embedding-policy invariant regressed |
| Process containment | Test source-size and process-memory failure containment | `cargo test --test sandbox process::` | The official untrusted-code entrypoint is not contained |
| Product embedding API | Exercise structured results, inert value transport, authorized external functions, and persistent isolated sessions through real workers | `cargo test --test product` | An embedding surface bypassed the process boundary or broke its lifecycle contract |
| Adversarial boundary | Exercise every CLI budget and common policy bypass through the sandbox-default `mnpy` CLI | `cargo test --test sandbox boundary::` | The public process boundary differs from the in-process policy |
| Executable sandbox examples | Run one checked-in Python program in real CPython and real `mnpy`, asserting either exact parity or an intentional sandbox difference | `cargo test --test sandbox examples::` | A documented user-visible boundary no longer matches the real executables |
| Deterministic differential discovery | Generate, compare, classify, and minimize real-process cases across syntax, runtime, stdlib, and security | `tests/run.sh --discovery` | A generated root cause must be fixed, accepted by scope, and promoted to a regression |

`tests/run.sh` is the authoritative orchestration entrypoint. Its
default mode runs the complete Rust suite and 1024 fixed-seed differential
cases; `--focused` runs the security-focused Rust slice and 64 generated cases;
`--discovery` runs the Python harness plus generated real-process comparison.

## Boundary Dimensions

The adversarial boundary set must retain executable coverage for:

- every exposed resource control: source bytes, VM instructions, call depth,
  captured output, VM allocation, and worker process memory;
- every input channel: stdin, `-c`, and source files;
- the complete required stdlib allowlist and rejection of compatibility-only
  modules;
- host capability modules covering I/O, network, process, signal, threads,
  debugger, locale, and C extensions;
- import bypass attempts through `__import__`, dotted children, `sys.modules`,
  root modules, and symlink escape;
- policy propagation from the entry source into imported root modules;
- deterministic failure: nonzero status, bounded completion time, and a stable
  sandbox error category.

## Discovery Queue

Deterministic differential generation and minimization are implemented. The
remaining discovery expansion areas are stateful mutable-container models, a
macOS/Linux release-profile process matrix, and a persisted crash corpus for
parser, compiler, VM, and import-loader inputs, each guarded by a wall-clock timeout.

New exploratory cases belong in a focused set first. Promote a discovered
supported behavior into `cpython_subset` and `cpython_diff`; promote a security
failure into `sandbox_boundary` or `sandbox_process` and keep it in the release
gate permanently. Every process-level sandbox requirement should also have a
readable program under `examples/sandbox/`; the E2E test must execute that exact
file through `include_str!` rather than maintain a second hidden copy.

---

## Sandbox Acceptance

# Sandbox MVP Acceptance

This file is the authoritative completion checklist for the MiniPython sandbox
MVP. A green CPython gap sweep proves only the current corpus; it does not mark
this checklist complete.

Status values:

- `proven`: implementation and executable evidence satisfy the MVP boundary.
- `in_progress`: useful implementation exists, but the stated acceptance proof
  is incomplete.
- `missing`: the sandbox control or evidence does not exist yet.
- `excluded`: explicitly outside the product stop-line.

The MVP is complete only when every required row below is `proven`, the release
gate passes, and every excluded row remains blocked.

| Area | Required | Status | Current evidence | Remaining acceptance |
| --- | --- | --- | --- | --- |
| Syntax frontend | yes | `proven` | `tests/README.md` tracks all 276 CPython 3.14.6 grammar rules with named Rust evidence | Keep the pinned inventory synchronized and retain focused syntax/error diffs |
| Core runtime | yes | `proven` | `tests/README.md` explicitly bounds all 13 `partial` coverage rows and the four partial CPython public groups while retaining their wider CPython status; the complete release gate covers functions, closures, classes, descriptors, MRO, exceptions, generators, async, containers, numbers, strings, bytes, array, and one-dimensional memoryview | Keep the bounded runtime gate green without expanding into documented CPython/host exclusions |
| Safe stdlib allowlist | yes | `proven` | The Sandbox Stdlib Manifest records supported/excluded surfaces and direct `cpython_diff` evidence for every required module; the complete release gate has no open `must_fix` or `should_fix` roots | Keep every supported surface executable and every excluded surface blocked |
| Import isolation | yes | `proven` | `sandbox_policy_*`, virtual-module, package, duplicate-name, invalid-name, and symlink-escape tests | Keep source modules confined to the canonical sandbox root and enforce explicit stdlib policy through cache and child imports |
| Host capability stop-line | yes | `proven` | Required allowlist tests block host I/O, network, process, signal, socket, C ABI, and C-extension modules | Keep every excluded module unavailable even through `__import__`, `sys.modules`, package children, and compatibility shims |
| Instruction budget | yes | `proven` | `instruction_budget_*` tests cover top-level code, functions, generators, `exec`, and imported source modules; CLI and `SandboxPolicy` use a finite default | Keep one shared budget across every nested VM path |
| Wall-clock deadline | yes | `proven` | The single public `mnpy` parent enforces a finite worker deadline across parsing, compilation, VM execution, and shutdown; `wall_clock_budget.py` is run in real CPython and real `mnpy` | Keep the deadline enabled on every supported host and terminate plus reap timed-out workers |
| Call-depth guard | yes | `proven` | `call_depth_guard_*` and `sandbox_policy_call_depth_guard_*` stop direct and imported-module recursion before host stack overflow; CLI and `SandboxPolicy` use a conservative finite default | Keep the frame limit shared across functions, dynamic execution, imports, generators, and coroutines; replace Rust-stack recursion with explicit VM frame scheduling before raising the default |
| Heap/allocation guard | yes | `proven` | A shared monotonic VM materialization budget recursively charges core values and supported stdlib buffers, with separate incremental mutation charges and the existing 64 MiB single-allocation guard. The single public `mnpy` entrypoint always runs compilation and execution in a child process: Unix uses kernel address/data limits where supported, while macOS monitors physical footprint and kills the worker on overflow. `allocation_budget_*` covers VM paths and `sandbox_process_contains_compiler_memory_pressure` proves compiler/AST containment outside VM accounting | Keep `mnpy` sandboxed by default with no public bypass; in-process APIs do not claim a complete host-memory boundary |
| Captured-output guard | yes | `proven` | `output_budget_*` and `sandbox_policy_output_budget_*` bound complete lines, partial prints, `exec`, and imported-module output under one shared byte budget; CLI and `SandboxPolicy` use a finite default | Keep every buffered-output path on the shared budget and avoid double charging nested VM output during merge |
| Batch CPython parity | yes | `proven` | Fixed 3.14.6 oracle, curated corpus, four-layer fixed-seed generation, real-process comparison, automatic minimization, categorized JSON/Markdown reports, root-cause grouping, filters, and priority gates | Keep generated and curated discovery representative; no open `must_fix` or `should_fix` root cause may remain at release |
| Security regression suite | yes | `proven` | Import boundary, symlink escape, dynamic `__import__` / from-import / dotted-child cache injection, instruction, call-depth, output, VM allocation, source-size, process-memory, and worker-crash containment regressions run in the release gate. `sandbox_boundary` additionally exercises every CLI budget, all source channels, compatibility-shim rejection, cache injection, and root-module policy through the official process entrypoint | Keep the process entrypoint and every policy/budget regression in both focused and complete gates; maintain the test-set taxonomy in `tests/README.md` |
| Structured Rust execution API | yes | `proven` | `tests/product.rs` runs `run`, `eval`, and `check` through a real worker and verifies exact output, inert values, structured exceptions, usage, and pre-spawn input rejection | Keep the CLI and Rust API on the same worker protocol and never expose VM objects as host values |
| Authorized external functions | yes | `proven` | Product tests cover explicit registration, absent names, positional and keyword transport, nested Python calls, catchable host errors, panic containment, opaque-value rejection, and input-name conflicts | Keep callbacks opt-in by exact name and restrict both directions to inert transport values |
| Persistent isolated sessions | yes | `proven` | Product tests prove globals, functions, mutations, inputs, and module cache persist without source replay; worker limits close the entire session and internal session-worker mode rejects direct invocation | Keep one isolated worker per session, serialize calls, reapply per-call budgets, and reap workers on close/drop |
| Support and stop-line docs | yes | `proven` | `README.md`, `README_CN.md`, `AGENTS.md`, this checklist, runtime coverage, migration notes, and the stdlib manifest describe the current supported and excluded boundary | Keep the checklist and public support matrix synchronized with implementation changes |

## Release Gate

Run `tests/run.sh` for the complete gate. During focused
sandbox-control development, `tests/run.sh --focused` runs the
security-policy slice plus a small generated discovery set; focused mode is not
sufficient for release completion. `tests/run.sh` is the only supported entrypoint.

The final release gate must run, in order:

1. Focused sandbox security and policy tests.
2. The complete language/runtime test suite.
3. CPython subset, differential, inventory, and manifest tests against
   `/opt/homebrew/bin/python3` at the pinned version.
4. The curated corpus plus 1024 fixed-seed cases across syntax, runtime,
   stdlib, and security, with generated failures minimized and no open
   `must_fix` or `should_fix` root cause.
5. `cargo fmt --check` and `git diff --check`.

Until all required rows are `proven`, the full MiniPython sandbox goal remains
active.

---

## Runtime Boundary

# Sandbox Runtime MVP Surface

This matrix defines the core runtime stopping point for the MiniPython sandbox
MVP. It does not claim complete CPython behavior outside the executable parity
modules in `tests/parity.rs`; the sandbox-safe MVP surface is accepted here only
with explicit exclusions.

## Required Runtime Surface

| Area | MVP support | Explicitly excluded from MVP completion |
| --- | --- | --- |
| Numbers | Integer, bigint, bool, float, complex literals; common arithmetic, comparisons, conversion, formatting, and public numeric protocols | Locale-sensitive formatting, exhaustive platform floating-point internals, and every CPython diagnostic variant |
| Strings and interpolation | Unicode strings, bytes literals, common string methods, formatting, f-strings, t-strings, escapes, and public tokenize split output | CPython's parser-internal token representation, full codec registry, locale behavior, and lone-surrogate storage outside Rust `String` |
| Containers | List, tuple, dict, set, frozenset, comprehensions, iteration, slicing, unpacking, mutation, comparison, and recursive display | CPython allocation/performance stress contracts, C-level layout, GC tracking, and thread-safety internals |
| Object model | Classes, inheritance, MRO, descriptors, properties, bound methods, metaclasses used by the supported surface, and public type metadata | C layout compatibility, weakref/GC lifecycle parity, refcounts, and unrestricted metaclass/internal slot parity |
| Functions and scope | Positional/keyword binding, defaults, annotations, closures, globals, nonlocals, decorators, recursion under sandbox limits, generators, coroutines, and async generators | CPython frame/opcode identity, exact stack size, specialization, and debugger integration |
| Exceptions | Builtin hierarchy, custom exceptions, chaining, traceback objects used by the supported runtime, exception groups, and public attributes | CPython traceback/frame implementation identity and interpreter-shutdown lifecycle |
| Bytes and bytearray | Construction, indexing/slicing, search, split/join, case and predicate methods, translation, mutation, copying, and sandbox allocation behavior | Binary pickle-byte identity, exhaustive full-buffer exporters, locale behavior, and C ABI matrices |
| Array | Pure-memory construction and mutation for the documented typecodes, slicing, iteration, bytes/list conversion, copying, and BytesIO transfer | Real file APIs, pickle reconstruction internals, platform C ABI layout guarantees, and every unsupported typecode edge |
| Memoryview | One-dimensional bytes, bytearray, and supported array exporters; read/write, slicing, cast subset, attributes, release, hashing, and BytesIO `readinto` | Multidimensional subviews/casts, ctypes exporters, full buffer protocol, GC callback timing, and refcount matrices |
| Async runtime | `async def`, await, async iteration/context management, coroutine/generator protocols, and supported ABC behavior | Event loops, threads, signals, sockets, subprocess integration, and host scheduling |

## Coverage Partial Disposition

Every current `partial` row in `tests/README.md` must remain listed
here. `accepted_mvp` means the required public sandbox behavior is implemented;
the row remains partial only because broader CPython behavior is intentionally
not claimed.

| Coverage row | Sandbox status | MVP boundary |
| --- | --- | --- |
| `NUMBER` | `accepted_mvp` | Common numeric syntax/runtime is required; locale and exhaustive platform/error matrices are excluded |
| `STRING` | `accepted_mvp` | Common literals, escapes, bytes, f-strings, and t-strings are required; full codec/lone-surrogate behavior is excluded |
| `STRING_RUNTIME` | `accepted_mvp` | Documented pure-memory string/bytes methods and formatting are required; full codec registry and locale behavior are excluded |
| `CONTAINER_RUNTIME` | `accepted_mvp` | Core container construction, iteration, slicing, mutation, comparison, and display are required; stress/performance/internal layout is excluded |
| `COLLECTIONS_ABC_RUNTIME` | `accepted_mvp` | Documented structural checks and mixin behavior are required; unrestricted ABCMeta internals are excluded |
| `FSTRING_START` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `FSTRING_MIDDLE` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `FSTRING_END` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `TSTRING_START` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `TSTRING_MIDDLE` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `TSTRING_END` | `accepted_mvp` | Public tokenizer split output and runtime semantics are required; collapsed parser-internal representation is accepted |
| `ERRORTOKEN` | `accepted_mvp` | Representative invalid characters, delimiters, strings, null bytes, and nesting errors are required; exhaustive tokenizer diagnostics are not |
| `ENCODING` | `accepted_mvp` | UTF-8, BOM/cookie detection, documented legacy labels, and decode errors are required; CPython's full codecs registry is excluded |

`OP` remains `out_of_scope_runtime`: it is a tokenizer/operator inventory row,
not a separate runtime implementation target.

## CPython Partial Group Disposition

| CPython group | Sandbox disposition |
| --- | --- |
| `test_builtin.py::BuiltinTest` | Keep public pure-memory builtins in scope; host `open`/`input`, filesystem, process/environment interaction, full pickle/deallocation details, and implementation optimization tests remain excluded |
| `test_builtin.py::TestType` | The current public dynamic type subset is accepted; lone-surrogate encoding branches outside Rust `String` remain excluded |
| `test_memoryview.py` direct methods | The documented one-dimensional public subset is accepted; multidimensional, ctypes, full buffer protocol, GC/refcount, and callback timing remain excluded |
| `test_bytes.py::BaseBytesTest` | The documented common bytes/bytearray surface is accepted; exhaustive buffer exporters, binary pickle identity, stress/performance, and every diagnostic permutation remain excluded |

## Completion Rule

This runtime surface is proven only when the complete language/runtime suite,
CPython subset/diff/inventory/manifest suites, and the sandbox MVP release gate
all pass. Untrusted source enters through the single public `mnpy` CLI, which combines the
VM budgets with child-process memory containment for compiler and host
allocations. New public runtime behavior must either gain direct evidence here
or be documented as excluded without crossing the host capability stop-line.

---

## E2E Matrix

# Sandbox E2E Requirement Matrix

This matrix is the process-level complement to `sandbox_mvp.md`. An
`example_e2e` row runs checked-in Python source in real CPython and real
sandbox-default `mnpy`. A `harness_only` row covers a transport or launcher
condition that cannot be represented by a valid Python source file.

| Requirement | Proof class | Example | Executable evidence |
| --- | --- | --- | --- |
| Safe stdlib remains useful | `example_e2e` | `examples/sandbox/safe_stdlib.py` | `real_cpython_and_mnpy_classify_the_complete_safe_stdlib_example` |
| Host I/O, network, process, and C ABI stay unavailable | `example_e2e` | `examples/sandbox/blocked_host_capabilities.py` | `real_cpython_and_mnpy_diverge_only_at_host_capability_boundary` |
| Entry source-size limit precedes execution | `example_e2e` | `examples/sandbox/source_size_budget.py` | `real_cpython_completes_while_mnpy_enforces_source_size_budget` |
| VM instruction budget stops finite CPU work | `example_e2e` | `examples/sandbox/instruction_budget.py` | `real_cpython_completes_while_mnpy_enforces_instruction_budget` |
| Call-depth budget stops finite recursion | `example_e2e` | `examples/sandbox/call_depth_budget.py` | `real_cpython_completes_while_mnpy_enforces_call_depth_budget` |
| Captured-output budget stops oversized output | `example_e2e` | `examples/sandbox/output_budget.py` | `real_cpython_completes_while_mnpy_enforces_output_budget` |
| VM allocation budget stops value growth | `example_e2e` | `examples/sandbox/allocation_budget.py` | `real_cpython_completes_while_mnpy_enforces_allocation_budget` |
| Parent wall-clock deadline covers pre-VM and VM work | `example_e2e` | `examples/sandbox/wall_clock_budget.py` | `real_cpython_completes_while_mnpy_enforces_wall_clock_budget` |
| Worker process memory contains compiler pressure | `example_e2e` | `examples/sandbox/compiler_memory_pressure_generator.py` | `real_cpython_completes_while_mnpy_contains_compiler_memory_pressure` |
| Script-directory imports work for safe modules | `example_e2e` | `examples/sandbox/import_root/main.py` | `real_cpython_and_mnpy_match_for_safe_script_directory_imports` |
| Imported source modules inherit policy | `example_e2e` | `examples/sandbox/blocked_import_root/main.py` | `real_mnpy_propagates_policy_into_script_directory_imports` |
| `sys.modules` injection cannot bypass policy | `example_e2e` | `examples/sandbox/cache_injection.py` | `real_cpython_accepts_cache_injection_while_mnpy_rechecks_policy` |
| `eval`, `exec`, and compiled code inherit policy | `example_e2e` | `examples/sandbox/dynamic_imports.py` | `real_cpython_allows_dynamic_imports_while_mnpy_reuses_the_sandbox_policy` |
| Module symlinks cannot escape the canonical root | `example_e2e` | `examples/sandbox/symlink_escape_main.py` | `real_cpython_follows_module_symlink_while_mnpy_rejects_root_escape` |
| Non-UTF-8 CLI input is rejected before parsing | `harness_only` | Not a valid `.py` text example | `sandbox_boundary_rejects_non_utf8_source_files` |
| Hidden worker mode has no public CLI bypass | `harness_only` | Launcher invariant, not Python behavior | `sandbox_process_rejects_direct_worker_invocation` |
| Worker crash or memory kill is reaped by the parent | `harness_only` | Process-status invariant | `sandbox_process_contains_compiler_memory_pressure` |

Every new sandbox requirement must enter this matrix. Prefer `example_e2e`;
use `harness_only` only when the condition cannot be expressed as valid Python
source. Each example E2E must run real executables and compare exit status,
stdout, stderr or exception category, and termination behavior as applicable.

---

## Discovery Baseline

# Differential Discovery Baseline

This is the checked-in evidence for the deterministic large discovery gate. It
does not claim exhaustive Python compatibility; it proves that the pinned case
generator, real-process comparison, classification, and minimization pipeline
completed one reproducible large run without an open supported-scope root cause.

## Pinned Run

- Date: `2026-07-15`
- CPython oracle: `/opt/homebrew/bin/python3`
- Required CPython version: `3.14.6`
- Seed: `20260710`
- Generated cases: `1024`
- Curated corpus cases: `32`
- Command: `tests/run.sh --discovery --seed 20260710 --generated-cases 1024`
- Generated layers: `syntax=256`, `runtime=256`, `stdlib=256`, `security=256`
- Generated root-cause templates: `17`

## Result

| Classification | Count |
| --- | ---: |
| `MATCH` | 790 |
| `INTENTIONAL_SANDBOX_BLOCK` | 258 |
| `UNSUPPORTED_OUT_OF_SCOPE` | 5 |
| `STDLIB_MISSING` | 1 |
| `CPYTHON_MISSING_COMPAT` | 1 |
| `CPYTHON_INTERNAL` | 1 |

All `1056` selected cases were either exact matches or explicit accepted scope
classifications. `open_root_causes` was empty, so the run found no open
`must_fix` or `should_fix` root cause. Shrinking was enabled; no reproducer was
written because no unaccepted difference was found.

The machine-readable local report is generated at
`reports/test-pipeline.json` and remains gitignored. Future generator changes
must rerun the same command, update this baseline from the report, and retain
the four-layer balance. New unaccepted differences must be minimized, fixed by
root cause, and promoted to checked-in regression evidence before this baseline
can move forward.

---

## Corpus Contract

# CPython Gap Corpus

This corpus feeds `tests/pipeline.py`. Prefer the unified
`tests/run.sh --discovery` entrypoint. It combines these curated
cases with deterministic generated cases, builds `mnpy`, uses
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
Run a focused slice with `tests/run.sh --module json`, or use
the `--module json` option directly when invoking the driver. Comma-separated
values such as `--module json,collections.abc` can cover related surfaces.

Each case must carry a concrete `root_cause` id. `category` is the broad failure
class, `modules` describes affected surface area, and `root_cause` is the repair
unit used for grouped work. Use
`tests/run.sh --root-cause json-loads-core` before
implementing a fix so one commit can cover all cases that share the same root
cause. The `--root-cause` filter is the preferred entry point when moving from
gap discovery to grouped repair. JSON reports include both a full
`root_cause_summary` and `open_root_causes`; use `open_root_causes` as the
machine-readable repair queue for root causes that still contain
`needs_triage` cases. Use `--fail-on-open` when the batch should fail on that
repair queue while continuing to accept explicitly classified sandbox,
compatibility, and CPython-internal gaps. The open-root-cause report also
includes the focused `tests/run.sh --root-cause ...` command
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

Generated discovery uses four required layers: `syntax`, `runtime`, `stdlib`,
and `security`. The default release run uses seed `20260710` and 1024 generated
cases. Each generated failure retains a root-cause priority and is minimized
while preserving its differential failure signature. Minimized reproducers are
written only under ignored `reports/differential-repros/`; a repaired root cause
must be promoted into a checked-in focused regression test rather than relying
on a generated report.
