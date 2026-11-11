// SPDX-License-Identifier: MIT

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process;

use minipython::{
    DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES, DEFAULT_SANDBOX_MAX_CALL_DEPTH,
    DEFAULT_SANDBOX_MAX_INSTRUCTIONS, DEFAULT_SANDBOX_MAX_OUTPUT_BYTES, ExecutionStatus,
    INTERNAL_WORKER_ENV, Sandbox, SandboxInputs, SandboxLimits, SandboxMode, TerminalKind,
    serve_python_binding, serve_session_worker, serve_worker_once,
};

const DEFAULT_MAX_PROCESS_MEMORY_BYTES: u64 = 256 * 1_048_576;
const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_TIME_MS: u64 = 5_000;

#[derive(Clone)]
struct Config {
    max_memory_bytes: u64,
    max_time_ms: u64,
    max_source_bytes: usize,
    max_steps: u64,
    max_depth: Option<usize>,
    max_output_bytes: usize,
    max_allocated_bytes: usize,
    bytes_warning: i64,
    root: Option<PathBuf>,
    status_file: Option<PathBuf>,
    mode: ExecutionMode,
    argv: Vec<String>,
}

#[derive(Clone, Copy)]
enum ExecutionMode {
    Exec,
    Eval,
    Check,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_memory_bytes: DEFAULT_MAX_PROCESS_MEMORY_BYTES,
            max_time_ms: DEFAULT_MAX_TIME_MS,
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_steps: DEFAULT_SANDBOX_MAX_INSTRUCTIONS,
            max_depth: DEFAULT_SANDBOX_MAX_CALL_DEPTH,
            max_output_bytes: DEFAULT_SANDBOX_MAX_OUTPUT_BYTES,
            max_allocated_bytes: DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES,
            bytes_warning: 0,
            root: None,
            status_file: None,
            mode: ExecutionMode::Exec,
            argv: Vec::new(),
        }
    }
}

enum SourceInput {
    Stdin,
    Command(String),
    File(PathBuf),
}

fn usage() -> ! {
    eprintln!(
        "usage: mnpy [options] [-c cmd [arg ...] | -e expr | --check src | file [arg ...] | - [arg ...]]"
    );
    process::exit(2);
}

fn parse_number<T: std::str::FromStr>(value: Option<&String>, option: &str) -> T {
    value
        .and_then(|value| value.parse::<T>().ok())
        .unwrap_or_else(|| {
            eprintln!("mnpy: {option} requires a non-negative integer");
            process::exit(2);
        })
}

fn parse_args(args: &[String]) -> (Config, SourceInput) {
    let mut config = Config::default();
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "-b" => {
                config.bytes_warning = (config.bytes_warning + 1).min(2);
                index += 1;
            }
            "-bb" => {
                config.bytes_warning = 2;
                index += 1;
            }
            "--max-memory-bytes" => {
                config.max_memory_bytes = parse_number(args.get(index + 1), "--max-memory-bytes");
                index += 2;
            }
            "--max-source-bytes" => {
                config.max_source_bytes = parse_number(args.get(index + 1), "--max-source-bytes");
                index += 2;
            }
            "--max-time-ms" => {
                config.max_time_ms = parse_number(args.get(index + 1), "--max-time-ms");
                index += 2;
            }
            "--max-steps" => {
                config.max_steps = parse_number(args.get(index + 1), "--max-steps");
                index += 2;
            }
            "--max-depth" => {
                config.max_depth = Some(parse_number(args.get(index + 1), "--max-depth"));
                index += 2;
            }
            "--max-output-bytes" => {
                config.max_output_bytes = parse_number(args.get(index + 1), "--max-output-bytes");
                index += 2;
            }
            "--max-allocated-bytes" => {
                config.max_allocated_bytes =
                    parse_number(args.get(index + 1), "--max-allocated-bytes");
                index += 2;
            }
            "--root" => {
                config.root = Some(PathBuf::from(
                    args.get(index + 1).unwrap_or_else(|| usage()),
                ));
                index += 2;
            }
            "--status-file" => {
                config.status_file = Some(PathBuf::from(
                    args.get(index + 1).unwrap_or_else(|| usage()),
                ));
                index += 2;
            }
            "-h" | "--help" => print_help(),
            "-V" | "--version" => print_version(),
            _ => break,
        }
    }

    // Like CPython, everything after the program (script path, `-c cmd`, or
    // `-` for stdin) belongs to the program as `sys.argv[1:]`.
    let input = match args.get(index).map(String::as_str) {
        None => {
            config.argv = vec![String::new()];
            SourceInput::Stdin
        }
        Some("-") => {
            config.argv = args[index..].to_vec();
            SourceInput::Stdin
        }
        Some("-c") => {
            config.mode = ExecutionMode::Exec;
            let source = args.get(index + 1).cloned().unwrap_or_else(|| usage());
            config.argv = std::iter::once("-c".to_string())
                .chain(args[index + 2..].iter().cloned())
                .collect();
            SourceInput::Command(source)
        }
        Some("-e") => {
            config.mode = ExecutionMode::Eval;
            let source = args.get(index + 1).cloned().unwrap_or_else(|| usage());
            if index + 2 != args.len() {
                usage();
            }
            SourceInput::Command(source)
        }
        Some("--check") => {
            config.mode = ExecutionMode::Check;
            let source = args.get(index + 1).cloned().unwrap_or_else(|| usage());
            if index + 2 != args.len() {
                usage();
            }
            SourceInput::Command(source)
        }
        Some(value) if value.starts_with('-') => usage(),
        Some(file) => {
            config.argv = args[index..].to_vec();
            SourceInput::File(PathBuf::from(file))
        }
    };
    (config, input)
}

fn print_help() -> ! {
    println!(
        "mnpy [options] [-c cmd [arg ...] | -e expr | --check src | file [arg ...] | - [arg ...]]"
    );
    println!("  -c cmd                   execute a program passed as a string");
    println!("  -e expr                  evaluate an expression and print the result");
    println!("  --check src              compile source and check for errors");
    println!("  -b, -bb                  warn or error on bytes/string comparisons");
    println!(
        "  --max-memory-bytes n     process address/data limit (default: {DEFAULT_MAX_PROCESS_MEMORY_BYTES})"
    );
    println!("  --max-source-bytes n     source input limit (default: {DEFAULT_MAX_SOURCE_BYTES})");
    println!("  --max-time-ms n          worker wall-clock limit (default: {DEFAULT_MAX_TIME_MS})");
    println!(
        "  --max-steps n            VM instruction limit (default: {DEFAULT_SANDBOX_MAX_INSTRUCTIONS})"
    );
    println!(
        "  --max-depth n            cap sys.setrecursionlimit() at n (default: no cap; the limit starts at 1000)"
    );
    println!(
        "  --max-output-bytes n     captured output limit (default: {DEFAULT_SANDBOX_MAX_OUTPUT_BYTES})"
    );
    println!(
        "  --max-allocated-bytes n  VM materialization limit (default: {DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES})"
    );
    println!("  --root path              import Python modules below a canonical sandbox root");
    println!("  --status-file path       write the run's result status as JSON to path");
    println!("  -V, --version            print the mnpy version and exit");
    println!("  -h, --help               show this help");
    process::exit(0);
}

fn print_version() -> ! {
    println!("mnpy {}", env!("CARGO_PKG_VERSION"));
    process::exit(0);
}

fn read_limited(mut reader: impl Read, limit: usize) -> Result<String, String> {
    let take = u64::try_from(limit).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .by_ref()
        .take(take)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("sandbox error: failed to read source: {error}"))?;
    if bytes.len() > limit {
        return Err(format!("sandbox error: source exceeds {limit} byte limit"));
    }
    String::from_utf8(bytes).map_err(|_| "sandbox error: source must be valid UTF-8".to_string())
}

fn read_source(input: SourceInput, limit: usize) -> Result<String, String> {
    match input {
        SourceInput::Stdin => read_limited(io::stdin().lock(), limit),
        SourceInput::Command(source) => {
            if source.len() > limit {
                Err(format!("sandbox error: source exceeds {limit} byte limit"))
            } else {
                Ok(source)
            }
        }
        SourceInput::File(path) => {
            let file = File::open(&path).map_err(|error| {
                format!("sandbox error: failed to open {}: {error}", path.display())
            })?;
            read_limited(file, limit)
        }
    }
}

/// The JSON object mnpy writes to its `--status-file` path: the run's
/// `ExecutionStatus`, and the terminal condition when the sandbox, not the
/// program, ended the run.
fn status_file_contents(status: ExecutionStatus, terminal: Option<TerminalKind>) -> String {
    let payload = serde_json::json!({
        "status": status,
        "terminal": terminal,
    });
    let mut contents = serde_json::to_string(&payload).expect("the status payload serializes");
    contents.push('\n');
    contents
}

/// Write the run's status to the `--status-file` path, replacing any file
/// there.
///
/// The write is separate from the report of its failure: a run writes its
/// status before it prints any output, so the caller can report a failed
/// write once the output is out.
fn try_write_status_file(
    path: Option<&Path>,
    status: ExecutionStatus,
    terminal: Option<TerminalKind>,
) -> io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    fs::write(path, status_file_contents(status, terminal))
}

/// Report a status file that could not be written. A status file that
/// cannot be written is fatal: the run's result cannot be reported any
/// other way, so mnpy exits 2 instead of the exit code the run would
/// have had.
fn report_status_file_failure(path: &Path, error: &io::Error) -> ! {
    eprintln!("mnpy: cannot write status file {}: {error}", path.display());
    process::exit(2);
}

/// Write the run's status to the `--status-file` path, replacing any file
/// there. A status file that cannot be written is fatal: the run's result
/// cannot be reported any other way, so mnpy exits 2 instead of the exit
/// code the run would have had.
fn write_status_file(path: Option<&Path>, status: ExecutionStatus, terminal: Option<TerminalKind>) {
    let Some(path) = path else {
        return;
    };
    if let Err(error) = try_write_status_file(Some(path), status, terminal) {
        report_status_file_failure(path, &error);
    }
}

fn run_parent(config: Config, source: String) -> ! {
    let status_file = config.status_file.clone();
    let executable = env::current_exe().unwrap_or_else(|error| {
        // The status file is written before the message is printed: a
        // print can fail (a closed stderr pipe panics), and the status
        // file is the only record of how the run ended, so it must not
        // be lost to a failed print.
        write_status_file(
            status_file.as_deref(),
            ExecutionStatus::WorkerCrash,
            Some(TerminalKind::WorkerFailure),
        );
        eprintln!("sandbox error: cannot locate worker executable: {error}");
        process::exit(2);
    });
    let limits = SandboxLimits {
        max_process_memory_bytes: config.max_memory_bytes,
        max_time_ms: config.max_time_ms,
        max_source_bytes: config.max_source_bytes,
        max_instructions: config.max_steps,
        max_call_depth: config.max_depth,
        max_output_bytes: config.max_output_bytes,
        max_allocated_bytes: config.max_allocated_bytes,
        bytes_warning: config.bytes_warning,
    };
    let mut sandbox = Sandbox::new(executable)
        .with_limits(limits)
        .with_argv(config.argv);
    if let Some(root) = config.root {
        sandbox = sandbox.with_root(root);
    }
    let mode = match config.mode {
        ExecutionMode::Exec => SandboxMode::Exec,
        ExecutionMode::Eval => SandboxMode::Eval,
        ExecutionMode::Check => SandboxMode::Check,
    };
    let result = sandbox.execute(mode, &source, SandboxInputs::new());
    // The exit code is decided, and the status file written, before any of
    // the run's output: printing can fail (a closed stdout pipe panics),
    // and the status file is the only record of how the run ended, so it
    // must not be lost to a failed print.
    let exited = result.exit_code.is_some();
    let exit_code = match result.exit_code {
        Some(code) => code,
        None if result.status == ExecutionStatus::Success => 0,
        None => 1,
    };
    let status_written =
        try_write_status_file(status_file.as_deref(), result.status, result.terminal);
    print!("{}", result.stdout);
    eprint!("{}", result.stderr);
    if exited {
        // An uncaught SystemExit already wrote any message to stderr.
        let _ = io::stdout().flush();
    } else if result.status == ExecutionStatus::Success {
        if mode == SandboxMode::Eval {
            println!("{}", result.value_display.unwrap_or_default());
        }
    } else {
        // Uncaught Python exceptions get CPython's traceback shape; sandbox,
        // worker, and syntax failures keep their one-line phase prefix.
        eprint!("{}", result.error_report());
    }
    if let Err(error) = status_written {
        if let Some(path) = status_file.as_deref() {
            report_status_file_failure(path, &error);
        }
    }
    process::exit(exit_code);
}

fn infer_file_root(config: &mut Config, input: &SourceInput) {
    if config.root.is_some() || !matches!(config.mode, ExecutionMode::Exec) {
        return;
    }
    let SourceInput::File(path) = input else {
        return;
    };
    config.root = Some(
        path.parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| std::path::Path::new("."))
            .to_path_buf(),
    );
}

fn reject_direct_worker_invocation(argument: &str) -> ! {
    eprintln!("mnpy: {argument} is an internal implementation detail");
    process::exit(2);
}

pub(crate) fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| {
        matches!(
            arg.as_str(),
            "--worker" | "--session-worker" | "--python-binding"
        )
    }) {
        if env::var(INTERNAL_WORKER_ENV).as_deref() != Ok("1") {
            reject_direct_worker_invocation(&args[1]);
        }
        process::exit(match args[1].as_str() {
            "--worker" => serve_worker_once(),
            "--session-worker" => serve_session_worker(),
            "--python-binding" => serve_python_binding(),
            _ => unreachable!(),
        });
    }
    let (mut config, input) = parse_args(&args);
    infer_file_root(&mut config, &input);
    let source = read_source(input, config.max_source_bytes).unwrap_or_else(|error| {
        // The status file is written before the message is printed: a
        // print can fail (a closed stderr pipe panics), and the status
        // file is the only record of how the run ended, so it must not
        // be lost to a failed print.
        write_status_file(config.status_file.as_deref(), ExecutionStatus::Error, None);
        eprintln!("{error}");
        process::exit(1);
    });
    run_parent(config, source);
}
