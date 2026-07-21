use std::path::Path;
use std::process::{Command, Output};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use minipython::{
    ExecutionPhase, ExecutionStatus, ExternalCall, ExternalFunctionError, Sandbox, SandboxInputs,
    SandboxLimits, SandboxValue,
};

fn sandbox() -> Sandbox {
    Sandbox::new(env!("CARGO_BIN_EXE_mnpy"))
}

fn run_python_binding(source: &str) -> Output {
    Command::new("/opt/homebrew/bin/python3")
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
from minipython_sandbox import OpaqueValue, Sandbox

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

with Sandbox(limits=Limits(max_time_ms=1, max_instructions=100_000_000)) as sandbox:
    session = sandbox.session()
    timed_out = session.run("while True:\n    pass\n")
    assert timed_out.status == "time_limit"
    assert session.closed

    replacement = sandbox.session()
    assert replacement.eval("40 + 2").value == 42
    replacement.close()
"#,
    );
}
