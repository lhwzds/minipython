# minipython

A Python interpreter written in Rust.

## Goal

Build a maintainable, safe Rust Python: fully cover the Python syntax frontend, incrementally implement the core runtime and standard library subset, and migrate CPython tests by semantics; classify and tag CPython internal implementation tests rather than replicating CPython to pass them.

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
