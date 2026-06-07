# minipython

A Python interpreter written in Rust.

## Goal

Build a maintainable, sandbox-focused Rust Python rather than a full CPython
clone. MiniPython should fully cover the Python syntax frontend where practical,
incrementally implement core runtime semantics and a safe pure-memory standard
library subset, and migrate CPython tests by public behavior. CPython internal
implementation tests should be classified and tagged rather than copied as
runtime requirements.

## Scope

In scope:

- CPython-compatible syntax frontend coverage: tokenizer, parser, AST,
  compiler lowering, and user-visible syntax/error classification.
- Core runtime semantics: object model, descriptors, MRO, functions,
  closures, generators, async constructs, exceptions, containers, numbers,
  strings, bytes, bytearray, array, and memoryview behavior.
- Safe pure-memory standard library modules: `builtins`, `sys`, `types`,
  `collections`, `math`, `array`, `copy`, `io.BytesIO`, `operator`,
  `functools`, `itertools`, and `json`.
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
- Locale-sensitive behavior unless it is explicitly promoted into the sandbox
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

## Architecture

```
Source → Lexer → Parser → AST → Compiler → Bytecode → VM → Output
```

A register-based VM with 80+ instructions and 60+ value types.

## License

MIT
