use std::env;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus, Stdio};
use std::thread;
#[cfg(target_os = "macos")]
use std::time::Duration;

use minipython::{
    DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES, DEFAULT_SANDBOX_MAX_CALL_DEPTH,
    DEFAULT_SANDBOX_MAX_INSTRUCTIONS, DEFAULT_SANDBOX_MAX_OUTPUT_BYTES, SANDBOX_STDLIB_ALLOWLIST,
    SandboxPolicy, compile_source, eval_source_with_sandbox_dir_and_policy,
    eval_source_with_virtual_modules_and_policy, run_source_with_sandbox_dir_and_policy,
    run_source_with_virtual_modules_and_policy,
};

const DEFAULT_MAX_PROCESS_MEMORY_BYTES: u64 = 256 * 1_048_576;
const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const INTERNAL_WORKER_ENV: &str = "MINIPYTHON_INTERNAL_WORKER";

#[derive(Clone)]
struct Config {
    max_memory_bytes: u64,
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
            "--internal-mode" if env::var(INTERNAL_WORKER_ENV).as_deref() == Ok("1") => {
                config.mode = match args.get(index + 1).map(String::as_str) {
                    Some("exec") => ExecutionMode::Exec,
                    Some("eval") => ExecutionMode::Eval,
                    Some("check") => ExecutionMode::Check,
                    _ => usage(),
                };
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

#[cfg(all(unix, not(target_os = "macos")))]
fn apply_process_memory_limit(command: &mut Command, bytes: u64) {
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(move || {
            let limit = libc::rlimit {
                rlim_cur: bytes as libc::rlim_t,
                rlim_max: bytes as libc::rlim_t,
            };
            if libc::setrlimit(libc::RLIMIT_AS, &limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            if libc::setrlimit(libc::RLIMIT_DATA, &limit) != 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
}

#[cfg(target_os = "macos")]
fn apply_process_memory_limit(_command: &mut Command, _bytes: u64) {}

#[cfg(not(unix))]
fn apply_process_memory_limit(_command: &mut Command, _bytes: u64) {
    eprintln!("sandbox error: process memory limits require a Unix host");
    process::exit(2);
}

#[cfg(target_os = "macos")]
fn process_memory_bytes(child: &process::Child) -> io::Result<u64> {
    let mut usage = unsafe { std::mem::zeroed::<libc::rusage_info_v0>() };
    let result = unsafe {
        libc::proc_pid_rusage(
            child.id() as libc::c_int,
            libc::RUSAGE_INFO_V0,
            &mut usage as *mut _ as *mut libc::rusage_info_t,
        )
    };
    if result == 0 {
        Ok(usage.ri_phys_footprint.max(usage.ri_resident_size))
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "macos")]
fn wait_for_worker(
    child: &mut process::Child,
    max_memory_bytes: u64,
) -> io::Result<(ExitStatus, bool)> {
    loop {
        if let Some(status) = child.try_wait()? {
            return Ok((status, false));
        }
        match process_memory_bytes(child) {
            Ok(bytes) if bytes > max_memory_bytes => {
                child.kill()?;
                return child.wait().map(|status| (status, true));
            }
            Ok(_) => {}
            Err(error) if error.raw_os_error() == Some(libc::ESRCH) => {}
            Err(error) => return Err(error),
        }
        thread::sleep(Duration::from_millis(2));
    }
}

#[cfg(not(target_os = "macos"))]
fn wait_for_worker(
    child: &mut process::Child,
    _max_memory_bytes: u64,
) -> io::Result<(ExitStatus, bool)> {
    child.wait().map(|status| (status, false))
}

fn worker_args(config: &Config) -> Vec<String> {
    let mode = match config.mode {
        ExecutionMode::Exec => "exec",
        ExecutionMode::Eval => "eval",
        ExecutionMode::Check => "check",
    };
    let mut args = vec![
        "--worker".to_string(),
        "--internal-mode".to_string(),
        mode.to_string(),
        "--max-steps".to_string(),
        config.max_steps.to_string(),
        "--max-source-bytes".to_string(),
        config.max_source_bytes.to_string(),
        "--max-depth".to_string(),
        config.max_depth.to_string(),
        "--max-output-bytes".to_string(),
        config.max_output_bytes.to_string(),
        "--max-allocated-bytes".to_string(),
        config.max_allocated_bytes.to_string(),
    ];
    if config.bytes_warning == 1 {
        args.push("-b".to_string());
    } else if config.bytes_warning >= 2 {
        args.push("-bb".to_string());
    }
    if let Some(root) = &config.root {
        args.push("--root".to_string());
        args.push(root.display().to_string());
    }
    args
}

fn run_parent(config: Config, source: String) -> ! {
    let executable = env::current_exe().unwrap_or_else(|error| {
        eprintln!("sandbox error: cannot locate worker executable: {error}");
        process::exit(2);
    });
    let mut command = Command::new(executable);
    command
        .args(worker_args(&config))
        .env(INTERNAL_WORKER_ENV, "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    apply_process_memory_limit(&mut command, config.max_memory_bytes);
    let mut child = command.spawn().unwrap_or_else(|error| {
        eprintln!("sandbox error: failed to start memory-limited worker: {error}");
        process::exit(1);
    });
    child
        .stdin
        .take()
        .expect("worker stdin is piped")
        .write_all(source.as_bytes())
        .unwrap_or_else(|error| {
            let _ = child.kill();
            eprintln!("sandbox error: failed to send source to worker: {error}");
            process::exit(1);
        });
    let mut stdout = child.stdout.take().expect("worker stdout is piped");
    let mut stderr = child.stderr.take().expect("worker stderr is piped");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });
    let (status, memory_limit_exceeded) = wait_for_worker(&mut child, config.max_memory_bytes)
        .unwrap_or_else(|error| {
            eprintln!("sandbox error: failed to wait for worker: {error}");
            process::exit(1);
        });
    let stdout = stdout_reader
        .join()
        .expect("worker stdout reader panicked")
        .unwrap_or_else(|error| {
            eprintln!("sandbox error: failed to read worker stdout: {error}");
            process::exit(1);
        });
    let stderr = stderr_reader
        .join()
        .expect("worker stderr reader panicked")
        .unwrap_or_else(|error| {
            eprintln!("sandbox error: failed to read worker stderr: {error}");
            process::exit(1);
        });
    print!("{}", String::from_utf8_lossy(&stdout));
    eprint!("{}", String::from_utf8_lossy(&stderr));
    if status.success() {
        process::exit(0);
    }
    if memory_limit_exceeded
        || status.code().is_none()
        || status.code().is_some_and(|code| code >= 125)
    {
        eprintln!("sandbox error: worker exceeded process limits or crashed");
    }
    process::exit(1);
}

fn parse_worker_config(args: &[String]) -> Config {
    let mut forwarded = vec!["mnpy".to_string()];
    forwarded.extend_from_slice(&args[2..]);
    let (config, input) = parse_args(&forwarded);
    if !matches!(input, SourceInput::Stdin) {
        usage();
    }
    config
}

fn run_worker(config: Config) -> ! {
    let source =
        read_limited(io::stdin().lock(), config.max_source_bytes).unwrap_or_else(|error| {
            eprintln!("{error}");
            process::exit(1);
        });
    let policy = SandboxPolicy::allow_stdlib_modules(SANDBOX_STDLIB_ALLOWLIST.iter().copied())
        .expect("built-in sandbox stdlib allowlist is valid")
        .with_bytes_warning(config.bytes_warning)
        .with_max_instructions(config.max_steps)
        .with_max_call_depth(config.max_depth)
        .with_max_output_bytes(config.max_output_bytes)
        .with_max_allocated_bytes(config.max_allocated_bytes);
    match config.mode {
        ExecutionMode::Exec => {
            let result = if let Some(root) = config.root {
                run_source_with_sandbox_dir_and_policy(&source, root, policy)
            } else {
                run_source_with_virtual_modules_and_policy(&source, [], policy)
            };
            match result {
                Ok(lines) => {
                    for line in lines {
                        println!("{line}");
                    }
                    process::exit(0);
                }
                Err(error) => {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
        }
        ExecutionMode::Eval => {
            let result = if let Some(root) = config.root {
                eval_source_with_sandbox_dir_and_policy(&source, root, policy)
            } else {
                eval_source_with_virtual_modules_and_policy(&source, [], policy)
            };
            match result {
                Ok(value) => {
                    println!("{value}");
                    process::exit(0);
                }
                Err(error) => {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
        }
        ExecutionMode::Check => match compile_source(&source) {
            Ok(()) => process::exit(0),
            Err(error) => {
                eprintln!("{error}");
                process::exit(1);
            }
        },
    }
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
        run_worker(parse_worker_config(&args));
    }
    let (mut config, input) = parse_args(&args);
    infer_file_root(&mut config, &input);
    let source = read_source(input, config.max_source_bytes).unwrap_or_else(|error| {
        eprintln!("{error}");
        process::exit(1);
    });
    run_parent(config, source);
}
