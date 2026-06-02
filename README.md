# minipython

A safe, sandboxed Python runtime for AI agents — written in Rust.

minipython executes Python code without any access to the host system (no file system, no network, no processes). It is designed as a controlled execution environment where AI agents can run code safely, with a configurable gate layer that explicitly grants access to specific system resources.

## Goals

- **Safe by default** — no system calls, no file access, no network
- **AI-first** — designed for AI agents to execute Python code in a controlled sandbox
- **Gate layer** — system access is opt-in via Rust-implemented gates with policy enforcement
- **Python compatible** — supports most Python 3.10+ syntax and standard builtins
- **Zero unsafe** — pure safe Rust, compile-time memory guarantees

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

### Pipeline

| Stage | Lines | Description |
|---|---|---|
| `lexer.rs` | ~6,400 | Tokenizer with encoding support, f-strings, t-strings, Unicode |
| `parser.rs` | ~9,700 | Recursive descent parser, full Python 3.10+ grammar |
| `ast.rs` | ~460 | AST node definitions (18 statement types, 30 expression types) |
| `compiler.rs` | ~9,200 | AST to bytecode with scope analysis |
| `bytecode.rs` | ~580 | 80+ register-based instructions |
| `vm.rs` | ~39,400 | VM with builtins, OOP, generators, async, formatting |
| `value.rs` | ~1,800 | 60+ runtime value types |

### Roadmap

- [ ] File system gate (controlled `os`, `pathlib`, `shutil`)
- [ ] Network gate (controlled `urllib`, `http`)
- [ ] Process gate (controlled `subprocess`)
- [ ] CPython stdlib pure Python modules import
- [ ] Rust-implemented C extension replacements (`_json`, `_sre`, `_datetime`, etc.)

## License

MIT
