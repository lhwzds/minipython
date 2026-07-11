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
            fs::create_dir_all(parent).expect("failed to create sandbox boundary fixture parent");
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
