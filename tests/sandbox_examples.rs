use std::io::Write;
use std::process::{Command, Output, Stdio};

const HOST_CAPABILITIES_EXAMPLE: &str =
    include_str!("../examples/sandbox/blocked_host_capabilities.py");
const INSTRUCTION_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/instruction_budget.py");
const CALL_DEPTH_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/call_depth_budget.py");
const OUTPUT_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/output_budget.py");
const ALLOCATION_BUDGET_EXAMPLE: &str = include_str!("../examples/sandbox/allocation_budget.py");

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
