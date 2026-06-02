# minipython

A Python interpreter written in Rust.

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

A register-based VM with 80+ instructions, 60+ value types, and built-in support for most Python 3.10+ syntax including match-case, async/await, f-strings, type parameters, and more.

## License

MIT
