use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

const HOST_CAPABILITIES_EXAMPLE: &str =
    include_str!("../examples/sandbox/blocked_host_capabilities.py");
const SAFE_STDLIB_EXAMPLE: &str = include_str!("../examples/sandbox/safe_stdlib.py");
const INSTRUCTION_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/instruction_budget.py");
const CALL_DEPTH_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/call_depth_budget.py");
const OUTPUT_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/output_budget.py");
const ALLOCATION_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/allocation_budget.py");
const WALL_CLOCK_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/wall_clock_budget.py");
const COMPILER_MEMORY_PRESSURE_GENERATOR: &str =
    include_str!("../examples/sandbox/compiler_memory_pressure_generator.py");
const CACHE_INJECTION_EXAMPLE: &str = include_str!("../examples/sandbox/cache_injection.py");
const DYNAMIC_IMPORTS_EXAMPLE: &str = include_str!("../examples/sandbox/dynamic_imports.py");
const SYMLINK_ESCAPE_MAIN: &str = include_str!("../examples/sandbox/symlink_escape_main.py");
const SYMLINK_ESCAPE_TARGET: &str = include_str!("../examples/sandbox/symlink_escape_target.py");
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
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sandbox/import_root/main.py");
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
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/sandbox/blocked_import_root/main.py");
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
