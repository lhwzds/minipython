// SPDX-License-Identifier: MIT

use std::path::Path;
use std::process::{Command, Output};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use minipython::HostClock;
use minipython::{
    Denial, DenialKind, ExecutionPhase, ExecutionStatus, ExternalCall, ExternalFunctionError,
    Sandbox, SandboxInputs, SandboxLimits, SandboxValue, TerminalKind,
};

/// The CPython 3.14.6 oracle executable: `$MINIPYTHON_CPYTHON`, or
/// `/opt/homebrew/bin/python3` when it is unset. `tests/run.sh` checks its
/// version against `.python-version` before any test runs.
fn cpython_oracle() -> String {
    std::env::var("MINIPYTHON_CPYTHON").unwrap_or_else(|_| "/opt/homebrew/bin/python3".to_string())
}

const ARGPARSE_CLI_EXAMPLE: &str = include_str!("../examples/sandbox/argparse_cli.py");

fn sandbox() -> Sandbox {
    Sandbox::new(env!("CARGO_BIN_EXE_mnpy"))
}

fn run_python_binding(source: &str) -> Output {
    Command::new(crate::cpython_oracle())
        .args(["-B", "-c", source])
        .env(
            "PYTHONPATH",
            Path::new(env!("CARGO_MANIFEST_DIR")).join("python"),
        )
        .env("MINIPYTHON_EXECUTABLE", env!("CARGO_BIN_EXE_mnpy"))
        .output()
        .expect("run Python binding client")
}

fn assert_python_binding_succeeds(source: &str) {
    let output = run_python_binding(source);
    assert!(
        output.status.success(),
        "Python binding client failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn structured_exec_preserves_exact_stdout_and_reports_usage() {
    let result = sandbox().run("print('a', end='')\nprint('b')\n");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.value, Some(SandboxValue::None));
    assert_eq!(result.stdout, "ab\n");
    assert_eq!(result.stderr, "");
    assert!(result.exception.is_none());
    assert!(result.usage.instructions > 0);
    assert_eq!(result.usage.output_bytes, 3);
    assert!(result.usage.wall_time_micros > 0);
}

#[test]
fn structured_eval_round_trips_nested_inputs_and_results() {
    let mut inputs = SandboxInputs::new();
    inputs.insert("answer".to_string(), SandboxValue::from(40_i64));
    inputs.insert(
        "items".to_string(),
        SandboxValue::List(vec![SandboxValue::from(2_i64), SandboxValue::from(5_i64)]),
    );
    inputs.insert("blob".to_string(), SandboxValue::Bytes(vec![0, 1, 255]));

    let result = sandbox().eval_with_inputs("{'answer': answer + items[1], 'blob': blob}", inputs);

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(
        result.value,
        Some(SandboxValue::Dict(vec![
            (SandboxValue::from("answer"), SandboxValue::from(45_i64),),
            (
                SandboxValue::from("blob"),
                SandboxValue::Bytes(vec![0, 1, 255]),
            ),
        ]))
    );
    assert_eq!(result.stdout, "");
}

#[test]
fn structured_runtime_error_keeps_prior_stdout_and_exception_shape() {
    let result = sandbox().run("print('before')\n1 / 0\n");

    assert_eq!(result.status, ExecutionStatus::Error);
    assert_eq!(result.stdout, "before\n");
    let exception = result.exception.expect("runtime exception");
    assert_eq!(exception.phase, ExecutionPhase::Runtime);
    assert_eq!(exception.type_name, "ZeroDivisionError");
    assert!(exception.message.contains("division by zero"));
    assert!(result.usage.instructions > 0);
}

// Structured results keep phase/type/message (`type(exc).__name__` and
// `str(exc)`), while `traceback` and `error_report()` carry the CPython-shaped
// stderr report the CLI prints. Phase prefixes remain for non-Python failures.
#[test]
fn structured_uncaught_exceptions_keep_fields_and_carry_cpython_report() {
    let cases = [
        (
            "class MyErr(Exception):\n    pass\nraise MyErr()\n",
            "MyErr",
            "",
            "Traceback (most recent call last):\nMyErr\n",
        ),
        (
            "{}['k']\n",
            "KeyError",
            "'k'",
            "Traceback (most recent call last):\nKeyError: 'k'\n",
        ),
        (
            "import json\njson.loads('{')\n",
            "JSONDecodeError",
            "Expecting property name enclosed in double quotes: line 1 column 2 (char 1)",
            "Traceback (most recent call last):\njson.decoder.JSONDecodeError: Expecting property name enclosed in double quotes: line 1 column 2 (char 1)\n",
        ),
        (
            "try:\n    1 / 0\nexcept ZeroDivisionError as error:\n    error.add_note('note')\n    raise ValueError('bad') from error\n",
            "ValueError",
            "bad",
            "Traceback (most recent call last):\nZeroDivisionError: division by zero\nnote\n\nThe above exception was the direct cause of the following exception:\n\nTraceback (most recent call last):\nValueError: bad\n",
        ),
    ];
    for (source, type_name, message, traceback) in cases {
        let result = sandbox().run(source);
        assert_eq!(result.status, ExecutionStatus::Error, "{source}");
        let exception = result.exception.as_ref().expect("runtime exception");
        assert_eq!(exception.phase, ExecutionPhase::Runtime, "{source}");
        assert_eq!(exception.type_name, type_name, "{source}");
        assert_eq!(exception.message, message, "{source}");
        assert_eq!(result.traceback.as_deref(), Some(traceback), "{source}");
        assert_eq!(result.error_report(), traceback, "{source}");
    }

    let empty = sandbox().run("raise ValueError\n");
    let exception = empty.exception.as_ref().expect("runtime exception");
    assert_eq!(exception.display_message(), "runtime error: ValueError");

    let exit = sandbox().run("import sys\nsys.exit('bye')\n");
    assert_eq!(exit.exit_code, Some(1));
    assert_eq!(exit.traceback, None);
    assert_eq!(exit.stderr, "bye\n");

    let syntax = sandbox().run("def broken(:\n");
    assert_eq!(syntax.traceback, None);
    let report = syntax.error_report();
    assert!(report.starts_with("parse error: "), "{report}");

    let limited = sandbox()
        .with_limits(SandboxLimits {
            max_instructions: 50,
            ..SandboxLimits::default()
        })
        .run("while True:\n    pass\n");
    assert_eq!(limited.traceback, None);
    assert_eq!(
        limited.error_report(),
        "sandbox error: instruction limit exceeded\n"
    );
}

#[test]
fn argv_is_exposed_as_sys_argv_in_runs_and_sessions() {
    let sandbox = sandbox().with_argv(["tool.py", "--name", "a b"]);
    let result = sandbox.run("import sys\nprint(sys.argv)\n");
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.stdout, "['tool.py', '--name', 'a b']\n");

    let mut session = sandbox.session().expect("start session");
    let first = session.run("def show():\n    import sys\n    return sys.argv\nprint(show())\n");
    assert_eq!(first.status, ExecutionStatus::Success);
    assert_eq!(first.stdout, "['tool.py', '--name', 'a b']\n");

    // argv is set when the session starts, not on every call: changes and
    // references made by one call survive into the next.
    let second = session.run(
        "import sys\nkept = sys.argv\nsys.argv.append('--more')\nsys.argv[0] = 'renamed.py'\n",
    );
    assert_eq!(second.status, ExecutionStatus::Success);
    let third = session.run("import sys\nprint(sys.argv, kept is sys.argv)\n");
    assert_eq!(third.status, ExecutionStatus::Success);
    assert_eq!(
        third.stdout,
        "['renamed.py', '--name', 'a b', '--more'] True\n"
    );
    session.close().expect("close session");
}

#[test]
fn argparse_programs_match_cpython_help_usage_errors_and_subcommands() {
    let dir = std::env::temp_dir().join(format!(
        "minipython-product-argparse-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create argparse fixture directory");
    let script = dir.join("argparse_cli.py");
    std::fs::write(&script, ARGPARSE_CLI_EXAMPLE).expect("write argparse fixture");
    let invocations: &[&[&str]] = &[
        &["--help"],
        &["stats", "[]", "--round", "many"],
        &["stats", "[2, 4]"],
        &["keys", "{\"b\": 1, \"a\": 2}", "--sort"],
        &["pick", "{\"a\": 1}", "z", "--missing", "error"],
        &["delete", "x"],
    ];
    for args in invocations {
        let cpython = Command::new(crate::cpython_oracle())
            .args(["-I", "-B"])
            .arg(&script)
            .args(*args)
            .env_remove("COLUMNS")
            .env_remove("LINES")
            .env_remove("FORCE_COLOR")
            .env_remove("NO_COLOR")
            .env_remove("PYTHON_COLORS")
            .output()
            .expect("run CPython oracle");
        let argv = std::iter::once("argparse_cli.py").chain(args.iter().copied());
        let result = sandbox().with_argv(argv).run(ARGPARSE_CLI_EXAMPLE);
        assert_eq!(
            result.stdout,
            String::from_utf8_lossy(&cpython.stdout),
            "stdout for {args:?}"
        );
        assert_eq!(
            result.stderr,
            String::from_utf8_lossy(&cpython.stderr),
            "stderr for {args:?}"
        );
        assert_eq!(result.exit_code, cpython.status.code(), "exit for {args:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pinned_clock_makes_datetime_deterministic_in_runs_and_sessions() {
    // 2024-03-10T12:34:56.789012Z
    let pinned = sandbox().with_clock(HostClock::fixed_unix_micros(1_710_074_096_789_012));
    let source = "import datetime\nprint(datetime.datetime.now(), datetime.date.today(), datetime.datetime.now(datetime.UTC).isoformat())\nprint(datetime.datetime.now().astimezone().tzname(), datetime.datetime.now().timestamp())";
    let expected = "2024-03-10 12:34:56.789012 2024-03-10 2024-03-10T12:34:56.789012+00:00\nUTC 1710074096.789012\n";
    let result = pinned.run(source);
    assert!(result.is_success(), "{result:?}");
    assert_eq!(result.stdout, expected);

    let mut session = pinned.session().expect("start session");
    let first = session.run(source);
    assert_eq!(first.stdout, expected, "{first:?}");
    let second = session.eval("datetime.datetime.now().isoformat()");
    assert_eq!(
        second.value,
        Some(SandboxValue::from("2024-03-10T12:34:56.789012"))
    );
    session.close().expect("close session");

    // The default is the real wall clock, still in a UTC local zone.
    let real = sandbox().eval(
        "__import__('datetime').datetime.now().year >= 2024 and __import__('datetime').datetime.now().astimezone().utcoffset().total_seconds() == 0",
    );
    assert_eq!(real.value, Some(SandboxValue::from(true)));
}

#[test]
fn uncaught_system_exit_reports_process_exit_codes() {
    let cases = [
        (
            "import sys\nprint('out')\nsys.exit()\n",
            ExecutionStatus::Success,
            0,
            "out\n",
            "",
        ),
        (
            "import sys\nsys.exit(3)\n",
            ExecutionStatus::Error,
            3,
            "",
            "",
        ),
        (
            "import sys\ndef main():\n    sys.exit('bad input')\nmain()\n",
            ExecutionStatus::Error,
            1,
            "",
            "bad input\n",
        ),
        (
            "raise SystemExit(256)\n",
            ExecutionStatus::Success,
            0,
            "",
            "",
        ),
        (
            "raise SystemExit(-1)\n",
            ExecutionStatus::Error,
            255,
            "",
            "",
        ),
        (
            "raise SystemExit(2 ** 70)\n",
            ExecutionStatus::Error,
            255,
            "",
            "",
        ),
        (
            "import sys\nsys.exit((1, 2))\n",
            ExecutionStatus::Error,
            1,
            "",
            "(1, 2)\n",
        ),
        (
            "class Done(SystemExit):\n    pass\nraise Done(7)\n",
            ExecutionStatus::Error,
            7,
            "",
            "",
        ),
    ];
    for (source, status, code, stdout, stderr) in cases {
        let result = sandbox().run(source);
        assert_eq!(result.status, status, "{source}");
        assert_eq!(result.exit_code, Some(code), "{source}");
        assert_eq!(result.stdout, stdout, "{source}");
        assert_eq!(result.stderr, stderr, "{source}");
    }

    let failed = sandbox().run("import sys\nsys.exit(3)\n");
    let exception = failed.exception.expect("SystemExit exception");
    assert_eq!(exception.phase, ExecutionPhase::Runtime);
    assert_eq!(exception.type_name, "SystemExit");
    assert_eq!(exception.message, "3");
}

#[test]
fn stdout_and_stderr_streams_are_captured_separately() {
    let result = sandbox().run(
        "import sys\nprint('out')\nprint('err', 1, file=sys.stderr)\nprint(sys.stdout.write('w\\n'), sys.stderr.write('e\\n'))\nclass Capture:\n    def __init__(self):\n        self.parts = []\n    def write(self, text):\n        self.parts.append(text)\n        return len(text)\nsaved = sys.stdout\nsys.stdout = Capture()\nprint('hidden')\ncaptured = sys.stdout.parts\nsys.stdout = saved\nprint(captured)\n",
    );
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.stdout, "out\nw\n2 2\n['hidden', '\\n']\n");
    assert_eq!(result.stderr, "err 1\ne\n");
}

#[test]
fn stdio_streams_expose_no_host_descriptors() {
    let result = sandbox().run(
        "import io, sys\nfor stream in (sys.stdin, sys.stdout, sys.stderr):\n    try:\n        stream.fileno()\n    except io.UnsupportedOperation as error:\n        print(stream.name, 'blocked', error)\nprint(repr(sys.stdin.read()), sys.stdin.readlines())\nprint('bye', file=sys.stderr)\nsys.exit('failed')\n",
    );
    assert_eq!(result.status, ExecutionStatus::Error);
    assert_eq!(result.exit_code, Some(1));
    assert_eq!(
        result.stdout,
        "<stdin> blocked fileno\n<stdout> blocked fileno\n<stderr> blocked fileno\n'' []\n"
    );
    assert_eq!(result.stderr, "bye\nfailed\n");
}

#[test]
fn budget_and_runtime_errors_never_report_an_exit_code() {
    let limits = SandboxLimits {
        max_instructions: 50,
        ..SandboxLimits::default()
    };
    let budget = sandbox().with_limits(limits).run("while True:\n    pass\n");
    assert_eq!(budget.status, ExecutionStatus::Error);
    assert_eq!(budget.exit_code, None);

    let caught = sandbox().run(
        "import sys\ntry:\n    sys.exit(4)\nexcept SystemExit as error:\n    print('caught', error.code)\n",
    );
    assert_eq!(caught.status, ExecutionStatus::Success);
    assert_eq!(caught.stdout, "caught 4\n");
    assert_eq!(caught.exit_code, None);

    assert_eq!(sandbox().run("raise ValueError('x')\n").exit_code, None);
    assert_eq!(sandbox().run("print(1)\n").exit_code, None);
}

/// Each VM terminal kind, the limits that make `T()` trip it, and `T`.
fn terminal_triggers() -> Vec<(TerminalKind, SandboxLimits, &'static str)> {
    let defaults = SandboxLimits::default();
    vec![
        (
            TerminalKind::InstructionLimit,
            SandboxLimits {
                max_instructions: 50_000,
                ..defaults.clone()
            },
            "def T():\n    while True:\n        pass\n",
        ),
        (
            TerminalKind::AllocationLimit,
            SandboxLimits {
                max_instructions: 50_000_000,
                max_allocated_bytes: 400_000,
                ..defaults.clone()
            },
            "def T():\n    items = []\n    while True:\n        items.append('y' * 1000)\n",
        ),
        (
            TerminalKind::OutputLimit,
            SandboxLimits {
                max_instructions: 50_000_000,
                max_output_bytes: 4_000,
                ..defaults
            },
            "def T():\n    while True:\n        print('z' * 100)\n",
        ),
    ]
}

/// Handler shapes that would swallow an ordinary exception raised by `T()`.
const TERMINAL_CONTEXTS: &[&str] = &[
    "T()",
    "def g():\n    try:\n        T()\n        yield 1\n    except BaseException:\n        yield 'CAUGHT'\nprint(list(g()))",
    "class C:\n    def __enter__(self):\n        return self\n    def __exit__(self, *exc):\n        print('CAUGHT')\n        return True\nwith C():\n    T()",
    "try:\n    T()\nfinally:\n    print('CAUGHT finally')",
    "import re\nre.sub('a', lambda m: T(), 'aaa')",
    "sorted([2, 1], key=lambda v: T())",
    "import json\njson.loads('[1]', object_hook=lambda d: T(), parse_int=lambda s: T())",
    "class A:\n    def __str__(self):\n        return T()\nprint(A())",
];

#[test]
fn terminal_conditions_are_typed_and_uncatchable_through_a_real_worker() {
    for (kind, limits, definition) in terminal_triggers() {
        let sandbox = sandbox().with_limits(limits);
        let mut session = sandbox.session().expect("start session");
        for context in TERMINAL_CONTEXTS {
            let indented = context
                .lines()
                .map(|line| format!("    {line}\n"))
                .collect::<String>();
            let source = format!(
                "{definition}print('BEFORE')\ntry:\n{indented}except BaseException as error:\n    print('CAUGHT', type(error).__name__)\nprint('CONTINUED')\n"
            );
            for (label, result) in [
                ("run", sandbox.run(&source)),
                ("session", session.run(&source)),
            ] {
                assert_eq!(
                    result.terminal,
                    Some(kind),
                    "{label} {kind:?} in {context:?}"
                );
                assert_eq!(result.status, ExecutionStatus::Error);
                // Only the program's own output before the condition; the
                // output trigger's lines are `z` lines.
                let lines = result
                    .stdout
                    .lines()
                    .filter(|line| !line.starts_with('z'))
                    .collect::<Vec<_>>();
                assert_eq!(lines, ["BEFORE"], "{label} {kind:?} in {context:?}");
                assert_eq!(result.traceback, None);
                assert_eq!(result.exit_code, None);
                assert_eq!(
                    result.error_report(),
                    format!("sandbox error: {}\n", kind.message())
                );
                let exception = result.exception.expect("terminal exception");
                assert_eq!(exception.phase, ExecutionPhase::Sandbox);
                assert_eq!(exception.type_name, "ResourceError");
                assert_eq!(exception.message, kind.message());
            }
            // The session survives a VM terminal condition with fresh
            // budgets and a cleared latch.
            let next = session.eval("40 + 2");
            assert_eq!(next.value, Some(SandboxValue::from(42_i64)), "{kind:?}");
            assert_eq!(next.terminal, None);
        }
        session.close().expect("close session");
    }
}

#[test]
fn parent_side_terminal_conditions_are_typed() {
    let timed_out = sandbox()
        .with_limits(SandboxLimits {
            max_time_ms: 200,
            max_instructions: 100_000_000_000,
            ..SandboxLimits::default()
        })
        .run("try:\n    while True:\n        pass\nexcept BaseException:\n    print('CAUGHT')\n");
    assert_eq!(timed_out.status, ExecutionStatus::TimeLimit);
    assert_eq!(timed_out.terminal, Some(TerminalKind::TimeLimit));
    assert_eq!(timed_out.stdout, "");

    // macOS kills the worker for its polled footprint; on Linux the kernel
    // limit makes an allocation fail and the worker abort. Both are the
    // memory limit, for a run and for a session.
    let oversized_sandbox = sandbox().with_limits(SandboxLimits {
        max_process_memory_bytes: 96 * 1_048_576,
        max_time_ms: 20_000,
        max_instructions: 100_000_000_000,
        max_allocated_bytes: usize::MAX,
        ..SandboxLimits::default()
    });
    let grow = "blocks = []\ntry:\n    while True:\n        blocks.append(bytearray(1_000_000))\nexcept BaseException:\n    print('CAUGHT')\n";
    let mut session = oversized_sandbox.session().expect("start session");
    for (label, oversized) in [
        ("run", oversized_sandbox.run(grow)),
        ("session", session.run(grow)),
    ] {
        assert_eq!(oversized.status, ExecutionStatus::MemoryLimit, "{label}");
        assert_eq!(
            oversized.terminal,
            Some(TerminalKind::MemoryLimit),
            "{label}"
        );
        assert_eq!(oversized.stdout, "", "{label}");
        assert_eq!(
            oversized.error_report(),
            "sandbox error: worker memory limit exceeded\n",
            "{label}"
        );
    }

    let missing = Sandbox::new(Path::new("/definitely/not/a/minipython-worker")).run("pass");
    assert_eq!(missing.terminal, Some(TerminalKind::WorkerFailure));

    // Pre-spawn validation failures are not terminal: nothing ran.
    let oversized_source = sandbox()
        .with_limits(SandboxLimits {
            max_source_bytes: 4,
            ..SandboxLimits::default()
        })
        .run("print(1)\n");
    assert_eq!(oversized_source.status, ExecutionStatus::Error);
    assert_eq!(oversized_source.terminal, None);
    assert_eq!(sandbox().run("raise ValueError('x')\n").terminal, None);
    assert_eq!(sandbox().run("print(1)\n").terminal, None);
}

fn denial(kind: DenialKind, target: &str, count: u32) -> Denial {
    Denial {
        kind,
        target: target.to_string(),
        count,
    }
}

const DENIAL_PROBE: &str = "import json, io, builtins\ntry:\n    import os\nexcept ImportError:\n    pass\ntry:\n    open('/etc/passwd')\nexcept NameError:\n    pass\ntry:\n    io.open\nexcept AttributeError:\n    pass\ntry:\n    builtins.input\nexcept AttributeError:\n    pass\nfor _ in range(3):\n    try:\n        __import__('socket')\n    except ImportError:\n        pass\ntry:\n    from subprocess import run\nexcept ImportError:\n    pass\nprint('done')\n";

fn expected_probe_denials() -> Vec<Denial> {
    vec![
        denial(DenialKind::Import, "os", 1),
        denial(DenialKind::Builtin, "open", 1),
        denial(DenialKind::Attribute, "io.open", 1),
        denial(DenialKind::Attribute, "builtins.input", 1),
        denial(DenialKind::Import, "socket", 3),
        denial(DenialKind::Import, "subprocess", 1),
    ]
}

#[test]
fn denials_record_blocked_capabilities_even_when_caught() {
    let result = sandbox().run(DENIAL_PROBE);
    assert!(result.is_success(), "{:?}", result.exception);
    assert_eq!(result.stdout, "done\n");
    assert_eq!(result.denials, expected_probe_denials());

    // Allowed modules, ordinary missing names, and a program's own `open`
    // are not denials.
    let clean = sandbox().run(
        "import json, re, datetime\ntry:\n    undefined_name\nexcept NameError:\n    pass\ndef open(path):\n    return path\nprint(open('x'))\n",
    );
    assert!(clean.is_success());
    assert_eq!(clean.denials, []);

    // Each session call reports its own denials.
    let mut session = sandbox().session().expect("start session");
    assert_eq!(session.run(DENIAL_PROBE).denials, expected_probe_denials());
    assert_eq!(session.run("print(1)\n").denials, []);
    let uncaught = session.run("import ctypes\n");
    assert_eq!(
        uncaught.exception.expect("blocked import").type_name,
        "ModuleNotFoundError"
    );
    assert_eq!(uncaught.denials, [denial(DenialKind::Import, "ctypes", 1)]);
    session.close().expect("close session");

    // A terminal condition keeps the denials recorded before it, and the
    // log is bounded.
    let spun = sandbox()
        .with_limits(SandboxLimits {
            max_instructions: 200_000,
            ..SandboxLimits::default()
        })
        .run("try:\n    import os\nexcept ImportError:\n    pass\nfor index in range(10**9):\n    try:\n        __import__('blocked%d' % index)\n    except ImportError:\n        pass\n");
    assert_eq!(spun.terminal, Some(TerminalKind::InstructionLimit));
    assert_eq!(spun.denials.len(), minipython::MAX_DENIALS);
    assert_eq!(spun.denials[0], denial(DenialKind::Import, "os", 1));
}

#[test]
fn invalid_inputs_are_rejected_before_starting_a_worker() {
    let unavailable_worker = Sandbox::new(Path::new("/definitely/not/a/minipython-worker"));

    let mut reserved = SandboxInputs::new();
    reserved.insert("class".to_string(), SandboxValue::from(1_i64));
    let reserved_result = unavailable_worker.run_with_inputs("pass", reserved);
    assert_eq!(reserved_result.status, ExecutionStatus::Error);
    assert_eq!(
        reserved_result
            .exception
            .expect("input exception")
            .type_name,
        "SandboxInputError"
    );

    let mut opaque = SandboxInputs::new();
    opaque.insert(
        "value".to_string(),
        SandboxValue::Opaque {
            type_name: "object".to_string(),
            display: "<object>".to_string(),
        },
    );
    let opaque_result = unavailable_worker.run_with_inputs("pass", opaque);
    assert_eq!(opaque_result.status, ExecutionStatus::Error);
    assert_eq!(
        opaque_result.exception.expect("input exception").type_name,
        "SandboxInputError"
    );
}

#[test]
fn worker_start_failure_is_returned_as_a_structured_crash() {
    let result = Sandbox::new(Path::new("/definitely/not/a/minipython-worker")).run("pass");

    assert_eq!(result.status, ExecutionStatus::WorkerCrash);
    let exception = result.exception.expect("worker exception");
    assert_eq!(exception.phase, ExecutionPhase::Worker);
    assert_eq!(exception.type_name, "WorkerError");
    assert!(
        exception
            .message
            .contains("failed to start isolated worker")
    );
}

#[test]
fn invalid_worker_protocol_is_returned_as_a_structured_crash() {
    let executable = std::env::current_exe().expect("current test executable");
    let result = Sandbox::new(executable).run("pass");

    assert_eq!(result.status, ExecutionStatus::WorkerCrash);
    assert_eq!(result.terminal, Some(TerminalKind::WorkerFailure));
    let exception = result.exception.expect("worker exception");
    assert_eq!(exception.phase, ExecutionPhase::Worker);
    assert_eq!(exception.type_name, "WorkerError");
}

#[test]
fn unsupported_runtime_objects_cross_the_boundary_as_opaque_values() {
    let result = sandbox().eval("object()");

    assert_eq!(result.status, ExecutionStatus::Success);
    let Some(SandboxValue::Opaque { type_name, display }) = result.value else {
        panic!("expected opaque result, got {:?}", result.value);
    };
    assert_eq!(type_name, "object");
    assert!(display.contains("object"));
}

#[test]
fn syntax_check_returns_structured_compile_failures() {
    let ok = sandbox().check("value = 1\n");
    assert_eq!(ok.status, ExecutionStatus::Success);

    let error = sandbox().check("if:\n    pass\n");
    assert_eq!(error.status, ExecutionStatus::Error);
    let exception = error.exception.expect("syntax exception");
    assert!(matches!(
        exception.phase,
        ExecutionPhase::Lex | ExecutionPhase::Parse | ExecutionPhase::Compile
    ));
    assert_eq!(exception.type_name, "SyntaxError");
}

#[test]
fn external_functions_are_explicit_and_work_inside_nested_python_calls() {
    let observed = Arc::new(Mutex::new(None::<ExternalCall>));
    let callback_observed = observed.clone();
    let mut sandbox = sandbox();
    sandbox
        .register_external_function("host_add", move |call| {
            *callback_observed.lock().expect("observed call lock") = Some(call);
            Ok(SandboxValue::from(42_i64))
        })
        .expect("register host_add");

    let result =
        sandbox.run("def calculate():\n    return host_add(40, delta=2)\nprint(calculate())\n");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.stdout, "42\n");
    let call = observed
        .lock()
        .expect("observed call lock")
        .clone()
        .expect("host_add call");
    assert_eq!(call.name, "host_add");
    assert_eq!(call.args, vec![SandboxValue::from(40_i64)]);
    assert_eq!(
        call.keywords,
        vec![("delta".to_string(), SandboxValue::from(2_i64))]
    );
}

#[test]
fn external_function_errors_are_catchable_python_exceptions() {
    let mut sandbox = sandbox();
    sandbox
        .register_external_function("host_fail", |_| {
            Err(ExternalFunctionError::new("ValueError", "rejected by host"))
        })
        .expect("register host_fail");

    let result = sandbox.run(
        "try:\n    host_fail()\nexcept ValueError as error:\n    print(type(error).__name__, str(error))\n",
    );

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.stdout, "ValueError rejected by host\n");
}

#[test]
fn external_function_unknown_errors_fall_back_to_catchable_runtime_error() {
    let mut sandbox = sandbox();
    sandbox
        .register_external_function("host_fail", |_| {
            Err(ExternalFunctionError::new(
                "HostApplicationError",
                "rejected by host",
            ))
        })
        .expect("register host_fail");

    let result = sandbox.run(
        "try:\n    host_fail()\nexcept RuntimeError as error:\n    print(type(error).__name__, str(error))\n",
    );

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.stdout, "RuntimeError rejected by host\n");
}

#[test]
fn external_function_boundary_rejects_runtime_objects_without_calling_the_host() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let callback_call_count = call_count.clone();
    let mut sandbox = sandbox();
    sandbox
        .register_external_function("host_value", move |_| {
            callback_call_count.fetch_add(1, Ordering::SeqCst);
            Ok(SandboxValue::None)
        })
        .expect("register host_value");

    let result = sandbox
        .run("try:\n    host_value(object())\nexcept TypeError as error:\n    print(str(error))\n");

    assert_eq!(result.status, ExecutionStatus::Success);
    assert!(
        result
            .stdout
            .contains("external function values cannot contain 'object' objects")
    );
    assert_eq!(call_count.load(Ordering::SeqCst), 0);
}

#[test]
fn external_function_panics_and_invalid_returns_do_not_cross_the_boundary() {
    let mut sandbox = sandbox();
    sandbox
        .register_external_function("host_panic", |_| panic!("host implementation detail"))
        .expect("register host_panic");
    sandbox
        .register_external_function("host_opaque", |_| {
            Ok(SandboxValue::Opaque {
                type_name: "host_object".to_string(),
                display: "secret".to_string(),
            })
        })
        .expect("register host_opaque");

    let result = sandbox.run(
        "try:\n    host_panic()\nexcept RuntimeError as error:\n    print(type(error).__name__, str(error))\ntry:\n    host_opaque()\nexcept TypeError as error:\n    print(type(error).__name__, str(error))\n",
    );

    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(
        result.stdout,
        "RuntimeError external function panicked\nTypeError external function returned an opaque or excessively nested value\n"
    );
}

#[test]
fn external_function_registration_rejects_reserved_and_duplicate_names() {
    let unregistered = sandbox().run("host_call()\n");
    assert_eq!(unregistered.status, ExecutionStatus::Error);
    assert_eq!(
        unregistered
            .exception
            .expect("unregistered name exception")
            .type_name,
        "NameError"
    );

    let mut sandbox = sandbox();
    assert!(
        sandbox
            .register_external_function("class", |_| Ok(SandboxValue::None))
            .is_err()
    );
    sandbox
        .register_external_function("host_call", |_| Ok(SandboxValue::None))
        .expect("register host_call");
    assert!(
        sandbox
            .register_external_function("host_call", |_| Ok(SandboxValue::None))
            .is_err()
    );

    let mut conflicting_inputs = SandboxInputs::new();
    conflicting_inputs.insert("host_call".to_string(), SandboxValue::from(1_i64));
    let conflict = sandbox.run_with_inputs("pass", conflicting_inputs);
    assert_eq!(conflict.status, ExecutionStatus::Error);
    assert_eq!(
        conflict
            .exception
            .expect("input conflict exception")
            .type_name,
        "SandboxInputError"
    );
}

#[test]
fn session_preserves_globals_functions_and_module_cache_without_replay() {
    let calls = Arc::new(AtomicUsize::new(0));
    let callback_calls = calls.clone();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sandbox/import_root");
    let mut sandbox = sandbox().with_root(root);
    sandbox
        .register_external_function("host_mark", move |_| {
            callback_calls.fetch_add(1, Ordering::SeqCst);
            Ok(SandboxValue::None)
        })
        .expect("register host_mark");
    let mut session = sandbox.session().expect("start session");

    let setup = session.run(
        "import plugin\nevents = [plugin.VALUE]\ndef total(delta):\n    return events[0] + delta\nhost_mark('once')\nplugin.VALUE = 10\n",
    );
    assert_eq!(setup.status, ExecutionStatus::Success);

    let value = session.eval("total(plugin.VALUE)");
    assert_eq!(value.status, ExecutionStatus::Success);
    assert_eq!(value.value, Some(SandboxValue::from(17_i64)));

    let cached_module = session.run("import plugin\nprint(plugin.VALUE)\n");
    assert_eq!(cached_module.status, ExecutionStatus::Success);
    assert_eq!(cached_module.stdout, "10\n");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    session.close().expect("close session");
}

#[test]
fn session_keeps_prior_mutations_after_an_exception_and_accepts_new_inputs() {
    let mut session = sandbox().session().expect("start session");

    let failed = session.run("value = 40\n1 / 0\n");
    assert_eq!(failed.status, ExecutionStatus::Error);
    assert_eq!(
        failed.exception.expect("runtime exception").type_name,
        "ZeroDivisionError"
    );

    let value = session.eval("value + 2");
    assert_eq!(value.status, ExecutionStatus::Success);
    assert_eq!(value.value, Some(SandboxValue::from(42_i64)));

    let mut inputs = SandboxInputs::new();
    inputs.insert("factor".to_string(), SandboxValue::from(3_i64));
    let input_value = session.eval_with_inputs("value * factor", inputs);
    assert_eq!(input_value.value, Some(SandboxValue::from(120_i64)));
    assert_eq!(
        session.eval("factor").value,
        Some(SandboxValue::from(3_i64))
    );
}

#[test]
fn session_limit_termination_closes_the_worker() {
    let limits = SandboxLimits {
        max_time_ms: 1,
        max_instructions: 100_000_000,
        ..SandboxLimits::default()
    };
    let mut session = sandbox()
        .with_limits(limits)
        .session()
        .expect("start session");

    let timed_out = session.run("while True:\n    pass\n");
    assert_eq!(timed_out.status, ExecutionStatus::TimeLimit);
    assert_eq!(timed_out.terminal, Some(TerminalKind::TimeLimit));
    assert!(session.is_closed());

    let after_timeout = session.run("print('unreachable')\n");
    assert_eq!(after_timeout.status, ExecutionStatus::WorkerCrash);
    assert_eq!(
        after_timeout
            .exception
            .expect("closed session exception")
            .message,
        "session is closed"
    );
}

#[test]
fn pure_python_binding_returns_structured_values_and_exceptions() {
    assert_python_binding_succeeds(
        r#"
from minipython_sandbox import Limits, OpaqueValue, Sandbox

with Sandbox() as sandbox:
    result = sandbox.eval(
        "{'answer': answer + 2, 'blob': blob, 'pair': pair}",
        {"answer": 40, "blob": b"\x00\xff", "pair": (True, None)},
    )
    assert result.is_success
    assert result.value == {
        "answer": 42,
        "blob": b"\x00\xff",
        "pair": (True, None),
    }
    assert result.stdout == ""
    assert result.usage.instructions > 0

    failed = sandbox.run("print('before')\n1 / 0\n")
    assert failed.status == "error"
    assert failed.stdout == "before\n"
    assert failed.exception.phase == "runtime"
    assert failed.exception.type_name == "ZeroDivisionError"
    assert failed.exception.message == "division by zero"
    assert failed.traceback == (
        "Traceback (most recent call last):\nZeroDivisionError: division by zero\n"
    )
    assert failed.terminal is None

with Sandbox(limits=Limits(max_instructions=50)) as sandbox:
    spun = sandbox.run("try:\n    while True:\n        pass\nexcept BaseException:\n    print('caught')\n")
    assert spun.status == "error"
    assert spun.terminal == "instruction_limit"
    assert spun.stdout == ""
    assert spun.exception.message == "instruction limit exceeded"

    probe = sandbox.run("try:\n    import os\nexcept ImportError:\n    pass\n")
    assert [(d.kind, d.target, d.count) for d in probe.denials] == [("import", "os", 1)]
    assert sandbox.run("print(1)\n").denials == ()

    opaque = sandbox.eval("object()")
    assert isinstance(opaque.value, OpaqueValue)
    assert opaque.value.type_name == "object"
"#,
    );
}

#[test]
fn pure_python_binding_external_functions_are_explicit_and_catchable() {
    assert_python_binding_succeeds(
        r#"
from minipython_sandbox import Sandbox

calls = []
def host_add(value, *, delta):
    calls.append((value, delta))
    return value + delta

def host_fail():
    raise ValueError("rejected by Python host")

with Sandbox(external_functions={"host_add": host_add, "host_fail": host_fail}) as sandbox:
    result = sandbox.run(
        "print(host_add(40, delta=2))\n"
        "try:\n"
        "    host_fail()\n"
        "except ValueError as error:\n"
        "    print(type(error).__name__, str(error))\n"
    )
    assert result.is_success
    assert result.stdout == "42\nValueError rejected by Python host\n"
    assert calls == [(40, 2)]

    missing = sandbox.run("unregistered_host_call()")
    assert missing.status == "error"
    assert missing.exception.type_name == "NameError"
"#,
    );
}

#[test]
fn pure_python_binding_negative_paths_do_not_poison_the_transport() {
    assert_python_binding_succeeds(
        r#"
from minipython_sandbox import Sandbox

sandbox = None
def reenter():
    return sandbox.eval("1").value

with Sandbox(external_functions={"reenter": reenter}) as sandbox:
    reentrant = sandbox.run(
        "try:\n"
        "    reenter()\n"
        "except RuntimeError as error:\n"
        "    print(type(error).__name__, str(error))\n"
    )
    assert reentrant.is_success
    assert reentrant.stdout == "RuntimeError binding calls cannot be reentrant\n"

    cycle = []
    cycle.append(cycle)
    try:
        sandbox.eval("value", {"value": cycle})
    except TypeError as error:
        assert str(error) == "binding values cannot contain cycles"
    else:
        raise AssertionError("cyclic input was accepted")

    try:
        sandbox.run(123)
    except TypeError as error:
        assert str(error) == "source must be a string"
    else:
        raise AssertionError("non-string source was accepted")

    assert sandbox.eval("40 + 2").value == 42
"#,
    );
}

#[test]
fn pure_python_binding_cleanup_preserves_disconnect_error() {
    assert_python_binding_succeeds(
        r#"
from minipython_sandbox import Sandbox, SandboxBindingError

sandbox = Sandbox()
sandbox._process.kill()
sandbox._process.wait()
try:
    sandbox.close()
except SandboxBindingError:
    pass
else:
    raise AssertionError("broker disconnect was not reported")

assert sandbox._closed
assert sandbox._process.poll() is not None
"#,
    );
}

#[test]
fn pure_python_binding_constructor_failure_has_no_destructor_error() {
    let output = run_python_binding(
        r#"
import gc
from minipython_sandbox import Sandbox

try:
    Sandbox(executable="/definitely/not/a/minipython-worker")
except FileNotFoundError:
    pass
else:
    raise AssertionError("missing executable was accepted")

gc.collect()
print("clean")
"#,
    );

    assert!(
        output.status.success(),
        "Python binding client failed\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "clean\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "");
}

#[test]
fn pure_python_binding_session_persists_state_and_closes_after_timeout() {
    assert_python_binding_succeeds(
        r#"
from minipython_sandbox import Limits, Sandbox

marks = []
with Sandbox(external_functions={"host_mark": lambda: marks.append("once")}) as sandbox:
    with sandbox.session() as session:
        setup = session.run(
            "value = 40\n"
            "def add(delta):\n"
            "    return value + delta\n"
            "host_mark()\n"
        )
        assert setup.is_success
        assert session.eval("add(2)").value == 42
        assert marks == ["once"]

with Sandbox(limits=Limits(max_time_ms=200, max_instructions=100_000_000)) as sandbox:
    session = sandbox.session()
    timed_out = session.run("while True:\n    pass\n")
    assert timed_out.status == "time_limit"
    assert timed_out.terminal == "time_limit"
    assert session.closed

    replacement = sandbox.session()
    assert replacement.eval("40 + 2").value == 42
    replacement.close()
"#,
    );
}

fn recursion_sandbox(max_call_depth: usize) -> Sandbox {
    sandbox().with_limits(SandboxLimits {
        max_call_depth: Some(max_call_depth),
        max_instructions: 10_000_000,
        ..SandboxLimits::default()
    })
}

#[test]
fn recursion_limit_follows_the_call_depth_cap_through_a_real_worker() {
    let source = "import sys\ndef down(n):\n    return 0 if n == 0 else 1 + down(n - 1)\ndef forever():\n    return forever()\nprint(sys.getrecursionlimit(), down(990))\ntry:\n    forever()\nexcept RecursionError as error:\n    print('caught', error)\n";
    for cap in [1000, 10_000] {
        let result = recursion_sandbox(cap).run(source);
        assert_eq!(
            result.status,
            ExecutionStatus::Success,
            "{:?}",
            result.exception
        );
        assert_eq!(
            result.stdout, "1000 990\ncaught maximum recursion depth exceeded\n",
            "cap {cap}"
        );
    }

    // No operator cap by default: CPython's limit, and the program may raise it.
    let default = sandbox()
        .with_limits(SandboxLimits {
            max_instructions: 10_000_000,
            ..SandboxLimits::default()
        })
        .run(
            "import sys\nprint(sys.getrecursionlimit())\nsys.setrecursionlimit(100000)\nprint(sys.getrecursionlimit())\ndef down(n):\n    return 0 if n == 0 else 1 + down(n - 1)\nprint(down(1500))\n",
        );
    assert_eq!(
        default.status,
        ExecutionStatus::Success,
        "{:?}",
        default.exception
    );
    assert_eq!(default.stdout, "1000\n100000\n1500\n");
}

#[test]
fn uncaught_recursion_error_is_a_runtime_exception_with_a_cpython_report() {
    let result = recursion_sandbox(1000).run("def forever():\n    return forever()\nforever()\n");
    assert_eq!(result.status, ExecutionStatus::Error);
    let exception = result.exception.as_ref().expect("runtime exception");
    assert_eq!(exception.phase, ExecutionPhase::Runtime);
    assert_eq!(exception.type_name, "RecursionError");
    assert_eq!(exception.message, "maximum recursion depth exceeded");
    assert_eq!(
        result.error_report(),
        "Traceback (most recent call last):\nRecursionError: maximum recursion depth exceeded\n"
    );

    let nested =
        recursion_sandbox(1000).run("import json\njson.loads('[' * 10**6 + ']' * 10**6)\n");
    let exception = nested.exception.as_ref().expect("runtime exception");
    assert_eq!(exception.type_name, "RecursionError");
    assert!(
        exception
            .message
            .ends_with(" kB) while decoding a JSON array from a unicode string"),
        "{}",
        exception.message
    );
}

#[test]
fn session_keeps_its_recursion_limit_and_cap_between_calls() {
    let mut session = recursion_sandbox(5000).session().expect("start session");
    let setup = session.run("import sys\nsys.setrecursionlimit(1500)\n");
    assert_eq!(setup.status, ExecutionStatus::Success);
    assert_eq!(
        session.eval("sys.getrecursionlimit()").value,
        Some(SandboxValue::from(1500_i64))
    );
    let deep = session.run(
        "def down(n):\n    return 0 if n == 0 else 1 + down(n - 1)\nprint(down(1490))\ntry:\n    down(1600)\nexcept RecursionError as error:\n    print(error)\ntry:\n    sys.setrecursionlimit(5001)\nexcept ValueError as error:\n    print(error)\n",
    );
    assert_eq!(
        deep.stdout,
        "1490\nmaximum recursion depth exceeded\nrecursion limit 5001 exceeds the sandbox call-depth limit 5000\n"
    );
    session.close().expect("close session");
}
