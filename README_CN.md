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
- 安全纯内存标准库模块：`builtins`、`sys`、`types`、`collections`、`collections.abc`、`math`、`math.integer`、`array`、`copy`、`io.BytesIO`、`io.StringIO`（Rust 原生内存文本流，newline 模式、位置和错误信息与 CPython 3.14.6 一致）、`operator`、`functools`、`itertools`、`json`、`__future__`（支持全部 future 语句，包括 `from __future__ import annotations` 的 PEP 563 字符串注解）、`argparse`（Rust 原生实现，解析结果、help/usage 文本、错误信息和退出码与 CPython 3.14.6 一致；`FileType` 和 `fromfile_prefix_chars` 走 sandbox 中被阻断的 `open`，sandbox 没有终端，因此 help 从不着色）、`re`（Rust 原生正则引擎，`str` 和 bytes 的语法、错误信息、匹配结果以及 `Pattern`/`Match` 对象与 CPython 3.14.6 一致；匹配在显式栈上运行，并计入指令和内存分配预算，因此灾难性回溯会以 sandbox 错误结束，而不会挂住）、`datetime`（Rust 原生实现，算术、解析、格式化和错误信息与 CPython 3.14.6 一致；时钟和本地时区遵循“使用”一节中的约定，不提供 `zoneinfo`）和 `csv`（Rust 原生实现，reader、writer、dialect、`DictReader`/`DictWriter` 和 `Sniffer` 的行为与错误信息与 CPython 3.14.6 一致；可读取任意字符串可迭代对象，并写入任何带 `write` 方法的对象，例如 `io.StringIO` 或 `sys.stdout`，因为目前还不能访问文件）。
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

0.1.0 尚未发布；发布前请从源码构建（见下文）。从 0.1.0 起将提供预编译的
`mnpy`：macOS（arm64、x86-64）和 Linux（arm64、x86-64，静态链接的 musl 构建）：

```bash
npm install -g minipython            # npm 包名 `minipython`，命令为 `mnpy`
brew install lhwzds/tap/minipython   # Homebrew tap
mnpy --version
```

也可以从 GitHub Release 下载 `minipython-<target>.tar.gz` 和 `checksums.txt`，
校验 SHA-256 后把 `mnpy` 放到 `PATH` 上。从源码构建：

```bash
cargo build --release   # target/release/mnpy
```

在用它运行不可信代码之前，请先阅读 [CHANGELOG.md](CHANGELOG.md) 里 0.1.0 的限制：
`mnpy` 是纯计算沙箱（没有文件、没有网络，时钟可读），而且 worker 目前还不会降低
操作系统权限，所以不可信的多租户代码请放在容器里运行。漏洞请私下报告
（[SECURITY.md](SECURITY.md)）；发布流程和发布门禁见 [RELEASING.md](RELEASING.md)。

## 使用

```bash
mnpy script.py          # 运行文件
mnpy script.py a b      # sys.argv == ['script.py', 'a', 'b']
mnpy -c "print(1+2)"    # 执行代码字符串
mnpy -e "1 + 2 * 3"     # 求值表达式
echo "print(1)" | mnpy  # 管道输入
mnpy --version          # mnpy 0.1.0
```

与 CPython 一样，程序（`file`、`-c cmd` 或代表 stdin 的 `-`）之后的参数会成为
`sys.argv`；未捕获的 `SystemExit`（例如 `sys.exit(2)`）决定进程退出码：`None` 退出 0，
整数取低 8 位，其他值打印到 stderr 并退出 1。其他未捕获异常退出 1，并按 CPython 的
报告格式输出到 stderr：`Traceback (most recent call last):`、按 CPython 顺序排列的
cause/context 链，以及与 CPython 完全一致的最后一行（例如 `json.decoder.JSONDecodeError: ...`、
`MyErr`、`KeyError: 'k'`），随后是 `add_note()` 添加的 notes。由于 MiniPython 尚未记录源码行号，
`File "...", line N` 帧信息行会被省略；sandbox 限制和语法错误仍保留单行的
`sandbox error:` / `parse error:` 前缀。脚本目录中名字不是合法模块名的文件
（例如 `self-test.py`）会被导入索引跳过，因此仍可作为入口脚本运行。
`sys.stdout` 和 `sys.stderr` 是内存中的文本流：`print(file=...)`、`write()`、`writelines()`
以及重新绑定 `sys.stdout` 的行为与 CPython 一致；stdin 始终处于文件末尾；`fileno()`
抛出 `io.UnsupportedOperation`，不会暴露任何宿主文件描述符。
因此基于 `argparse` 的命令行脚本可以原样运行：`mnpy tool.py --help` 把 help 输出到 stdout
并退出 0；用法错误把 usage 行和 `tool.py: error: ...` 输出到 stderr 并退出 2，与 CPython
完全一致（help 按 CPython 非终端时的 80 列宽度排版）。

时钟和时区约定：`datetime` 读取宿主的真实时钟，sandbox 的本地时区始终是 UTC。
sandbox 从不读取 `TZ` 和 `/etc/localtime`，因此 `datetime.now()`、`date.today()`、`fromtimestamp()`、
naive `timestamp()` 和 `astimezone()` 的行为与在 `TZ=UTC` 下运行的 CPython 一致。嵌入方可以把时钟
固定到某个时刻，使运行结果可复现（见下文 `Sandbox::with_clock`）。

在 Linux 上 `math.cbrt` 返回正确舍入的立方根，而 Linux CPython（GNU libc）的结果对部分输入
在最后一两位上有偏差（那里 `cbrt(27.0)` 是 `3.0000000000000004`）；tests/README.md 列出了这些输入。

`mnpy` 始终通过 sandbox 边界运行代码：

```bash
mnpy --max-memory-bytes 134217728 -c "print(1 + 2)"
```

`mnpy` 固定使用安全 stdlib allowlist，并同时限制源代码大小、指令数、调用深度、
捕获输出、VM 分配和 worker 进程内存。它始终启动内部 worker，不提供关闭 sandbox
的公开参数。macOS 上由父进程监控 worker 的 physical footprint，其他 Unix 平台
使用内核进程限制（`RLIMIT_AS` 和 `RLIMIT_DATA`），内存耗尽时内存分配失败、worker
中止。两种情况都以内存限制结束（`ExecutionStatus::MemoryLimit`，
`sandbox error: worker exceeded process limits or crashed`）。Linux 的限制按地址空间
计算，包括整个解释器栈的预留，因此 worker 让 GNU libc 只使用一个 malloc arena，
而不是为每个线程预留 64 MiB。执行文件时，脚本目录默认作为 sandbox module root；`-c` 和
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

`ExecutionResult` 分别提供状态、返回值、精确捕获的 stdout/stderr、异常阶段/类型/文本、
资源使用量，以及未捕获的 `SystemExit` 结束程序时的 `exit_code`。未捕获的 Python 异常中，
`type_name` 是 `type(exc).__name__`，`message` 是 `str(exc)`，`traceback` 保存 CLI 输出的
CPython 格式报告；`error_report()` 返回该报告，非 Python 异常的失败则返回单行的
`display_message()`（`sandbox error: ...`、`parse error: ...`）。`Sandbox::with_argv`
为单次运行设置 `sys.argv`；会话只在启动时设置一次，之后的调用看到的是程序修改后的列表。`Sandbox::with_clock(HostClock::fixed_unix_micros(...))`
为每次运行和会话固定 `datetime` 读取的时钟；默认值 `HostClock::System` 是真实时钟，
in-process API 通过 `RuntimeOptions::with_clock` 使用同一设置。输入和返回值只能是惰性数据：`None`、布尔值、数字、字符串、
bytes、bytearray、list、tuple 和 dict。不支持的 runtime 对象只会作为输出专用的
`Opaque` 描述返回，不能重新注入 sandbox。worker 路径显式传入，使嵌入应用能够
决定实际执行哪个经过审核的 sidecar。

终止条件会结束整次运行：指令、分配和输出预算，原生栈硬上限，worker 的
wall-clock 与内存限制，以及 worker 故障。它们不是 Python 异常。任何 `except`
子句（包括 `except BaseException`）、`finally` 块、`with` 语句、generator 或原生
callback（`re.sub` 替换函数、排序 key、`json` hook，以及 builtin 调用的
`__str__`/`__repr__`）都不能观察到或拦截它，之前捕获的输出会保留。
`ExecutionResult::terminal` 用 `TerminalKind`（`InstructionLimit`、
`AllocationLimit`、`OutputLimit`、`NativeStackExhausted`、`TimeLimit`、
`MemoryLimit` 或 `WorkerFailure`）指明终止条件，其他结果均为 `None`；消息文本和
`error_report()` 保持不变。Session 在 VM 预算终止后仍可继续使用，下一次调用重新
获得预算；wall-clock 或内存终止则会关闭 Session。

拒绝日志：`ExecutionResult::denials` 列出 sandbox 在本次运行中拒绝的能力，无论程序
是否捕获了由此产生的异常。`Denial { kind, target, count }` 可以是被阻止的导入
（`DenialKind::Import`，目标为模块名，来自 `import`、`from ... import` 或
`__import__`），按名称查找的宿主 builtin（`DenialKind::Builtin`：`open`、`input`、
`help`、`exit`、`quit`、`copyright`、`credits` 或 `license`），或允许模块上的宿主
属性（`DenialKind::Attribute`：`builtins.open` 及上述其他 builtin，包括
`argparse.FileType` 和 `fromfile_prefix_chars` 触及的 `open`，以及 `io.open`、
`io.open_code` 或 `io.FileIO`）。每个条目只出现一次，
按首次出现的顺序排列，计数饱和递增；一次运行最多记录 `MAX_DENIALS`（32）个不同条目，
目标最长 128 字节，之后出现的不同条目会被丢弃。程序本身看到的仍然是 CPython 对缺失
名称抛出的异常，并且无法读取该日志。

宿主能力通过 external function 显式授权。只有嵌入应用注册了准确的全局名称后，
该函数才会出现在 Python 中：

```rust
use minipython::{ExternalFunctionError, SandboxValue};

sandbox.register_external_function("lookup_price", |call| {
    let Some(SandboxValue::String(symbol)) = call.args.first() else {
        return Err(ExternalFunctionError::new("TypeError", "symbol must be a string"));
    };
    Ok(SandboxValue::from(format!("quote:{symbol}")))
})?;
```

调用和返回值通过 worker 协议传输，并复用同一惰性数据 allowlist。runtime 对象不能
传给宿主，opaque 宿主值不能返回，callback 错误会成为 Python 可捕获异常，callback
panic 也不会穿透 `Sandbox::execute`。External callback 属于受信任宿主代码：它可以
主动执行应用明确授予的能力，其自身运行时间不能由 worker wall-clock limit 强制取消。

有状态的 Agent 工作流可以从同一个已配置 sandbox 创建 `SandboxSession`：

```rust
let mut session = sandbox.session()?;
let setup = session.run("balance = 40");
assert!(setup.is_success());
let result = session.eval("balance + 2");
assert_eq!(result.value, Some(SandboxValue::from(42_i64)));
session.close()?;
```

Session 持有一个隔离 worker，保留 Python globals、函数闭包、mutation 和模块缓存，
不会重放之前的源码。调用通过 `&mut self` 顺序执行，每次调用重新获得 VM 与
wall-clock 预算。进程内存或 wall-clock 超限会关闭整个 Session；后续调用直接失败，
不会复用可能已损坏的 worker。Session 被 drop 时也会有界关闭并回收 worker。

Worker 隔离保证：每次 `Sandbox` 执行（`run`、`eval`、`check`、`execute`）都会启动
一个新的 worker 进程，它只服务这一次执行，并在调用返回前被回收；每个 `SandboxSession`
在整个生命周期中独占一个 worker。worker 永远不会被之后的执行复用，不会在 Session 之间
共享（包括克隆出的 `Sandbox` 创建的 Session），也不会在其 Session 存活期间被一次性
执行使用；因此 Session 状态不会到达其他执行，关闭或 drop 的 Session 的 worker 会被回收。

Worker 启动卫生处理：父进程启动每个 worker 时只传入内部 worker 标记，环境变量
其余部分为空，因此嵌入方的变量（凭据、代理、`PATH`）不会到达 worker。worker 在读取
请求之前关闭除 stdin、stdout、stderr 之外的所有继承描述符，清空自身环境（不保留任何
变量：worker 中没有代码读取环境变量），把工作目录切换到 `/`，把 umask 设为 `077`，
并把 `RLIMIT_CORE` 设为零。由于 worker 在 `/` 中运行，父进程会在发送请求之前根据
自身当前目录解析相对的模块根目录，因此模块根目录相关错误现在显示绝对路径。

### Python Binding

`python/` 包让 Python 宿主使用同一产品 API，但不会把 MiniPython VM 加载进
宿主进程：

```python
from minipython_sandbox import Sandbox

with Sandbox(executable="target/release/mnpy") as sandbox:
    result = sandbox.eval("price + 2", {"price": 40})
    assert result.is_success
    assert result.value == 42
```

`Sandbox` 提供 `run`、`eval`、`check`、显式 `external_functions` 和一个持久
`session()`，惰性值与结构化结果契约（包括 `ExecutionResult.traceback` 和
`ExecutionResult.terminal`：`"instruction_limit"`、`"allocation_limit"`、
`"output_limit"`、`"native_stack_exhausted"`、`"time_limit"`、`"memory_limit"`
或 `"worker_failure"`；以及 `ExecutionResult.denials`：由
`Denial(kind, target, count)` 组成的 tuple，kind 为 `"import"`、`"builtin"` 和
`"attribute"`）和 Rust API 相同。该包是只依赖 Python
标准库的纯 Python 实现。它启动经过审核的 `mnpy` 中的私有 JSON-line broker；
broker 仍然委托 Rust `Sandbox` API 为每次一次性执行或 Session 创建并监控隔离的
MessagePack worker。这里没有 C extension、进程内 VM 路径、shell 调用或关闭 worker
限制的开关。

CLI 默认最多执行 1,000,000 条 VM 指令。可以用 `--max-steps N` 调整预算；
库调用方可以使用 `RuntimeOptions::with_max_instructions`，虚拟模块和 sandbox
目录模块的 `SandboxPolicy` 也使用相同的有限默认值。函数、generator、
coroutine、动态执行和模块导入共享同一份预算，不能通过嵌套执行重置。
worker 默认还有 5 秒 wall-clock deadline，覆盖解析、编译、VM 执行和退出阶段；
可以用 `--max-time-ms N` 调整。
Python 递归与 CPython 3.14.6 一致：Python frame 计入
`sys.getrecursionlimit()`，初始值为 1000，超出时抛出可捕获的
`RecursionError: maximum recursion depth exceeded`。`sys.setrecursionlimit()`
与 CPython 语义相同，接受任意更大的正整数。运维方可以通过 `--max-depth N`（或
`SandboxLimits::max_call_depth`、`RuntimeOptions::with_max_call_depth`）设置上限，
默认没有上限；设置上限后，初始值取 1000 与上限中的较小者，请求更大的值会抛出
`ValueError: recursion limit N exceeds the sandbox call-depth limit M`，不会被静默降低。所有入口都在专用解释器线程上运行 Python，
栈大小显式指定（进程内 256 MiB；worker 中为 `--max-memory-bytes` 的四分之一，
最多 256 MiB），不受 `RUST_MIN_STACK` 影响。原生栈保护会把解释器内部的深递归
（repr、比较、哈希、JSON、`copy.deepcopy`、迭代器链、解析器和编译器）转换为
CPython 的 `RecursionError`（解析器为 `MemoryError`），而不会让原生栈溢出；
深层嵌套的值释放时也不会产生原生递归。调高的限制超过解释器栈容量时，保护会抛出
CPython 的 C 栈报告 `RecursionError: Stack overflow (used N kB)`（附带操作上下文）。
调用共享被调函数的字节码而不复制，但每个 Python frame 仍占用堆内存（寄存器文件和
frame 状态，随函数体大小增长）。解释器会估算这部分内存，并允许存活 frame 使用解释器
栈的两倍（worker 中为 `--max-memory-bytes` 的一半）：超出时进入新 frame 会抛出同样
可捕获的 `RecursionError: Stack overflow (used N kB)`，正在运行的 frame 的寄存器
继续增长超出时则计入分配预算。因此经过大函数的深递归会以 Python 异常或预算错误结束，
而不会耗尽 worker 内存；但在默认限制下，它可能达不到 CPython 能达到的 1000 层：
在 release worker 中，30 条语句的函数体可以递归到上限，100 条语句的函数体约在
400 层停止，400 条语句的函数体约在 100 层停止。
CPython 的 Python 间调用不占用 C 栈，因此能达到远比 MiniPython 目前更深的调高限制
（进程内 release 约 50,000 层、debug 约 15,000 层；worker 中更少，其栈和内存取自
`--max-memory-bytes` 的一部分）。在保护余量内仍继续递归的代码会以不可捕获的
`sandbox error: native stack exhausted` 终止。
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
发现时使用 `tests/run.sh --discovery`；完整测试运行不带参数的 `tests/run.sh`；
完整发布门禁运行 `tests/run.sh --release-gate`（见 [RELEASING.md](RELEASING.md)）。
这是现在唯一的测试入口；旧 sandbox 和 gap-sweep runner 已删除。

## 测试

```bash
tests/run.sh --focused
tests/run.sh --discovery
tests/run.sh --discovery --seed 20260710 --generated-cases 1024
tests/run.sh
tests/run.sh --module json
tests/run.sh --root-cause json-loads-core
MINIPYTHON_CPYTHON_SRC=/path/to/cpython-3.14.6 tests/run.sh --release-gate
```

CPython 3.14.6 oracle 是 `MINIPYTHON_CPYTHON` 指定的可执行文件（默认
`/opt/homebrew/bin/python3`）；它的标准库和 `Lib/test` 路径通过 `sysconfig` 从
oracle 自身得到，所以在 Linux 和 CI 上只需设置这个变量，例如
`MINIPYTHON_CPYTHON=/usr/local/bin/python3.14 tests/run.sh`。没有 `Lib/test` 的安装
（官方 Docker 镜像会删除它）需要把 `MINIPYTHON_CPYTHON_LIB` 设为 3.14.6 源码树的 `Lib`
目录。语法审计从
`MINIPYTHON_CPYTHON_SRC` 指定的 CPython 3.14.6 源码树读取 `Grammar/python.gram`；
设置 `MINIPYTHON_REQUIRE_CPYTHON_SRC=1` 时，缺少源码树会让该审计失败而不是跳过，
发布门禁两者都要求。

统一流水线会运行 driver 单测、仓库 corpus，以及固定种子生成的 Python 程序；
每个程序都会交给真实 CPython 和默认启用 sandbox 的真实 `mnpy`。生成 case 在
语法、运行时、stdlib、安全四层之间均衡分配。每个 root cause 的一个代表性非预期
差异会被自动缩减，最小复现写入 `reports/differential-repros/`；全部原始差异仍会
保留在报告中。报告会保留原始/缩减源码、分类、root cause、seed、两侧输出和缩减
次数。只要生成结果中仍有开放的 `must_fix` 或 `should_fix`
根因，discovery 和 release 流水线就会失败。worker 崩溃（`CRASH`）或预期被打破
（`EXPECTED_CONTRACT_DIFF`）不论优先级，都会让每次带 gate 的运行失败：被标记为预期
差异的 case（例如有意的沙箱拦截）一旦 MiniPython 开始与 CPython 一致，就算违反约定，
所以被绕过的拦截或过期的标记都不会悄悄通过。报告还会记录 `mnpy` 二进制的 SHA-256
和被测的 git revision。流水线固定使用 `$MINIPYTHON_CPYTHON`（默认
`/opt/homebrew/bin/python3`）作为 CPython
oracle，用 `.python-version` 校验版本，并构建 `mnpy`。
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
Rust host / CLI -----------------> Sandbox API -> MessagePack worker -> Register VM
Python host -> private JSON broker -----------^
```

基于寄存器的虚拟机，包含 80+ 条指令和 60+ 种值类型。

解释器中所有触及宿主状态的调用都位于 `src/host/`：时钟与 UTC 本地时区
（`clock`），在任何 Python 运行之前把模块根目录读入内存（`module_root`），以及
worker 的启动卫生处理与协议通道（`worker`）。VM 本身没有文件、环境变量、进程、
网络、熵源或当前目录访问能力。worker 监管（启动、内存与 wall-clock 限制、kill 与
回收）留在父进程侧的 `src/product.rs`，`mnpy` 命令行位于 `src/sandbox.rs`。
`cargo test` 中的 lint（`tests/sandbox.rs` 的 `host_capability_lint`）会在
`src/host/` 之外出现任何触及宿主状态的调用时失败；已列出的父进程与 CLI 调用点
各自注明了原因，属于例外。

## 许可证

MiniPython 采用双重许可。核心部分（解释器、沙箱与嵌入 API）采用 MIT 许可，见
[LICENSE](LICENSE) 与 [LICENSES/MIT.txt](LICENSES/MIT.txt)。

部分模块（`re`、`argparse`、`csv`、`datetime`、异常报告以及注解反解析器）与测试
用例派生自 CPython 3.14.6，因此同时采用 PSF License 2.0，见
[LICENSE-PSF](LICENSE-PSF) 与 [LICENSES/PSF-2.0.txt](LICENSES/PSF-2.0.txt)。
派生自 CPython 的文件列在 [NOTICE](NOTICE) 中。

Python 是 Python Software Foundation 的商标。MiniPython 与 Python Software
Foundation 无隶属关系，也不代表其背书。
