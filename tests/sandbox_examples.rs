use std::io::Write;
use std::process::{Command, Output, Stdio};

const HOST_CAPABILITIES_EXAMPLE: &str =
    include_str!("../examples/sandbox/blocked_host_capabilities.py");

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
