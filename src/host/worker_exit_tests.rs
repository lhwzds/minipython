// SPDX-License-Identifier: MIT

use super::*;
use std::os::unix::process::ExitStatusExt;

fn outcome(
    status: i32,
    termination: WorkerTermination,
    result: Option<ExecutionResult>,
) -> WorkerOutcome {
    WorkerOutcome {
        status: ExitStatus::from_raw(status),
        termination,
        result,
        stream_ended: true,
    }
}

#[test]
fn parent_sigkill_is_memory_limit() {
    let result = classify_worker_outcome(
        outcome(libc::SIGKILL, WorkerTermination::MemoryLimit, None),
        "",
        Some(1024),
        true,
    );
    assert_eq!(result.terminal, Some(TerminalKind::MemoryLimit));
}

#[test]
fn abort_wins_over_memory_monitor() {
    let result = classify_worker_outcome(
        outcome(libc::SIGABRT, WorkerTermination::MemoryLimit, None),
        "memory allocation of 10 bytes failed",
        Some(1024),
        true,
    );
    assert_eq!(result.terminal, Some(TerminalKind::MemoryLimit));
}

#[test]
fn allocation_failure_with_limit_is_memory_limit() {
    let result = classify_worker_outcome(
        outcome(libc::SIGABRT, WorkerTermination::Completed, None),
        "memory allocation of 123 bytes failed",
        Some(1024),
        true,
    );
    assert_eq!(result.terminal, Some(TerminalKind::MemoryLimit));
}

#[test]
fn allocation_failure_on_nonzero_exit_with_limit_is_crash() {
    let result = classify_worker_outcome(
        outcome(1 << 8, WorkerTermination::Completed, None),
        "memory allocation of 10 bytes failed",
        Some(1024),
        true,
    );
    assert_eq!(result.status, ExecutionStatus::WorkerCrash);
    assert_eq!(result.terminal, Some(TerminalKind::WorkerFailure));
}

#[test]
fn allocation_failure_without_limit_is_crash() {
    let result = classify_worker_outcome(
        outcome(libc::SIGABRT, WorkerTermination::Completed, None),
        "memory allocation of 123 bytes failed",
        None,
        true,
    );
    assert_eq!(result.status, ExecutionStatus::WorkerCrash);
    assert_eq!(result.terminal, Some(TerminalKind::WorkerFailure));
}

#[test]
fn stack_overflow_with_allocation_failure_takes_precedence() {
    for memory_limit in [None, Some(1024)] {
        let result = classify_worker_outcome(
            outcome(libc::SIGABRT, WorkerTermination::Completed, None),
            "thread 'worker' has overflowed its stack\nmemory allocation of 10 bytes failed",
            memory_limit,
            true,
        );
        assert_eq!(result.terminal, Some(TerminalKind::NativeStackExhausted));
    }
}

#[test]
fn other_abort_with_memory_monitor_is_crash() {
    let result = classify_worker_outcome(
        outcome(libc::SIGABRT, WorkerTermination::MemoryLimit, None),
        "other stderr",
        Some(1024),
        true,
    );
    assert_eq!(
        result.error_report(),
        format!("sandbox error: worker crashed (signal {})\n", libc::SIGABRT)
    );
    assert_eq!(result.terminal, Some(TerminalKind::WorkerFailure));
}

#[test]
fn malformed_allocation_failure_is_crash() {
    for stderr in [
        "memory allocation of x bytes failed",
        "memory allocation of 10 bytes failed extra",
        "memory allocation of  bytes failed",
        "prefix memory allocation of 10 bytes failed",
        " memory allocation of 10 bytes failed",
        "memory allocation of 10 bytes failed ",
        "\tmemory allocation of 10 bytes failed\n",
        "memory allocation of 10 bytes failed\t\n",
    ] {
        let result = classify_worker_outcome(
            outcome(libc::SIGABRT, WorkerTermination::MemoryLimit, None),
            stderr,
            Some(1024),
            true,
        );
        assert_eq!(
            result.error_report(),
            format!("sandbox error: worker crashed (signal {})\n", libc::SIGABRT)
        );
        assert_eq!(result.terminal, Some(TerminalKind::WorkerFailure));
    }
}

#[test]
fn worker_diagnostics_preserve_whitespace_for_classification() {
    for stderr in [
        " memory allocation of 10 bytes failed\n",
        "memory allocation of 10 bytes failed \n",
    ] {
        let diagnostics = worker_diagnostics(stderr.as_bytes());
        assert_eq!(diagnostics, stderr);
        let result = classify_worker_outcome(
            outcome(libc::SIGABRT, WorkerTermination::Completed, None),
            &diagnostics,
            Some(1024),
            true,
        );
        assert_eq!(result.status, ExecutionStatus::WorkerCrash);
    }
}

#[test]
fn stack_overflow_abort_is_native_stack_exhausted() {
    let result = classify_worker_outcome(
        outcome(libc::SIGABRT, WorkerTermination::MemoryLimit, None),
        "thread 'worker' has overflowed its stack",
        Some(1024),
        true,
    );
    assert_eq!(result.terminal, Some(TerminalKind::NativeStackExhausted));
    assert_eq!(
        result.error_report(),
        "sandbox error: native stack exhausted\n"
    );
}

#[test]
fn other_signal_is_crash_even_with_monitor() {
    let result = classify_worker_outcome(
        outcome(libc::SIGSEGV, WorkerTermination::MemoryLimit, None),
        "has overflowed its stack",
        Some(1024),
        true,
    );
    assert_eq!(
        result.error_report(),
        format!("sandbox error: worker crashed (signal {})\n", libc::SIGSEGV)
    );
}

#[test]
fn unowned_sigkill_is_crash() {
    let result = classify_worker_outcome(
        outcome(libc::SIGKILL, WorkerTermination::Completed, None),
        "",
        Some(1024),
        true,
    );
    assert_eq!(
        result.error_report(),
        format!("sandbox error: worker crashed (signal {})\n", libc::SIGKILL)
    );
}

#[test]
fn successful_exit_with_result_wins_over_memory_monitor() {
    let complete = ExecutionResult::success(None, None, "ok".into(), ExecutionUsage::default());
    let result = classify_worker_outcome(
        outcome(0, WorkerTermination::MemoryLimit, Some(complete)),
        "",
        Some(1024),
        true,
    );
    assert_eq!(result.status, ExecutionStatus::Success);
    assert_eq!(result.stdout, "ok");
}

#[test]
fn whitespace_only_diagnostics_use_exit_and_protocol_details() {
    for (stream_ended, expected_type, expected_detail) in [
        (false, "WorkerCrashed", "worker exited with status 7"),
        (
            true,
            "WorkerError",
            "invalid worker response: cannot read frame header: the stream ended",
        ),
    ] {
        let mut exited = outcome(7 << 8, WorkerTermination::Completed, None);
        exited.stream_ended = stream_ended;
        let result = classify_worker_outcome(exited, " \t\n ", Some(1024), true);
        let exception = result.exception.expect("worker exception");
        assert_eq!(exception.type_name, expected_type);
        assert_eq!(exception.message, expected_detail);
    }
}

#[test]
fn displayed_worker_diagnostics_are_trimmed() {
    for one_shot in [false, true] {
        let result = classify_worker_outcome(
            outcome(7 << 8, WorkerTermination::Completed, None),
            " \tworker diagnostic\n\n",
            Some(1024),
            one_shot,
        );
        let exception = result.exception.expect("worker exception");
        assert_eq!(exception.message, "worker diagnostic");
        assert!(!exception.message.ends_with('\n'));
        assert_eq!(exception.type_name, "WorkerError");
    }
}

#[test]
fn signal_crash_type_depends_on_worker_kind() {
    for (one_shot, expected_type) in [(true, "WorkerCrashed"), (false, "WorkerError")] {
        let result = classify_worker_outcome(
            outcome(libc::SIGSEGV, WorkerTermination::Completed, None),
            "",
            Some(1024),
            one_shot,
        );
        assert_eq!(
            result.exception.expect("worker exception").type_name,
            expected_type
        );
    }
}

/// Removes a scratch file when the test ends, including when it panics.
struct ScratchScript {
    path: PathBuf,
}

impl Drop for ScratchScript {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

#[test]
fn one_shot_signal_crash_reports_worker_crashed() {
    use std::os::unix::fs::PermissionsExt;

    // The script lives next to the test executable rather than in the
    // temporary directory: a temporary directory can be mounted noexec, and
    // a script that cannot run would not kill the worker this test needs.
    let executable = std::env::current_exe().expect("locate the test executable");
    let directory = executable
        .parent()
        .expect("the test executable has a directory");
    let path = directory.join(format!("mnpy-crash-worker-{}", process::id()));
    // The exclusive create makes the file this test's own: when a file with
    // this name already exists the open fails and the test stops here. The
    // guard is created only after that open succeeded, so it never removes a
    // file this test did not create.
    let mut script = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create worker script");
    let _scratch = ScratchScript { path: path.clone() };
    script
        .write_all(b"#!/bin/sh\nulimit -c 0\n/bin/dd bs=1 count=4 of=/dev/null 2>/dev/null\nkill -SEGV $$\n")
        .expect("write worker script");
    drop(script);
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700))
        .expect("make worker script executable");
    let result = Sandbox::new(&path).run("pass");
    assert_eq!(result.status, ExecutionStatus::WorkerCrash);
    assert_eq!(
        result.exception.expect("worker exception").type_name,
        "WorkerCrashed"
    );
}
