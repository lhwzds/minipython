# minipython

用 Rust 实现的 Python 解释器。

## 目标

实现一个可维护、安全的 Rust Python：完整覆盖 Python 语法前端，逐步实现核心运行时和标准库子集，并按语义迁移 CPython 测试；对 CPython 内部实现测试做分类标记，而不是为了通过它们去复制 CPython。

## 安装

```bash
cargo build --release
```

## 使用

```bash
mnpy script.py          # 运行文件
mnpy -c "print(1+2)"    # 执行代码字符串
mnpy -e "1 + 2 * 3"     # 求值表达式
echo "print(1)" | mnpy  # 管道输入
```

## 架构

```
Source → Lexer → Parser → AST → Compiler → Bytecode → VM → Output
```

基于寄存器的虚拟机，包含 80+ 条指令和 60+ 种值类型。

## 许可证

MIT
