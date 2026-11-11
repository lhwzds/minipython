// SPDX-License-Identifier: MIT

use super::*;

#[test]
fn parent_termination_reason_selects_limit() {
    assert_eq!(
        non_unix_parent_termination(WorkerTermination::TimeLimit),
        Some(TerminalKind::TimeLimit)
    );
    assert_eq!(
        non_unix_parent_termination(WorkerTermination::MemoryLimit),
        Some(TerminalKind::MemoryLimit)
    );
    assert_eq!(
        non_unix_parent_termination(WorkerTermination::Completed),
        None
    );
}

#[cfg(windows)]
#[test]
fn successful_result_precedes_parent_termination() {
    use std::os::windows::process::ExitStatusExt;

    let result = ExecutionResult::success(None, None, "ok".into(), ExecutionUsage::default());
    let outcome = WorkerOutcome {
        status: ExitStatus::from_raw(0),
        termination: WorkerTermination::TimeLimit,
        result: Some(result),
        stream_ended: true,
    };
    assert_eq!(
        classify_worker_outcome(outcome, "", Some(1024), true).status,
        ExecutionStatus::Success
    );
}

#[cfg(windows)]
#[test]
fn terminated_worker_reports_parent_time_limit() {
    use std::os::windows::process::ExitStatusExt;

    let outcome = WorkerOutcome {
        status: ExitStatus::from_raw(1),
        termination: WorkerTermination::TimeLimit,
        result: None,
        stream_ended: true,
    };
    assert_eq!(
        classify_worker_outcome(outcome, "", Some(1024), true).terminal,
        Some(TerminalKind::TimeLimit)
    );
}
