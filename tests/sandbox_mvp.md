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
| Syntax frontend | yes | `proven` | `tests/cpython_grammar_inventory.md` tracks all 276 CPython 3.14.6 grammar rules with named Rust evidence | Keep the pinned inventory synchronized and retain focused syntax/error diffs |
| Core runtime | yes | `proven` | `tests/sandbox_runtime_mvp.md` explicitly bounds all 13 `partial` coverage rows and the four partial CPython public groups while retaining their wider CPython status; the complete release gate covers functions, closures, classes, descriptors, MRO, exceptions, generators, async, containers, numbers, strings, bytes, array, and one-dimensional memoryview | Keep the bounded runtime gate green without expanding into documented CPython/host exclusions |
| Safe stdlib allowlist | yes | `proven` | The Sandbox Stdlib Manifest records supported/excluded surfaces and direct `cpython_diff` evidence for every required module; the complete release gate has no open `must_fix` or `should_fix` roots | Keep every supported surface executable and every excluded surface blocked |
| Import isolation | yes | `proven` | `sandbox_policy_*`, virtual-module, package, duplicate-name, invalid-name, and symlink-escape tests | Keep source modules confined to the canonical sandbox root and enforce explicit stdlib policy through cache and child imports |
| Host capability stop-line | yes | `proven` | Required allowlist tests block host I/O, network, process, signal, socket, C ABI, and C-extension modules | Keep every excluded module unavailable even through `__import__`, `sys.modules`, package children, and compatibility shims |
| Instruction budget | yes | `proven` | `instruction_budget_*` tests cover top-level code, functions, generators, `exec`, and imported source modules; CLI and `SandboxPolicy` use a finite default | Keep one shared budget across every nested VM path |
| Call-depth guard | yes | `proven` | `call_depth_guard_*` and `sandbox_policy_call_depth_guard_*` stop direct and imported-module recursion before host stack overflow; CLI and `SandboxPolicy` use a conservative finite default | Keep the frame limit shared across functions, dynamic execution, imports, generators, and coroutines; replace Rust-stack recursion with explicit VM frame scheduling before raising the default |
| Heap/allocation guard | yes | `proven` | A shared monotonic VM materialization budget recursively charges core values and supported stdlib buffers, with separate incremental mutation charges and the existing 64 MiB single-allocation guard. The single public `mnpy` entrypoint always runs compilation and execution in a child process: Unix uses kernel address/data limits where supported, while macOS monitors physical footprint and kills the worker on overflow. `allocation_budget_*` covers VM paths and `sandbox_process_contains_compiler_memory_pressure` proves compiler/AST containment outside VM accounting | Keep `mnpy` sandboxed by default with no public bypass; in-process APIs do not claim a complete host-memory boundary |
| Captured-output guard | yes | `proven` | `output_budget_*` and `sandbox_policy_output_budget_*` bound complete lines, partial prints, `exec`, and imported-module output under one shared byte budget; CLI and `SandboxPolicy` use a finite default | Keep every buffered-output path on the shared budget and avoid double charging nested VM output during merge |
| Batch CPython parity | yes | `proven` | Fixed 3.14.6 oracle, categorized corpus, JSON/Markdown reports, root-cause grouping, filters, and `--fail-on-open` | Keep the corpus representative; no open `must_fix` or `should_fix` root cause may remain at release |
| Security regression suite | yes | `proven` | Import boundary, symlink escape, dynamic `__import__` / from-import / dotted-child cache injection, instruction, call-depth, output, VM allocation, source-size, process-memory, and worker-crash containment regressions run in the release gate. `sandbox_boundary` additionally exercises every CLI budget, all source channels, compatibility-shim rejection, cache injection, and root-module policy through the official process entrypoint | Keep the process entrypoint and every policy/budget regression in both focused and complete gates; maintain the test-set taxonomy in `tests/sandbox_test_strategy.md` |
| Support and stop-line docs | yes | `proven` | `README.md`, `README_CN.md`, `AGENTS.md`, this checklist, runtime coverage, migration notes, and the stdlib manifest describe the current supported and excluded boundary | Keep the checklist and public support matrix synchronized with implementation changes |

## Release Gate

Run `tools/run_sandbox_mvp_checks.sh` for the complete gate. During focused
sandbox-control development, `tools/run_sandbox_mvp_checks.sh --focused` runs
the security-policy slice plus the parity infrastructure checks; focused mode
is not sufficient for release completion.

The final release gate must run, in order:

1. Focused sandbox security and policy tests.
2. The complete language/runtime test suite.
3. CPython subset, differential, inventory, and manifest tests against
   `/opt/homebrew/bin/python3` at the pinned version.
4. `tools/run_cpython_gap_sweep.sh --fail-on-open` with no open `must_fix` or
   `should_fix` root cause.
5. `cargo fmt --check` and `git diff --check`.

Until all required rows are `proven`, the full MiniPython sandbox goal remains
active.
