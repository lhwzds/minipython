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
