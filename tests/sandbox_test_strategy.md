# Sandbox Test Strategy

This document separates CPython conformance evidence from sandbox security
evidence. A passing conformance corpus does not prove containment, and a passing
containment suite does not imply complete CPython behavior.

## Test Sets

| Test set | Purpose | Authoritative entrypoint | Failure found means |
| --- | --- | --- | --- |
| Grammar inventory | Keep the pinned CPython grammar rule names synchronized | `cargo test --test cpython_inventory` | Syntax inventory drift or missing named evidence |
| Runtime subset | Exercise the supported MiniPython behavior directly | `cargo test --test cpython_subset` | A supported runtime or stdlib behavior regressed |
| Differential parity | Run a real CPython 3.14.6 process against the MiniPython in-process API for focused behavior slices | `cargo test --test cpython_diff` | Observable parity regressed inside the declared surface |
| Manifest guards | Keep implementation, tests, allowlist, coverage, and docs synchronized | `cargo test --test cpython_manifest` | Evidence or scope documentation drifted |
| Full-process gap corpus | Run the same program in real CPython and the real sandbox-default `mnpy` executable, then compare exit status, stdout, stderr, exception shape, timeout, and crash state | `tools/run_cpython_gap_sweep.sh --fail-on-open` | An end-to-end root cause needs triage; corpus success alone is not completion |
| In-process sandbox policy | Test shared budgets, import policy, cache checks, and virtual/root modules | `cargo test --test language sandbox_policy` plus budget filters | A VM or embedding-policy invariant regressed |
| Process containment | Test source-size and process-memory failure containment | `cargo test --test sandbox_process` | The official untrusted-code entrypoint is not contained |
| Adversarial boundary | Exercise every CLI budget and common policy bypass through the sandbox-default `mnpy` CLI | `cargo test --test sandbox_boundary` | The public process boundary differs from the in-process policy |

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

These sets would improve defect discovery but are not required to preserve the
current sandbox MVP boundary:

1. Differential syntax generation using bounded grammar-derived programs,
   with every crash or mismatch minimized into a checked-in regression.
2. Stateful model tests for mutable containers, `BytesIO`, `array`, iterator
   buffers, and `json` recursion under allocation budgets.
3. A release-profile platform matrix on macOS and Linux for process-memory and
   signal/crash containment.
4. A persisted crash corpus for parser, compiler, VM, and import-loader inputs,
   each executed in `mnpy` with a wall-clock timeout.

New exploratory cases belong in a focused set first. Promote a discovered
supported behavior into `cpython_subset` and `cpython_diff`; promote a security
failure into `sandbox_boundary` or `sandbox_process` and keep it in the release
gate permanently.
