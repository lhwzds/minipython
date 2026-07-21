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
                "sandbox error: maximum call depth exceeded",
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
print(builtins.__name__, sys.__name__, types.__name__)
print(collections.__name__, collections.abc.__name__)
print(math.__name__, math.integer.__name__)
print(array.__name__, copy.__name__, io.__name__)
print(operator.__name__, functools.__name__, itertools.__name__, json.__name__)
"#;
        let output = run_stdin(&[], source);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            "builtins sys types\ncollections collections.abc\nmath math.integer\narray copy io\noperator functools itertools json\n"
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
            "re",
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
            "/opt/homebrew/bin/python3",
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
            "/opt/homebrew/bin/python3",
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
            b"2\nTrue\n1\n2 Sequence\n2.0\nmath.integer unavailable\nb'A'\nTrue\nb'a'\n5 6\n4\n1\n"
        );
        assert_eq!(
            mnpy.stdout,
            b"2\nTrue\n1\n2 Sequence\n2.0\nmath.integer 6\nb'A'\nTrue\nb'a'\n5 6\n4\n1\n"
        );
    }

    fn assert_cpython_completes_and_mnpy_blocks(
        source: &str,
        cpython_stdout: &str,
        mnpy_args: &[&str],
        expected_error: &str,
    ) {
        let cpython = run_source("/opt/homebrew/bin/python3", &["-I", "-B", "-"], source);
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
            "sandbox error: maximum call depth exceeded",
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
            "/opt/homebrew/bin/python3",
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

        let cpython = run_source(
            "/opt/homebrew/bin/python3",
            &["-I", "-B", "-"],
            &attack_source,
        );
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
                .contains("sandbox error: worker exceeded process limits or crashed")
        );
    }

    #[test]
    fn real_cpython_and_mnpy_match_for_safe_script_directory_imports() {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sandbox/import_root/main.py");
        let cpython = run_file("/opt/homebrew/bin/python3", &["-B"], &path);
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
        let cpython = run_file("/opt/homebrew/bin/python3", &["-B"], &path);
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
            "/opt/homebrew/bin/python3",
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
            "/opt/homebrew/bin/python3",
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

        let cpython = run_file("/opt/homebrew/bin/python3", &["-B"], &main);
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
        child
            .stdin
            .take()
            .expect("sandbox stdin is piped")
            .write_all(source.as_bytes())
            .expect("failed to send sandbox source");

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
                .contains("sandbox error: worker exceeded process limits or crashed"),
            "unexpected stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
