# MiniPython Agent Guide

This repository builds a sandbox-focused Rust Python, not a full CPython clone.
Keep changes aligned with the product scope documented in `README.md` and
`README_CN.md`.

## Scope

Continue implementing:

- CPython-compatible syntax frontend behavior where practical: tokenizer,
  parser, AST, compile lowering, and user-visible error classification.
- Core runtime semantics: object model, descriptors, MRO, functions, closures,
  generators, async constructs, exceptions, containers, numbers, strings,
  bytes, bytearray, array, and memoryview.
- Safe pure-memory standard library subset: `builtins`, `sys`, `types`,
  `collections`, `collections.abc`, `math`, `math.integer`, `array`, `copy`,
  `io.BytesIO`, `operator`, `functools`, `itertools`, and `json`.

Do not implement by default:

- Full CPython standard library coverage.
- Host I/O integration: real `open()`, file descriptors, TTY behavior,
  `input()`, or `pty`.
- Network or process integration: `socket`, `subprocess`, `signal`, and
  related modules.
- C ABI or C extension compatibility, including `_ssl`, `_socket`, `_ctypes`,
  and `_testcapi`.
- CPython implementation internals: refcounts, GC tracking, opcode identity,
  specialization, and exact `co_stacksize`.
- Default `pdb` integration or full `breakpoint()` environment-variable
  behavior.
- Locale-sensitive behavior unless it is explicitly promoted into scope.

## Evidence Rules

- CPython is the public behavior oracle, not an implementation source.
- Do not wholesale port CPython `Lib/` or internal implementation tests.
- Every supported stdlib module must have direct `cpython_diff` evidence.
- Partial stdlib modules must document supported and excluded surfaces in the
  migration, coverage, and sandbox manifest checks.
- Prefer small commits that each add implementation plus focused CPython
  subset/diff/manifest evidence.

## Validation

For focused runtime or stdlib changes, run the relevant `cargo test` filter,
then `cargo fmt --check` and `git diff --check` before committing.
