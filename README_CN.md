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

`mnpy` 始终通过 sandbox 边界运行代码：

```bash
mnpy --max-memory-bytes 134217728 -c "print(1 + 2)"
```

`mnpy` 固定使用安全 stdlib allowlist，并同时限制源代码大小、指令数、调用深度、
捕获输出、VM 分配和 worker 进程内存。它始终启动内部 worker，不提供关闭 sandbox
的公开参数。macOS 上由父进程监控 worker 的 physical footprint，其他 Unix 平台
使用内核进程限制。执行文件时，脚本目录默认作为 sandbox module root；`-c` 和
stdin 默认不暴露宿主模块目录，除非显式传入 `--root`。库 API 仍可用于 focused
runtime 和 parity 测试，但 in-process 调用不是运行不可信代码的正式安全边界。

### Rust 执行 API

嵌入方与 CLI 使用同一个 worker 安全边界。`Sandbox` 启动调用方指定的
`mnpy` worker，通过带长度前缀的 MessagePack 请求执行代码，并返回结构化结果，
不会把 VM 对象暴露给宿主：

```rust
use minipython::{Sandbox, SandboxInputs, SandboxValue};

let sandbox = Sandbox::new("target/release/mnpy");
let mut inputs = SandboxInputs::new();
inputs.insert("price".into(), SandboxValue::from(40_i64));

let result = sandbox.eval_with_inputs("price + 2", inputs);
assert!(result.is_success());
assert_eq!(result.value, Some(SandboxValue::from(42_i64)));
```

`ExecutionResult` 分别提供状态、返回值、精确捕获的 stdout/stderr、异常阶段/类型/文本
以及资源使用量。输入和返回值只能是惰性数据：`None`、布尔值、数字、字符串、
bytes、bytearray、list、tuple 和 dict。不支持的 runtime 对象只会作为输出专用的
`Opaque` 描述返回，不能重新注入 sandbox。worker 路径显式传入，使嵌入应用能够
决定实际执行哪个经过审核的 sidecar。

CLI 默认最多执行 1,000,000 条 VM 指令。可以用 `--max-steps N` 调整预算；
库调用方可以使用 `RuntimeOptions::with_max_instructions`，虚拟模块和 sandbox
目录模块的 `SandboxPolicy` 也使用相同的有限默认值。函数、generator、
coroutine、动态执行和模块导入共享同一份预算，不能通过嵌套执行重置。
worker 默认还有 5 秒 wall-clock deadline，覆盖解析、编译、VM 执行和退出阶段；
可以用 `--max-time-ms N` 调整。
嵌套 VM frame 默认也限制为 3 层，可以通过 `--max-depth N` 或
`RuntimeOptions::with_max_call_depth` 调整。
捕获输出默认限制为 1 MiB，嵌套执行共享同一份字节预算；可以通过
`--max-output-bytes N` 或 `RuntimeOptions::with_max_output_bytes` 调整。
核心 VM value materialization 默认共享一份单调递减的 8 MiB 预算，可以用
`--max-allocated-bytes N` 或 `RuntimeOptions::with_max_allocated_bytes` 调整。
它与现有 64 MiB 单次分配上限共同工作；`mnpy` 的子进程边界负责限制
compiler 和其他不在 VM value accounting 内的宿主分配。

可直接运行的边界示例位于 `examples/sandbox/`。例如执行
`mnpy examples/sandbox/blocked_host_capabilities.py`，可以看到 sandbox 明确
阻止的宿主 IO、网络、进程和 C ABI 能力。

整个 sandbox MVP 的完成状态以 `tests/README.md` 为准；仅仅 gap corpus
全绿不代表整个目标完成。
核心 runtime 的准确停手线以及所有仍为 partial 的 CPython coverage row 处理
方式记录在 `tests/README.md`。
开发 sandbox 控制时运行 `tests/run.sh --focused`；只运行持续差分
发现时使用 `tests/run.sh --discovery`；完整发布验收运行不带参数的
`tests/run.sh`。这是现在唯一的测试入口；旧 sandbox 和 gap-sweep runner 已删除。

## 测试

```bash
tests/run.sh --focused
tests/run.sh --discovery
tests/run.sh --discovery --seed 20260710 --generated-cases 1024
tests/run.sh
tests/run.sh --module json
tests/run.sh --root-cause json-loads-core
```

统一流水线会运行 driver 单测、仓库 corpus，以及固定种子生成的 Python 程序；
每个程序都会交给真实 CPython 和默认启用 sandbox 的真实 `mnpy`。生成 case 在
语法、运行时、stdlib、安全四层之间均衡分配。每个 root cause 的一个代表性非预期
差异会被自动缩减，最小复现写入 `reports/differential-repros/`；全部原始差异仍会
保留在报告中。报告会保留原始/缩减源码、分类、root cause、seed、两侧输出和缩减
次数。只要生成结果中仍有开放的 `must_fix` 或 `should_fix`
根因，discovery 和 release 流水线就会失败。流水线固定使用
`/opt/homebrew/bin/python3` 作为 CPython oracle，并用 `.python-version` 校验版本。
提升为支持面的行为仍然需要对应的 `cpython_subset`、`cpython_diff`、manifest、
coverage 和 migration 证据。
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
失败，同时把已接受的 sandbox/compatibility gap 留在报告里。open root-cause
report 也会写出对应的
`tests/run.sh --root-cause ...` 聚焦重跑命令。

## 架构

```
Host → Rust API / CLI → MessagePack worker → Lexer → Parser → Compiler → Register VM
```

基于寄存器的虚拟机，包含 80+ 条指令和 60+ 种值类型。

## 许可证

MIT
