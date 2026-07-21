use std::path::Path;

use minipython::{ExecutionPhase, ExecutionStatus, Sandbox, SandboxInputs, SandboxValue};

fn sandbox() -> Sandbox {
    Sandbox::new(env!("CARGO_BIN_EXE_mnpy"))
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
