use std::env;
use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;
use std::process;

use minipython::{
    DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES, DEFAULT_SANDBOX_MAX_CALL_DEPTH,
    DEFAULT_SANDBOX_MAX_INSTRUCTIONS, DEFAULT_SANDBOX_MAX_OUTPUT_BYTES, ExecutionStatus,
    INTERNAL_WORKER_ENV, Sandbox, SandboxInputs, SandboxLimits, SandboxMode, serve_worker_once,
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
    max_depth: usize,
    max_output_bytes: usize,
    max_allocated_bytes: usize,
    bytes_warning: i64,
    root: Option<PathBuf>,
    mode: ExecutionMode,
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
            mode: ExecutionMode::Exec,
        }
    }
}

enum SourceInput {
    Stdin,
    Command(String),
    File(PathBuf),
}

fn usage() -> ! {
    eprintln!("usage: mnpy [options] [-c cmd | -e expr | --check src | file]");
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
                config.max_depth = parse_number(args.get(index + 1), "--max-depth");
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
            "-h" | "--help" => print_help(),
            _ => break,
        }
    }

    let input = match args.get(index).map(String::as_str) {
        None => SourceInput::Stdin,
        Some("-c") => {
            config.mode = ExecutionMode::Exec;
            let source = args.get(index + 1).cloned().unwrap_or_else(|| usage());
            if index + 2 != args.len() {
                usage();
            }
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
            if index + 1 != args.len() {
                usage();
            }
            SourceInput::File(PathBuf::from(file))
        }
    };
    (config, input)
}

fn print_help() -> ! {
    println!("mnpy [options] [-c cmd | -e expr | --check src | file]");
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
        "  --max-depth n            VM call-depth limit (default: {DEFAULT_SANDBOX_MAX_CALL_DEPTH})"
    );
    println!(
        "  --max-output-bytes n     captured output limit (default: {DEFAULT_SANDBOX_MAX_OUTPUT_BYTES})"
    );
    println!(
        "  --max-allocated-bytes n  VM materialization limit (default: {DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES})"
    );
    println!("  --root path              import Python modules below a canonical sandbox root");
    println!("  -h, --help               show this help");
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

fn run_parent(config: Config, source: String) -> ! {
    let executable = env::current_exe().unwrap_or_else(|error| {
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
    let mut sandbox = Sandbox::new(executable).with_limits(limits);
    if let Some(root) = config.root {
        sandbox = sandbox.with_root(root);
    }
    let mode = match config.mode {
        ExecutionMode::Exec => SandboxMode::Exec,
        ExecutionMode::Eval => SandboxMode::Eval,
        ExecutionMode::Check => SandboxMode::Check,
    };
    let result = sandbox.execute(mode, &source, SandboxInputs::new());
    print!("{}", result.stdout);
    eprint!("{}", result.stderr);
    if result.status == ExecutionStatus::Success {
        if mode == SandboxMode::Eval {
            println!("{}", result.value_display.unwrap_or_default());
        }
        process::exit(0);
    }
    if let Some(exception) = result.exception {
        eprintln!("{}", exception.display_message());
    }
    process::exit(1);
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

fn reject_direct_worker_invocation() -> ! {
    eprintln!("mnpy: --worker is an internal implementation detail");
    process::exit(2);
}

pub(crate) fn main() {
    let args = env::args().collect::<Vec<_>>();
    if args.get(1).is_some_and(|arg| arg == "--worker") {
        if env::var(INTERNAL_WORKER_ENV).as_deref() != Ok("1") {
            reject_direct_worker_invocation();
        }
        process::exit(serve_worker_once());
    }
    let (mut config, input) = parse_args(&args);
    infer_file_root(&mut config, &input);
    let source = read_source(input, config.max_source_bytes).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(1);
    });
    run_parent(config, source);
}
