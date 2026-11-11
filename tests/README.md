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
| Product embedding API | Exercise Rust and pure Python structured results, inert value transport, authorized external functions, and persistent isolated sessions through real workers | `cargo test --test product` | An embedding surface bypassed the process boundary or broke its lifecycle contract |
| Adversarial boundary | Exercise every CLI budget and common policy bypass through the sandbox-default `mnpy` CLI, and compare uncaught-exception reports with real CPython in file, `-c`, and stdin modes | `cargo test --test sandbox boundary::` | The public process boundary differs from the in-process policy |
| Capability contract | Assert every host capability from common Python-sandbox-escape writeups (files, stdio descriptors, processes, signals, network, environment, clock, entropy, C ABI, interpreter internals, the import system, builtins tampering, object-graph and frame walks, `__reduce__`) is denied by default with CPython's exception for a missing name, a matching denial-log entry, and no host effect | `cargo test --test sandbox capability_contract::` | A sandbox escape or host read; stop and fix it before anything else |
| Host capability lint | Keep every host-touching call in `src/host/`, apart from the counted parent and CLI call sites | `cargo test --test sandbox host_capability_lint::` | New host access outside the host-capability module |
| Worker processes | Check real worker processes: one per execution, one unshared per session, cleared environment, `/` working directory, only stdio descriptors | `cargo test --test sandbox worker_processes::` | A worker was reused, shared, or kept something it inherited |
| Executable sandbox examples | Run one checked-in Python program in real CPython and real `mnpy`, asserting either exact parity or an intentional sandbox difference | `cargo test --test sandbox examples::` | A documented user-visible boundary no longer matches the real executables |
| Deterministic differential discovery | Generate, compare, classify, and minimize real-process cases across syntax, runtime, stdlib, and security | `tests/run.sh --discovery` | A generated root cause must be fixed, accepted by scope, and promoted to a regression |

`tests/run.sh` is the authoritative orchestration entrypoint. Its
default mode runs the complete Rust suite and 1024 fixed-seed differential
cases; `--focused` runs the security-focused Rust slice and 64 generated cases;
`--discovery` runs the Python harness plus generated real-process comparison.

## CPython Oracle

Every test that runs CPython runs the executable named by
`MINIPYTHON_CPYTHON`, which must be a CPython 3.14.6 `python3`. When it is
unset the oracle is `/opt/homebrew/bin/python3`, the pinned Homebrew install.
`tests/run.sh` resolves it (a bare name such as `python3.14` is looked up on
`PATH`), checks its version against `.python-version`, refuses to run on a
mismatch, and exports it to `cargo test`, `tests/pipeline.py`, and the release
gate. The Rust suites (`tests/parity.rs`, `tests/product.rs`,
`tests/sandbox.rs`) and `tests/pipeline.py` read the same variable with the
same default, so a direct `cargo test` on another machine only needs
`MINIPYTHON_CPYTHON` set:

```bash
MINIPYTHON_CPYTHON=/usr/local/bin/python3.14 tests/run.sh
```

## CPython Oracle Files

Some parity tests read CPython 3.14.6 files as oracle input; none are copied
into this repository.

- Files the pinned installation ships are required evidence and are read from
  its standard library: `$MINIPYTHON_CPYTHON_LIB` when set, otherwise the
  directory the oracle itself reports as `sysconfig.get_path('stdlib')`, so
  no install prefix is hard-coded. The oracle extractors of
  `tests/oracles/` read the same directory. An install without `Lib/test`
  (the official CPython Docker images strip it, so the Linux gate image sets
  the variable to the verified 3.14.6 source tarball's `Lib/`) needs
  `MINIPYTHON_CPYTHON_LIB`; the release gate's preflight fails when a file
  below is missing or when an override's `_strptime.py`, `datetime.py`,
  `re/_parser.py`, `csv.py`, and `ast.py` differ from the oracle's own. The
  files: `test/mathdata/formatfloat_testcases.txt`
  and `test/mathdata/floating_points.txt` (the float format and repr dataset
  diffs and subsets), `test/test_ast/snippets.py` and
  `test/test_ast/data/ast_repr.txt` (the AST repr snapshot subsets),
  `test/test_ast/snippets.py` with `test/test_ast/utils.py` (the snippet
  `to_tuple()`, parse, `ast.compare()` and position-order subsets, and the
  start-mode subset's `to_tuple()` helper), `test/support/numbers.py` (the
  valid and invalid underscore-literal tables of the tokenizer and numeric
  literal subsets),
  `test/test_compile.py` (the full-file compile round trip), and the
  standard-library sources of the stdlib compile seed subsets. A missing file
  fails its test.
- `Grammar/python.gram` is not installed, so
  `inventory::cpython_grammar_inventory_rules_match_current_cpython_grammar`
  (the audit that the grammar inventory lists exactly CPython's rules) reads it
  from a CPython 3.14.6 source checkout at `$MINIPYTHON_CPYTHON_SRC`. Without
  one the test returns early after printing a `SKIPPED` notice straight to
  the test output (bypassing output capture); the harness still reports it as
  `ok`, so such a run carries no grammar-audit evidence. Set
  `MINIPYTHON_REQUIRE_CPYTHON_SRC=1` to make a missing checkout fail instead.
  The inventory lists exactly the 268 rules of the 3.14.6 grammar.
- `test/test_ast/data/ast_repr.txt` matches MiniPython exactly; the full
  snapshot subset compares every entry.

## Docstrings

The `__doc__` text of MiniPython's builtin types, functions and modules is
its own short description, not CPython's wording, so no test pins docstring
text. Tests check only that `__doc__` is a non-empty `str`, or `None` where
CPython has none. Diff and subset programs that print a docstring first pass
it through a `_doc` helper defined at the top of the program, which turns any
non-empty string into the marker `<doc>` and passes everything else through;
docstrings the program defines itself (function, class and module docstrings
and strings it assigns to `__doc__`) are listed in the helper and are printed
unchanged.

## json metadata

CPython 3.14.6's `json` module defines `__author__` (the original author's
name and e-mail address) and `__version__` (`'2.0.9'`). MiniPython's `json`
defines neither: no supported behaviour depends on them, and the author line
is personal data that does not belong in this repository. The subsets
`cpython_json_module_author_metadata_subset` and
`cpython_json_module_version_metadata_subset` assert that both are absent,
and there is no diff test for them.

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
| Syntax frontend | yes | `proven` | `tests/parity.rs` (`mod inventory`) tracks the 268 grammar rules of CPython 3.14.6 `Grammar/python.gram` with named Rust evidence, and the grammar audit checks the list against the pinned source tree (see CPython Oracle Files) | Keep the pinned inventory synchronized and retain focused syntax/error diffs |
| Core runtime | yes | `proven` | `tests/README.md` explicitly bounds all 13 `partial` coverage rows and the four partial CPython public groups while retaining their wider CPython status; the complete release gate covers functions, closures, classes, descriptors, MRO, exceptions, generators, async, containers, numbers, strings, bytes, array, and one-dimensional memoryview | Keep the bounded runtime gate green without expanding into documented CPython/host exclusions |
| Safe stdlib allowlist | yes | `proven` | The Sandbox Stdlib Manifest records supported/excluded surfaces and direct `cpython_diff` evidence for every required module; the complete release gate has no open `must_fix` or `should_fix` roots | Keep every supported surface executable and every excluded surface blocked |
| Import isolation | yes | `proven` | `sandbox_policy_*`, virtual-module, package, duplicate-name, invalid-name, and symlink-escape tests | Keep source modules confined to the canonical sandbox root and enforce explicit stdlib policy through cache and child imports |
| Host capability stop-line | yes | `proven` | Required allowlist tests block host I/O, network, process, signal, socket, C ABI, and C-extension modules; `every_host_capability_is_denied_by_default` runs over 100 escape probes, each in its own worker, asserting the exact exception, the denial log, no created marker file, no connection to a listening local port, and no canary secret in the output; `the_module_root_exposes_only_python_modules` keeps non-module and hidden files in a root unreadable | Keep every excluded module unavailable even through `__import__`, `sys.modules`, package children, and compatibility shims |
| Host capability module | yes | `proven` | Every interpreter call that reaches host state is in `src/host/` (`clock`, `module_root`, `worker`); `host_capability_lint` greps `src/` for file, environment, process, network, clock, entropy, raw OS, stdio, and current-directory calls and fails on any outside `src/host/` except the counted parent-side (`src/product.rs`, `src/binding.rs`) and CLI (`src/sandbox.rs`) call sites listed with their reasons | Add new host access only in `src/host/`; a new parent or CLI call site needs an `ALLOWED` entry with its reason |
| Host clock and local zone | yes | `proven` | `datetime` reads time only through `src/host/clock.rs`: the real wall clock by default, or a pinned instant (`HostClock::Fixed` through `Sandbox::with_clock` or `RuntimeOptions::with_clock`); the local zone is always UTC and `TZ` and `/etc/localtime` are never read (`sandbox_boundary_reads_the_wall_clock_in_a_utc_local_zone`, `a_pinned_clock_makes_datetime_deterministic_in_runs_and_sessions`, `cpython_datetime_pinned_clock_subset`, and the `cpython_datetime_*_diff` cases, whose CPython oracle runs with `TZ=UTC`) | Keep every clock and local-time read in the provider and change the default only there |
| Instruction budget | yes | `proven` | `instruction_budget_*` tests cover top-level code, functions, generators, `exec`, imported source modules, and native regular expression compilation and matching (catastrophic backtracking), and `datetime` `strptime` matching, which runs on an explicit stack; CLI and `SandboxPolicy` use a finite default | Keep one shared budget across every nested VM path |
| Wall-clock deadline | yes | `proven` | The single public `mnpy` parent enforces a finite worker deadline across parsing, compilation, VM execution, and shutdown; `wall_clock_budget.py` is run in real CPython and real `mnpy` | Keep the deadline enabled on every supported host and terminate plus reap timed-out workers |
| Call-depth guard | yes | `proven` | Python frames count against `sys.getrecursionlimit()` with CPython 3.14.6 semantics (default 1000, any larger positive limit accepted, an optional operator cap), and exceeding it raises a catchable `RecursionError` (`call_depth_guard_*`, `sandbox_policy_call_depth_guard_*`, the recursion `cpython_*_diff` cases, `sandbox_boundary_recursion_*`, and the product recursion tests at caps 1000 and 10000). Python runs on a dedicated, explicitly sized interpreter stack, call instructions leave the interpreter loop's frame before the callee runs, and a native stack guard turns deep interpreter recursion (repr, comparison, hashing, JSON, deepcopy, iterator chains, parser, compiler, argparse, regular expression parsing and compilation; the regular expression matcher never recurses natively) into `RecursionError`/`MemoryError` (`native_stack_guard_*`); calls share the callee's bytecode, and the estimated heap of live frames is bounded at twice the interpreter stack, so recursion through large function bodies ends in the same catchable `Stack overflow` RecursionError instead of exhausting worker memory (`sandbox_boundary_recursion_through_large_frames_ends_in_recursion_error`); deep values are freed iteratively (`deep_structures_*`, plus the opt-in `million_level_*` run). By default the CLI, `Sandbox`, and `SandboxPolicy` impose no operator cap, matching CPython; raised limits deeper than the interpreter stack end in CPython's `Stack overflow (used N kB)` RecursionError | Keep every recursive native path guarded and the frame limit shared across functions, dynamic execution, imports, generators, and coroutines; move Python frames off the native stack (M1b) so raised limits reach CPython's depths |
| Heap/allocation guard | yes | `proven` | A shared monotonic VM materialization budget recursively charges core values and supported stdlib buffers (including the regular expression matcher's backtracking stack beyond 64 KiB, `io.StringIO` buffer growth, and written `csv` records), with separate incremental mutation charges and the existing 64 MiB single-allocation guard. The single public `mnpy` entrypoint always runs compilation and execution in a child process: Unix uses kernel address/data limits where supported, while macOS monitors physical footprint and kills the worker on overflow. `allocation_budget_*` covers VM paths and `sandbox_process_contains_compiler_memory_pressure` proves compiler/AST containment outside VM accounting | Keep `mnpy` sandboxed by default with no public bypass; in-process APIs do not claim a complete host-memory boundary |
| Captured-output guard | yes | `proven` | `output_budget_*` and `sandbox_policy_output_budget_*` bound complete lines, partial prints, `exec`, and imported-module output under one shared byte budget; CLI and `SandboxPolicy` use a finite default | Keep every buffered-output path on the shared budget and avoid double charging nested VM output during merge |
| Terminal conditions | yes | `proven` | Instruction, allocation, and output budgets, the native-stack ceiling, wall-clock and memory termination, and worker failures are a typed `TerminalKind` (`src/terminal.rs`) reported in `ExecutionResult::terminal`, never a Python exception: a per-thread latch stops exception dispatch, error-to-exception conversion, every instruction boundary, and every VM run once one trips (`no_handler_survives_a_terminal_condition` covers each VM kind inside generators, `__exit__`, `finally`, `except*`, coroutines, `re.sub` callables, sort keys, `json` hooks, `__str__`/`__repr__` from builtins, `exec`, and class bodies; `swallowing_the_carrier_in_native_code_does_not_resume_python`; `terminal_conditions_are_typed_and_uncatchable_through_a_real_worker`; `parent_side_terminal_conditions_are_typed`) | Keep every new budget or ceiling a `TerminalKind` tripped through `terminal::trip` |
| Denial log | yes | `proven` | Blocked imports, host builtins looked up by name, and host attributes of allowed modules are recorded per execution as bounded, deduplicated `Denial { kind, target, count }` entries in `ExecutionResult::denials` and the Python binding (`src/denial.rs`; `denials_are_deduplicated_counted_and_bounded`, `denials_record_blocked_capabilities_even_when_caught`, `every_host_capability_is_denied_by_default`) | Record every new denied capability through `crate::denial` |
| Batch CPython parity | yes | `proven` | Fixed 3.14.6 oracle, curated corpus, four-layer fixed-seed generation, real-process comparison, automatic minimization, categorized JSON/Markdown reports, root-cause grouping, filters, and priority gates | Keep generated and curated discovery representative; no open `must_fix` or `should_fix` root cause may remain at release |
| Security regression suite | yes | `proven` | Import boundary, symlink escape, dynamic `__import__` / from-import / dotted-child cache injection, instruction, call-depth, output, VM allocation, source-size, process-memory, and worker-crash containment regressions run in the release gate. `sandbox_boundary` additionally exercises every CLI budget, all source channels, compatibility-shim rejection, cache injection, and root-module policy through the official process entrypoint | Keep the process entrypoint and every policy/budget regression in both focused and complete gates; maintain the test-set taxonomy in `tests/README.md` |
| Worker start-up hygiene | yes | `proven` | The parent spawns every worker with `env_clear()` plus only the internal marker; before reading a request the worker closes descriptors above stderr, clears its environment, changes to `/`, sets umask `077`, and sets `RLIMIT_CORE` to 0 (`src/host/worker.rs`; `worker_setup_clears_environment_directory_descriptors_and_core_dumps` re-executes the test binary with a secret variable and a leaked descriptor; `workers_start_with_a_cleared_environment_in_root_and_only_stdio` checks a real session worker's initial environment, working directory, and open descriptors) | Keep the hygiene ahead of the first request read; the privilege drop (seccomp, Landlock, `no_new_privs`, a macOS sandbox profile) is the next step |
| Structured Rust execution API | yes | `proven` | `tests/product.rs` runs `run`, `eval`, and `check` through a real worker and verifies exact output, inert values, structured exceptions, `sys.argv` injection, CPython `SystemExit` exit codes, usage, and pre-spawn input rejection | Keep the CLI and Rust API on the same worker protocol and never expose VM objects as host values |
| Authorized external functions | yes | `proven` | Product tests cover explicit registration, absent names, positional and keyword transport, nested Python calls, catchable built-in host errors, unknown-error fallback, panic containment, opaque-value rejection, and input-name conflicts | Keep callbacks opt-in by exact name and restrict both directions to inert transport values |
| Persistent isolated sessions | yes | `proven` | Product tests prove globals, functions, mutations, inputs, and module cache persist without source replay; worker limits close the entire session and internal session-worker mode rejects direct invocation; `every_execution_and_every_session_gets_its_own_worker` logs real worker PIDs to prove one new, reaped worker per one-shot execution and exactly one unshared worker per session, including concurrent sessions from a cloned `Sandbox` | Keep one isolated worker per session, serialize calls, reapply per-call budgets, and reap workers on close/drop |
| Pure Python binding | yes | `proven` | `tests/product.rs` runs the real CPython 3.14.6 client through the private JSON broker and isolated worker, covering structured values, exceptions, explicit callbacks, persistent sessions, timeout closure, constructor failure, input validation, reentrancy rejection, and disconnect cleanup; process tests reject direct broker invocation | Keep the package dependency-free and prevent any in-process VM, C ABI, shell, or worker-bypass path |
| Support and stop-line docs | yes | `proven` | `README.md`, `README_CN.md`, `AGENTS.md`, this checklist, runtime coverage, migration notes, and the stdlib manifest describe the current supported and excluded boundary | Keep the checklist and public support matrix synchronized with implementation changes |

## Release Gate

`tests/run.sh` runs the complete test suite. A release candidate must pass
`tests/run.sh --release-gate`, the complete release gate, plus the manual
checklist it writes; `RELEASING.md` gives the release procedure. During focused
sandbox-control development, `tests/run.sh --focused` runs the security-policy
slice plus a small generated discovery set; focused mode is not sufficient for
release completion. `tests/run.sh` is the only supported entrypoint.

```bash
MINIPYTHON_CPYTHON_SRC=/path/to/cpython-3.14.6 tests/run.sh --release-gate
tests/run.sh --release-gate --keep-going   # run every step, report every failure
```

The gate (`tests/release_gate/gate.py`) runs these steps in order, stops at the
first failing one unless `--keep-going` is given, and writes
`reports/release-gate/report.md` and `report.json` (gitignored) with each
step's numbers and logs under `reports/release-gate/logs/`:

0. Preflight: the working tree is committed (`--allow-dirty` is for gate
   development only), the oracle is CPython 3.14.6, `MINIPYTHON_CPYTHON_SRC`
   is a 3.14.6 source checkout (`Include/patchlevel.h`), the versions in
   `Cargo.toml`, `npm/package.json`, `python/pyproject.toml`, and
   `sys.implementation.version` agree, and `CHANGELOG.md` has that version.
1. (a) `cargo fmt --check`, then `cargo test --release --all-targets --locked
   --no-fail-fast` with `MINIPYTHON_REQUIRE_CPYTHON_SRC=1`. Any failed test,
   any `SKIPPED` line, and any ignored test other than the documented
   `million_level_structures_are_freed_without_native_recursion` (about 2 GB
   per structure) fails the step: skipped evidence is never a pass. This step
   also runs the security suites: the capability contract,
   `host_capability_lint`, worker process isolation, and the budget tests.
2. (b) `tests/test_pipeline.py`, then the curated corpus plus the 1024
   fixed-seed generated cases (seed `20260710`) against the release `mnpy`,
   with shrinking and `--fail-priority must_fix,should_fix`. The status and
   triage counts must equal `tests/release_gate/discovery-baseline.json`
   exactly (today 794 `MATCH`, 266 accepted gaps). A changed count fails the
   gate; `--update-discovery-baseline` rewrites the file from the run (which
   still fails), and that rewrite must be committed on its own with the reason
   for every changed count.
3. (c) The oracle corpora in `tests/oracles/` (below): every scenario identical
   in CPython and `mnpy` apart from the exclusions listed in
   `tests/oracles/corpora.toml`, each with its reason; an exclusion that
   starts matching is stale and fails the step.
4. (conformance) The layer-zero conformance corpus of `tests/conformance/`:
   `tests/conformance/run.py` compares the release `mnpy` with the recorded
   CPython 3.14.6 observations of `tests/conformance/cases/` and must exit 0
   (every difference is a reviewed entry of `expected-failures.toml`);
   `tests/conformance/status.py --check` must find the conformance status
   block of the root `README.md` up to date; `tests/conformance/run.py
   --check-oracle --cpython P` compares the recorded observations with the
   pinned CPython, writes no oracle or report (the cases it executes are
   trusted, tracked code), and must exit 0, so a difference of the official
   CPython 3.14.6 on a host fails the step as oracle drift; and
   the unit tests of the corpus, which cover the step itself, must pass.
5. (d) `tests/release_gate/leak_scan.py` over every tracked file and over the
   messages and identities of every commit since the last `v*` tag (since the
   root while there is none): home directories, `/Volumes`, `/private`
   temporary paths, private host names (`ts.net`, `.local`, `.lan`,
   `.internal`), IP addresses outside loopback and the documentation ranges,
   email addresses other than the author's and documentation domains, and
   token shapes. Only entries of `tests/release_gate/leak-allowlist.toml`,
   each with its reason, are accepted, and an unused entry fails.
6. (licensing) The dual licence (`tests/release_gate/licensing.py`, step key
   `licensing`): every tracked source file (`*.rs`, `*.py`, `*.sh`, `*.js`,
   and `*.toml` under `src/`, `tests/`, `ci/`, `npm/`, `python/`,
   `examples/`, and `homebrew-tap-template/`) carries an
   `SPDX-License-Identifier:` header within its first five lines, or a path
   rule of the root `REUSE.toml` covers it; the header has to be in a
   comment, a tag inside a string is text and not a header; only `MIT`,
   `PSF-2.0 AND MIT`, and `MIT AND PSF-2.0` are allowed. A file whose
   header names PSF-2.0 needs a line that starts with
   `// Derived from CPython` (or `# Derived from CPython`) and a
   `Changes:` line in its first ten lines, and must be listed in the root
   `NOTICE`; the NOTICE holds nothing but paths, one per line, and every
   path it lists must exist and carry a PSF-2.0 header of its own (a
   REUSE.toml rule does not make the entry a PSF-2.0 file), so a stale
   entry fails.
   `LICENSES/MIT.txt` and `LICENSES/PSF-2.0.txt`
   must exist and not be empty. `cargo metadata --format-version 1 --locked`
   must find no dependency whose licence expression offers no licence of
   the allowlist (MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC,
   Unicode-3.0, Unicode-DFS-2016, Zlib, 0BSD, CC0-1.0, PSF-2.0, and
   `Apache-2.0 WITH LLVM-exception`; `OR`, `AND`, and parentheses are
   allowed). The Unlicense is a public-domain dedication and is no
   licence of its own here: memchr, a transitive dependency of
   serde_json, is dual licensed (Unlicense OR MIT) and the project uses
   it under MIT, so a choice with a branch of the allowlist passes,
   while a dependency published under the Unlicense alone fails. Every
   CPython file a derived file names has to exist in the CPython source
   tree, and the NOTICE at the root of the repository and the copy the
   npm package carries have to name the same origin the header of the
   file names. Finally
   `tests/release_gate/provenance_names.py` compares every tracked Rust
   file that declares no PSF-2.0 licence (also the ones outside the
   directories that need a licence header) with the CPython source tree in
   `$MINIPYTHON_CPYTHON_SRC`: a file
   that shares four or more internal names with one CPython source file is
   derived from CPython after all, so it fails the step unless it is
   declared as `PSF-2.0 AND MIT` and listed in NOTICE. The step runs the
   unit tests of `tests/release_gate/test_licensing.py` first.
7. (e) Packaging smoke: `cargo build --release --locked` for the host's
   release target (on Linux the musl target the release ships), the release
   archive from `ci/package-release.sh` with its `checksums.txt`,
   `npm test`, `npm pack`, and a global install of the packed tarball into a
   throwaway prefix whose `postinstall` reads the local archive
   (`MINIPYTHON_NPM_ARCHIVE_DIR`) and verifies its checksum; the installed
   `mnpy --version` and two `examples/sandbox/argparse_cli.py` runs (one
   success, one usage error) must match CPython's exit status, stdout, and
   stderr. The installed binary also runs `tests/release_gate/math_probe.py`:
   `math.cbrt` against CPython with the exclusions of
   `tests/oracles/math/cbrt-exclusions.txt`, and the C library functions of
   `tests/oracles/math/libm_probe.py` against the build the test suites ran,
   which must agree apart from the listed entries of
   `tests/oracles/math/libm-shipped-differences.txt` (see "Platform-dependent
   behavior"). On macOS the Homebrew formula template is rendered for the local
   archive, installed from a throwaway local tap, tested with its `test do`
   block and the same smoke runs, audited with `brew audit --strict`, and
   removed again; the real tap is never touched. On Linux the rendered formula
   is only syntax-checked when Ruby is present; the macOS gate covers it.
8. (f) The manual checklist the script cannot do is written into the report
   with blanks to fill in: the real-program corpus rerun on the candidate (no
   regressions against the previous release), the two independent reviews of
   the release diff with verified findings and fixed blockers, and the Linux
   gate result (`ci/linux-gate.Dockerfile`).

### Dual licence

MiniPython is dual licensed: the files we wrote ourselves are MIT, and the
files derived from CPython are `PSF-2.0 AND MIT`. The release gate checks
that claim on every candidate (the `licensing` step).

- Every tracked source file (`*.rs`, `*.py`, `*.sh`, `*.js`, and `*.toml`
  under `src/`, `tests/`, `ci/`, `npm/`, `python/`, `examples/`, and
  `homebrew-tap-template/`) carries an `SPDX-License-Identifier:` line
  within its first five lines, or a path rule of the root `REUSE.toml`
  covers it. The line has to be in a comment of the language of the file
  (`//` for Rust and JavaScript, `#` for Python, shell and TOML); a tag
  inside a string (a multi-line string, a doc example) is text and not a
  header:

  ```toml
  version = 1

  [[annotations]]
  path = "tests/**"
  SPDX-License-Identifier = "MIT AND PSF-2.0"
  ```

  Only `MIT`, `PSF-2.0 AND MIT`, and `MIT AND PSF-2.0` are allowed.
- A file derived from CPython says so in its first ten lines and is listed
  in the root `NOTICE`, one path per line:

  ```rust
  // SPDX-License-Identifier: PSF-2.0 AND MIT
  // Derived from CPython Lib/argparse.py
  // Changes: rewritten in Rust, the public behaviour is unchanged.
  ```

  The NOTICE holds nothing but paths: every line that is not empty and
  not a comment names a file, and a line that names no file of the
  repository fails the step. Every path NOTICE lists must exist and carry
  a PSF-2.0 header of its own, so a stale entry fails the step. A
  REUSE.toml rule does not make a NOTICE entry a PSF-2.0 file: the file
  has to say so itself.

  Every CPython file a derived file names has to exist in the CPython
  source tree of `$MINIPYTHON_CPYTHON_SRC` (`Objects/complexobject.c`,
  not `Python/complexobject.c`), and the NOTICE at the root of the
  repository and the copy the npm package carries have to name the same
  origin the header of the file names.
- `LICENSES/MIT.txt` and `LICENSES/PSF-2.0.txt` hold the two licence texts.
- The Cargo dependencies may only use MIT, Apache-2.0, BSD-2-Clause,
  BSD-3-Clause, ISC, Unicode-3.0, Unicode-DFS-2016, Zlib, 0BSD,
  CC0-1.0, PSF-2.0, and `Apache-2.0 WITH LLVM-exception` (`OR`, `AND`,
  and parentheses are allowed). The expression has to offer a licence of
  the allowlist: a choice (`OR`) passes when one of its branches is
  allowed, a conjunction (`AND`) only when every one of its factors is.
  The Unlicense is a public-domain dedication and is no licence of its
  own here: memchr, a transitive dependency of serde_json, is dual
  licensed (Unlicense OR MIT) and the project uses it under MIT, so it
  passes as a branch of a choice, while a dependency published under the
  Unlicense alone fails.
- `tests/release_gate/provenance_names.py` compares the internal names of
  every tracked Rust file that declares no PSF-2.0 licence (also the ones
  outside `src/`, `tests/`, and the other directories that need a licence
  header) with the CPython source tree in
  `$MINIPYTHON_CPYTHON_SRC` (the gate requires it). The names of our own
  files count when they are written in code style (an underscore or mixed
  case); the CPython index holds every identifier CPython has, so the
  `privatealpha` of CPython is the `private_alpha` of ours. String
  literals and comments are removed (a Rust string may span lines, a
  lifetime is not a string; the comments of the language are read before
  the strings, so the quote of a comment does not open a string and a name
  in a comment is not a name), the names are normalised (no underscores,
  lower case), and hex-like tokens are ignored. A name that occurs in at most
  three CPython source files (every source file type under `Lib`,
  `Modules`, `Objects`, `Python`, `Parser`, and `Include`, without
  `Lib/test`, `idlelib`, `clinic`, `_testcapi*`, `tkinter`, and
  `turtledemo`) and that is not public vocabulary (the names in the
  `` `code` `` spans, `role` spans, and directive signatures of
  `Doc/**/*.rst` (a signature counts with its parameters, not with the
  prose behind it), the names of `Lib/token.py`, `Lib/keyword.py`,
  `Lib/encodings/aliases.py`, `Parser/Python.asdl`, `Grammar/python.gram`,
  and `Include/internal/pycore_token.h` (the strings of those files count
  as well: the keywords of `Lib/keyword.py` and the aliases of
  `Lib/encodings/aliases.py` are string literals), and every name built
  from a module name or a public name followed by a public name) is an
  internal name. A file that shares four or more of them with one CPython
  file is reported as
  `file:1: provenance: N internal names shared with REF: a, b, c` and
  fails the step: such a file is derived from CPython and must say so.

### Oracle corpora

`tests/oracles/run_oracles.py` generates each corpus of
`tests/oracles/corpora.toml` under `reports/release-gate/oracles/`, runs every
program in the pinned CPython (`-E -s -B -W ignore`, `TZ=UTC`, `LC_ALL=C`,
matching the sandbox's UTC clock and lack of locale) and in the release `mnpy`
(with raised step, time, output, allocation, and source limits), and compares
them. Randomized generators are ours and use fixed seeds. Extractors read
CPython's own `Lib/test` from the pinned install at gate time and write the
derived programs under `reports/` only; nothing extracted from CPython is
committed.

| Module | Corpus | Source | Seeds x size |
| --- | --- | --- | --- |
| `re` | `test-re-extract` | extractor over `Lib/test/test_re.py` (constant `re.*` calls and pattern/template error checks) | - |
| `re` | `re-tests-table` | extractor over `Lib/test/re_tests.py` (each row as search, findall, ignore-case, fullmatch, sub, split, bytes, and pos/endpos calls) | - |
| `re` | `api` | hand-written module surface, flags, objects, and errors | - |
| `re` | `random-ascii`, `random-unicode`, `random-bytes`, `random-errors` | random patterns, subjects, flags, and API calls; `errors` mutates patterns to be invalid | 10-19 x 4000 each |
| `re` | `deep-captures` | random nested groups, repetition, backreferences, and conditionals | 1-5 x 4000 |
| `datetime` | `random` | random construction, arithmetic, comparison, formatting, parsing, and time zones | 1-3 x 80 programs x 300 |
| `csv` | `random` | random writer/reader round trips, dialects, `Sniffer`, and `io.StringIO` operations | 1-3 x 600 programs |
| `csv` | `test-csv-extract` | `Lib/test/test_csv.py` run method by method on a small unittest shim | - |
| `io` | `test-memoryio-extract` | the `StringIO` classes of `Lib/test/test_memoryio.py` on the same shim | - |
| `exceptions` | `report-probes` | uncaught-exception reports (stderr compared without CPython's frame lines) | - |
| `exceptions` | `handlers` | hand-written `except`, `except*`, and handler-flow programs | - |
| `syntax` | `rejected-in-3.14.6` | syntax a newer development grammar accepts and 3.14.6 rejects (`lazy import`, unpacking in comprehensions) | - |
| `str` | `unicode-predicates` | `str.is*` predicates and whitespace handling, including every code point | - |

The documented exclusions are `dir(re)` (CPython's implementation imports and
the undocumented `re.Scanner`), a `test_csv` test that needs the `string`
module, and the exit status of an uncaught `KeyboardInterrupt` (CPython
re-raises `SIGINT`; `mnpy` exits 1). A corpus may also list
`platform_exclusions`: the `datetime` corpus does, for the scenarios whose
result the platform C library decides, which are skipped on Linux and counted
as documented exclusions there (see "Platform-dependent behavior" below). The
step takes about two minutes on an M-series Mac with eight jobs; the whole
gate takes about ten minutes, dominated by the release build and `cargo test`.

### Platform-dependent behavior

CPython hands some work to the platform C library, so the oracle itself
differs between macOS and Linux. MiniPython matches the CPython of the
platform it runs on wherever it can, and the diff suites and oracle corpora
stay strict on both: the same programs, and only the platform-scoped
exclusions listed here.

- `math.cbrt`: CPython returns the C library's `cbrt`. On macOS MiniPython
  calls the same system `cbrt` (Rust's `f64::cbrt`), so the two match. On
  Linux it uses the `libm` crate's correctly rounded `cbrt` (MIT, a port of
  CORE-MATH's) for gnu and musl builds alike: `f64::cbrt` would be
  compiler-builtins' in a gnu build but musl's own in the static musl release,
  and the two differ. Linux CPython uses GNU libc's, which (through 2.43)
  errs by up to two units in the last place (`cbrt(27.0)` is
  `3.0000000000000004`). MiniPython does not reproduce it: that would mean
  copying LGPL code, and glibc's development branch has itself replaced it
  with a correctly rounded `cbrt`. The differences are a Linux-scoped
  exclusion: `tests/oracles/math/cbrt-exclusions.txt` lists, per
  architecture (24 inputs on aarch64, 26 on x86_64), each input of
  `cbrt_case.py` whose oracle result differs and that result. Every other
  input must match exactly, a listed input that matches or is not exercised
  fails, and mnpy must stay within two units in the last place. Both
  `cpython_math_cbrt_diff_subset` and the release gate's check of the shipped
  binary apply these rules.
- `math.log`, `log2`, `log10` of integers too large for a float: CPython's
  result is `log(x) + log(2) * e` for the integer written as `x * 2**e` with
  a correctly rounded `x` in `[0.5, 1)`. On Apple silicon the oracle computes
  the sum as one fused multiply-add (clang contracts it); on Linux and Intel
  macOS it rounds the product first. MiniPython does the same per target
  (`cpython_math_big_int_log_diff`).

- `complex ** complex`: CPython computes the power with the platform C
  library, so its last bits differ between macOS and Linux, while MiniPython
  computes a power with a complex exponent one fixed way, in polar form. The
  two results differ by a few units in the last place: `(1+2j) ** (3+4j)` is
  `(0.12900959407446697+0.03392409290517014j)` on macOS and
  `(0.129009594074467+0.03392409290517014j)` on Linux. Bit-exact agreement on
  every platform is not a goal for 0.1.0, so the bit-exact diff case
  `cpython_complex_power_and_repr_diff_subset` no longer holds the two
  complex-power expressions. The new diff test
  `cpython_complex_power_accuracy_diff` runs those two expressions and
  twelve more pairs with a non-integer exponent, mixed signs and magnitudes
  from 1e-3 to 1e3 through MiniPython and CPython, and holds every real and
  imaginary component of the two results to eight units in the last place;
  the special values (an infinite or a not-a-number operand, a zero and a
  signed zero base, `OverflowError` and `ZeroDivisionError`) have to agree
  exactly. The special values are compared component by component and
  exactly: the sign of a zero is part of the result, so the cases produce
  negative zeros in the real component and in the imaginary component
  (which further cases do so depends on the platform's C library), and the
  test fails when the cases stop producing them. An infinite
  result of a finite operand never reaches the caller: CPython turns it into
  an `OverflowError: complex exponentiation` and MiniPython does the same,
  so that outcome is held exactly too; the rules of the comparison for an
  infinite component (an infinity is not a finite number and not the other
  infinity) are held by `complex_power_special_values_are_compared_exactly`.

- `datetime.strftime`: CPython hands the format to the platform C library, so
  its results differ between macOS and Linux, while MiniPython renders every
  format one fixed way: the way macOS's C library does. On macOS the two
  agree exactly. On Linux they may differ from Linux CPython for two groups
  of inputs: years below 1000 with `%Y`, `%G`, `%C`, `%F`, `%c` or `%x`,
  because glibc does not zero-pad the year the way macOS does (the year 999
  is `0999` on macOS and `999` on Linux), and unknown or platform-specific
  directives and the padding flags on them and on `%y`, `%C`, `%g`, `%G` and
  `%Y`, which the two C libraries render differently. Those cases are a
  documented, Linux-scoped exclusion and not a pass. The datetime oracle
  corpus marks the scenarios it generates that really call an affected
  format (`tests/oracles/datetime/random_corpus.py` writes the marks to
  `<program>.platform-keys.json`); a scenario that computes its result itself
  is never marked, so it is compared on every platform: that is `ctime()`,
  which CPython formats itself, with a zero-padded four-digit year, and
  `isocalendar()`, `isoweekday()` and `timetuple()`, which are pure
  computations. On Linux `tests/oracles/run_oracles.py` skips the marked
  scenarios and counts every one of them as a documented exclusion with the
  reason `platform strftime (Linux)`, never as a pass; a scenario `mnpy` did
  not print is a mismatch, never an exclusion.
  `cpython_datetime_strftime_and_format_diff` compares the year-bearing
  directives of a year below 1000 apart and skips exactly them on Linux.
  Every other scenario and directive is still compared on both platforms.
  `tests/oracles/test_platform_exclusions.py` holds the regression tests of
  the marks and of the runner's accounting; the
  `cpython_datetime_strftime_platform_exclusion_diff` test runs them. A
  glibc-faithful `strftime` was rejected for this release because it would
  mean translating LGPL glibc code.

- The sign of a NaN that arithmetic produced is not defined by Python: it
  depends on the hardware and on the build of the interpreter, so CPython
  does not agree with itself across platforms and builds. The `nan-as-repr`
  and `neg-nan-as-repr` cases of `cpython_float_inf_nan_string_subset` and
  of its differential twin `cpython_float_inf_nan_string_diff_subset`
  (`tests/parity.rs`) therefore print the representation, the strings and the
  infinity and not-a-number predicates of those two values, and not their
  sign: on x86_64 the CPython of the `python:3.14-slim` image gives `-1.0`
  where the CPython of `actions/setup-python` gives `1.0`, and on aarch64
  every build gives `1.0`. The sign of a parsed NaN (`float('-nan')`) and of
  an infinity is defined, and the tests still hold those exactly.

- `datetime.strftime` with a `timetuple()` override that returns something
  else than a tuple: CPython's own rejection of the wrong type differs
  between the platforms: Linux reports `'int' object does not support
  indexing`, macOS reports `Tuple or struct_time argument required`.
  MiniPython always reports the macOS message. The case is therefore
  compared apart, in the platform part of
  `cpython_datetime_strftime_and_format_diff`: macOS compares it exactly and
  Linux skips it and reports it as an excluded platform case, together with
  the year-bearing directives of a year below 1000. The exclusion is
  registered in `tests/oracles/corpora.toml` (`platform_exclusions`) and
  guarded by `tests/oracles/test_platform_exclusions.py`.

- The shipped Linux binary: the release links musl statically, so the
  math functions CPython takes from GNU libc come from musl's libm in it,
  while the gnu build the suites test imports GNU libc's and matches Linux
  CPython. Gate step (e) runs `tests/oracles/math/libm_probe.py` (200 inputs
  each of 30 functions) through both: on aarch64 they differ in `acos` (19),
  `asin` (11), `atan2` (23), `cos` (7), `log10` (6), `sin` (13), `sinh`
  (23), `tan` (8), and `tanh` (1), each by the last place or two. Those
  entries are listed, not fixed, in
  `tests/oracles/math/libm-shipped-differences.txt`; any other difference
  fails the gate, and a listed one that disappears is stale. On x86_64 musl
  differs from GNU libc in the same functions plus `acosh` (1), in 112
  inputs in all; those entries were measured on GitHub Actions
  ubuntu-latest and are listed in the same file.

### Linux gate

`ci/linux-gate.Dockerfile` builds an image with a Rust toolchain, the musl
target and tools, Node.js, and CPython 3.14.6 from the official
`python:3.14.6` image, plus the CPython v3.14.6 source tree for
`Grammar/python.gram` and, through `MINIPYTHON_CPYTHON_LIB`, for the
`Lib/test` files the image strips, and runs the same gate inside it
(RELEASING.md gives the command).

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
| Numbers | Integer, bigint, bool, float, complex literals; common arithmetic, comparisons, conversion, formatting, and public numeric protocols | Locale-sensitive formatting, exhaustive platform floating-point internals, every CPython diagnostic variant, and the last bits of `math.erf`, `erfc`, `gamma` and `lgamma`: MiniPython takes them from the MIT `libm` crate (the same on every host), CPython from the platform C library (erf/erfc, so macOS and Linux differ) or its own approximation (gamma/lgamma); `cpython_math_special_function_accuracy_diff` bounds the gap at 8 ulps for erf/erfc and 16 for gamma/lgamma (lgamma in ulps of max(\|result\|, 1)), and special values, poles, overflow and exact factorials match exactly |
| Strings and interpolation | Unicode strings, bytes literals, common string methods, formatting, f-strings, t-strings, escapes, and public tokenize split output | CPython's parser-internal token representation, full codec registry, locale behavior, and lone-surrogate storage outside Rust `String` |
| Containers | List, tuple, dict, set, frozenset, comprehensions, iteration, slicing, unpacking, mutation, comparison, and recursive display | CPython allocation/performance stress contracts, C-level layout, GC tracking, and thread-safety internals |
| Object model | Classes, inheritance, MRO, descriptors, properties, bound methods, metaclasses used by the supported surface, and public type metadata | C layout compatibility, weakref/GC lifecycle parity, refcounts, and unrestricted metaclass/internal slot parity |
| Functions and scope | Positional/keyword binding, defaults, annotations, closures, globals, nonlocals, decorators, recursion under sandbox limits, generators, coroutines, and async generators | CPython frame/opcode identity, exact stack size, specialization, and debugger integration |
| Exceptions | Builtin hierarchy, custom exceptions, chaining, traceback objects used by the supported runtime, exception groups, and public attributes | CPython traceback/frame implementation identity and interpreter-shutdown lifecycle |
| Bytes and bytearray | Construction, indexing/slicing, search, split/join, case and predicate methods, translation, mutation, copying, and sandbox allocation behavior | Binary pickle-byte identity, exhaustive full-buffer exporters, locale behavior, and C ABI matrices |
| Array | Pure-memory construction and mutation for the documented typecodes, slicing, iteration, bytes/list conversion, copying, and BytesIO transfer | Real file APIs, pickle reconstruction internals, platform C ABI layout guarantees, and every unsupported typecode edge |
| Memoryview | One-dimensional bytes, bytearray, and supported array exporters; read/write, slicing, cast subset, attributes, release, hashing, and BytesIO `readinto` | Multidimensional subviews/casts, ctypes exporters, full buffer protocol, GC callback timing, and refcount matrices |
| Async runtime | `async def`, await, async iteration/context management, coroutine/generator protocols, and supported ABC behavior | Event loops, threads, signals, sockets, subprocess integration, and host scheduling |
| io.StringIO | Native `io.StringIO` (`src/vm/stringio.rs`, an ordinary `_io.StringIO` class whose methods are Rust builtins, built with `src/vm/native_class.rs`): the constructor with `initial_value` and every `newline` mode (`None` translating, `''` universal, `'\n'`, `'\r'`, `'\r\n'`) and the `newlines` record; `read`, `readline`, `readlines` with sizes and hints; `write`, `writelines`, `getvalue`; `seek`/`tell` on code-point positions with only zero current/end-relative seeks; overseek NUL padding and mid-buffer overwrites; `truncate`; iteration; `close`/`closed` with CPython's closed-file messages (its own methods without, the inherited `_IOBase` ones with the trailing period); the context manager; `readable`/`writable`/`seekable`/`isatty`, `fileno`/`detach` raising `io.UnsupportedOperation`, `line_buffering`, `encoding`, `errors`, `_check*`; `__getstate__`/`__setstate__`, `copy.copy`/`copy.deepcopy`, instance `__dict__`, subclasses (`__next__` reaches `readline`, `writelines` reaches `write`, and `__exit__` reaches `close` through attribute lookup, as in CPython), and uninitialized subclass instances; `print(file=...)` and rebinding `sys.stdout`. Buffer growth, including overseek padding, is charged to the allocation budget | `io.TextIOBase`/`io.IOBase` and the `_io._TextIOBase`/`_io._IOBase` bases (the MRO is `StringIO, object`); pickling (`pickle` is not in the sandbox); lone surrogates (strings are Rust `String`s); method `__doc__` strings; `__sizeof__`; runtime-wide gaps StringIO inherits: `io.UnsupportedOperation.__module__` reads `builtins` (shared with `BytesIO`) and `dir()` uses the VM's generic instance listing |
| argparse | Native Rust `argparse` (`src/vm/argparse*.rs`): every public and private class CPython 3.14.6 defines (`ArgumentParser`, `Namespace`, `Action` and all built-in actions including `BooleanOptionalAction`, the five help formatters, `FileType`, `ArgumentError`, `ArgumentTypeError`, `_SubParsersAction`, `_ActionsContainer`, `_ArgumentGroup`, `_MutuallyExclusiveGroup`) with methods reached through attribute lookup so subclass overrides apply where CPython applies them; all constructor and `add_argument` parameters, nargs forms, type conversion and its error messages, choices and `suggest_on_error`, defaults, negative-number and `--` handling, abbreviations, groups, mutually exclusive groups, subparsers, parents, conflict handlers, intermixed and known-args parsing, deprecated arguments, help/usage text with CPython's wrapping at the non-terminal width, usage errors on `sys.stderr` with exit status 2 | Help color and `COLUMNS` sizing (the sandbox has no terminal or environment); `FileType` files and `fromfile_prefix_chars` (they reach the sandbox's absent `open` and raise `NameError`); `DeprecationWarning`/`PendingDeprecationWarning` output from `add_argument_group(prefix_chars=...)` and `FileType`; identity-only checks against a separately constructed `'==SUPPRESS=='` string; `FileType('rb'/'wb')('-')` (the in-memory standard streams have no `.buffer`); `_negative_number_matcher`, `_whitespace_matcher`, and `_long_break_matcher` regex attributes and `_colorize` theme internals; runtime-wide limits that argparse inherits: `vars(namespace)`/`namespace.__dict__` are snapshots, so mutating them does not change the namespace, and `ArgumentError` subclasses cannot override `__init__`/`__str__` |
| re | Native Rust `re` (`src/vm/re.rs`, `src/vm/re/*.rs`): `compile`, `search`, `match`, `fullmatch`, `split`, `findall`, `finditer`, `sub`, `subn`, `escape`, `purge`, and the pattern cache; `PatternError`/`error` with `msg`, `pattern`, `pos`, `lineno`, `colno`; `RegexFlag` values (`A`/`I`/`L`/`M`/`S`/`X`/`U`/`NOFLAG`/`DEBUG`) with CPython's reprs, `name`, `value`, and bit operations; `Pattern` and `Match` objects with every attribute and method, `pos`/`endpos`, repr, equality, hashing, copying, `__class_getitem__`, and the non-subclassable, non-instantiable types; the full 3.14 syntax (groups, named groups and backreferences, atomic groups, possessive quantifiers, conditionals, look-ahead and fixed-width look-behind, inline and scoped flags, verbose mode, `\A \Z \z \b \B`, Unicode and ASCII `\d \s \w`, octal/`\x`/`\u`/`\U`/`\N{...}` escapes) with CPython's error messages, positions, `FutureWarning`s, and `RecursionError` nesting limit; CPython's case-insensitive rules and Unicode 16 tables; sre's capture and backtracking semantics, 3.7+ empty-match rules, replacement templates, callable replacements, and the deprecated positional `count`/`maxsplit`/`flags`; `str` and bytes-like subjects. Matching runs on an explicit stack: every matcher step is charged to the instruction budget and stack growth beyond 64 KiB to the allocation budget | `re.Scanner` and the `re._parser`/`re._compiler`/`re._constants`/`re._casefix` internals (and the module's `copyreg`/`enum`/`functools` attributes); `re.DEBUG` output (the flag is accepted, nothing is printed); locale behavior beyond the C locale for bytes patterns; `RegexFlag` as an `enum.IntFlag` class (`repr(re.RegexFlag)`, iterating or indexing the class, `__members__`, and identity of values built with `re.RegexFlag(n)`); pickling and weak references to patterns; group values of a `bytearray` subject are copied when the match is made instead of read from the live buffer; warnings are recorded by `warnings.catch_warnings` and error filters but not printed to stderr; backtracking-heavy matches over long inputs can exhaust the sandbox budgets where CPython completes |
| datetime | Native Rust `datetime` (`src/vm/datetime*.rs`): `date`, `time`, `datetime`, `timedelta`, `tzinfo`, `timezone` (with `utc`, `min`, `max`), `UTC`, `MINYEAR`, `MAXYEAR`, and the `IsoCalendarDate` and `time.struct_time` result types, all subclassable; every constructor and classmethod (`today`, `now`, `utcnow`, `fromtimestamp`, `utcfromtimestamp`, `fromordinal`, `fromisocalendar`, `fromisoformat` with every 3.11+ form, `strptime` for all three types, `combine`) with CPython's argument binding, range checks, and messages; all arithmetic and comparisons with the naive/aware rules, PEP 495 `fold`, and `OverflowError` ranges; `timedelta` normalization with half-even float rounding and every operator by int, float, and timedelta; hashes consistent with equality across zones; `replace`, `copy.replace`, `isoformat`, `ctime`, `strftime`, `__format__`, `timetuple`/`utctimetuple`, `timestamp`, `astimezone`, `utcoffset`/`dst`/`tzname`/`fromutc` through Python `tzinfo` subclasses with CPython's result checks, `__reduce__`/`__reduce_ex__`, and copy/deepcopy through them; `utcnow`/`utcfromtimestamp` `DeprecationWarning`s reach `warnings.catch_warnings` like other runtime warnings; `strftime` in the C locale for the directives macOS and glibc render alike (`%a %A %b %B %c %C %d %D %e %F %g %G %h %H %I %j %k %l %m %M %n %p %r %R %S %t %T %u %U %V %w %W %x %X %y %Y %%`, their `E`/`O` forms, and the `-`, `_`, `0` flags on numeric fields) plus CPython's `%f`, `%z`, `%:z`, `%Z`; the wall clock (or a pinned instant) and the UTC local zone come from `src/host/clock.rs` | `zoneinfo` and IANA zone data; the `datetime_CAPI` capsule; locale-specific names; the `time` module (not allowlisted; `struct_time` is reachable only through `timetuple()`); platform-dependent `strftime` input (`%s`, `%P`, `%+`, `%v`, other letters, flags on `%y %C %g %G %Y`, and `%c` before year 1000), which MiniPython renders one fixed way (unknown directives are copied through as glibc does); host local time zones (the local zone is always UTC); printing `DeprecationWarning` to stderr (the runtime records warnings without printing them); `pickle` itself (not allowlisted; `__reduce__` values match CPython); hash values; a repeated `strptime` directive raises `ValueError` instead of `re.PatternError`; runtime-wide limits: a datetime displayed inside a container shows a user `tzinfo` with the default object repr even when it defines `__repr__`, and container keys compare aware values with different user `tzinfo` objects by identity |
| csv | Native Rust `csv` (`src/vm/csv.rs`, `src/vm/csv/*.rs`) covering both CPython halves as ordinary classes with Rust methods: the `_csv` accelerator (`reader`, `writer`, `_csv.Dialect` exposed as `csv._Dialect` with every parameter and CPython's conversion order, validation checks, and messages, `register_dialect`, `get_dialect`, `unregister_dialect`, `list_dialects`, `field_size_limit`, `Error`) and `Lib/csv.py` (`Dialect`, `excel`, `excel_tab`, `unix_dialect`, `DictReader`, `DictWriter`, `Sniffer`, `__all__`, `__version__`, `QUOTE_MINIMAL`/`QUOTE_ALL`/`QUOTE_NONNUMERIC`/`QUOTE_NONE`/`QUOTE_STRINGS`/`QUOTE_NOTNULL`); the reader's state machine (quoted fields across lines, `strict`, escapes, `skipinitialspace`, line breaks in unquoted fields, non-`str` rows, `line_num`, float and `None` conversion per quoting mode, the field size limit, re-entrance detection); the writer's quoting and escaping decisions, the `need to escape` and empty-field errors, `None` and number formatting, and any target with a callable `write` (user classes, `StringIO`, `sys.stdout`); `DictReader` fieldname laziness, `restkey`/`restval`, and blank-row skipping; `DictWriter` `writeheader`, `restval`, and `extrasaction` errors; the `Sniffer.sniff`/`has_header` heuristics (regular expressions through the native `re`); C types that cannot be instantiated, copied, or given new attributes; the registry and field limit are per-interpreter module state; written records are charged to the allocation budget | Real files (`open()` is blocked until the file provider lands, so inputs are lists, `str.splitlines()`, `StringIO`, and other iterables); locale; `import _csv` (the types keep their `_csv` names); pickling; `DictWriter._dict_to_list` returns an iterator over a list instead of a generator; several unknown `DictWriter` fields are listed in the VM's set order (CPython's is hash-randomized); runtime-wide gaps csv inherits: builtin function reprs (`<builtin _csv.reader>`) and their `__doc__`, the module-less repr of `DictReader[str]`, the keyword-argument message of `csv.Error(x=1)`, and `dir()` listings |
| Future statements | Native `__future__` module (`all_feature_names`, ten `_Feature` objects, `CO_*` flags); placement, unknown-feature, `braces`, and `*` diagnostics with CPython locations; PEP 563 string annotations produced by the `src/unparse.rs` port of CPython's annotation unparser; `compile()` future flags, `dont_inherit`, and `exec()`/`eval()` inheritance; `barry_as_FLUFL` | The `u` string prefix inside string annotations, which MiniPython's AST does not keep |

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
| Call-depth cap turns finite recursion into `RecursionError` | `example_e2e` | `examples/sandbox/call_depth_budget.py` | `real_cpython_completes_while_mnpy_enforces_call_depth_budget` |
| Recursion under the defaults matches CPython, including the uncaught report | `example_e2e` | `examples/sandbox/recursion_limit.py` | `real_cpython_and_mnpy_match_for_recursion_under_the_defaults` |
| Captured-output budget stops oversized output | `example_e2e` | `examples/sandbox/output_budget.py` | `real_cpython_completes_while_mnpy_enforces_output_budget` |
| VM allocation budget stops value growth | `example_e2e` | `examples/sandbox/allocation_budget.py` | `real_cpython_completes_while_mnpy_enforces_allocation_budget` |
| Parent wall-clock deadline covers pre-VM and VM work | `example_e2e` | `examples/sandbox/wall_clock_budget.py` | `real_cpython_completes_while_mnpy_enforces_wall_clock_budget` |
| Worker process memory contains compiler pressure | `example_e2e` | `examples/sandbox/compiler_memory_pressure_generator.py` | `real_cpython_completes_while_mnpy_contains_compiler_memory_pressure` |
| Script-directory imports work for safe modules | `example_e2e` | `examples/sandbox/import_root/main.py` | `real_cpython_and_mnpy_match_for_safe_script_directory_imports` |
| Imported source modules inherit policy | `example_e2e` | `examples/sandbox/blocked_import_root/main.py` | `real_mnpy_propagates_policy_into_script_directory_imports` |
| `sys.modules` injection cannot bypass policy | `example_e2e` | `examples/sandbox/cache_injection.py` | `real_cpython_accepts_cache_injection_while_mnpy_rechecks_policy` |
| `eval`, `exec`, and compiled code inherit policy | `example_e2e` | `examples/sandbox/dynamic_imports.py` | `real_cpython_allows_dynamic_imports_while_mnpy_reuses_the_sandbox_policy` |
| Module symlinks cannot escape the canonical root | `example_e2e` | `examples/sandbox/symlink_escape_main.py` | `real_cpython_follows_module_symlink_while_mnpy_rejects_root_escape` |
| argparse command-line programs match CPython (help, usage errors, subcommands, exit status) | `example_e2e` | `examples/sandbox/argparse_cli.py` | `real_cpython_and_mnpy_match_for_the_argparse_cli_example` |
| re text-processing programs match CPython (extraction, callable substitution, user-supplied patterns and their errors, verbose patterns) | `example_e2e` | `examples/sandbox/re_text_tool.py` | `real_cpython_and_mnpy_match_for_the_re_text_tool_example` |
| csv data programs match CPython (DictReader over in-memory text, reports written to `sys.stdout`, DictWriter conversion, sniffing, strict parsing errors, exit status) | `example_e2e` | `examples/sandbox/csv_report_tool.py` | `real_cpython_and_mnpy_match_for_the_csv_report_tool_example` |
| `datetime` reads the real wall clock in a UTC local zone, ignoring the host `TZ` | `harness_only` | Depends on the host clock and environment, not on Python source | `sandbox_boundary_reads_the_wall_clock_in_a_utc_local_zone` |
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

- Date: `2026-10-30`
- CPython oracle: `$MINIPYTHON_CPYTHON` (here `/opt/homebrew/bin/python3`)
- Required CPython version: `3.14.6`
- Seed: `20260710`
- Generated cases: `1024`
- Curated corpus cases: `36`
- Command: `tests/run.sh --discovery --seed 20260710 --generated-cases 1024`
- Generated layers: `syntax=256`, `runtime=256`, `stdlib=256`, `security=256`
- Generated root-cause templates: `17`

## Result

| Classification | Count |
| --- | ---: |
| `MATCH` | 794 |
| `INTENTIONAL_SANDBOX_BLOCK` | 258 |
| `UNSUPPORTED_OUT_OF_SCOPE` | 5 |
| `STDLIB_MISSING` | 1 |
| `CPYTHON_MISSING_COMPAT` | 1 |
| `CPYTHON_INTERNAL` | 1 |

All `1060` selected cases were either exact matches or explicit accepted scope
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
`$MINIPYTHON_CPYTHON` (default `/opt/homebrew/bin/python3`) as the fixed CPython
oracle, and checks it against the pinned `.python-version`.

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

## Layer-zero conformance

The separate black-box identity, rich-comparison and hash corpus and its
runner are documented in [conformance/README.md](conformance/README.md).

The conformance status block of the root `README.md` is generated from a
runner report:

```sh
python3 -I tests/conformance/run.py --mnpy path/to/mnpy --out path/to/report.json
python3 -I tests/conformance/status.py --report path/to/report.json
```

`status.py --report path/to/report.json --check` writes nothing and exits 1
when the block of `README.md` differs from the one the report implies; the
release gate runs it, so a stale block fails the gate. The block names the
oracle and states where the numbers were measured; that line is fixed, so the
same report implies the same block on every host.
