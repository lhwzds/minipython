# minipython

A Python interpreter written in Rust.

## Goal

Build a maintainable, sandbox-focused Rust Python rather than a full CPython
clone. MiniPython should fully cover the Python syntax frontend where practical,
incrementally implement core runtime semantics and a safe pure-memory standard
library subset, and migrate CPython tests by public behavior. CPython internal
implementation tests should be classified and tagged rather than copied as
runtime requirements.

CPython is the behavior oracle, not an implementation source. MiniPython must
not wholesale port CPython `Lib/`; standard-library behavior is accepted only
when the supported and excluded sandbox surfaces are documented and backed by
direct differential evidence.

## Scope

In scope:

- CPython-compatible syntax frontend coverage: tokenizer, parser, AST,
  compiler lowering, and user-visible syntax/error classification.
- Core runtime semantics: object model, descriptors, MRO, functions,
  closures, generators, async constructs, exceptions, containers, numbers,
  strings, bytes, bytearray, array, and memoryview behavior.
- Safe pure-memory standard library modules: `builtins`, `sys`, `types`,
  `collections`, `collections.abc`, `math`, `math.integer`, `array`, `copy`,
  `io.BytesIO`, `io.StringIO` (a native in-memory text stream with
  CPython 3.14.6 newline modes, positions, and error messages), `operator`,
  `functools`, `itertools`, `json`, `__future__` (every future statement,
  including PEP 563 string annotations for
  `from __future__ import annotations`), `argparse`
  (a native Rust module with CPython 3.14.6 parsing, help/usage text, error
  messages, and exit status; `FileType` and `fromfile_prefix_chars` reach
  the sandbox's blocked `open`, and help is never colored because the sandbox
  has no terminal), `re` (a native Rust regular expression engine with
  CPython 3.14.6 syntax, error messages, matching results, and `Pattern`/
  `Match` objects for `str` and bytes; matching runs on an explicit stack and
  is charged to the instruction and allocation budgets, so catastrophic
  backtracking ends with a sandbox error instead of hanging), `datetime`
  (a native Rust module with CPython 3.14.6 arithmetic, parsing, formatting,
  and error messages; it reads the clock and local time zone described under
  Usage, and `zoneinfo` is not provided), and `csv` (a native Rust module
  with CPython 3.14.6 reader, writer, dialect, `DictReader`/`DictWriter`, and
  `Sniffer` behavior and error messages; it reads any iterable of strings and
  writes to any object with a `write` method, such as `io.StringIO` or
  `sys.stdout`, since files are not available yet).
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
- locale-sensitive behavior unless it is explicitly promoted into the sandbox
  runtime requirements.

<!-- conformance-status:start -->

## Conformance status

Oracle: CPython 3.14.6.

Snapshot from one macOS release-build run.

Totals: 59 leaves, 392 cases, 291 matching, 101 differing, 0 unsupported, 0 crash, 0 timeout.

| Protocol | Leaves | Cases | Matching | Differing | Unsupported | Crash | Timeout |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| callback | 2 | 93 | 77 | 16 | 0 | 0 | 0 |
| hash | 13 | 71 | 35 | 36 | 0 | 0 | 0 |
| identity | 15 | 79 | 71 | 8 | 0 | 0 | 0 |
| rich-comparison | 29 | 149 | 108 | 41 | 0 | 0 | 0 |

A differing case is a known mismatch that still has to be fixed; the goal is that every case matches.

<!-- conformance-status:end -->

## Install

Version 0.1.0 is not released yet; until it is, build from source (see
below). From 0.1.0, prebuilt `mnpy` binaries will be published for macOS
(arm64, x86-64) and Linux (arm64, x86-64; static musl builds):

```bash
npm install -g minipython            # npm package `minipython`, command `mnpy`
brew install lhwzds/tap/minipython   # Homebrew tap
mnpy --version
```

Or download `minipython-<target>.tar.gz` and `checksums.txt` from the GitHub
Release, check the SHA-256, and put `mnpy` on your `PATH`. To build from
source:

```bash
cargo build --release   # target/release/mnpy
```

Before relying on it for untrusted code, read the 0.1.0 limits in
[CHANGELOG.md](CHANGELOG.md): `mnpy` is a pure-compute sandbox (no files, no
network, a readable clock), and its worker does not drop operating-system
privileges yet, so run untrusted multi-tenant code inside a container.
Vulnerabilities are reported privately ([SECURITY.md](SECURITY.md)); the
release procedure and its gate are in [RELEASING.md](RELEASING.md).

## Usage

```bash
mnpy script.py          # run a file
mnpy script.py a b      # sys.argv == ['script.py', 'a', 'b']
mnpy -c "print(1+2)"    # execute a string
mnpy -e "1 + 2 * 3"     # evaluate an expression
echo "print(1)" | mnpy  # pipe input
mnpy --version          # mnpy 0.1.0
```

`--status-file PATH` writes the run's result to `PATH` as one JSON object
followed by a newline, replacing any file there: `{"status": S, "terminal": T}`.
`S` names the run's status (`Success`, `Error`, `TimeLimit`, `MemoryLimit` or
`WorkerCrash`); `T` names the terminal condition that ended the run
(`InstructionLimit`, `AllocationLimit`, `OutputLimit`, `NativeStackExhausted`,
`TimeLimit`, `MemoryLimit` or `WorkerFailure`), or is `null` when the sandbox
did not end the run: a program that ended on its own, and a source `mnpy`
could not read. The file is written on every exit path after the options are
parsed, so a caller can tell a worker crash from an ordinary Python error
without reading stderr, which the program's own output shares with mnpy's
report. A usage error, `-h` and `-V` end before that, so they write no
file. If the file cannot be written, `mnpy` prints
`mnpy: cannot write status file ...` and exits 2, because the run's result
cannot be reported any other way. Without the option, `mnpy` writes no file
and behaves exactly as before.

As with CPython, everything after the program (`file`, `-c cmd`, or `-` for
stdin) becomes `sys.argv`, and an uncaught `SystemExit` (for example
`sys.exit(2)`) sets the process exit status: `None` exits 0, integers exit with
their low 8 bits, and any other value is printed to stderr and exits 1. Any
other uncaught exception exits 1 and prints CPython's report shape to stderr:
`Traceback (most recent call last):`, chained causes and contexts in CPython's
order, and the exact final line CPython prints (`json.decoder.JSONDecodeError:
...`, `MyErr`, `KeyError: 'k'`) followed by any `add_note()` notes. The
`File "...", line N` frame lines are omitted because MiniPython does not track
source lines yet; sandbox limits and syntax errors keep their one-line
`sandbox error:` / `parse error:` prefixes. Files in
the script directory whose names are not module names, such as `self-test.py`,
are skipped by the import index, so they can still be run as the entry script.
`sys.stdout` and `sys.stderr` are in-memory text streams: `print(file=...)`,
`write()`, `writelines()` and rebinding `sys.stdout` behave like CPython, stdin
is always at end of file, and `fileno()` raises `io.UnsupportedOperation` so no
host file descriptor is ever exposed.
Command-line scripts built on `argparse` therefore run unchanged:
`mnpy tool.py --help` prints help to stdout and exits 0, and a usage error
prints the usage line and `tool.py: error: ...` to stderr and exits 2, exactly
as CPython does (help is formatted for CPython's non-terminal width of 80
columns).

Clock and time zone contract: `datetime` reads the host's real wall clock,
and the sandbox's local time zone is always UTC. `TZ` and `/etc/localtime`
are never read, so `datetime.now()`, `date.today()`, `fromtimestamp()`, naive
`timestamp()`, and `astimezone()` behave as they do in CPython running with
`TZ=UTC`. Embedders can pin the clock to a fixed instant for deterministic
runs (`Sandbox::with_clock`, below).

On Linux `math.cbrt` returns the correctly rounded cube root, where Linux
CPython's (GNU libc's) is off in the last place or two for some inputs
(`cbrt(27.0)` is `3.0000000000000004` there); tests/README.md lists them.

`mnpy` always runs code through the sandbox boundary:

```bash
mnpy --max-memory-bytes 134217728 -c "print(1 + 2)"
```

`mnpy` applies the fixed safe-stdlib allowlist, source-size, instruction,
call-depth, captured-output, VM-allocation, and child-process memory limits. It
always launches an internal worker; there is no public flag that disables the
sandbox. On macOS the parent monitors the worker's physical footprint; on other
Unix hosts it uses kernel process limits (`RLIMIT_AS` and `RLIMIT_DATA`), under
which running out of memory makes an allocation fail and the worker abort.
Either way the run ends with the memory limit (`ExecutionStatus::MemoryLimit`,
`sandbox error: worker memory limit exceeded`). The Linux limit
counts address space, including the whole interpreter stack reservation, so
the worker keeps GNU libc to a single malloc arena instead of reserving 64 MiB
per thread. File execution uses the script's
directory as the default sandbox module root; `-c` and stdin expose no host
module root unless `--root` is passed. Library APIs remain useful for focused
runtime and parity tests, but in-process calls are not the supported untrusted-
code boundary.

### Rust execution API

Embedders use the same worker boundary as the CLI. `Sandbox` starts the supplied
`mnpy` worker executable, sends a length-framed MessagePack request, and returns
a structured result without exposing VM objects to the host:

```rust
use minipython::{Sandbox, SandboxInputs, SandboxValue};

let sandbox = Sandbox::new("target/release/mnpy");
let mut inputs = SandboxInputs::new();
inputs.insert("price".into(), SandboxValue::from(40_i64));

let result = sandbox.eval_with_inputs("price + 2", inputs);
assert!(result.is_success());
assert_eq!(result.value, Some(SandboxValue::from(42_i64)));
```

`ExecutionResult` separates status, value, exact captured stdout/stderr,
exception phase/type/message, resource usage, and `exit_code` when an uncaught
`SystemExit` ended the program. For an uncaught Python exception, `type_name` is
`type(exc).__name__`, `message` is `str(exc)`, and `traceback` holds the
CPython-shaped report the CLI prints; `error_report()` returns that report, or
the one-line `display_message()` (`sandbox error: ...`, `parse error: ...`) for
failures that are not Python exceptions. `Sandbox::with_argv` sets `sys.argv` for runs
and, once when it starts, for a session, whose later calls see the list as the
program left it. `Sandbox::with_clock(HostClock::fixed_unix_micros(...))` pins the
clock `datetime` reads in every run and session; the default,
`HostClock::System`, is the real wall clock, and in-process APIs take the same
setting through `RuntimeOptions::with_clock`. Inputs and returned values are
restricted to inert data (`None`, booleans, numbers, strings, bytes, bytearrays,
lists, tuples, and dictionaries). Unsupported runtime objects are output-only
`Opaque` descriptions and cannot be injected back into the sandbox. The worker
path is explicit so an embedding application controls which reviewed sidecar it
executes.

Terminal conditions end the whole run: the instruction, allocation, and output
budgets, the native-stack ceiling, the worker's wall-clock and memory limits,
and worker failures. They are not Python exceptions. No `except` clause (not
even `except BaseException`), `finally` block, `with` statement, generator, or
native callback (an `re.sub` replacement function, a sort key, a `json` hook,
or a `__str__`/`__repr__` called by a builtin) observes or survives one, and
output captured before it is kept. `ExecutionResult::terminal` names the
condition as a `TerminalKind` (`InstructionLimit`, `AllocationLimit`,
`OutputLimit`, `NativeStackExhausted`, `TimeLimit`, `MemoryLimit`, or
`WorkerFailure`), and is `None` for every other outcome; messages and
`error_report()` are unchanged. A session survives a VM budget condition,
whose next call starts with fresh budgets, but not a wall-clock or memory
termination.

Denial log: `ExecutionResult::denials` lists what the sandbox refused during
the run, whether or not the program caught the resulting exception. A
`Denial { kind, target, count }` is a blocked import (`DenialKind::Import`,
the module name, from `import`, `from ... import`, or `__import__`), a host
builtin looked up by name (`DenialKind::Builtin`: `open`, `input`, `help`,
`exit`, `quit`, `copyright`, `credits`, or `license`), or a host attribute of
an allowed module (`DenialKind::Attribute`: `builtins.open` and the other
builtins above, including the `open` that `argparse.FileType` and
`fromfile_prefix_chars` reach, `io.open`, `io.open_code`, or `io.FileIO`). Entries appear once each, in
first-seen order, with a saturating count; a run records at most
`MAX_DENIALS` (32) distinct entries with targets of at most 128 bytes and
drops later distinct ones. The program itself still sees CPython's exception
for a missing name, and cannot read the log.

Host capabilities are opt-in external functions. A function exists in Python
only after the embedding application registers its exact global name:

```rust
use minipython::{ExternalFunctionError, SandboxValue};

sandbox.register_external_function("lookup_price", |call| {
    let Some(SandboxValue::String(symbol)) = call.args.first() else {
        return Err(ExternalFunctionError::new("TypeError", "symbol must be a string"));
    };
    Ok(SandboxValue::from(format!("quote:{symbol}")))
})?;
```

Calls and responses cross the worker protocol using the same inert value
allowlist. Runtime objects cannot be passed to the host, opaque host values
cannot be returned, callback errors become catchable Python exceptions, and a
callback panic cannot unwind through `Sandbox::execute`. External callbacks are
trusted host code: they may deliberately perform capabilities granted by the
application, and their own runtime is not forcibly cancellable by the worker's
wall-clock limit.

For stateful agent workflows, create a `SandboxSession` from the same configured
sandbox:

```rust
let mut session = sandbox.session()?;
let setup = session.run("balance = 40");
assert!(setup.is_success());
let result = session.eval("balance + 2");
assert_eq!(result.value, Some(SandboxValue::from(42_i64)));
session.close()?;
```

A session keeps one isolated worker and preserves Python globals, function
closures, mutations, and the module cache without replaying earlier source.
Calls are sequential through `&mut self`; every call receives fresh VM and
wall-clock budgets. A process-memory or wall-clock termination closes the whole
session, and later calls fail instead of reusing a possibly corrupted worker.
Dropping a session performs a bounded close and reaps the worker.

Worker isolation guarantee: every `Sandbox` execution (`run`, `eval`,
`check`, `execute`) starts a new worker process that serves that one
execution and is reaped before the call returns, and every `SandboxSession`
owns exactly one worker for its whole life. A worker is never reused by a
later execution, shared between sessions (including sessions of a cloned
`Sandbox`), or used by a one-shot execution while its session is alive; a
session's state therefore never reaches another execution, and a closed or
dropped session's worker is reaped.

Worker start-up hygiene: the parent starts every worker with an empty
environment apart from the internal worker marker, so the embedder's
variables (credentials, proxies, `PATH`) never reach it. Before reading its
request the worker closes every inherited descriptor except stdin, stdout,
and stderr, clears its own environment (no variable is kept: nothing in the
worker reads one), changes its working directory to `/`, sets its umask to
`077`, and sets `RLIMIT_CORE` to zero. Because the worker runs in `/`, the
parent resolves a relative module root against its own current directory
before sending the request, so module-root errors now name the absolute
path.

### Python binding

The `python/` package exposes the same product surface to a Python host without
loading MiniPython into that host process:

```python
from minipython_sandbox import Sandbox

with Sandbox(executable="target/release/mnpy") as sandbox:
    result = sandbox.eval("price + 2", {"price": 40})
    assert result.is_success
    assert result.value == 42
```

`Sandbox` supports `run`, `eval`, `check`, explicit `external_functions`, and
one persistent `session()` using the same inert values and structured results
as the Rust API, including `ExecutionResult.traceback` and
`ExecutionResult.terminal` (`"instruction_limit"`, `"allocation_limit"`,
`"output_limit"`, `"native_stack_exhausted"`, `"time_limit"`,
`"memory_limit"`, or `"worker_failure"`) and `ExecutionResult.denials`, a
tuple of `Denial(kind, target, count)` whose kinds are `"import"`,
`"builtin"`, and `"attribute"`. The package is pure Python and uses only the standard library.
It starts a private JSON-line broker in the reviewed `mnpy` executable; that
broker delegates every execution to the Rust `Sandbox` API, which still starts
and monitors the isolated MessagePack worker. There is no C extension,
in-process VM path, shell invocation, or flag that disables worker limits.

CLI execution is bounded to 1,000,000 VM instructions by default. Use
`--max-steps N` to select a smaller or larger budget. Library callers can use
`RuntimeOptions::with_max_instructions`; `SandboxPolicy` also applies the same
finite default to virtual and sandbox-directory modules. The budget is shared
across functions, generators, coroutines, dynamic execution, and imports.
The worker also has a 5-second wall-clock deadline by default, covering parsing,
compilation, VM execution, and shutdown. Use `--max-time-ms N` to configure it.
Python recursion follows CPython 3.14.6: Python frames count against
`sys.getrecursionlimit()`, which starts at 1000, and going deeper raises a
catchable `RecursionError: maximum recursion depth exceeded`.
`sys.setrecursionlimit()` has CPython's semantics, including accepting any
larger positive limit. An operator may cap it with `--max-depth N` (or
`SandboxLimits::max_call_depth`, `RuntimeOptions::with_max_call_depth`); there
is no cap by default. Under a cap the limit starts at the smaller of 1000 and
the cap, and asking for more raises
`ValueError: recursion limit N exceeds the sandbox call-depth limit M`
instead of being silently lowered. Every entry point runs Python on a dedicated interpreter
thread with an explicitly sized stack (256 MiB in process; in the worker a
quarter of `--max-memory-bytes`, at most 256 MiB), independent of
`RUST_MIN_STACK`. A native stack guard turns deep recursion inside the
interpreter (repr, comparison, hashing, JSON, `copy.deepcopy`, iterator
chains, the parser, and the compiler) into CPython's `RecursionError` (or the
parser's `MemoryError`) instead of a native stack overflow, and deeply nested
values are freed without native recursion. When a raised limit outruns the
interpreter stack, the guard raises CPython's C-stack report,
`RecursionError: Stack overflow (used N kB)` with the operation's context.
Calls share the callee's bytecode instead of copying it, but each Python frame
still holds heap memory (its register file and frame state, which grow with
the size of the function body). The interpreter estimates it and allows live
frames twice the interpreter stack (half of `--max-memory-bytes` in the
worker): entering a frame beyond that raises the same catchable
`RecursionError: Stack overflow (used N kB)`, and a running frame whose
registers grow past it is charged to the allocation budget. Deep recursion
through large functions therefore ends in a Python error or a budget error
rather than exhausting worker memory, but under the default limits it can stop
short of the 1000-frame limit CPython reaches: in a release worker, recursion
through 30-statement bodies reaches the limit, while 100-statement bodies stop
after about 400 frames and 400-statement bodies after about 100.
CPython runs Python-to-Python calls without C recursion, so it reaches far
deeper raised limits than MiniPython does today (about 50,000 frames in
release and 15,000 in debug in process; less in a worker, whose stack and
memory are a share of `--max-memory-bytes`). Code that keeps recursing
inside the guard's headroom stops with the uncatchable
`sandbox error: native stack exhausted`.
Captured output is bounded to 1 MiB by default and shares one byte budget across
nested execution; use `--max-output-bytes N` or
`RuntimeOptions::with_max_output_bytes` to configure it.
Core VM value materialization has a shared monotonic 8 MiB default budget,
configurable with `--max-allocated-bytes N` or
`RuntimeOptions::with_max_allocated_bytes`. This complements the existing
64 MiB single-allocation guard. The `mnpy` child-process boundary covers
compiler and host allocations outside VM value accounting.

Runnable boundary examples live under `examples/sandbox/`. For example,
`mnpy examples/sandbox/blocked_host_capabilities.py` shows the host I/O,
network, process, and C-ABI capabilities that the sandbox intentionally blocks.

The current whole-project sandbox completion state is tracked in
`tests/README.md`. A green parity corpus alone is not a completion signal.
The exact core runtime stopping point, including the disposition of every
CPython coverage row that remains partial, is in `tests/README.md`.
Run `tests/run.sh --focused` while developing sandbox controls,
`tests/run.sh --discovery` for differential discovery only,
`tests/run.sh` for the complete test suite, and
`tests/run.sh --release-gate` for the complete release gate
([RELEASING.md](RELEASING.md)). This is now the only test entrypoint; the old
sandbox and gap-sweep runners were removed.

## Testing

```bash
tests/run.sh --focused
tests/run.sh --discovery
tests/run.sh --discovery --seed 20260710 --generated-cases 1024
tests/run.sh
tests/run.sh --module json
tests/run.sh --root-cause json-loads-core
MINIPYTHON_CPYTHON_SRC=/path/to/cpython-3.14.6 tests/run.sh --release-gate
```

The CPython 3.14.6 oracle is the executable named by `MINIPYTHON_CPYTHON`
(default `/opt/homebrew/bin/python3`); its standard library and `Lib/test`
paths come from the oracle itself through `sysconfig`, so Linux and CI only
need the variable set, for example
`MINIPYTHON_CPYTHON=/usr/local/bin/python3.14 tests/run.sh`. An install
without `Lib/test` (the official Docker images strip it) needs
`MINIPYTHON_CPYTHON_LIB` set to the `Lib` directory of the 3.14.6 source
tree. The grammar audit
reads `Grammar/python.gram` from a CPython 3.14.6 source tree named by
`MINIPYTHON_CPYTHON_SRC`; `MINIPYTHON_REQUIRE_CPYTHON_SRC=1` makes a missing
tree fail instead of skipping that audit, and the release gate requires both.

The unified pipeline runs driver unit tests, the checked-in corpus, and
deterministic generated programs through real CPython and the sandbox-default
`mnpy`. The fixed seed is balanced across syntax, runtime, stdlib, and security
layers. Unexpected generated differences are minimized and written under
`reports/differential-repros/`; one representative per root cause is minimized
while every original difference remains in the report. Reports preserve original
and minimized source, classification, root cause, seed, interpreter output, and
shrink attempts.
The release and discovery modes fail while a generated `must_fix` or
`should_fix` root cause remains open. A worker crash (`CRASH`) or a broken
expectation (`EXPECTED_CONTRACT_DIFF`) fails every gated run regardless of
priority; a case marked as an expected difference, such as an intentional
sandbox block, breaks its contract when MiniPython starts matching CPython, so a
bypassed block or a stale marker can never pass silently. Reports also record
the `mnpy` binary's SHA-256 and the git revision under test. The pipeline uses
`$MINIPYTHON_CPYTHON` (default `/opt/homebrew/bin/python3`) as the fixed CPython
oracle, checks it against the pinned `.python-version`, and builds `mnpy`. Promoted behavior still needs focused
`cpython_subset`, `cpython_diff`, manifest, coverage, and migration evidence.
Gap reports record both the required pinned CPython version and the actual
oracle/driver interpreter paths so a stale oracle cannot hide in the results.
Use `--module` to focus a batch run on one affected surface, for example
`json`, `collections.abc`, or `math.integer`. The report keeps interpreter
`status` separate from workflow `triage_status`: passing cases, accepted
sandbox/compatibility gaps, and unexpected diffs that need root-cause work are
machine-readable in the JSON output.
Use `--root-cause` when moving from discovery to repair so a commit can address
one grouped cause while covering all affected cases in the report. JSON reports
also include `open_root_causes`, the current machine-readable repair queue for
root causes that still have `needs_triage` cases. The runner enables
`--fail-on-open` so unexpected open root causes fail the batch while accepted
sandbox/compatibility gaps stay visible in the report. Open root-cause reports
also include the focused `tests/run.sh --root-cause ...`
command to rerun the grouped repair slice.

## Architecture

```
Rust host / CLI -----------------> Sandbox API -> MessagePack worker -> Register VM
Python host -> private JSON broker -----------^
```

A register-based VM with 80+ instructions and 60+ value types.

Every interpreter call that touches host state lives in `src/host/`: the
clock and UTC local zone (`clock`), reading a module root into memory before
any Python runs (`module_root`), and the worker's start-up hygiene and
protocol channel (`worker`). The VM itself has no file, environment,
process, network, entropy, or current-directory access. Supervising a worker
(spawning, memory and wall-clock enforcement, kill and reap) stays on the
parent side in `src/product.rs`, and the `mnpy` command line in
`src/sandbox.rs`. A lint in `cargo test` (`host_capability_lint` in
`tests/sandbox.rs`) fails on any host-touching call outside `src/host/`
except the listed parent and CLI call sites, each with its reason.

## License

MiniPython is dual licensed. The core -- the interpreter, the sandbox, and
the embedding API -- is MIT; see [LICENSE](LICENSE) and
[LICENSES/MIT.txt](LICENSES/MIT.txt).

Some modules (`re`, `argparse`, `csv`, `datetime`, the exception reports, and
the annotation unparser) and the test cases are derived from CPython 3.14.6
and are therefore also under the PSF License 2.0; see [LICENSE-PSF](LICENSE-PSF)
and [LICENSES/PSF-2.0.txt](LICENSES/PSF-2.0.txt). The files derived from
CPython are listed in [NOTICE](NOTICE).

Python is a trademark of the Python Software Foundation. MiniPython is not
affiliated with or endorsed by the Python Software Foundation.
