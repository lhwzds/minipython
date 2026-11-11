// SPDX-License-Identifier: MIT

/// The CPython 3.14.6 oracle executable: `$MINIPYTHON_CPYTHON`, or
/// `/opt/homebrew/bin/python3` when it is unset. `tests/run.sh` checks its
/// version against `.python-version` before any test runs.
fn cpython_oracle() -> String {
    std::env::var("MINIPYTHON_CPYTHON").unwrap_or_else(|_| "/opt/homebrew/bin/python3".to_string())
}

mod boundary {
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::thread;
    use std::time::{Duration, Instant};

    const PROCESS_TIMEOUT: Duration = Duration::from_secs(10);
    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir {
        path: PathBuf,
    }

    impl TestDir {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "minipython-sandbox-boundary-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("failed to create sandbox boundary test directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
            let path = self.path.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .expect("failed to create sandbox boundary fixture parent");
            }
            fs::write(path, bytes).expect("failed to write sandbox boundary fixture");
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run_sandbox(args: &[&str], stdin: Option<&[u8]>) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start mnpy");

        if let Some(bytes) = stdin {
            child
                .stdin
                .as_mut()
                .expect("sandbox stdin is piped")
                .write_all(bytes)
                .expect("failed to send sandbox source");
        }
        drop(child.stdin.take());

        let started = Instant::now();
        loop {
            if child.try_wait().expect("failed to poll mnpy").is_some() {
                return child
                    .wait_with_output()
                    .expect("failed to collect mnpy output");
            }
            if started.elapsed() >= PROCESS_TIMEOUT {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .expect("failed to collect timed-out mnpy output");
                panic!(
                    "mnpy exceeded {:?}; stdout={:?}, stderr={:?}",
                    PROCESS_TIMEOUT,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn run_stdin(args: &[&str], source: &str) -> Output {
        run_sandbox(args, Some(source.as_bytes()))
    }

    const RECURSION_PROBE: &str = "import json, sys\ndef down(n):\n    return 0 if n == 0 else 1 + down(n - 1)\ndef forever():\n    return forever()\nprint(sys.getrecursionlimit(), down(990))\ntry:\n    forever()\nexcept RecursionError as error:\n    print('caught', error)\ntry:\n    json.loads('[' * 10**6 + ']' * 10**6)\nexcept RecursionError as error:\n    print('json', type(error).__name__)\nforever()\n";

    #[test]
    fn sandbox_boundary_recursion_errors_are_catchable_and_never_crash_the_worker() {
        for depth in ["1000", "10000"] {
            let output = run_stdin(
                &["--max-depth", depth, "--max-steps", "10000000", "-"],
                RECURSION_PROBE,
            );
            assert_eq!(output.status.code(), Some(1), "--max-depth {depth}");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "1000 990\ncaught maximum recursion depth exceeded\njson RecursionError\n",
                "--max-depth {depth}"
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stderr),
                "Traceback (most recent call last):\nRecursionError: maximum recursion depth exceeded\n",
                "--max-depth {depth}"
            );
        }
    }

    fn recursive_body(statements: usize, depth: usize) -> String {
        let body: String = (0..statements)
            .map(|index| format!("    a{index} = k + {index}\n"))
            .collect();
        format!(
            "def f(k):\n{body}    if k == 0:\n        return 0\n    return f(k - 1)\n\
             try:\n    print(f({depth}))\nexcept RecursionError as error:\n    \
             print('caught', str(error).startswith('Stack overflow (used '))\n"
        )
    }

    #[test]
    fn sandbox_boundary_recursion_through_large_frames_ends_in_recursion_error() {
        // Calls share the callee's bytecode, so a 30-statement function
        // recursing 900 deep completes under the default limits as in
        // CPython (a per-call copy of the code used to exhaust the worker).
        let output = run_stdin(&[], &recursive_body(30, 900));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "0\n");

        // Larger frames hit the frame heap allowance first: the recursion
        // ends in a catchable RecursionError, never in a worker kill.
        let output = run_stdin(&[], &recursive_body(400, 990));
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "caught True\n");

        // So do small frames under a raised limit, before their heap outgrows
        // the worker's memory.
        let raised = "import sys\nsys.setrecursionlimit(1000000)\ndef f(k):\n    a = k + 1\n    b = k + 2\n    c = k + 3\n    return f(k + 1)\ntry:\n    f(0)\nexcept RecursionError as error:\n    print('caught', str(error).startswith('Stack overflow (used '))\n";
        let output = run_stdin(&["--max-steps", "100000000"], raised);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "caught True\n");
    }

    #[test]
    fn sandbox_boundary_raised_limits_and_small_stacks_stay_catchable() {
        // The limit may be raised to the cap but not past it.
        let raised = "import sys\nsys.setrecursionlimit(10000)\nprint(sys.getrecursionlimit())\ntry:\n    sys.setrecursionlimit(10001)\nexcept ValueError as error:\n    print(error)\nsys.setrecursionlimit(2000)\ndef forever(n):\n    return forever(n + 1)\ntry:\n    forever(0)\nexcept RecursionError as error:\n    print('caught', error)\n";
        let output = run_stdin(
            &["--max-depth", "10000", "--max-steps", "10000000", "-"],
            raised,
        );
        assert!(
            output.status.success(),
            "{:?}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "10000\nrecursion limit 10001 exceeds the sandbox call-depth limit 10000\ncaught maximum recursion depth exceeded\n"
        );

        // A smaller memory limit gives the worker a smaller stack; native
        // recursion still ends at the stack guard, never in a worker crash.
        // This probe peaks near 30 MB resident, so the small case uses 64 MiB
        // (a 16 MiB stack) to leave headroom for the parent's memory polling.
        let nested = "import json\ntry:\n    json.loads('[' * 10**6 + ']' * 10**6)\nexcept RecursionError as error:\n    print(type(error).__name__, str(error).endswith(' while decoding a JSON array from a unicode string'))\n";
        for memory in ["268435456", "67108864"] {
            let output = run_stdin(
                &["--max-depth", "10000", "--max-memory-bytes", memory, "-"],
                nested,
            );
            assert!(
                output.status.success(),
                "{:?}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                "RecursionError True\n",
                "--max-memory-bytes {memory}"
            );
        }
    }

    fn assert_failed_with(output: &Output, expected: &str) {
        assert!(!output.status.success(), "sandbox unexpectedly succeeded");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected),
            "expected stderr to contain {expected:?}, got {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn sandbox_boundary_enforces_each_vm_budget_through_process_entrypoint() {
        let cases = [
            (
                "instruction",
                vec!["--max-steps", "100"],
                "while True:\n    pass\n",
                "sandbox error: instruction limit exceeded",
            ),
            (
                "call-depth",
                vec!["--max-depth", "3", "--max-steps", "10000"],
                "def recurse():\n    recurse()\nrecurse()\n",
                "RecursionError: maximum recursion depth exceeded",
            ),
            (
                "output",
                vec!["--max-output-bytes", "16"],
                "print('x' * 64)\n",
                "sandbox error: output limit exceeded",
            ),
            (
                "allocation",
                vec!["--max-allocated-bytes", "256"],
                "payload = 'x' * 1024\n",
                "sandbox error: allocation limit exceeded",
            ),
        ];

        for (name, args, source, expected) in cases {
            let output = run_stdin(&args, source);
            assert_failed_with(&output, expected);
            assert!(
                output.stdout.is_empty(),
                "{name} budget emitted unexpected stdout: {:?}",
                String::from_utf8_lossy(&output.stdout)
            );
        }
    }

    #[test]
    fn sandbox_boundary_exposes_the_complete_required_stdlib_allowlist() {
        let source = r#"
import builtins, sys, types, collections, collections.abc
import math, math.integer, array, copy, io, operator, functools, itertools, json
import __future__, argparse, re, datetime, csv
print(builtins.__name__, sys.__name__, types.__name__)
print(collections.__name__, collections.abc.__name__)
print(math.__name__, math.integer.__name__)
print(array.__name__, copy.__name__, io.__name__)
print(operator.__name__, functools.__name__, itertools.__name__, json.__name__)
print(__future__.__name__, argparse.__name__, re.__name__, datetime.__name__, csv.__name__)
"#;
        let output = run_stdin(&[], source);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "builtins sys types\ncollections collections.abc\nmath math.integer\narray copy io\noperator functools itertools json\n__future__ argparse re datetime csv\n"
        );
    }

    #[test]
    fn sandbox_boundary_reads_the_wall_clock_in_a_utc_local_zone() {
        // The CLI uses the real clock; the local zone is UTC even when the
        // host sets another TZ.
        let source = "import datetime\nnow = datetime.datetime.now()\nutc = datetime.datetime.now(datetime.UTC)\nprint(abs(utc.replace(tzinfo=None) - now) < datetime.timedelta(seconds=5), now.year >= 2024)\nprint(now.astimezone().tzname(), now.astimezone().utcoffset(), datetime.datetime.fromtimestamp(0), datetime.datetime(2024, 1, 1).timestamp())\n";
        let output = Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .env("TZ", "America/Los_Angeles")
            .arg("-c")
            .arg(source)
            .stdin(Stdio::null())
            .output()
            .expect("run mnpy");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "True True\nUTC 0:00:00 1970-01-01 00:00:00 1704067200.0\n"
        );
    }

    #[test]
    fn sandbox_boundary_blocks_capabilities_and_compatibility_shims() {
        let blocked = [
            "asyncio",
            "http",
            "locale",
            "multiprocessing",
            "pdb",
            "pty",
            "signal",
            "socket",
            "ssl",
            "subprocess",
            "threading",
            "urllib",
            "_ctypes",
            "_socket",
            "_ssl",
            "_testcapi",
            "annotationlib",
            "ast",
            "decimal",
            "dis",
            "enum",
            "fractions",
            "inspect",
            "os",
            "os.path",
            "pickle",
            "string",
            "test",
            "time",
            "typing",
            "unittest",
            "warnings",
            "weakref",
        ];
        let names = blocked
            .iter()
            .map(|name| format!("{name:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let source = format!(
            "for name in [{names}]:\n    try:\n        __import__(name, fromlist=['*'])\n    except ModuleNotFoundError:\n        print(name, 'blocked')\n    else:\n        print(name, 'ALLOWED')\n"
        );
        let output = run_stdin(&[], &source);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            !stdout.contains("ALLOWED"),
            "unexpected capability: {stdout}"
        );
        assert_eq!(stdout.lines().count(), blocked.len());
        for name in blocked {
            assert!(stdout.contains(&format!("{name} blocked")));
        }
    }

    #[test]
    fn sandbox_boundary_rechecks_policy_after_sys_modules_injection() {
        let source = r#"
import sys
for name in ['socket', 'subprocess', 'ast', 'os.path']:
    sys.modules[name] = 'injected'
    try:
        __import__(name, fromlist=['*'])
    except ModuleNotFoundError:
        print(name, 'blocked')
    else:
        print(name, 'ALLOWED')
"#;
        let output = run_stdin(&[], source);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "socket blocked\nsubprocess blocked\nast blocked\nos.path blocked\n"
        );
    }

    #[test]
    fn sandbox_boundary_applies_policy_inside_root_modules() {
        let root = TestDir::new("root-policy");
        root.write("plugin.py", "import subprocess\nVALUE = 1\n");
        let root_arg = root.path().to_string_lossy();
        let output = run_stdin(&["--root", root_arg.as_ref()], "import plugin\n");
        assert_failed_with(&output, "ModuleNotFoundError: No module named 'subprocess'");
    }

    #[test]
    fn sandbox_boundary_loads_safe_modules_from_the_canonical_root() {
        let root = TestDir::new("safe-root");
        root.write(
            "plugin.py",
            "import json\nVALUE = json.loads('{\"value\": 7}')['value']\n",
        );
        root.write("main.py", "import plugin\nprint(plugin.VALUE)\n");
        let main_arg = root.path().join("main.py");
        let main_arg = main_arg.to_string_lossy();
        let output = run_sandbox(&[main_arg.as_ref()], None);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "7\n");
    }

    #[test]
    #[cfg(unix)]
    fn sandbox_boundary_rejects_root_symlink_escape() {
        let root = TestDir::new("symlink-root");
        let outside = TestDir::new("symlink-outside");
        outside.write("escape.py", "print('escaped')\n");
        std::os::unix::fs::symlink(
            outside.path().join("escape.py"),
            root.path().join("escape.py"),
        )
        .expect("failed to create sandbox boundary symlink");

        let root_arg = root.path().to_string_lossy();
        let output = run_stdin(&["--root", root_arg.as_ref()], "print('safe')\n");
        assert_failed_with(&output, "sandbox error: module path escapes sandbox root");
        assert!(!String::from_utf8_lossy(&output.stdout).contains("escaped"));
    }

    #[test]
    fn sandbox_boundary_passes_program_arguments_and_exit_codes() {
        let dir = TestDir::new("program-arguments");
        dir.write(
            "self-test.py",
            "import sys\nimport helper\nprint(sys.argv[1:], helper.VALUE)\nif '--fail' in sys.argv:\n    sys.exit(3)\nif '--message' in sys.argv:\n    sys.exit('bad input')\n",
        );
        dir.write("helper.py", "VALUE = 42\n");
        dir.write("other-tool.py", "raise RuntimeError('never imported')\n");
        let script = dir.path().join("self-test.py");
        let script = script.to_str().expect("UTF-8 fixture path");

        let ok = run_sandbox(&[script, "a b", "--opt=1"], None);
        assert!(
            ok.status.success(),
            "stderr={:?}",
            String::from_utf8_lossy(&ok.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&ok.stdout),
            "['a b', '--opt=1'] 42\n"
        );

        let failed = run_sandbox(&[script, "--fail"], None);
        assert_eq!(failed.status.code(), Some(3));
        assert_eq!(String::from_utf8_lossy(&failed.stdout), "['--fail'] 42\n");
        assert_eq!(String::from_utf8_lossy(&failed.stderr), "");

        let message = run_sandbox(&[script, "--message"], None);
        assert_eq!(message.status.code(), Some(1));
        assert_eq!(String::from_utf8_lossy(&message.stderr), "bad input\n");

        let command = run_sandbox(&["-c", "import sys; print(sys.argv)", "x", "-y"], None);
        assert!(command.status.success());
        assert_eq!(
            String::from_utf8_lossy(&command.stdout),
            "['-c', 'x', '-y']\n"
        );

        let named_stdin = run_sandbox(&["-", "p"], Some(b"import sys; print(sys.argv)\n"));
        assert_eq!(String::from_utf8_lossy(&named_stdin.stdout), "['-', 'p']\n");
        let piped = run_sandbox(&[], Some(b"import sys; print(sys.argv)\n"));
        assert_eq!(String::from_utf8_lossy(&piped.stdout), "['']\n");
    }

    #[test]
    fn sandbox_boundary_runs_regex_programs_and_meters_backtracking() {
        let output = run_stdin(
            &[],
            "import re\n\
             print(re.findall(r'[\\w.]+@\\w+\\.com', 'ann@x.com, bo.b@y.com'))\n\
             print(re.sub(r'\\d+', lambda m: str(int(m.group()) * 2), 'x1 y22'))\n\
             try:\n    re.compile('(?P<a>')\nexcept re.error as error:\n    print(error)\n",
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "['ann@x.com', 'bo.b@y.com']\nx2 y44\nmissing ), unterminated subpattern at position 0\n"
        );

        // Catastrophic backtracking is charged to the instruction budget, so
        // the default limits end it promptly; `except` cannot swallow it.
        let started = Instant::now();
        let redos = run_stdin(
            &[],
            "import re\ntry:\n    re.match(r'(a+)+$', 'a' * 64 + 'b')\nexcept BaseException:\n    print('caught')\n",
        );
        assert_failed_with(&redos, "sandbox error: instruction limit exceeded");
        assert!(redos.stdout.is_empty());
        assert!(started.elapsed() < Duration::from_secs(30));
    }

    #[test]
    fn sandbox_boundary_strptime_over_long_input_never_crashes_the_worker() {
        // strptime matches on an explicit stack: long whitespace and literal
        // runs complete as in CPython under the default limits, and a match
        // that outgrows the instruction budget ends in the sandbox error.
        let output = run_stdin(
            &[],
            "import datetime\n\
             print(datetime.datetime.strptime(' ' * 500000, ' '))\n\
             print(datetime.datetime.strptime('a' * 500000, 'a' * 500000))\n",
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "1900-01-01 00:00:00\n1900-01-01 00:00:00\n"
        );
        let metered = run_stdin(
            &["--max-steps", "20000"],
            "import datetime\ntry:\n    datetime.datetime.strptime(' ' * 500000, ' ')\nexcept BaseException:\n    print('caught')\n",
        );
        assert_failed_with(&metered, "sandbox error: instruction limit exceeded");
        assert!(metered.stdout.is_empty());
    }

    #[test]
    fn sandbox_boundary_csv_sniffer_over_many_lines_never_crashes_the_worker() {
        // The delimiter scan counts characters line by line instead of
        // storing a frequency table per line, so a sample of many short
        // lines stays within the default memory limits. (The quote scan's
        // regular expressions charge about 1.2 million instructions here,
        // so the instruction budget is raised.)
        let output = run_stdin(
            &["--max-steps", "20000000"],
            "import csv\nd = csv.Sniffer().sniff('x\\n' * 300000)\nprint(repr(d.delimiter), d.skipinitialspace)\n",
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "'x' False\n");
    }

    #[test]
    fn sandbox_boundary_many_attributes_stay_linear_under_the_default_limits() {
        // Recording attribute insertion order is amortized O(1) per new
        // name, for instances and classes alike.
        let output = run_stdin(
            &[],
            "class C:\n    pass\nc = C()\nfor i in range(10000):\n    setattr(c, 'a%d' % i, i)\n\
             class K:\n    pass\nfor i in range(5000):\n    setattr(K, 'k%d' % i, i)\n\
             print(len(vars(c)), c.a9999, K.k4999)\n",
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "10000 9999 4999\n");
    }

    #[test]
    fn sandbox_boundary_runs_argparse_programs_like_cpython() {
        let dir = TestDir::new("argparse-program");
        dir.write(
            "greet-tool.py",
            "import argparse, json, sys\n\
             parser = argparse.ArgumentParser(description='Greet people.')\n\
             parser.add_argument('--times', type=int, default=1, help='repeat count')\n\
             sub = parser.add_subparsers(dest='cmd', required=True)\n\
             hello = sub.add_parser('hello', help='say hello')\n\
             hello.add_argument('names', nargs='+')\n\
             args = parser.parse_args()\n\
             print(json.dumps({'cmd': args.cmd, 'names': args.names * args.times}))\n",
        );
        let script = dir.path().join("greet-tool.py");
        let script = script.to_str().expect("UTF-8 fixture path");

        let help = run_sandbox(&[script, "--help"], None);
        assert_eq!(help.status.code(), Some(0));
        assert_eq!(String::from_utf8_lossy(&help.stderr), "");
        assert_eq!(
            String::from_utf8_lossy(&help.stdout),
            "usage: greet-tool.py [-h] [--times TIMES] {hello} ...\n\n\
             Greet people.\n\n\
             positional arguments:\n\
            \x20 {hello}\n\
            \x20   hello        say hello\n\n\
             options:\n\
            \x20 -h, --help     show this help message and exit\n\
            \x20 --times TIMES  repeat count\n"
        );

        let bad = run_sandbox(&[script, "--times", "x", "hello", "a"], None);
        assert_eq!(bad.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&bad.stdout), "");
        assert_eq!(
            String::from_utf8_lossy(&bad.stderr),
            "usage: greet-tool.py [-h] [--times TIMES] {hello} ...\n\
             greet-tool.py: error: argument --times: invalid int value: 'x'\n"
        );

        let missing = run_sandbox(&[script], None);
        assert_eq!(missing.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&missing.stderr)
                .ends_with("greet-tool.py: error: the following arguments are required: cmd\n")
        );

        let sub = run_sandbox(&[script, "--times", "2", "hello", "ann", "bo"], None);
        assert_eq!(sub.status.code(), Some(0));
        assert_eq!(
            String::from_utf8_lossy(&sub.stdout),
            "{\"cmd\": \"hello\", \"names\": [\"ann\", \"bo\", \"ann\", \"bo\"]}\n"
        );
    }

    #[test]
    fn sandbox_boundary_routes_stderr_and_exit_messages() {
        let output = run_sandbox(
            &[
                "-c",
                "import sys\nprint('out')\nprint('warn', file=sys.stderr)\nsys.stderr.write('detail\\n')\nsys.exit(2)\n",
            ],
            None,
        );
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "out\n");
        assert_eq!(String::from_utf8_lossy(&output.stderr), "warn\ndetail\n");
    }

    #[test]
    fn sandbox_boundary_exit_messages_share_the_output_budget() {
        // The SystemExit message is written to stderr like other output, so
        // it is charged to the output budget.
        let output = run_sandbox(&["-c", "raise SystemExit('x' * 2000000)"], None);
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "sandbox error: output limit exceeded\n"
        );
        let output = run_sandbox(
            &[
                "--max-output-bytes",
                "12",
                "-c",
                "print('12345678')\nraise SystemExit('bye')",
            ],
            None,
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(String::from_utf8_lossy(&output.stdout), "12345678\n");
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "sandbox error: output limit exceeded\n"
        );

        // A budget error raised while formatting the exit value ends the run
        // with that error instead of falling back to the value's repr.
        let output = run_sandbox(
            &[
                "-c",
                "class E:\n    def __str__(self):\n        while True:\n            pass\nraise SystemExit(E())",
            ],
            None,
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "sandbox error: instruction limit exceeded\n"
        );

        // A Python error from str() is cleared, as in CPython, which then
        // writes only the newline and exits with status 1.
        let output = run_sandbox(
            &[
                "-c",
                "class E:\n    def __str__(self):\n        raise ValueError('boom')\nraise SystemExit(E())",
            ],
            None,
        );
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(String::from_utf8_lossy(&output.stderr), "\n");
    }

    #[test]
    fn sandbox_boundary_applies_source_limits_to_command_and_file_inputs() {
        let command_output = run_sandbox(
            &["--max-source-bytes", "8", "-c", "print('too large')"],
            None,
        );
        assert_failed_with(
            &command_output,
            "sandbox error: source exceeds 8 byte limit",
        );

        let root = TestDir::new("source-file");
        root.write("large.py", "print('too large')\n");
        let file_arg = root.path().join("large.py");
        let file_arg = file_arg.to_string_lossy();
        let file_output = run_sandbox(&["--max-source-bytes", "8", file_arg.as_ref()], None);
        assert_failed_with(&file_output, "sandbox error: source exceeds 8 byte limit");
    }

    /// CPython's stderr without the `File "...", line N, in name` frame lines
    /// and the source/caret lines under them (also inside exception-group
    /// margins). mnpy prints everything else of the report verbatim.
    fn without_traceback_frames(stderr: &str) -> String {
        let mut kept = String::new();
        let mut in_frame = false;
        for line in stderr.split_inclusive('\n') {
            let content = match line.find("| ") {
                Some(end) if line[..end].trim().is_empty() => &line[end + 2..],
                _ => line,
            };
            if content.starts_with("  File \"") {
                in_frame = true;
                continue;
            }
            if in_frame && content.starts_with("    ") {
                continue;
            }
            in_frame = false;
            kept.push_str(line);
        }
        kept
    }

    fn run_cpython(args: &[&str], stdin: Option<&str>, cwd: &Path) -> Output {
        let mut child = Command::new(crate::cpython_oracle())
            .args(["-E", "-s", "-B"])
            .args(args)
            .current_dir(cwd)
            .env_remove("FORCE_COLOR")
            .env_remove("PYTHON_COLORS")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start CPython");
        if let Some(source) = stdin {
            child
                .stdin
                .as_mut()
                .expect("CPython stdin is piped")
                .write_all(source.as_bytes())
                .expect("failed to send CPython source");
        }
        drop(child.stdin.take());
        child
            .wait_with_output()
            .expect("failed to collect CPython output")
    }

    // An uncaught exception prints CPython's report shape to stderr in every
    // input mode: the traceback header, chained sections in CPython order,
    // and the exact `format_exception_only` line (module-qualified type,
    // `str(exc)` when non-empty, notes). Frame lines are the only omission.
    #[test]
    fn sandbox_boundary_reports_uncaught_exceptions_like_cpython() {
        let cases: &[&str] = &[
            "import sys\nprint('out')\nprint('err', file=sys.stderr)\nraise ValueError('boom')\n",
            "raise ValueError\n",
            "raise ValueError('')\n",
            "d = {}\nd['k']\n",
            "raise KeyError('a', 'b')\n",
            "class MyErr(Exception):\n    pass\nraise MyErr()\n",
            "class MyErr(Exception):\n    pass\nraise MyErr('bad thing', 2)\n",
            "class MyErr(KeyError):\n    pass\nraise MyErr('k')\n",
            "class E(ValueError):\n    def __str__(self):\n        return 'custom ' + self.args[0]\nraise E('x')\n",
            "class E(Exception):\n    def __str__(self):\n        raise RuntimeError('nope')\nraise E()\n",
            "class A:\n    class E(Exception):\n        pass\nraise A.E('q')\n",
            "import json\njson.loads('{')\n",
            "import argparse\nraise argparse.ArgumentTypeError('bad')\n",
            "e = ValueError('v')\ne.add_note('one')\ne.add_note('two\\nlines')\nraise e\n",
            "e = ValueError('v')\ne.__notes__ = ['a', 3]\nraise e\n",
            "try:\n    {}['a']\nexcept KeyError as e:\n    raise ValueError('converted') from e\n",
            "try:\n    1 / 0\nexcept ZeroDivisionError:\n    raise ValueError('during')\n",
            "try:\n    1 / 0\nexcept ZeroDivisionError:\n    raise ValueError('hidden') from None\n",
            "raise ValueError('x') from KeyError('k')\n",
            "try:\n    try:\n        raise KeyError('a')\n    except KeyError as e:\n        raise TypeError('b') from e\nexcept TypeError:\n    raise ValueError('c')\n",
            "e = ExceptionGroup('outer', [ValueError(1), ExceptionGroup('inner', [TypeError('t')])])\ne.add_note('group note')\nraise e\n",
            "def f():\n    raise TypeError('in f')\nf()\n",
            "undefined_name\n",
            "next(iter([]))\n",
            "assert 1 == 2, 'msg'\n",
            "import sys\nsys.exit('bye')\n",
        ];
        let root = TestDir::new("uncaught-exceptions");
        root.write("mymod.py", "class ModErr(Exception):\n    pass\n");
        root.write(
            "from_module.py",
            "import mymod\nraise mymod.ModErr('from module')\n",
        );
        let mut runs = vec![(
            "file from_module.py".to_string(),
            run_cpython(&["from_module.py"], None, root.path()),
            run_sandbox(
                &[root.path().join("from_module.py").to_str().unwrap()],
                None,
            ),
        )];
        for (index, source) in cases.iter().enumerate() {
            let name = format!("case{index}.py");
            root.write(&name, source);
            let path = root.path().join(&name);
            let path = path.to_str().expect("UTF-8 temp path");
            runs.push((
                format!("file {source:?}"),
                run_cpython(&[&name], None, root.path()),
                run_sandbox(&[path], None),
            ));
            runs.push((
                format!("-c {source:?}"),
                run_cpython(&["-c", source], None, root.path()),
                run_sandbox(&["-c", source], None),
            ));
            runs.push((
                format!("stdin {source:?}"),
                run_cpython(&["-"], Some(source), root.path()),
                run_stdin(&[], source),
            ));
        }
        for (label, cpython, mnpy) in runs {
            let cpython_stderr = String::from_utf8_lossy(&cpython.stderr);
            assert_eq!(mnpy.status.code(), cpython.status.code(), "{label}");
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stdout),
                String::from_utf8_lossy(&cpython.stdout),
                "{label}"
            );
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stderr),
                without_traceback_frames(&cpython_stderr),
                "{label}\nCPython stderr:\n{cpython_stderr}"
            );
        }
    }

    #[test]
    fn sandbox_boundary_rejects_non_utf8_source_files() {
        let root = TestDir::new("non-utf8-source");
        root.write("invalid.py", [0xff, 0xfe, b'\n']);
        let file_arg = root.path().join("invalid.py");
        let file_arg = file_arg.to_string_lossy();
        let output = run_sandbox(&[file_arg.as_ref()], None);
        assert_failed_with(&output, "sandbox error: source must be valid UTF-8");
    }
}

mod examples {
    use std::fs;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};

    const HOST_CAPABILITIES_EXAMPLE: &str =
        include_str!("../examples/sandbox/blocked_host_capabilities.py");
    const SAFE_STDLIB_EXAMPLE: &str = include_str!("../examples/sandbox/safe_stdlib.py");
    const INSTRUCTION_BUDGET_EXAMPLE: &str =
        include_str!("../examples/sandbox/instruction_budget.py");
    const SOURCE_SIZE_BUDGET_EXAMPLE: &str =
        include_str!("../examples/sandbox/source_size_budget.py");
    const CALL_DEPTH_BUDGET_EXAMPLE: &str =
        include_str!("../examples/sandbox/call_depth_budget.py");
    const OUTPUT_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/output_budget.py");
    const ALLOCATION_BUDGET_EXAMPLE: &str =
        include_str!("../examples/sandbox/allocation_budget.py");
    const WALL_CLOCK_BUDGET_EXAMPLE: &str =
        include_str!("../examples/sandbox/wall_clock_budget.py");
    const COMPILER_MEMORY_PRESSURE_GENERATOR: &str =
        include_str!("../examples/sandbox/compiler_memory_pressure_generator.py");
    const CACHE_INJECTION_EXAMPLE: &str = include_str!("../examples/sandbox/cache_injection.py");
    const DYNAMIC_IMPORTS_EXAMPLE: &str = include_str!("../examples/sandbox/dynamic_imports.py");
    const SYMLINK_ESCAPE_MAIN: &str = include_str!("../examples/sandbox/symlink_escape_main.py");
    const SYMLINK_ESCAPE_TARGET: &str =
        include_str!("../examples/sandbox/symlink_escape_target.py");
    const ARGPARSE_CLI_EXAMPLE: &str = include_str!("../examples/sandbox/argparse_cli.py");
    const RE_TEXT_TOOL_EXAMPLE: &str = include_str!("../examples/sandbox/re_text_tool.py");
    const CSV_REPORT_TOOL_EXAMPLE: &str = include_str!("../examples/sandbox/csv_report_tool.py");
    const RECURSION_LIMIT_EXAMPLE: &str = include_str!("../examples/sandbox/recursion_limit.py");
    static NEXT_TEMP_DIR: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(label: &str) -> Self {
            let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "minipython-sandbox-example-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("failed to create sandbox example directory");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn run_source(executable: &str, args: &[&str], source: &str) -> Output {
        let mut child = Command::new(executable)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap_or_else(|error| panic!("failed to start {executable}: {error}"));
        child
            .stdin
            .take()
            .expect("child stdin is piped")
            .write_all(source.as_bytes())
            .expect("failed to write example source");
        child.wait_with_output().expect("failed to collect output")
    }

    fn run_file(executable: &str, args: &[&str], path: &Path) -> Output {
        Command::new(executable)
            .args(args)
            .arg(path)
            .output()
            .unwrap_or_else(|error| panic!("failed to run {executable}: {error}"))
    }

    #[test]
    fn real_cpython_and_mnpy_diverge_only_at_host_capability_boundary() {
        let cpython = run_source(
            &crate::cpython_oracle(),
            &["-I", "-B", "-"],
            HOST_CAPABILITIES_EXAMPLE,
        );
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&cpython.stdout),
            "builtin open available\n\
         builtin input available\n\
         module os available\n\
         module socket available\n\
         module subprocess available\n\
         module _ctypes available\n"
        );

        let mnpy = run_source(env!("CARGO_BIN_EXE_mnpy"), &[], HOST_CAPABILITIES_EXAMPLE);
        assert!(
            mnpy.status.success(),
            "{}",
            String::from_utf8_lossy(&mnpy.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&mnpy.stdout),
            "builtin open blocked\n\
         builtin input blocked\n\
         module os blocked\n\
         module socket blocked\n\
         module subprocess blocked\n\
         module _ctypes blocked\n"
        );
    }

    #[test]
    fn real_cpython_and_mnpy_classify_the_complete_safe_stdlib_example() {
        let cpython = run_source(
            &crate::cpython_oracle(),
            &["-I", "-B", "-"],
            SAFE_STDLIB_EXAMPLE,
        );
        let mnpy = run_source(env!("CARGO_BIN_EXE_mnpy"), &[], SAFE_STDLIB_EXAMPLE);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert!(
            mnpy.status.success(),
            "{}",
            String::from_utf8_lossy(&mnpy.stderr)
        );
        assert_eq!(
            cpython.stdout,
            b"2\nTrue\n1\n2 Sequence\n2.0\nmath.integer unavailable\nb'A'\nTrue\nb'a'\n5 6\n4\n1\n16777216 {'value': 'Undefined', 'return': 'list[int]'}\nNamespace(count=3) usage: safe [-h] [--count COUNT]\nexample at ann ['1', '22']\n2024-03-27 Wed 27 Mar 2024-03-01T12:00:00+00:00\n[{'name': 'Widget, large', 'qty': '2'}] \"x,y\",1.5,\n"
        );
        assert_eq!(
            mnpy.stdout,
            b"2\nTrue\n1\n2 Sequence\n2.0\nmath.integer 6\nb'A'\nTrue\nb'a'\n5 6\n4\n1\n16777216 {'value': 'Undefined', 'return': 'list[int]'}\nNamespace(count=3) usage: safe [-h] [--count COUNT]\nexample at ann ['1', '22']\n2024-03-27 Wed 27 Mar 2024-03-01T12:00:00+00:00\n[{'name': 'Widget, large', 'qty': '2'}] \"x,y\",1.5,\n"
        );
    }

    fn assert_cpython_completes_and_mnpy_blocks(
        source: &str,
        cpython_stdout: &str,
        mnpy_args: &[&str],
        expected_error: &str,
    ) {
        let cpython = run_source(&crate::cpython_oracle(), &["-I", "-B", "-"], source);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&cpython.stdout), cpython_stdout);

        let mnpy = run_source(env!("CARGO_BIN_EXE_mnpy"), mnpy_args, source);
        assert!(!mnpy.status.success(), "mnpy unexpectedly completed");
        assert!(
            String::from_utf8_lossy(&mnpy.stderr).contains(expected_error),
            "expected {expected_error:?}, got {:?}",
            String::from_utf8_lossy(&mnpy.stderr)
        );
    }

    #[test]
    fn real_cpython_completes_while_mnpy_enforces_instruction_budget() {
        assert_cpython_completes_and_mnpy_blocks(
            INSTRUCTION_BUDGET_EXAMPLE,
            "499500\n",
            &["--max-steps", "100"],
            "sandbox error: instruction limit exceeded",
        );
    }

    #[test]
    fn real_cpython_completes_while_mnpy_enforces_source_size_budget() {
        assert_cpython_completes_and_mnpy_blocks(
            SOURCE_SIZE_BUDGET_EXAMPLE,
            "source accepted\n",
            &["--max-source-bytes", "8"],
            "sandbox error: source exceeds 8 byte limit",
        );
    }

    #[test]
    fn real_cpython_completes_while_mnpy_enforces_call_depth_budget() {
        assert_cpython_completes_and_mnpy_blocks(
            CALL_DEPTH_BUDGET_EXAMPLE,
            "10\n",
            &["--max-depth", "3", "--max-steps", "10000"],
            "RecursionError: maximum recursion depth exceeded",
        );
    }

    #[test]
    fn real_cpython_completes_while_mnpy_enforces_output_budget() {
        assert_cpython_completes_and_mnpy_blocks(
            OUTPUT_BUDGET_EXAMPLE,
            &format!("{}\n", "x".repeat(64)),
            &["--max-output-bytes", "16"],
            "sandbox error: output limit exceeded",
        );
    }

    #[test]
    fn real_cpython_completes_while_mnpy_enforces_allocation_budget() {
        assert_cpython_completes_and_mnpy_blocks(
            ALLOCATION_BUDGET_EXAMPLE,
            "1024\n",
            &["--max-allocated-bytes", "256"],
            "sandbox error: allocation limit exceeded",
        );
    }

    #[test]
    fn real_cpython_completes_while_mnpy_enforces_wall_clock_budget() {
        assert_cpython_completes_and_mnpy_blocks(
            WALL_CLOCK_BUDGET_EXAMPLE,
            "499999500000\n",
            &["--max-time-ms", "1", "--max-steps", "100000000"],
            "sandbox error: worker wall-clock limit exceeded",
        );
    }

    #[test]
    #[cfg(unix)]
    fn real_cpython_completes_while_mnpy_contains_compiler_memory_pressure() {
        let generated = run_source(
            &crate::cpython_oracle(),
            &["-I", "-B", "-"],
            COMPILER_MEMORY_PRESSURE_GENERATOR,
        );
        assert!(
            generated.status.success(),
            "{}",
            String::from_utf8_lossy(&generated.stderr)
        );
        let attack_source = String::from_utf8(generated.stdout).expect("generator emitted UTF-8");
        assert!(attack_source.len() > 240_000 && attack_source.len() < 524_288);

        let cpython = run_source(&crate::cpython_oracle(), &["-I", "-B", "-"], &attack_source);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert_eq!(cpython.stdout, b"120000\n");

        let mnpy = run_source(
            env!("CARGO_BIN_EXE_mnpy"),
            &[
                "--max-memory-bytes",
                "67108864",
                "--max-source-bytes",
                "524288",
            ],
            &attack_source,
        );
        assert!(!mnpy.status.success());
        assert!(
            String::from_utf8_lossy(&mnpy.stderr)
                .contains("sandbox error: worker memory limit exceeded")
        );
    }

    #[test]
    fn real_cpython_and_mnpy_match_for_safe_script_directory_imports() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sandbox/import_root/main.py");
        let cpython = run_file(&crate::cpython_oracle(), &["-B"], &path);
        let mnpy = run_file(env!("CARGO_BIN_EXE_mnpy"), &[], &path);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert!(
            mnpy.status.success(),
            "{}",
            String::from_utf8_lossy(&mnpy.stderr)
        );
        assert_eq!(cpython.stdout, b"7\n");
        assert_eq!(mnpy.stdout, cpython.stdout);
    }

    #[test]
    fn real_mnpy_propagates_policy_into_script_directory_imports() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/sandbox/blocked_import_root/main.py");
        let cpython = run_file(&crate::cpython_oracle(), &["-B"], &path);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert_eq!(cpython.stdout, b"subprocess\n");

        let mnpy = run_file(env!("CARGO_BIN_EXE_mnpy"), &[], &path);
        assert!(!mnpy.status.success());
        assert!(
            String::from_utf8_lossy(&mnpy.stderr)
                .contains("ModuleNotFoundError: No module named 'subprocess'")
        );
    }

    #[test]
    fn real_cpython_accepts_cache_injection_while_mnpy_rechecks_policy() {
        let cpython = run_source(
            &crate::cpython_oracle(),
            &["-I", "-B", "-"],
            CACHE_INJECTION_EXAMPLE,
        );
        let mnpy = run_source(env!("CARGO_BIN_EXE_mnpy"), &[], CACHE_INJECTION_EXAMPLE);
        assert!(cpython.status.success());
        assert!(mnpy.status.success());
        assert_eq!(cpython.stdout, b"socket available\n");
        assert_eq!(mnpy.stdout, b"socket blocked\n");
    }

    #[test]
    fn real_cpython_allows_dynamic_imports_while_mnpy_reuses_the_sandbox_policy() {
        let cpython = run_source(
            &crate::cpython_oracle(),
            &["-I", "-B", "-"],
            DYNAMIC_IMPORTS_EXAMPLE,
        );
        let mnpy = run_source(env!("CARGO_BIN_EXE_mnpy"), &[], DYNAMIC_IMPORTS_EXAMPLE);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert!(
            mnpy.status.success(),
            "{}",
            String::from_utf8_lossy(&mnpy.stderr)
        );
        assert_eq!(
            cpython.stdout,
            b"eval-import available\nexec-import available\ncompiled-import available\n"
        );
        assert_eq!(
            mnpy.stdout,
            b"eval-import blocked\nexec-import blocked\ncompiled-import blocked\n"
        );
    }

    /// Runs a script file the way a user would, with no terminal-size or
    /// color environment (the sandbox never has a terminal).
    fn run_cli(executable: &str, interpreter_args: &[&str], path: &Path, args: &[&str]) -> Output {
        Command::new(executable)
            .args(interpreter_args)
            .arg(path)
            .args(args)
            .env_remove("COLUMNS")
            .env_remove("LINES")
            .env_remove("FORCE_COLOR")
            .env_remove("NO_COLOR")
            .env_remove("PYTHON_COLORS")
            .output()
            .unwrap_or_else(|error| panic!("failed to run {executable}: {error}"))
    }

    #[test]
    fn real_cpython_and_mnpy_match_for_recursion_under_the_defaults() {
        let cpython = run_source(
            &crate::cpython_oracle(),
            &["-I", "-B", "-"],
            RECURSION_LIMIT_EXAMPLE,
        );
        let mnpy = run_source(env!("CARGO_BIN_EXE_mnpy"), &["-"], RECURSION_LIMIT_EXAMPLE);
        assert_eq!(
            String::from_utf8_lossy(&cpython.stdout),
            "1000\n990\ncaught: maximum recursion depth exceeded\n1500\n"
        );
        assert_eq!(
            String::from_utf8_lossy(&mnpy.stdout),
            String::from_utf8_lossy(&cpython.stdout)
        );
        assert_eq!(cpython.status.code(), Some(1));
        assert_eq!(mnpy.status.code(), Some(1));
        let last_line = |output: &Output| {
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .last()
                .unwrap_or_default()
                .to_string()
        };
        assert_eq!(
            last_line(&cpython),
            "RecursionError: maximum recursion depth exceeded"
        );
        assert_eq!(last_line(&mnpy), last_line(&cpython));
        assert!(
            String::from_utf8_lossy(&mnpy.stderr)
                .starts_with("Traceback (most recent call last):\n")
        );
    }

    #[test]
    fn real_cpython_and_mnpy_match_for_the_argparse_cli_example() {
        let dir = TestDir::new("argparse-cli");
        let script = dir.path().join("argparse_cli.py");
        fs::write(&script, ARGPARSE_CLI_EXAMPLE).expect("failed to write argparse example");
        let invocations: &[&[&str]] = &[
            &["--help"],
            &[],
            &["stats", "[3, 1, 4, 1, 5]", "--round", "2"],
            &["-vv", "--indent", "2", "stats", "[3, 1, 4, 1, 5]"],
            &["stats", "[]", "--round", "many"],
            &["stats", "[]"],
            &["stats", "nope"],
            &["--indent", "-1", "stats", "[1]"],
            &["keys", "{\"b\": 1, \"a\": 2}", "--sort"],
            &["k", "{\"b\": 1, \"a\": 2}", "--reverse", "--sort"],
            &["keys", "-h"],
            &["pick", "{\"a\": 1}", "a", "b", "--missing", "null"],
            &["pick", "{\"a\": 1}", "a", "b", "--missing", "error"],
            &["pick", "{\"a\": 1}", "--missing", "maybe"],
            &["delete", "x"],
            &["stats", "[1]", "--unknown"],
        ];
        let mut statuses = Vec::new();
        for args in invocations {
            let cpython = run_cli(&crate::cpython_oracle(), &["-I", "-B"], &script, args);
            let mnpy = run_cli(env!("CARGO_BIN_EXE_mnpy"), &[], &script, args);
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stdout),
                String::from_utf8_lossy(&cpython.stdout),
                "stdout differs for {args:?}"
            );
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stderr),
                String::from_utf8_lossy(&cpython.stderr),
                "stderr differs for {args:?}"
            );
            assert_eq!(
                mnpy.status.code(),
                cpython.status.code(),
                "status differs for {args:?}"
            );
            statuses.push(cpython.status.code().unwrap_or(-1));
        }
        // Help exits 0, usage errors exit 2 with usage on stderr, and the
        // tool's own failure exits 1.
        assert_eq!(statuses, [0, 2, 0, 0, 2, 2, 2, 2, 0, 2, 0, 0, 1, 2, 2, 2]);
    }

    #[test]
    fn real_cpython_and_mnpy_match_for_the_re_text_tool_example() {
        let dir = TestDir::new("re-text-tool");
        let script = dir.path().join("re_text_tool.py");
        fs::write(&script, RE_TEXT_TOOL_EXAMPLE).expect("failed to write re example");
        let invocations: &[&[&str]] = &[
            &[
                "extract",
                "Mail ann@example.com or bo.b+x@mail.co.uk, see https://example.com/a?b=1. Due 2026-10-29 and 2027-01-02",
            ],
            &["extract", "no matches here"],
            &[
                "redact",
                "card 4111 1111 1111 1111, pin 1234,  phone 555-123-4567",
            ],
            &[
                "grep",
                "err(or)?",
                "ok",
                "Error: disk",
                "warning: errand",
                "-i",
            ],
            &["grep", "err(or)?", "ok", "Error: disk", "warning: errand"],
            &[
                "grep",
                "^(?P<level>[A-Z]+):\\s+(?P<text>.*)$",
                "INFO:  started",
                "x",
                "WARN: low",
            ],
            &["grep", "(unclosed", "x"],
            &["grep", "a{2,1}", "x"],
            &[
                "config",
                "name = demo",
                "retries=3",
                "# comment",
                "",
                "bad line",
                "path: /tmp/x ",
            ],
            &["extract"],
        ];
        let mut statuses = Vec::new();
        for args in invocations {
            let cpython = run_cli(&crate::cpython_oracle(), &["-I", "-B"], &script, args);
            let mnpy = run_cli(env!("CARGO_BIN_EXE_mnpy"), &[], &script, args);
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stdout),
                String::from_utf8_lossy(&cpython.stdout),
                "stdout differs for {args:?}"
            );
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stderr),
                String::from_utf8_lossy(&cpython.stderr),
                "stderr differs for {args:?}"
            );
            assert_eq!(
                mnpy.status.code(),
                cpython.status.code(),
                "status differs for {args:?}"
            );
            statuses.push(cpython.status.code().unwrap_or(-1));
        }
        // Bad patterns and usage errors exit 2.
        assert_eq!(statuses, [0, 0, 0, 0, 0, 0, 2, 2, 0, 2]);
    }

    #[test]
    fn real_cpython_and_mnpy_match_for_the_csv_report_tool_example() {
        let dir = TestDir::new("csv-report-tool");
        let script = dir.path().join("csv_report_tool.py");
        fs::write(&script, CSV_REPORT_TOOL_EXAMPLE).expect("failed to write csv example");
        let data = "region,product,amount\r\nnorth,widget,10.5\r\nsouth,\"gadget, large\",3\r\nnorth,\"say \"\"hi\"\"\",4.25\r\neast,widget,oops\r\n,thing,1\r\n";
        let invocations: &[&[&str]] = &[
            &["summary", data, "--by", "region", "--sum", "amount"],
            &["summary", data, "--by", "product", "--sum", "amount"],
            &["convert", data],
            &["convert", data, "--to", "tsv"],
            &["convert", "a,b\n1,\"multi\nline\"\n", "--to", "json"],
            &[
                "sniff",
                "name;age;city\nann;30;\"Paris, FR\"\nbob;25;Rome\n",
            ],
            &["sniff", "x|y\n1|2\n3|4\n"],
            &["check", "a,b\n\"1\",2\n3,4,5\n"],
            &["check", "a,b\n\"1\"x,2\n"],
            &["check", "a,\"open\n"],
            &["summary", data, "--by", "region"],
            &["convert", data, "--to", "xml"],
            &["--help"],
        ];
        let mut statuses = Vec::new();
        for args in invocations {
            let cpython = run_cli(&crate::cpython_oracle(), &["-I", "-B"], &script, args);
            let mnpy = run_cli(env!("CARGO_BIN_EXE_mnpy"), &[], &script, args);
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stdout),
                String::from_utf8_lossy(&cpython.stdout),
                "stdout differs for {args:?}"
            );
            assert_eq!(
                String::from_utf8_lossy(&mnpy.stderr),
                String::from_utf8_lossy(&cpython.stderr),
                "stderr differs for {args:?}"
            );
            assert_eq!(
                mnpy.status.code(),
                cpython.status.code(),
                "status differs for {args:?}"
            );
            statuses.push(cpython.status.code().unwrap_or(-1));
        }
        // Malformed input exits 1 and usage errors exit 2.
        assert_eq!(statuses, [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 2, 2, 0]);
    }

    #[test]
    #[cfg(unix)]
    fn real_cpython_follows_module_symlink_while_mnpy_rejects_root_escape() {
        let root = TestDir::new("symlink-root");
        let outside = TestDir::new("symlink-outside");
        let main = root.path().join("main.py");
        fs::write(&main, SYMLINK_ESCAPE_MAIN).expect("failed to write symlink example main");
        let target = outside.path().join("escape.py");
        fs::write(&target, SYMLINK_ESCAPE_TARGET).expect("failed to write symlink example target");
        std::os::unix::fs::symlink(&target, root.path().join("escape.py"))
            .expect("failed to create escaping module symlink");

        let cpython = run_file(&crate::cpython_oracle(), &["-B"], &main);
        assert!(
            cpython.status.success(),
            "{}",
            String::from_utf8_lossy(&cpython.stderr)
        );
        assert_eq!(cpython.stdout, b"symlink target executed\n");

        let mnpy = run_file(env!("CARGO_BIN_EXE_mnpy"), &[], &main);
        assert!(!mnpy.status.success());
        assert!(
            String::from_utf8_lossy(&mnpy.stderr)
                .contains("sandbox error: module path escapes sandbox root")
        );
    }
}

mod process {
    use std::io::Write;
    use std::process::{Command, Output, Stdio};
    use std::thread;
    use std::time::{Duration, Instant};

    const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

    fn run_sandbox(args: &[&str], source: &str) -> Output {
        let mut child = Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start mnpy");
        // A rejected invocation exits without reading its input, so the
        // write may find the pipe already closed.
        match child
            .stdin
            .take()
            .expect("sandbox stdin is piped")
            .write_all(source.as_bytes())
        {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(error) => panic!("failed to send sandbox source: {error}"),
        }

        let started = Instant::now();
        loop {
            if child.try_wait().expect("failed to poll mnpy").is_some() {
                return child
                    .wait_with_output()
                    .expect("failed to collect mnpy output");
            }
            if started.elapsed() >= PROCESS_TIMEOUT {
                let _ = child.kill();
                let output = child
                    .wait_with_output()
                    .expect("failed to collect timed-out mnpy output");
                panic!(
                    "mnpy exceeded {:?}; stdout={:?}, stderr={:?}",
                    PROCESS_TIMEOUT,
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
            }
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    fn sandbox_process_prints_its_version() {
        for flag in ["--version", "-V"] {
            let output = run_sandbox(&[flag], "");
            assert!(output.status.success(), "{flag}");
            assert_eq!(
                String::from_utf8_lossy(&output.stdout),
                format!("mnpy {}\n", env!("CARGO_PKG_VERSION")),
                "{flag}"
            );
            assert!(output.stderr.is_empty(), "{flag}");
        }
    }

    #[test]
    fn sandbox_process_runs_safe_source() {
        let output = run_sandbox(&[], "import json\nprint(json.dumps({'ok': True}))\n");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "{\"ok\": true}\n");
    }

    #[test]
    fn sandbox_process_single_entrypoint_keeps_eval_and_check_isolated() {
        let eval = run_sandbox(&["-e", "1 + 2 * 3"], "");
        assert!(
            eval.status.success(),
            "{}",
            String::from_utf8_lossy(&eval.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&eval.stdout), "7\n");

        let check = run_sandbox(&["--check", "value = 1"], "");
        assert!(
            check.status.success(),
            "{}",
            String::from_utf8_lossy(&check.stderr)
        );
        assert!(check.stdout.is_empty());

        let invalid = run_sandbox(&["--check", "value ="], "");
        assert!(!invalid.status.success());
        assert!(String::from_utf8_lossy(&invalid.stderr).contains("parse error:"));

        let bytes_warning = run_sandbox(&["-bb", "-c", "print(b'x' == 'x')"], "");
        assert!(!bytes_warning.status.success());
        assert!(
            String::from_utf8_lossy(&bytes_warning.stderr)
                .contains("BytesWarning: Comparison between bytes and string")
        );
    }

    #[test]
    fn sandbox_process_rejects_direct_worker_invocation() {
        let output = run_sandbox(&["--worker"], "print('bypass')\n");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "mnpy: --worker is an internal implementation detail\n"
        );

        let output = run_sandbox(&["--session-worker"], "print('bypass')\n");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "mnpy: --session-worker is an internal implementation detail\n"
        );

        let output = run_sandbox(&["--python-binding"], "{}");
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "mnpy: --python-binding is an internal implementation detail\n"
        );
    }

    #[test]
    fn sandbox_process_blocks_modules_outside_the_allowlist() {
        let output = run_sandbox(&[], "import subprocess\n");
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("ModuleNotFoundError: No module named 'subprocess'")
        );
    }

    #[test]
    fn sandbox_process_rejects_oversized_source_before_execution() {
        let output = run_sandbox(&["--max-source-bytes", "8"], "print('too large')\n");
        assert!(!output.status.success());
        assert_eq!(
            String::from_utf8_lossy(&output.stderr),
            "sandbox error: source exceeds 8 byte limit\n"
        );
    }

    #[test]
    #[cfg(unix)]
    fn sandbox_process_contains_compiler_memory_pressure() {
        let mut source = String::from("values = [");
        for _ in 0..120_000 {
            source.push_str("0,");
        }
        source.push_str("]\nprint(len(values))\n");

        let output = run_sandbox(
            &[
                "--max-memory-bytes",
                "67108864",
                "--max-source-bytes",
                "524288",
            ],
            &source,
        );
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("sandbox error: worker memory limit exceeded"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// Fails on any host-touching call in `src/` outside `src/host/`, the one
/// module allowed to reach host state, except the parent-side and CLI call
/// sites listed in `ALLOWED`, each with the reason it is there. Counts are
/// exact, so a new use (or a removed one) must update this list.
mod host_capability_lint {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::{Path, PathBuf};

    /// Source text that reaches host state: files, environment, processes,
    /// network, clocks, entropy, raw OS calls, the process's own stdio, and
    /// the current directory.
    const HOST_NEEDLES: &[&str] = &[
        "std::fs",
        "fs::",
        "File::",
        "OpenOptions",
        "std::env",
        "env::",
        "current_dir",
        "current_exe",
        "temp_dir",
        "home_dir",
        "path::absolute",
        "canonicalize",
        "std::process",
        "process::",
        "Command::",
        "Stdio::",
        "std::net",
        "TcpStream",
        "TcpListener",
        "UdpSocket",
        "SystemTime",
        "Instant",
        "UNIX_EPOCH",
        "getrandom",
        "rand::",
        "OsRng",
        "thread_rng",
        "libc::",
        "std::os::",
        "unsafe",
        "extern \"C\"",
        "asm!",
        "#[link",
        "io::stdin",
        "io::stdout",
        "io::stderr",
        "println!",
        "print!",
        "eprintln!",
        "eprint!",
        "dbg!",
    ];

    /// The directory whose code may touch host state freely.
    const HOST_MODULE: &str = "src/host/";

    /// `(file, needle, exact count, why it is allowed)`.
    const ALLOWED: &[(&str, &str, usize, &str)] = &[
        // `mnpy` command line: the trusted parent of every worker.
        (
            "src/sandbox.rs",
            "std::env",
            1,
            "CLI: imports the argument reader",
        ),
        (
            "src/sandbox.rs",
            "env::",
            3,
            "CLI: reads its arguments, locates itself as the worker executable, and checks the internal worker marker before a worker mode",
        ),
        (
            "src/sandbox.rs",
            "current_exe",
            1,
            "CLI: the worker is this same reviewed executable",
        ),
        (
            "src/sandbox.rs",
            "std::fs",
            1,
            "CLI: imports File for the script named on the command line",
        ),
        (
            "src/sandbox.rs",
            "fs::",
            2,
            "CLI: imports File for the script named on the command line and writes the `--status-file` result",
        ),
        (
            "src/sandbox.rs",
            "File::",
            1,
            "CLI: reads the script file before any Python runs",
        ),
        (
            "src/sandbox.rs",
            "io::stdin",
            1,
            "CLI: reads piped source for `mnpy -`",
        ),
        (
            "src/sandbox.rs",
            "io::stdout",
            1,
            "CLI: flushes relayed program output before exiting",
        ),
        (
            "src/sandbox.rs",
            "print!",
            1,
            "CLI: relays the worker's captured stdout",
        ),
        (
            "src/sandbox.rs",
            "println!",
            18,
            "CLI: help text, `--version`, and the `-e` result",
        ),
        (
            "src/sandbox.rs",
            "eprint!",
            2,
            "CLI: relays the worker's captured stderr and error report",
        ),
        (
            "src/sandbox.rs",
            "eprintln!",
            6,
            "CLI: usage, option, and start-up errors, and a status file that cannot be written",
        ),
        (
            "src/sandbox.rs",
            "std::process",
            1,
            "CLI: imports process::exit",
        ),
        (
            "src/sandbox.rs",
            "process::",
            10,
            "CLI: sets the process exit status",
        ),
        // Parent-side worker supervision.
        (
            "src/product.rs",
            "std::process",
            1,
            "parent: imports the worker process types",
        ),
        (
            "src/product.rs",
            "process::",
            14,
            "parent: spawns, polls, kills, and reaps worker processes, and reads a dead worker's signal",
        ),
        (
            "src/product.rs",
            "Command::",
            1,
            "parent: builds every worker process, with a cleared environment",
        ),
        (
            "src/product.rs",
            "Stdio::",
            3,
            "parent: pipes the worker's stdin, stdout, and stderr",
        ),
        (
            "src/product.rs",
            "path::absolute",
            1,
            "parent: resolves a relative module root against the embedder's directory, because the worker runs in /",
        ),
        (
            "src/product.rs",
            "Instant",
            7,
            "parent: wall-clock deadline, shutdown grace period, and reported wall time",
        ),
        (
            "src/product.rs",
            "std::os::",
            2,
            "parent: CommandExt::pre_exec for the worker's kernel memory limits, and ExitStatusExt for the signal that ended a worker",
        ),
        (
            "src/product.rs",
            "libc::",
            14,
            "parent: RLIMIT_AS/RLIMIT_DATA in the child before exec, macOS footprint polling, and SIGABRT from a failed allocation under those limits",
        ),
        (
            "src/product.rs",
            "unsafe",
            3,
            "parent: the pre_exec rlimit hook and proc_pid_rusage",
        ),
        // Python binding broker: a trusted host-side process.
        (
            "src/binding.rs",
            "std::env",
            1,
            "broker: locates itself as the worker executable",
        ),
        (
            "src/binding.rs",
            "env::",
            1,
            "broker: locates itself as the worker executable",
        ),
        (
            "src/binding.rs",
            "current_exe",
            1,
            "broker: the worker is this same reviewed executable",
        ),
        (
            "src/binding.rs",
            "io::stdin",
            1,
            "broker: reads JSON-line commands from the Python host",
        ),
        (
            "src/binding.rs",
            "io::stdout",
            1,
            "broker: writes JSON-line responses to the Python host",
        ),
    ];

    fn is_identifier(ch: char) -> bool {
        ch.is_alphanumeric() || ch == '_'
    }

    /// Occurrences of `needle` in `line` that are not part of a longer
    /// identifier (`Instant` but not `Instantiate`).
    fn occurrences(line: &str, needle: &str) -> usize {
        let starts_word = needle.chars().next().is_some_and(is_identifier);
        let ends_word = needle.chars().last().is_some_and(is_identifier);
        line.match_indices(needle)
            .filter(|(index, _)| {
                let before = line[..*index].chars().next_back();
                let after = line[index + needle.len()..].chars().next();
                !(starts_word && before.is_some_and(is_identifier))
                    && !(ends_word && after.is_some_and(is_identifier))
            })
            .count()
    }

    fn rust_files(dir: &Path, files: &mut Vec<PathBuf>) {
        for entry in fs::read_dir(dir).expect("read source directory") {
            let path = entry.expect("source directory entry").path();
            if path.is_dir() {
                rust_files(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    fn host_touching_uses() -> BTreeMap<(String, &'static str), usize> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        rust_files(&root.join("src"), &mut files);
        let mut uses = BTreeMap::new();
        for path in files {
            let relative = path
                .strip_prefix(root)
                .expect("source below the manifest")
                .to_string_lossy()
                .replace('\\', "/");
            if relative.starts_with(HOST_MODULE) {
                continue;
            }
            let source = fs::read_to_string(&path).expect("read source file");
            for line in source.lines() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                for needle in HOST_NEEDLES {
                    let count = occurrences(line, needle);
                    if count > 0 {
                        *uses.entry((relative.clone(), *needle)).or_default() += count;
                    }
                }
            }
        }
        uses
    }

    #[test]
    fn host_state_is_reached_only_through_the_host_module() {
        let uses = host_touching_uses();
        let allowed = ALLOWED
            .iter()
            .map(|(file, needle, count, _)| ((file.to_string(), *needle), *count))
            .collect::<BTreeMap<_, _>>();
        let unexpected = uses
            .iter()
            .filter(|(key, count)| allowed.get(*key) != Some(count))
            .map(|((file, needle), count)| format!("{file}: {count} x `{needle}`"))
            .collect::<Vec<_>>();
        let stale = allowed
            .iter()
            .filter(|(key, _)| !uses.contains_key(*key))
            .map(|((file, needle), count)| format!("{file}: {count} x `{needle}`"))
            .collect::<Vec<_>>();
        assert!(
            unexpected.is_empty() && stale.is_empty(),
            "host-touching calls belong in {HOST_MODULE}; outside it only the reviewed parent and CLI call sites in ALLOWED may remain.\nunexpected or changed: {unexpected:#?}\nallowed but gone: {stale:#?}"
        );
    }

    #[test]
    fn every_allowance_names_a_reason_and_is_listed_once() {
        let mut seen = std::collections::BTreeSet::new();
        for (file, needle, count, reason) in ALLOWED {
            assert!(*count > 0 && !reason.trim().is_empty(), "{file} {needle}");
            assert!(!file.starts_with(HOST_MODULE), "{file} needs no allowance");
            assert!(HOST_NEEDLES.contains(needle), "{needle} is not a needle");
            assert!(
                seen.insert((*file, *needle)),
                "{file} {needle} listed twice"
            );
        }
    }

    #[test]
    fn the_lint_sees_through_imports_and_ignores_longer_identifiers() {
        assert_eq!(occurrences("use std::fs::File;", "std::fs"), 1);
        assert_eq!(occurrences("use std::fs::File;", "fs::"), 1);
        assert_eq!(occurrences("let x = Instant::now();", "Instant"), 1);
        assert_eq!(occurrences("fn instantiate(Instantiate)", "Instant"), 0);
        assert_eq!(occurrences("SessionCommand::Close", "Command::"), 0);
        assert_eq!(occurrences("subprocess::call", "process::"), 0);
        assert_eq!(occurrences("eprintln!(\"x\")", "println!"), 0);
        assert_eq!(occurrences("eprintln!(\"x\")", "eprintln!"), 1);
    }
}

mod worker_exit_classification {
    use minipython::{ExecutionStatus, Sandbox, SandboxLimits};

    fn sandbox() -> Sandbox {
        Sandbox::new(env!("CARGO_BIN_EXE_mnpy")).with_limits(SandboxLimits {
            max_process_memory_bytes: 256 * 1_048_576,
            ..SandboxLimits::default()
        })
    }

    #[test]
    fn program_output_cannot_forge_worker_allocation_failure() {
        let result = sandbox().run(
            "import sys\nprint('memory allocation of 10 bytes failed', file=sys.stderr)\nsys.stdout.write('memory allocation of 10 bytes failed\\n')\n",
        );

        assert_eq!(result.status, ExecutionStatus::Success, "{result:?}");
        assert_eq!(result.terminal, None, "{result:?}");
        assert_eq!(result.exception, None, "{result:?}");
        assert_eq!(result.stdout, "memory allocation of 10 bytes failed\n");
        assert_eq!(result.stderr, "memory allocation of 10 bytes failed\n");
    }

    #[test]
    fn raw_fd_access_requires_revisiting_worker_exit_classification() {
        // If any of these becomes available, revisit the worker exit classification rule.
        for (source, exception) in [
            ("import os", "ModuleNotFoundError"),
            ("import subprocess", "ModuleNotFoundError"),
            ("import _io", "ModuleNotFoundError"),
            ("open('/dev/stderr', 'w')", "NameError"),
            ("import io\nio.FileIO(2)", "AttributeError"),
            ("import sys\nsys.stderr.buffer", "AttributeError"),
        ] {
            let result = sandbox().run(source);
            assert_eq!(
                result.status,
                ExecutionStatus::Error,
                "{source}: {result:?}"
            );
            assert_eq!(result.terminal, None, "{source}: {result:?}");
            assert_eq!(
                result
                    .exception
                    .as_ref()
                    .map(|error| error.type_name.as_str()),
                Some(exception),
                "{source}: {result:?}"
            );
        }
    }
}

/// Worker processes as the operating system sees them: which process ran
/// each execution, what it inherited, and where it runs. Every worker here
/// is started through a logging wrapper script that `exec`s the real `mnpy`,
/// so the wrapper's PID is the worker's PID.
#[cfg(unix)]
mod worker_processes {
    use std::collections::BTreeSet;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use std::sync::atomic::{AtomicU64, Ordering};

    use minipython::{ExecutionStatus, Sandbox, SandboxValue};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    const PROBE_ENV: &str = "MINIPYTHON_WORKER_PROCESS_PROBE";
    const SECRET_ENV: &str = "MINIPYTHON_TEST_SECRET_API_TOKEN";
    const SECRET_VALUE: &str = "sk-live-must-never-reach-a-worker";

    struct LoggedWorker {
        dir: PathBuf,
    }

    impl LoggedWorker {
        /// A wrapper that records its PID and initial environment, and
        /// whether descriptor `watched_fd` was inherited, then becomes `mnpy`.
        fn new(label: &str, watched_fd: Option<i32>) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "minipython-worker-processes-{label}-{}-{}",
                std::process::id(),
                NEXT_DIR.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&dir).expect("create wrapper directory");
            let watched = watched_fd
                .map(|fd| {
                    format!(
                        "[ -e /dev/fd/{fd} ] && echo inherited > '{}/fd.'$$\n",
                        dir.display()
                    )
                })
                .unwrap_or_default();
            let script = format!(
                "#!/bin/sh\necho $$ >> '{dir}/pids'\n/usr/bin/env > '{dir}/env.'$$\n{watched}exec '{mnpy}' \"$@\"\n",
                dir = dir.display(),
                mnpy = env!("CARGO_BIN_EXE_mnpy"),
            );
            let wrapper = dir.join("worker.sh");
            fs::write(&wrapper, script).expect("write wrapper");
            fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755))
                .expect("make wrapper executable");
            Self { dir }
        }

        fn sandbox(&self) -> Sandbox {
            Sandbox::new(self.dir.join("worker.sh"))
        }

        fn pids(&self) -> Vec<u32> {
            fs::read_to_string(self.dir.join("pids"))
                .unwrap_or_default()
                .lines()
                .map(|line| line.trim().parse().expect("logged PID"))
                .collect()
        }

        fn initial_environment(&self, pid: u32) -> BTreeSet<String> {
            fs::read_to_string(self.dir.join(format!("env.{pid}")))
                .expect("logged environment")
                .lines()
                .map(|line| line.split('=').next().unwrap_or_default().to_string())
                .collect()
        }

        fn inherited(&self, pid: u32) -> bool {
            self.dir.join(format!("fd.{pid}")).exists()
        }
    }

    impl Drop for LoggedWorker {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }

    fn is_running(pid: u32) -> bool {
        Command::new("/bin/kill")
            .args(["-0", &pid.to_string()])
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    /// The live process's working directory and open descriptor numbers.
    fn directory_and_descriptors(pid: u32) -> (PathBuf, BTreeSet<i32>) {
        if Path::new("/proc/self/fd").exists() {
            let cwd = fs::read_link(format!("/proc/{pid}/cwd")).expect("worker cwd");
            let fds = fs::read_dir(format!("/proc/{pid}/fd"))
                .expect("worker descriptors")
                .filter_map(Result::ok)
                .filter_map(|entry| entry.file_name().to_str()?.parse().ok())
                .collect();
            return (cwd, fds);
        }
        let lsof = |selector: &str| {
            let output = Command::new("/usr/sbin/lsof")
                .args(["-a", "-p", &pid.to_string(), "-d", selector, "-F", "fn"])
                .output()
                .expect("run lsof");
            String::from_utf8_lossy(&output.stdout).into_owned()
        };
        let cwd = lsof("cwd")
            .lines()
            .find_map(|line| line.strip_prefix('n'))
            .map(PathBuf::from)
            .expect("worker cwd");
        let fds = lsof("0-65535")
            .lines()
            .filter_map(|line| line.strip_prefix('f')?.parse().ok())
            .collect();
        (cwd, fds)
    }

    /// Runs in a re-executed copy of this test binary whose environment
    /// holds a secret and which has leaked a descriptor without
    /// close-on-exec, the embedder mistakes worker start-up must undo.
    fn hygiene_probe() {
        let leaked = fs::File::open(env!("CARGO_BIN_EXE_mnpy")).expect("open a file to leak");
        let fd = std::os::fd::AsRawFd::as_raw_fd(&leaked);
        let status = Command::new("/bin/sh")
            .args(["-c", &format!("[ -e /dev/fd/{fd} ]")])
            .status()
            .expect("check inheritance");
        assert!(!status.success(), "Rust opens files close-on-exec");
        clear_close_on_exec(fd);

        let worker = LoggedWorker::new("hygiene", Some(fd));
        let mut session = worker.sandbox().session().expect("start session");
        assert_eq!(
            session.eval("40 + 2").value,
            Some(SandboxValue::from(42_i64))
        );
        let pids = worker.pids();
        assert_eq!(pids.len(), 1);
        let pid = pids[0];

        // The parent passed nothing but the internal worker marker (the
        // wrapper's shell adds only its own bookkeeping variables).
        let environment = worker.initial_environment(pid);
        assert!(environment.contains("MINIPYTHON_INTERNAL_WORKER"));
        let unexpected = environment
            .iter()
            .filter(|name| {
                !matches!(
                    name.as_str(),
                    "MINIPYTHON_INTERNAL_WORKER" | "PWD" | "OLDPWD" | "SHLVL" | "_"
                )
            })
            .collect::<Vec<_>>();
        assert!(
            unexpected.is_empty(),
            "parent environment leaked: {unexpected:?}"
        );
        assert!(std::env::var_os(SECRET_ENV).is_some() && std::env::var_os("HOME").is_some());

        // The worker inherited the leaked descriptor, then closed it, and
        // runs in `/`.
        assert!(worker.inherited(pid), "the wrapper saw fd {fd}");
        let (cwd, fds) = directory_and_descriptors(pid);
        assert_eq!(cwd, Path::new("/"));
        assert_eq!(fds, BTreeSet::from([0, 1, 2]), "worker descriptors");

        session.close().expect("close session");
        assert!(!is_running(pid), "the session worker is reaped");
        drop(leaked);
        println!("worker hygiene probe passed");
    }

    fn clear_close_on_exec(fd: i32) {
        // SAFETY: changes the flags of a descriptor this test owns.
        unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFD);
            assert_ne!(flags, -1);
            assert_eq!(libc::fcntl(fd, libc::F_SETFD, flags & !libc::FD_CLOEXEC), 0);
        }
    }

    #[test]
    fn workers_start_with_a_cleared_environment_in_root_and_only_stdio() {
        if std::env::var_os(PROBE_ENV).is_some() {
            hygiene_probe();
            return;
        }
        let output = Command::new(std::env::current_exe().expect("test binary"))
            .args([
                "worker_processes::workers_start_with_a_cleared_environment_in_root_and_only_stdio",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(PROBE_ENV, "1")
            .env(SECRET_ENV, SECRET_VALUE)
            .output()
            .expect("run the probe");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && stdout.contains("worker hygiene probe passed"),
            "probe failed\nstdout: {stdout}\nstderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn every_execution_and_every_session_gets_its_own_worker() {
        let worker = LoggedWorker::new("isolation", None);
        let sandbox = worker.sandbox();

        // Independent executions never share a process or state.
        assert!(sandbox.run("leak = 1\n").is_success());
        let lookup = sandbox.run("print(leak)\n");
        assert_eq!(lookup.status, ExecutionStatus::Error);
        assert_eq!(lookup.exception.expect("NameError").type_name, "NameError");
        assert!(sandbox.eval("1").is_success());
        let one_shot = worker.pids();
        assert_eq!(one_shot.len(), 3);
        assert_eq!(one_shot.iter().collect::<BTreeSet<_>>().len(), 3);
        assert!(
            one_shot.iter().all(|pid| !is_running(*pid)),
            "one-shot workers are reaped"
        );

        // Two live sessions from the same (and a cloned) sandbox each own
        // one worker for all of their calls.
        let mut first = sandbox.session().expect("first session");
        let mut second = sandbox.clone().session().expect("second session");
        for round in 0_i64..3 {
            assert!(first.run(&format!("x = {round}\n")).is_success());
            assert!(second.run(&format!("x = {}\n", round + 100)).is_success());
            assert_eq!(first.eval("x").value, Some(SandboxValue::from(round)));
            assert_eq!(
                second.eval("x").value,
                Some(SandboxValue::from(round + 100))
            );
        }
        // A one-shot run while the sessions are alive uses a fresh worker
        // and sees neither session's state.
        let outside = sandbox.run("print(x)\n");
        assert_eq!(outside.exception.expect("NameError").type_name, "NameError");

        let pids = worker.pids();
        assert_eq!(
            pids.len(),
            6,
            "3 one-shot runs, 2 sessions, 1 more run: {pids:?}"
        );
        let (first_pid, second_pid) = (pids[3], pids[4]);
        assert_eq!(pids.iter().collect::<BTreeSet<_>>().len(), 6, "{pids:?}");
        assert!(is_running(first_pid) && is_running(second_pid));

        first.close().expect("close first session");
        drop(second);
        assert!(
            !is_running(first_pid) && !is_running(second_pid),
            "session workers are reaped"
        );
    }
}

/// The capability contract: every host capability a Python sandbox escape
/// usually reaches for is denied by default, with the precise exception
/// CPython raises when the name does not exist, a matching entry in the
/// denial log, and no effect on the host. Each probe runs in its own
/// worker, so one probe's `sys.modules` or builtins tampering cannot help
/// another.
mod capability_contract {
    use std::fs;
    use std::io::ErrorKind;
    use std::net::TcpListener;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    use minipython::{Denial, DenialKind, Sandbox};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    /// A secret outside the module root and one inside it that is not a
    /// module; no probe may ever print either.
    const CANARY: &str = "CANARY-host-secret-5f1e";

    use DenialKind::{Attribute, Builtin, Import};

    /// `(label, body, expected last line, denials)`. The body runs inside
    /// `try`; the last line is `Type: message` for the exception it raised,
    /// or `no error`. `MARKER` is an absolute host path no probe may create
    /// and `PORT` a listening local port no probe may connect to.
    const PROBES: &[(&str, &str, &str, &[(DenialKind, &str)])] = &[
        // Files.
        (
            "open",
            "open('/etc/passwd')",
            "NameError: name 'open' is not defined",
            &[(Builtin, "open")],
        ),
        (
            "open for writing",
            "open(MARKER, 'w')",
            "NameError: name 'open' is not defined",
            &[(Builtin, "open")],
        ),
        (
            "builtins.open",
            "import builtins\nbuiltins.open(MARKER, 'w')",
            "AttributeError: module 'builtins' has no attribute 'open'",
            &[(Attribute, "builtins.open")],
        ),
        (
            "getattr(builtins, 'op' + 'en')",
            "import builtins\ngetattr(builtins, 'op' + 'en')",
            "AttributeError: module 'builtins' has no attribute 'open'",
            &[(Attribute, "builtins.open")],
        ),
        (
            "builtins.__dict__['open']",
            "import builtins\nbuiltins.__dict__['open']",
            "KeyError: 'open'",
            &[],
        ),
        (
            "vars(builtins)['open']",
            "import builtins\nvars(builtins)['open']",
            "KeyError: 'open'",
            &[],
        ),
        (
            "io.open",
            "import io\nio.open(MARKER, 'w')",
            "AttributeError: module 'io' has no attribute 'open'",
            &[(Attribute, "io.open")],
        ),
        (
            "io.FileIO",
            "import io\nio.FileIO(MARKER, 'w')",
            "AttributeError: module 'io' has no attribute 'FileIO'",
            &[(Attribute, "io.FileIO")],
        ),
        (
            "io.open_code",
            "import io\nio.open_code('/etc/passwd')",
            "AttributeError: module 'io' has no attribute 'open_code'",
            &[(Attribute, "io.open_code")],
        ),
        (
            "_io",
            "import _io",
            "ModuleNotFoundError: No module named '_io'",
            &[(Import, "_io")],
        ),
        (
            "os",
            "import os\nos.system('touch ' + MARKER)",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "os.path",
            "import os.path",
            "ModuleNotFoundError: No module named 'os.path'",
            &[(Import, "os.path")],
        ),
        (
            "from os import",
            "from os import system",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "posix",
            "import posix",
            "ModuleNotFoundError: No module named 'posix'",
            &[(Import, "posix")],
        ),
        (
            "pathlib",
            "import pathlib\npathlib.Path(MARKER).touch()",
            "ModuleNotFoundError: No module named 'pathlib'",
            &[(Import, "pathlib")],
        ),
        (
            "shutil",
            "import shutil",
            "ModuleNotFoundError: No module named 'shutil'",
            &[(Import, "shutil")],
        ),
        (
            "tempfile",
            "import tempfile",
            "ModuleNotFoundError: No module named 'tempfile'",
            &[(Import, "tempfile")],
        ),
        (
            "mmap",
            "import mmap",
            "ModuleNotFoundError: No module named 'mmap'",
            &[(Import, "mmap")],
        ),
        (
            "fcntl",
            "import fcntl",
            "ModuleNotFoundError: No module named 'fcntl'",
            &[(Import, "fcntl")],
        ),
        (
            "sys.path",
            "import sys\nsys.path.insert(0, '/etc')\nimport passwd",
            "ModuleNotFoundError: No module named 'passwd'",
            &[(Import, "passwd")],
        ),
        (
            "argparse.FileType",
            "import argparse\nargparse.FileType('r')('/etc/passwd')",
            "NameError: name 'open' is not defined",
            &[(Attribute, "builtins.open")],
        ),
        (
            "argparse fromfile",
            "import argparse\nargparse.ArgumentParser(prog='probe', fromfile_prefix_chars='@').parse_args(['@/etc/passwd'])",
            "NameError: name 'open' is not defined",
            &[(Attribute, "builtins.open")],
        ),
        (
            "compile() filename",
            "exec(compile('1 / 0', '/etc/passwd', 'exec'))",
            "ZeroDivisionError: division by zero",
            &[],
        ),
        // Standard streams and descriptors.
        (
            "input",
            "input()",
            "NameError: name 'input' is not defined",
            &[(Builtin, "input")],
        ),
        (
            "sys.stdin.fileno",
            "import sys\nsys.stdin.fileno()",
            "UnsupportedOperation: fileno",
            &[],
        ),
        (
            "sys.__stdout__.fileno",
            "import sys\nsys.__stdout__.fileno()",
            "UnsupportedOperation: fileno",
            &[],
        ),
        (
            "sys.stdin.read",
            "import sys\nprint(repr(sys.stdin.read()))",
            "no error",
            &[],
        ),
        // Processes and signals.
        (
            "subprocess",
            "import subprocess\nsubprocess.run(['touch', MARKER])",
            "ModuleNotFoundError: No module named 'subprocess'",
            &[(Import, "subprocess")],
        ),
        (
            "_posixsubprocess",
            "import _posixsubprocess",
            "ModuleNotFoundError: No module named '_posixsubprocess'",
            &[(Import, "_posixsubprocess")],
        ),
        (
            "multiprocessing",
            "import multiprocessing",
            "ModuleNotFoundError: No module named 'multiprocessing'",
            &[(Import, "multiprocessing")],
        ),
        (
            "pty",
            "import pty",
            "ModuleNotFoundError: No module named 'pty'",
            &[(Import, "pty")],
        ),
        (
            "signal",
            "import signal",
            "ModuleNotFoundError: No module named 'signal'",
            &[(Import, "signal")],
        ),
        (
            "threading",
            "import threading",
            "ModuleNotFoundError: No module named 'threading'",
            &[(Import, "threading")],
        ),
        (
            "_thread",
            "import _thread",
            "ModuleNotFoundError: No module named '_thread'",
            &[(Import, "_thread")],
        ),
        (
            "exit",
            "exit()",
            "NameError: name 'exit' is not defined",
            &[(Builtin, "exit")],
        ),
        (
            "quit",
            "quit()",
            "NameError: name 'quit' is not defined",
            &[(Builtin, "quit")],
        ),
        (
            "help",
            "help()",
            "NameError: name 'help' is not defined",
            &[(Builtin, "help")],
        ),
        (
            "license",
            "license()",
            "NameError: name 'license' is not defined",
            &[(Builtin, "license")],
        ),
        ("breakpoint", "print(breakpoint())", "no error", &[]),
        // Network.
        (
            "socket",
            "import socket\nsocket.create_connection(('127.0.0.1', PORT))",
            "ModuleNotFoundError: No module named 'socket'",
            &[(Import, "socket")],
        ),
        (
            "_socket",
            "import _socket",
            "ModuleNotFoundError: No module named '_socket'",
            &[(Import, "_socket")],
        ),
        (
            "ssl",
            "import ssl",
            "ModuleNotFoundError: No module named 'ssl'",
            &[(Import, "ssl")],
        ),
        (
            "select",
            "import select",
            "ModuleNotFoundError: No module named 'select'",
            &[(Import, "select")],
        ),
        (
            "asyncio",
            "import asyncio",
            "ModuleNotFoundError: No module named 'asyncio'",
            &[(Import, "asyncio")],
        ),
        (
            "urllib.request",
            "import urllib.request",
            "ModuleNotFoundError: No module named 'urllib.request'",
            &[(Import, "urllib.request")],
        ),
        (
            "http.client",
            "import http.client",
            "ModuleNotFoundError: No module named 'http.client'",
            &[(Import, "http.client")],
        ),
        // Environment, clock, and entropy.
        (
            "time",
            "import time",
            "ModuleNotFoundError: No module named 'time'",
            &[(Import, "time")],
        ),
        (
            "random",
            "import random",
            "ModuleNotFoundError: No module named 'random'",
            &[(Import, "random")],
        ),
        (
            "secrets",
            "import secrets",
            "ModuleNotFoundError: No module named 'secrets'",
            &[(Import, "secrets")],
        ),
        (
            "uuid",
            "import uuid",
            "ModuleNotFoundError: No module named 'uuid'",
            &[(Import, "uuid")],
        ),
        (
            "platform",
            "import platform",
            "ModuleNotFoundError: No module named 'platform'",
            &[(Import, "platform")],
        ),
        (
            "sys.executable",
            "import sys\nsys.executable",
            "AttributeError: module 'sys' has no attribute 'executable'",
            &[],
        ),
        // C ABI and interpreter internals.
        (
            "ctypes",
            "import ctypes",
            "ModuleNotFoundError: No module named 'ctypes'",
            &[(Import, "ctypes")],
        ),
        (
            "_ctypes",
            "import _ctypes",
            "ModuleNotFoundError: No module named '_ctypes'",
            &[(Import, "_ctypes")],
        ),
        (
            "gc",
            "import gc",
            "ModuleNotFoundError: No module named 'gc'",
            &[(Import, "gc")],
        ),
        (
            "inspect",
            "import inspect",
            "ModuleNotFoundError: No module named 'inspect'",
            &[(Import, "inspect")],
        ),
        (
            "marshal",
            "import marshal",
            "ModuleNotFoundError: No module named 'marshal'",
            &[(Import, "marshal")],
        ),
        (
            "pickle",
            "import pickle",
            "ModuleNotFoundError: No module named 'pickle'",
            &[(Import, "pickle")],
        ),
        (
            "copyreg",
            "import copyreg",
            "ModuleNotFoundError: No module named 'copyreg'",
            &[(Import, "copyreg")],
        ),
        (
            "site",
            "import site",
            "ModuleNotFoundError: No module named 'site'",
            &[(Import, "site")],
        ),
        (
            "code",
            "import code",
            "ModuleNotFoundError: No module named 'code'",
            &[(Import, "code")],
        ),
        (
            "pdb",
            "import pdb",
            "ModuleNotFoundError: No module named 'pdb'",
            &[(Import, "pdb")],
        ),
        // The import system.
        (
            "__import__",
            "__import__('o' + 's')",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "__import__ fromlist",
            "__import__('os', globals(), locals(), ['system'], 0)",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "builtins.__import__",
            "import builtins\nbuiltins.__import__('subprocess')",
            "ModuleNotFoundError: No module named 'subprocess'",
            &[(Import, "subprocess")],
        ),
        (
            "exec import",
            "exec('import os')",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "eval __import__",
            "eval(\"__import__('os')\")",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "compiled code object",
            "exec(compile('import socket', '<probe>', 'exec'))",
            "ModuleNotFoundError: No module named 'socket'",
            &[(Import, "socket")],
        ),
        (
            "importlib",
            "import importlib\nimportlib.import_module('os')",
            "ModuleNotFoundError: No module named 'importlib'",
            &[(Import, "importlib")],
        ),
        (
            "importlib.util",
            "import importlib.util",
            "ModuleNotFoundError: No module named 'importlib.util'",
            &[(Import, "importlib.util")],
        ),
        (
            "_imp",
            "import _imp",
            "ModuleNotFoundError: No module named '_imp'",
            &[(Import, "_imp")],
        ),
        (
            "_frozen_importlib",
            "import _frozen_importlib",
            "ModuleNotFoundError: No module named '_frozen_importlib'",
            &[(Import, "_frozen_importlib")],
        ),
        (
            "zipimport",
            "import zipimport",
            "ModuleNotFoundError: No module named 'zipimport'",
            &[(Import, "zipimport")],
        ),
        (
            "sys.meta_path",
            "import sys\nsys.meta_path",
            "AttributeError: module 'sys' has no attribute 'meta_path'",
            &[],
        ),
        (
            "sys.path_hooks",
            "import sys\nsys.path_hooks",
            "AttributeError: module 'sys' has no attribute 'path_hooks'",
            &[],
        ),
        (
            "__loader__",
            "__loader__",
            "NameError: name '__loader__' is not defined",
            &[],
        ),
        (
            "__spec__",
            "__spec__",
            "NameError: name '__spec__' is not defined",
            &[],
        ),
        (
            "sys.modules lookup",
            "import sys\nsys.modules['os']",
            "KeyError: 'os'",
            &[],
        ),
        (
            "sys.modules injection",
            "import sys, types\nsys.modules['os'] = types.ModuleType('os')\nimport os",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "sys.modules child injection",
            "import sys, types\nsys.modules['os'] = types.ModuleType('os')\nimport os.path",
            "ModuleNotFoundError: No module named 'os.path'",
            &[(Import, "os.path")],
        ),
        (
            "a forged module",
            "import types\ntypes.ModuleType('os').system",
            "AttributeError: module 'os' has no attribute 'system'",
            &[],
        ),
        (
            "no host module is loaded",
            "import sys, json, re, csv, datetime, argparse, io, copy\nprint(sorted(name for name in sys.modules if name in ('os', 'posix', 'io', '_io', 'time')))",
            "no error",
            &[],
        ),
        // Builtins tampering.
        (
            "__builtins__",
            "__builtins__",
            "NameError: name '__builtins__' is not defined",
            &[],
        ),
        (
            "globals()['__builtins__']",
            "globals()['__builtins__']",
            "KeyError: '__builtins__'",
            &[],
        ),
        (
            "__globals__['__builtins__']",
            "(lambda: 0).__globals__['__builtins__']",
            "KeyError: '__builtins__'",
            &[],
        ),
        (
            "__builtins__['open']",
            "(lambda: 0).__builtins__['open']",
            "KeyError: 'open'",
            &[],
        ),
        (
            "__builtins__['__import__']",
            "(lambda: 0).__builtins__['__import__']('os')",
            "ModuleNotFoundError: No module named 'os'",
            &[(Import, "os")],
        ),
        (
            "replaced __import__",
            "import builtins, sys\nbuiltins.__import__ = lambda *args, **kwargs: 'forged'\nprint(__import__('os'), 'os' in sys.modules)",
            "no error",
            &[],
        ),
        (
            "exec with forged builtins",
            "exec('print(open)', {'__builtins__': {'open': 'forged', 'print': print}})",
            "no error",
            &[],
        ),
        // Object-graph walks.
        (
            "object.__subclasses__",
            "().__class__.__base__.__subclasses__()",
            "AttributeError: type object 'object' has no attribute '__subclasses__'",
            &[],
        ),
        (
            "type.__subclasses__",
            "type.__subclasses__(object)",
            "AttributeError: type object 'type' has no attribute '__subclasses__'",
            &[],
        ),
        (
            "__mro__ walk",
            "''.__class__.__mro__[-1].__subclasses__()",
            "AttributeError: type object 'object' has no attribute '__subclasses__'",
            &[],
        ),
        (
            "slot wrapper __globals__",
            "''.__class__.__init__.__globals__",
            "AttributeError: <builtin object.__init__> has no attribute '__globals__'",
            &[],
        ),
        (
            "str.format walk",
            "'{0.__init__.__globals__}'.format(object())",
            "AttributeError: method has no attribute '__globals__'",
            &[],
        ),
        (
            "__code__.co_code",
            "(lambda: 0).__code__.co_code",
            "AttributeError: 'code' object has no attribute 'co_code'",
            &[],
        ),
        (
            "__code__.replace",
            "(lambda: 0).__code__.replace(co_consts=())",
            "AttributeError: 'code' object has no attribute 'replace'",
            &[],
        ),
        (
            "types.CodeType",
            "import types\ntypes.CodeType(0, 0, 0, 0, 0, 0, b'', (), (), (), '', '', '', 0, b'', b'', (), ())",
            "TypeError: cannot create 'code' instances",
            &[],
        ),
        (
            "types.FrameType",
            "import types\ntypes.FrameType()",
            "TypeError: cannot create 'frame' instances",
            &[],
        ),
        (
            "__closure__",
            "def outer():\n    hidden = 'cell'\n    return lambda: hidden\nprint(outer().__closure__[0].cell_contents)",
            "no error",
            &[],
        ),
        (
            "sys._getframe().f_builtins",
            "import sys\nsys._getframe().f_builtins",
            "AttributeError: 'frame' object has no attribute 'f_builtins'",
            &[],
        ),
        (
            "f_back to the top",
            "import sys\nframe = sys._getframe()\nwhile frame.f_back is not None:\n    frame = frame.f_back\nprint(sorted(frame.f_globals))",
            "no error",
            &[],
        ),
        (
            "f_globals['__builtins__']",
            "import sys\nsys._getframe().f_globals['__builtins__']",
            "KeyError: '__builtins__'",
            &[],
        ),
        (
            "gi_frame.f_builtins",
            "(x for x in ()).gi_frame.f_builtins",
            "AttributeError: 'frame' object has no attribute 'f_builtins'",
            &[],
        ),
        (
            "tb_frame",
            "try:\n    1 / 0\nexcept ZeroDivisionError as error:\n    error.__traceback__.tb_frame",
            "AttributeError: 'traceback' object has no attribute 'tb_frame'",
            &[],
        ),
        (
            "__reduce_ex__",
            "(1).__reduce_ex__(2)",
            "AttributeError: 'int' object has no attribute '__reduce_ex__'",
            &[],
        ),
        (
            "__reduce__",
            "object().__reduce__()",
            "AttributeError: 'object' object has no attribute '__reduce__'",
            &[],
        ),
        // MiniPython's `copy` does not call a class's `__reduce__` yet (a
        // compatibility gap); were it to, the callable could only reach the
        // sandbox's own blocked import.
        (
            "copy through __reduce__",
            "import copy\nclass Evil:\n    def __reduce__(self):\n        return (eval, (\"__import__('os')\",))\ncopy.copy(Evil())",
            "no error",
            &[],
        ),
    ];

    fn sandbox_with_root() -> (Sandbox, PathBuf, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "minipython-capability-contract-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        let root = dir.join("root");
        fs::create_dir_all(root.join("pkg")).expect("create module root");
        fs::write(dir.join("outside.txt"), CANARY).expect("write outside secret");
        fs::write(root.join("secret.txt"), CANARY).expect("write root secret");
        fs::write(root.join(".env"), format!("TOKEN={CANARY}\n")).expect("write dotfile");
        fs::write(root.join("helper.py"), "VALUE = 'helper'\n").expect("write module");
        fs::write(root.join("pkg/__init__.py"), "").expect("write package");
        (
            Sandbox::new(env!("CARGO_BIN_EXE_mnpy")).with_root(&root),
            dir,
            root,
        )
    }

    #[test]
    fn every_host_capability_is_denied_by_default() {
        let (sandbox, dir, _root) = sandbox_with_root();
        let marker = dir.join("marker-created-by-an-escape");
        let listener = TcpListener::bind("127.0.0.1:0").expect("listen locally");
        listener
            .set_nonblocking(true)
            .expect("nonblocking listener");
        let port = listener.local_addr().expect("listener address").port();

        let failures = std::thread::scope(|scope| {
            let workers = PROBES
                .chunks(PROBES.len().div_ceil(8))
                .map(|chunk| {
                    let sandbox = sandbox.clone();
                    let marker = marker.clone();
                    scope.spawn(move || {
                        let mut failures = Vec::new();
                        for (label, body, expected, denials) in chunk {
                            let indented = body
                                .lines()
                                .map(|line| format!("    {line}\n"))
                                .collect::<String>();
                            let source = format!(
                                "MARKER = {marker:?}\nPORT = {port}\ntry:\n{indented}except BaseException as error:\n    print(type(error).__name__ + ': ' + str(error))\nelse:\n    print('no error')\n",
                                marker = marker.display().to_string(),
                            );
                            let result = sandbox.run(&source);
                            let last = result.stdout.lines().last().unwrap_or_default();
                            let expected_denials = denials
                                .iter()
                                .map(|(kind, target)| Denial {
                                    kind: *kind,
                                    target: target.to_string(),
                                    count: 1,
                                })
                                .collect::<Vec<_>>();
                            if !result.is_success()
                                || last != *expected
                                || result.denials != expected_denials
                                || result.stdout.contains(CANARY)
                            {
                                failures.push(format!(
                                    "{label}: status {:?}, last line {last:?} (want {expected:?}), denials {:?} (want {expected_denials:?}), exception {:?}",
                                    result.status, result.denials, result.exception
                                ));
                            }
                        }
                        failures
                    })
                })
                .collect::<Vec<_>>();
            workers
                .into_iter()
                .flat_map(|worker| worker.join().expect("probe thread"))
                .collect::<Vec<_>>()
        });
        assert!(
            failures.is_empty(),
            "capability contract broken:\n{}",
            failures.join("\n")
        );

        // No probe had a host effect.
        assert!(!marker.exists(), "a probe created {}", marker.display());
        match listener.accept() {
            Err(error) if error.kind() == ErrorKind::WouldBlock => {}
            other => panic!("a probe reached the network: {other:?}"),
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_module_root_exposes_only_python_modules() {
        let (sandbox, dir, root) = sandbox_with_root();
        let result = sandbox.run(
            "import helper, pkg, sys\nprint(helper.VALUE, sorted(name for name in sys.modules if name in ('helper', 'pkg')))\nfor name in ('secret', 'outside', 'env', '.env', '..outside'):\n    try:\n        __import__(name)\n    except ImportError as error:\n        print(type(error).__name__, error)\nprint(getattr(helper, '__file__', 'no __file__'))\n",
        );
        assert!(result.is_success(), "{:?}", result.exception);
        assert!(!result.stdout.contains(CANARY));
        assert!(
            !result.stdout.contains(&*root.to_string_lossy()),
            "no host path leaks: {}",
            result.stdout
        );
        assert_eq!(
            result.stdout,
            "helper ['helper', 'pkg']\nModuleNotFoundError No module named 'secret'\nModuleNotFoundError No module named 'outside'\nModuleNotFoundError No module named 'env'\nModuleNotFoundError No module named '.env'\nModuleNotFoundError No module named '..outside'\n<virtual>/helper.py\n"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}

mod status_file {
    use std::fs;
    use std::io::{BufRead, BufReader, Read};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Output, Stdio};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);
    static NEXT_SCRATCH_DIR: AtomicU64 = AtomicU64::new(0);

    /// A scratch directory under the system temporary directory. The
    /// directory is removed when the test ends, including when it panics.
    struct ScratchDir {
        path: PathBuf,
    }

    impl ScratchDir {
        fn new(label: &str) -> Self {
            let sequence = NEXT_SCRATCH_DIR.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "minipython-status-file-{label}-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("failed to create the status file test directory");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn join(&self, name: &str) -> PathBuf {
            self.path.join(name)
        }
    }

    impl Drop for ScratchDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn run(args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .args(args)
            .output()
            .expect("failed to run mnpy")
    }

    fn run_in(directory: &Path, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .current_dir(directory)
            .args(args)
            .output()
            .expect("failed to run mnpy")
    }

    fn status_path(path: &Path) -> &str {
        path.to_str().expect("the status file path is valid UTF-8")
    }

    fn read_status(path: &Path) -> String {
        fs::read_to_string(path).expect("mnpy did not write its status file")
    }

    #[test]
    fn a_successful_run_records_success() {
        let scratch = ScratchDir::new("success");
        let status = scratch.join("status.json");
        let output = run(&["--status-file", status_path(&status), "-c", "print('ok')"]);

        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Success\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn an_uncaught_exception_records_an_error() {
        let scratch = ScratchDir::new("error");
        let status = scratch.join("status.json");
        let output = run(&[
            "--status-file",
            status_path(&status),
            "-c",
            "raise ValueError('boom')",
        ]);

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn the_instruction_limit_records_its_terminal_condition() {
        let scratch = ScratchDir::new("limit");
        let status = scratch.join("status.json");
        let output = run(&[
            "--status-file",
            status_path(&status),
            "--max-steps",
            "1000",
            "-c",
            "while True: pass",
        ]);

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":\"InstructionLimit\"}\n"
        );
    }

    #[test]
    fn a_missing_source_file_records_an_error() {
        let scratch = ScratchDir::new("missing-source");
        let status = scratch.join("status.json");
        let missing = scratch.join("missing.py");
        let output = run(&["--status-file", status_path(&status), status_path(&missing)]);

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn a_sys_exit_records_the_exit_code_and_an_error() {
        let scratch = ScratchDir::new("sys-exit");
        let status = scratch.join("status.json");
        let output = run(&[
            "--status-file",
            status_path(&status),
            "-c",
            "import sys; sys.exit(3)",
        ]);

        assert_eq!(output.status.code(), Some(3));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn a_raised_system_exit_records_success() {
        let scratch = ScratchDir::new("raise-system-exit");
        let status = scratch.join("status.json");
        let output = run(&[
            "--status-file",
            status_path(&status),
            "-c",
            "raise SystemExit",
        ]);

        assert_eq!(output.status.code(), Some(0));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Success\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn a_source_that_is_not_utf8_records_an_error() {
        let scratch = ScratchDir::new("non-utf8-source");
        let status = scratch.join("status.json");
        let source = scratch.join("latin1.py");
        fs::write(&source, [0xff, 0xfe, b'x']).expect("failed to write the source file");
        let output = run(&["--status-file", status_path(&status), status_path(&source)]);

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn a_source_over_the_size_limit_records_an_error() {
        let scratch = ScratchDir::new("oversized-source");
        let status = scratch.join("status.json");
        let source = scratch.join("big.py");
        fs::write(&source, "x = 1\n".repeat(8)).expect("failed to write the source file");
        let output = run(&[
            "--status-file",
            status_path(&status),
            "--max-source-bytes",
            "16",
            status_path(&source),
        ]);

        assert_eq!(output.status.code(), Some(1));
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":null}\n"
        );
    }

    #[test]
    fn no_status_file_is_written_without_the_option() {
        let scratch = ScratchDir::new("no-option");
        let output = run_in(scratch.path(), &["-c", "print('ok')"]);

        assert_eq!(output.status.code(), Some(0));
        let entries = fs::read_dir(scratch.path())
            .expect("failed to read the status file test directory")
            .collect::<Vec<_>>();
        assert!(
            entries.is_empty(),
            "mnpy wrote files without --status-file: {:?}",
            entries
        );
    }

    #[test]
    fn an_unwritable_status_file_path_fails_with_exit_two() {
        let scratch = ScratchDir::new("unwritable");
        let status = scratch.join("missing-directory").join("status.json");
        let output = run(&["--status-file", status_path(&status), "-c", "print('ok')"]);

        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("mnpy: cannot write status file"),
            "unexpected stderr: {stderr}"
        );
        assert!(
            stderr.contains(status_path(&status)),
            "the message does not name the status file: {stderr}"
        );
    }

    /// Wait for a child that is still running, and kill it when it takes
    /// too long. The child's stderr is read after it exits, so a child
    /// that writes a lot to it cannot block the wait. A child whose
    /// stderr was already taken has no stderr left to read.
    fn wait_for(child: &mut Child) -> (std::process::ExitStatus, String) {
        let mut stderr = child.stderr.take();
        let started = Instant::now();
        let status = loop {
            match child.try_wait().expect("failed to poll mnpy") {
                Some(status) => break status,
                None => {
                    if started.elapsed() >= PROCESS_TIMEOUT {
                        let _ = child.kill();
                        break child.wait().expect("failed to reap the timed-out mnpy");
                    }
                    thread::sleep(Duration::from_millis(5));
                }
            }
        };
        let mut text = String::new();
        if let Some(stderr) = stderr.as_mut() {
            let _ = stderr.read_to_string(&mut text);
        }
        (status, text)
    }

    #[test]
    fn a_closed_stdout_pipe_still_leaves_the_status_file() {
        let scratch = ScratchDir::new("closed-pipe");
        let status = scratch.join("status.json");
        let mut child = Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .args([
                "--status-file",
                status_path(&status),
                "-c",
                "for index in range(10000):\n    print('0123456789abcdef0123456789abcdef')",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start mnpy");

        // Read the first line, then close the pipe. The run cannot print
        // the rest of its output, and a print that fails must not cost
        // the run its status file. The read happens in a thread, so a
        // run that prints nothing cannot hang the test.
        let stdout = child.stdout.take().expect("the child's stdout is piped");
        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut first_line = String::new();
            let read = reader.read_line(&mut first_line);
            drop(reader);
            let _ = sender.send((read, first_line));
        });
        let (read, first_line) = match receiver.recv_timeout(PROCESS_TIMEOUT) {
            Ok(received) => received,
            Err(_) => {
                // The run printed nothing at all: stop it before the
                // test fails, so no mnpy is left behind.
                let _ = child.kill();
                let _ = child.wait();
                panic!("mnpy printed no output line");
            }
        };
        read.expect("failed to read the first output line");

        let (_exit, stderr) = wait_for(&mut child);

        assert!(
            first_line.starts_with("0123456789abcdef"),
            "unexpected first line: {first_line}"
        );
        // The exit code is whatever the failed print left behind, so it
        // is not checked here: the status file is what the gap pipeline
        // reads, and it must say the run itself succeeded.
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Success\",\"terminal\":null}\n",
            "the status file is missing or wrong; stderr: {stderr}"
        );
    }

    #[test]
    fn a_closed_stderr_pipe_still_leaves_the_status_file_for_a_missing_source() {
        let scratch = ScratchDir::new("closed-stderr-missing-source");
        let status = scratch.join("status.json");
        let missing = scratch.join("missing.py");
        let mut child = Command::new(env!("CARGO_BIN_EXE_mnpy"))
            .args(["--status-file", status_path(&status), status_path(&missing)])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to start mnpy");

        // The child's stderr is closed before it can report the missing
        // source, so the report cannot be printed: a print that fails
        // must not cost the run its status file.
        drop(child.stderr.take().expect("the child's stderr is piped"));

        let (_exit, stderr) = wait_for(&mut child);

        // The exit code is whatever the failed print left behind, so it
        // is not checked here: the status file is what the gap pipeline
        // reads, and it must record the error.
        assert!(
            status.exists(),
            "the status file is missing; stderr: {stderr}"
        );
        assert_eq!(
            read_status(&status),
            "{\"status\":\"Error\",\"terminal\":null}\n",
            "the status file is missing or wrong; stderr: {stderr}"
        );
    }
}
