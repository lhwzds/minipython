# minipython

用 Rust 实现的 Python 解释器。

## 目标

实现一个可维护、面向 sandbox 的 Rust Python，而不是完整复制
CPython。MiniPython 应尽量完整覆盖 Python 语法前端，逐步实现核心运行时
语义和安全的纯内存标准库子集，并按公共行为迁移 CPython 测试；对 CPython
内部实现测试做分类标记，而不是为了通过它们去复制 CPython。

CPython 是行为 oracle，不是实现来源。MiniPython 不应该 wholesale 搬 CPython
`Lib/`；标准库行为只有在支持面和排除面写入 sandbox 文档，并有直接
differential 证据支撑时才算进入 scope。

## 范围

范围内：

- 尽量兼容 CPython 的语法前端：tokenizer、parser、AST、compile 降级和用户可见的语法/错误分类。
- 核心运行时语义：对象模型、descriptor、MRO、函数、闭包、generator、async、异常、容器、数字、字符串、bytes、bytearray、array 和 memoryview。
- 安全纯内存标准库模块：`builtins`、`sys`、`types`、`collections`、`collections.abc`、`math`、`math.integer`、`array`、`copy`、`io.BytesIO`、`operator`、`functools`、`itertools` 和 `json`。
  为了支持已迁移的 CPython 测试，runtime 可以存在额外的纯内存 compatibility shim；但除非写入 migration manifest 并明确支持面和排除面，否则它们不自动扩大默认产品 scope。
- 通过可执行 differential tests 迁移 CPython 公共行为。每个 bundled stdlib 模块必须有对应的 `cpython_diff` case，支持面才算完成；如果只做 subset，必须在 migration 和 coverage 记录里写清支持 API 和排除 API。

默认不做：

- 完整 CPython 标准库。
- 宿主 IO 集成，例如真实 `open()`、file descriptor、TTY、`input()` 和 `pty`。
- 网络和进程集成，例如 `socket`、`subprocess` 和 `signal`。
- C ABI 和 C 扩展兼容，例如 `_ssl`、`_socket`、`_ctypes` 和 `_testcapi`。
- CPython 内部实现契约，例如 refcount、GC tracking、bytecode/opcode identity、解释器 specialization 和精确 `co_stacksize`。
- 默认 `pdb` 集成和完整 `breakpoint()` 环境变量行为。
- locale-sensitive 行为，除非后续明确提升为 sandbox runtime 需求。

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

## 测试

```bash
/opt/homebrew/bin/python3 tools/test_cpython_gap_sweep.py
tools/run_cpython_gap_sweep.sh
tools/run_cpython_gap_sweep.sh --module json
tools/run_cpython_gap_sweep.sh --root-cause json-loads-core
```

第一条命令会快速测试 gap-sweep driver 本身。gap sweep 会固定使用
`/opt/homebrew/bin/python3` 作为 CPython oracle，并用 `.python-version` 校验
版本，先构建 `mnpy`，再比较有边界的 corpus。它是发现差异的循环；提升为支持
面的行为仍然需要对应的 `cpython_subset`、`cpython_diff`、manifest、coverage
和 migration 证据。
gap 报告会同时记录要求的固定 CPython 版本和实际 oracle/driver interpreter
路径，避免过期 oracle 混进结果里。使用 `--module` 可以聚焦一次批量运行，
例如只跑 `json`、`collections.abc` 或 `math.integer`。报告会把解释器输出的
`status` 和工作流用的 `triage_status` 分开：通过的 case、已接受的
sandbox/compatibility gap、以及需要按 root cause 继续修的非预期 diff 都会写入
机器可读 JSON。
从发现阶段进入修复阶段时使用 `--root-cause`，让一个 commit 聚焦一个分组原因，
同时覆盖报告里受影响的所有 case。JSON report 也会写出
`open_root_causes`，作为当前仍有 `needs_triage` case 的机器可读修复队列。
runner 会启用 `--fail-on-open`，让未预期的 open root cause 直接使批量运行
失败，同时把已接受的 sandbox/compatibility gap 留在报告里。

## 架构

```
Source → Lexer → Parser → AST → Compiler → Bytecode → VM → Output
```

基于寄存器的虚拟机，包含 80+ 条指令和 60+ 种值类型。

## 许可证

MIT
