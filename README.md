# minipython

A Python interpreter written in Rust.

## Goal

实现一个可维护、安全的 Rust Python：完整覆盖 Python 语法前端，逐步实现核心运行时和标准库子集，并按语义迁移 CPython 测试；对 CPython 内部实现测试做分类标记，而不是为了通过它们去复制 CPython。

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
