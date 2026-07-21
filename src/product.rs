use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt;
use std::io::{self, Read, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::process::{self, Command, ExitStatus, Stdio};
use std::rc::Rc;
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

use crate::compiler::{compile_eval, compile_with_options};
use crate::lexer::{lex_for_parse, lex_with_spans_for_parse};
use crate::parser::{parse, parse_eval};
use crate::value::{
    Value, byte_array_value, bytes_value, dict_value, external_function_value, float_value,
    list_value, tuple_value,
};
use crate::vm::{ExternalCallHandler, SourceModule, Vm, VmExecution, VmSessionState};
use crate::{
    DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES, DEFAULT_SANDBOX_MAX_CALL_DEPTH,
    DEFAULT_SANDBOX_MAX_INSTRUCTIONS, DEFAULT_SANDBOX_MAX_OUTPUT_BYTES, SANDBOX_STDLIB_ALLOWLIST,
    SandboxPolicy, compile_options_for_spanned_tokens, reject_too_complex_source,
    runtime_options_for_sandbox_policy, virtual_modules_from_sandbox_dir, vm_stdlib_import_policy,
};

const DEFAULT_MAX_PROCESS_MEMORY_BYTES: u64 = 256 * 1_048_576;
const DEFAULT_MAX_SOURCE_BYTES: usize = 1_048_576;
const DEFAULT_MAX_TIME_MS: u64 = 5_000;
const MAX_PROTOCOL_FRAME_BYTES: usize = 32 * 1_048_576;

pub const INTERNAL_WORKER_ENV: &str = "MINIPYTHON_INTERNAL_WORKER";

pub type SandboxInputs = BTreeMap<String, SandboxValue>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SandboxValue {
    None,
    Bool(bool),
    Integer(BigInt),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    ByteArray(Vec<u8>),
    List(Vec<SandboxValue>),
    Tuple(Vec<SandboxValue>),
    Dict(Vec<(SandboxValue, SandboxValue)>),
    Opaque { type_name: String, display: String },
}

impl From<bool> for SandboxValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for SandboxValue {
    fn from(value: i64) -> Self {
        Self::Integer(value.into())
    }
}

impl From<f64> for SandboxValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<String> for SandboxValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for SandboxValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalCall {
    pub name: String,
    pub args: Vec<SandboxValue>,
    pub keywords: Vec<(String, SandboxValue)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalFunctionError {
    pub type_name: String,
    pub message: String,
}

impl ExternalFunctionError {
    pub fn new(type_name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            type_name: type_name.into(),
            message: message.into(),
        }
    }
}

type ExternalFunction =
    Arc<dyn Fn(ExternalCall) -> Result<SandboxValue, ExternalFunctionError> + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SandboxMode {
    Exec,
    Eval,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Error,
    TimeLimit,
    MemoryLimit,
    WorkerCrash,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionPhase {
    Decode,
    Lex,
    Parse,
    Compile,
    Runtime,
    Sandbox,
    Worker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionException {
    pub phase: ExecutionPhase,
    pub type_name: String,
    pub message: String,
}

impl ExecutionException {
    pub fn display_message(&self) -> String {
        let prefix = match self.phase {
            ExecutionPhase::Decode => "decode error",
            ExecutionPhase::Lex => "lex error",
            ExecutionPhase::Parse => "parse error",
            ExecutionPhase::Compile => "compile error",
            ExecutionPhase::Runtime => "runtime error",
            ExecutionPhase::Sandbox | ExecutionPhase::Worker => "sandbox error",
        };
        if self.phase == ExecutionPhase::Runtime && !self.type_name.is_empty() {
            format!("{prefix}: {}: {}", self.type_name, self.message)
        } else {
            format!("{prefix}: {}", self.message)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ExecutionUsage {
    pub instructions: u64,
    pub output_bytes: usize,
    pub allocated_bytes: usize,
    pub wall_time_micros: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub status: ExecutionStatus,
    pub value: Option<SandboxValue>,
    pub value_display: Option<String>,
    pub stdout: String,
    pub stderr: String,
    pub exception: Option<ExecutionException>,
    pub usage: ExecutionUsage,
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool {
        self.status == ExecutionStatus::Success
    }

    fn success(
        value: Option<SandboxValue>,
        value_display: Option<String>,
        stdout: String,
        usage: ExecutionUsage,
    ) -> Self {
        Self {
            status: ExecutionStatus::Success,
            value,
            value_display,
            stdout,
            stderr: String::new(),
            exception: None,
            usage,
        }
    }

    fn error(status: ExecutionStatus, exception: ExecutionException) -> Self {
        Self {
            status,
            value: None,
            value_display: None,
            stdout: String::new(),
            stderr: String::new(),
            exception: Some(exception),
            usage: ExecutionUsage::default(),
        }
    }

    fn execution_error(error: String, stdout: String, usage: ExecutionUsage) -> Self {
        Self {
            status: ExecutionStatus::Error,
            value: None,
            value_display: None,
            stdout,
            stderr: String::new(),
            exception: Some(classify_execution_error(&error)),
            usage,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxLimits {
    pub max_process_memory_bytes: u64,
    pub max_time_ms: u64,
    pub max_source_bytes: usize,
    pub max_instructions: u64,
    pub max_call_depth: usize,
    pub max_output_bytes: usize,
    pub max_allocated_bytes: usize,
    pub bytes_warning: i64,
}

impl Default for SandboxLimits {
    fn default() -> Self {
        Self {
            max_process_memory_bytes: DEFAULT_MAX_PROCESS_MEMORY_BYTES,
            max_time_ms: DEFAULT_MAX_TIME_MS,
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            max_instructions: DEFAULT_SANDBOX_MAX_INSTRUCTIONS,
            max_call_depth: DEFAULT_SANDBOX_MAX_CALL_DEPTH,
            max_output_bytes: DEFAULT_SANDBOX_MAX_OUTPUT_BYTES,
            max_allocated_bytes: DEFAULT_SANDBOX_MAX_ALLOCATED_BYTES,
            bytes_warning: 0,
        }
    }
}

#[derive(Clone)]
pub struct Sandbox {
    worker_path: PathBuf,
    limits: SandboxLimits,
    root: Option<PathBuf>,
    external_functions: BTreeMap<String, ExternalFunction>,
}

impl fmt::Debug for Sandbox {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Sandbox")
            .field("worker_path", &self.worker_path)
            .field("limits", &self.limits)
            .field("root", &self.root)
            .field(
                "external_functions",
                &self.external_functions.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Sandbox {
    pub fn new(worker_path: impl Into<PathBuf>) -> Self {
        Self {
            worker_path: worker_path.into(),
            limits: SandboxLimits::default(),
            root: None,
            external_functions: BTreeMap::new(),
        }
    }

    pub fn with_limits(mut self, limits: SandboxLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn with_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.root = Some(root.into());
        self
    }

    pub fn register_external_function<F>(
        &mut self,
        name: impl Into<String>,
        function: F,
    ) -> Result<(), String>
    where
        F: Fn(ExternalCall) -> Result<SandboxValue, ExternalFunctionError> + Send + Sync + 'static,
    {
        let name = name.into();
        if !valid_input_name(&name) {
            return Err(format!(
                "invalid or reserved external function name '{name}'"
            ));
        }
        if self.external_functions.contains_key(&name) {
            return Err(format!("external function '{name}' is already registered"));
        }
        self.external_functions.insert(name, Arc::new(function));
        Ok(())
    }

    pub fn session(&self) -> Result<SandboxSession, String> {
        SandboxSession::start(self)
    }

    pub fn run(&self, source: &str) -> ExecutionResult {
        self.run_with_inputs(source, SandboxInputs::new())
    }

    pub fn run_with_inputs(&self, source: &str, inputs: SandboxInputs) -> ExecutionResult {
        self.execute(SandboxMode::Exec, source, inputs)
    }

    pub fn eval(&self, source: &str) -> ExecutionResult {
        self.eval_with_inputs(source, SandboxInputs::new())
    }

    pub fn eval_with_inputs(&self, source: &str, inputs: SandboxInputs) -> ExecutionResult {
        self.execute(SandboxMode::Eval, source, inputs)
    }

    pub fn check(&self, source: &str) -> ExecutionResult {
        self.execute(SandboxMode::Check, source, SandboxInputs::new())
    }

    pub fn execute(
        &self,
        mode: SandboxMode,
        source: &str,
        inputs: SandboxInputs,
    ) -> ExecutionResult {
        if let Some(error) =
            validate_execution_request(source, &inputs, &self.limits, &self.external_functions)
        {
            return error;
        }

        let request = WorkerRequest {
            version: 1,
            mode,
            source: source.to_string(),
            inputs,
            limits: self.limits.clone(),
            root: self.root.clone(),
            external_functions: self.external_functions.keys().cloned().collect(),
        };
        let request = match encode_frame(&request) {
            Ok(request) if request.len() <= MAX_PROTOCOL_FRAME_BYTES + 4 => request,
            Ok(_) => {
                return sandbox_error(
                    ExecutionStatus::Error,
                    "ResourceError",
                    "encoded request exceeds protocol limit".to_string(),
                );
            }
            Err(error) => return worker_error(format!("cannot encode worker request: {error}")),
        };

        let started = Instant::now();
        let mut command = Command::new(&self.worker_path);
        command
            .arg("--worker")
            .env(INTERNAL_WORKER_ENV, "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_process_memory_limit(&mut command, self.limits.max_process_memory_bytes);
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => {
                return worker_error(format!(
                    "failed to start isolated worker '{}': {error}",
                    self.worker_path.display()
                ));
            }
        };
        let mut worker_stdin = child.stdin.take().expect("worker stdin is piped");
        if let Err(error) = worker_stdin.write_all(&request) {
            let _ = child.kill();
            let _ = child.wait();
            return worker_error(format!("failed to send request to worker: {error}"));
        }

        let mut stdout = child.stdout.take().expect("worker stdout is piped");
        let mut stderr = child.stderr.take().expect("worker stderr is piped");
        let (message_sender, message_receiver) = mpsc::channel();
        let stdout_reader = thread::spawn(move || {
            loop {
                let message = read_frame::<_, WorkerMessage>(&mut stdout);
                let terminal =
                    message.is_err() || matches!(message, Ok(WorkerMessage::Complete(_)));
                if message_sender.send(message).is_err() || terminal {
                    break;
                }
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });
        let outcome = match drive_worker(
            &mut child,
            &mut worker_stdin,
            &message_receiver,
            &self.external_functions,
            self.limits.max_process_memory_bytes,
            self.limits.max_time_ms,
        ) {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return worker_error(error);
            }
        };
        drop(worker_stdin);
        if stdout_reader.join().is_err() {
            return worker_error("worker response reader panicked".to_string());
        }
        let worker_stderr = match stderr_reader.join() {
            Ok(Ok(bytes)) => String::from_utf8_lossy(&bytes).trim().to_string(),
            Ok(Err(error)) => format!("failed to read worker diagnostics: {error}"),
            Err(_) => "worker diagnostics reader panicked".to_string(),
        };

        let mut result = match outcome.termination {
            WorkerTermination::TimeLimit => sandbox_error(
                ExecutionStatus::TimeLimit,
                "ResourceError",
                "worker wall-clock limit exceeded".to_string(),
            ),
            WorkerTermination::MemoryLimit => sandbox_error(
                ExecutionStatus::MemoryLimit,
                "ResourceError",
                "worker exceeded process limits or crashed".to_string(),
            ),
            WorkerTermination::Completed if outcome.status.success() => outcome
                .result
                .unwrap_or_else(|| worker_error("worker exited without a result".to_string())),
            WorkerTermination::Completed => {
                let detail = if outcome.status.code().is_none()
                    || outcome.status.code().is_some_and(|code| code >= 125)
                {
                    "worker exceeded process limits or crashed".to_string()
                } else if worker_stderr.is_empty() {
                    outcome
                        .status
                        .code()
                        .map(|code| format!("worker exited with status {code}"))
                        .unwrap_or_else(|| "worker terminated without an exit status".to_string())
                } else {
                    worker_stderr
                };
                sandbox_error(ExecutionStatus::WorkerCrash, "WorkerCrashed", detail)
            }
        };
        result.usage.wall_time_micros =
            u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        result
    }
}

pub struct SandboxSession {
    child: Option<process::Child>,
    worker_stdin: Option<process::ChildStdin>,
    messages: mpsc::Receiver<Result<WorkerMessage, String>>,
    stdout_reader: Option<thread::JoinHandle<()>>,
    stderr_reader: Option<thread::JoinHandle<io::Result<Vec<u8>>>>,
    limits: SandboxLimits,
    root: Option<PathBuf>,
    external_functions: BTreeMap<String, ExternalFunction>,
    closed: bool,
}

impl fmt::Debug for SandboxSession {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SandboxSession")
            .field("worker_pid", &self.child.as_ref().map(process::Child::id))
            .field("limits", &self.limits)
            .field("root", &self.root)
            .field(
                "external_functions",
                &self.external_functions.keys().collect::<Vec<_>>(),
            )
            .field("closed", &self.closed)
            .finish()
    }
}

impl SandboxSession {
    fn start(sandbox: &Sandbox) -> Result<Self, String> {
        let mut command = Command::new(&sandbox.worker_path);
        command
            .arg("--session-worker")
            .env(INTERNAL_WORKER_ENV, "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        apply_process_memory_limit(&mut command, sandbox.limits.max_process_memory_bytes);
        let mut child = command.spawn().map_err(|error| {
            format!(
                "failed to start isolated session worker '{}': {error}",
                sandbox.worker_path.display()
            )
        })?;
        let worker_stdin = child.stdin.take().expect("session worker stdin is piped");
        let mut stdout = child.stdout.take().expect("session worker stdout is piped");
        let mut stderr = child.stderr.take().expect("session worker stderr is piped");
        let (message_sender, messages) = mpsc::channel();
        let stdout_reader = thread::spawn(move || {
            loop {
                let message = read_frame::<_, WorkerMessage>(&mut stdout);
                let terminal = message.is_err();
                if message_sender.send(message).is_err() || terminal {
                    break;
                }
            }
        });
        let stderr_reader = thread::spawn(move || {
            let mut bytes = Vec::new();
            stderr.read_to_end(&mut bytes).map(|_| bytes)
        });
        Ok(Self {
            child: Some(child),
            worker_stdin: Some(worker_stdin),
            messages,
            stdout_reader: Some(stdout_reader),
            stderr_reader: Some(stderr_reader),
            limits: sandbox.limits.clone(),
            root: sandbox.root.clone(),
            external_functions: sandbox.external_functions.clone(),
            closed: false,
        })
    }

    pub fn run(&mut self, source: &str) -> ExecutionResult {
        self.run_with_inputs(source, SandboxInputs::new())
    }

    pub fn run_with_inputs(&mut self, source: &str, inputs: SandboxInputs) -> ExecutionResult {
        self.execute(SandboxMode::Exec, source, inputs)
    }

    pub fn eval(&mut self, source: &str) -> ExecutionResult {
        self.eval_with_inputs(source, SandboxInputs::new())
    }

    pub fn eval_with_inputs(&mut self, source: &str, inputs: SandboxInputs) -> ExecutionResult {
        self.execute(SandboxMode::Eval, source, inputs)
    }

    pub fn check(&mut self, source: &str) -> ExecutionResult {
        self.execute(SandboxMode::Check, source, SandboxInputs::new())
    }

    pub fn is_closed(&self) -> bool {
        self.closed
    }

    pub fn close(mut self) -> Result<(), String> {
        self.shutdown()
    }

    fn execute(
        &mut self,
        mode: SandboxMode,
        source: &str,
        inputs: SandboxInputs,
    ) -> ExecutionResult {
        if self.closed {
            return worker_error("session is closed".to_string());
        }
        if let Some(error) =
            validate_execution_request(source, &inputs, &self.limits, &self.external_functions)
        {
            return error;
        }
        let request = WorkerRequest {
            version: 1,
            mode,
            source: source.to_string(),
            inputs,
            limits: self.limits.clone(),
            root: self.root.clone(),
            external_functions: self.external_functions.keys().cloned().collect(),
        };
        let Some(worker_stdin) = self.worker_stdin.as_mut() else {
            self.closed = true;
            return worker_error("session worker input is closed".to_string());
        };
        if let Err(error) = write_frame(worker_stdin, &SessionCommand::Execute(request)) {
            let _ = self.shutdown();
            return worker_error(format!("failed to send session request: {error}"));
        }

        let started = Instant::now();
        let outcome = {
            let Some(child) = self.child.as_mut() else {
                self.closed = true;
                return worker_error("session worker is unavailable".to_string());
            };
            let worker_stdin = self
                .worker_stdin
                .as_mut()
                .expect("open session has worker input");
            drive_session_execution(
                child,
                worker_stdin,
                &self.messages,
                &self.external_functions,
                self.limits.max_process_memory_bytes,
                self.limits.max_time_ms,
            )
        };
        let mut result = match outcome {
            Ok(SessionExecutionOutcome::Complete(result)) => result,
            Ok(SessionExecutionOutcome::TimeLimit) => {
                self.finish_worker();
                sandbox_error(
                    ExecutionStatus::TimeLimit,
                    "ResourceError",
                    "worker wall-clock limit exceeded".to_string(),
                )
            }
            Ok(SessionExecutionOutcome::MemoryLimit) => {
                self.finish_worker();
                sandbox_error(
                    ExecutionStatus::MemoryLimit,
                    "ResourceError",
                    "worker exceeded process limits or crashed".to_string(),
                )
            }
            Ok(SessionExecutionOutcome::Exited(status)) => {
                let diagnostics = self.finish_worker();
                worker_error(worker_exit_detail(status, &diagnostics))
            }
            Err(error) => {
                let _ = self.shutdown();
                worker_error(error)
            }
        };
        result.usage.wall_time_micros =
            u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        result
    }

    fn finish_worker(&mut self) -> String {
        self.closed = true;
        self.worker_stdin.take();
        if let Some(mut child) = self.child.take() {
            let _ = child.wait();
        }
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        self.stderr_reader
            .take()
            .and_then(|reader| reader.join().ok())
            .and_then(Result::ok)
            .map(|bytes| String::from_utf8_lossy(&bytes).trim().to_string())
            .unwrap_or_default()
    }

    fn shutdown(&mut self) -> Result<(), String> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        let mut first_error = None;
        if let Some(mut worker_stdin) = self.worker_stdin.take() {
            if let Err(error) = write_frame(&mut worker_stdin, &SessionCommand::Close) {
                first_error = Some(format!("failed to close session worker: {error}"));
            }
        }
        if let Some(mut child) = self.child.take() {
            let deadline = Instant::now() + Duration::from_secs(1);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) if Instant::now() < deadline => {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Ok(None) => {
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    Err(error) => {
                        first_error.get_or_insert_with(|| {
                            format!("failed to poll session worker shutdown: {error}")
                        });
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                }
            }
        }
        if let Some(reader) = self.stdout_reader.take() {
            let _ = reader.join();
        }
        if let Some(reader) = self.stderr_reader.take() {
            let _ = reader.join();
        }
        match first_error {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }
}

impl Drop for SandboxSession {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerRequest {
    version: u8,
    mode: SandboxMode,
    source: String,
    inputs: SandboxInputs,
    limits: SandboxLimits,
    root: Option<PathBuf>,
    external_functions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
enum WorkerMessage {
    ExternalCall { call_id: u64, call: ExternalCall },
    Complete(ExecutionResult),
}

#[derive(Debug, Serialize, Deserialize)]
struct HostCallResponse {
    call_id: u64,
    result: Result<SandboxValue, ExternalFunctionError>,
}

#[derive(Debug, Serialize, Deserialize)]
enum SessionCommand {
    Execute(WorkerRequest),
    Close,
}

enum WorkerBridgeCommand {
    ExternalCall {
        call_id: u64,
        call: ExternalCall,
        response: mpsc::Sender<Result<SandboxValue, ExternalFunctionError>>,
    },
    Complete(ExecutionResult),
}

pub fn serve_worker_once() -> i32 {
    let request = match {
        let stdin = io::stdin();
        read_frame::<_, WorkerRequest>(stdin.lock())
    } {
        Ok(request) => request,
        Err(error) => {
            eprintln!("worker protocol error: {error}");
            return 2;
        }
    };
    if request.version != 1 {
        eprintln!(
            "worker protocol error: unsupported version {}",
            request.version
        );
        return 2;
    }
    let mut session_state = None;
    match execute_worker_protocol_cycle(request, &mut session_state) {
        Ok(()) => 0,
        Err(error) => {
            eprintln!("worker protocol error: {error}");
            2
        }
    }
}

pub fn serve_session_worker() -> i32 {
    let mut session_state = None;
    loop {
        let command = match {
            let stdin = io::stdin();
            read_frame::<_, SessionCommand>(stdin.lock())
        } {
            Ok(command) => command,
            Err(error) => {
                eprintln!("worker protocol error: {error}");
                return 2;
            }
        };
        let request = match command {
            SessionCommand::Execute(request) => request,
            SessionCommand::Close => return 0,
        };
        if request.version != 1 {
            eprintln!(
                "worker protocol error: unsupported version {}",
                request.version
            );
            return 2;
        }
        if let Err(error) = execute_worker_protocol_cycle(request, &mut session_state) {
            eprintln!("worker protocol error: {error}");
            return 2;
        }
    }
}

fn execute_worker_protocol_cycle(
    request: WorkerRequest,
    session_state: &mut Option<VmSessionState>,
) -> Result<(), String> {
    let (bridge_sender, bridge_receiver) = mpsc::channel();
    let bridge = thread::spawn(move || run_worker_bridge(bridge_receiver));
    let handler = worker_external_call_handler(bridge_sender.clone());
    let result = execute_worker_request(request, handler, session_state);
    bridge_sender
        .send(WorkerBridgeCommand::Complete(result))
        .map_err(|_| "response bridge stopped early".to_string())?;
    drop(bridge_sender);
    match bridge.join() {
        Ok(result) => result,
        Err(_) => Err("response bridge panicked".to_string()),
    }
}

fn run_worker_bridge(receiver: mpsc::Receiver<WorkerBridgeCommand>) -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    while let Ok(command) = receiver.recv() {
        match command {
            WorkerBridgeCommand::ExternalCall {
                call_id,
                call,
                response,
            } => {
                if let Err(error) =
                    write_frame(&mut output, &WorkerMessage::ExternalCall { call_id, call })
                {
                    let _ = response.send(Err(ExternalFunctionError::new(
                        "RuntimeError",
                        format!("external function bridge write failed: {error}"),
                    )));
                    return Err(error);
                }
                let host_response = match read_frame::<_, HostCallResponse>(&mut input) {
                    Ok(host_response) => host_response,
                    Err(error) => {
                        let _ = response.send(Err(ExternalFunctionError::new(
                            "RuntimeError",
                            format!("external function bridge read failed: {error}"),
                        )));
                        return Err(error);
                    }
                };
                if host_response.call_id != call_id {
                    let error = format!(
                        "external function response id {} does not match call id {call_id}",
                        host_response.call_id
                    );
                    let _ = response.send(Err(ExternalFunctionError::new(
                        "RuntimeError",
                        error.clone(),
                    )));
                    return Err(error);
                }
                if response.send(host_response.result).is_err() {
                    return Err("external function caller stopped before its response".to_string());
                }
            }
            WorkerBridgeCommand::Complete(result) => {
                return write_frame(&mut output, &WorkerMessage::Complete(result));
            }
        }
    }
    Err("worker bridge closed without a final result".to_string())
}

fn worker_external_call_handler(sender: mpsc::Sender<WorkerBridgeCommand>) -> ExternalCallHandler {
    let next_call_id = Rc::new(Cell::new(1_u64));
    Rc::new(move |name, args, keywords| {
        let args = args
            .iter()
            .map(sandbox_value_from_internal_checked)
            .collect::<Result<Vec<_>, _>>()?;
        let keywords = keywords
            .iter()
            .map(|(name, value)| Ok((name.clone(), sandbox_value_from_internal_checked(value)?)))
            .collect::<Result<Vec<_>, String>>()?;
        let call_id = next_call_id.get();
        next_call_id.set(call_id.saturating_add(1));
        let (response_sender, response_receiver) = mpsc::channel();
        sender
            .send(WorkerBridgeCommand::ExternalCall {
                call_id,
                call: ExternalCall {
                    name,
                    args,
                    keywords,
                },
                response: response_sender,
            })
            .map_err(|_| "RuntimeError: external function bridge is unavailable".to_string())?;
        match response_receiver.recv() {
            Ok(Ok(value)) => Ok(sandbox_value_to_internal(&value)),
            Ok(Err(error)) => Err(format_external_function_error(error)),
            Err(_) => Err("RuntimeError: external function response was lost".to_string()),
        }
    })
}

fn execute_worker_request(
    request: WorkerRequest,
    external_call_handler: ExternalCallHandler,
    session_state: &mut Option<VmSessionState>,
) -> ExecutionResult {
    if request.source.len() > request.limits.max_source_bytes {
        return sandbox_error(
            ExecutionStatus::Error,
            "ResourceError",
            format!(
                "source exceeds {} byte limit",
                request.limits.max_source_bytes
            ),
        );
    }
    if let Some(name) = request.inputs.keys().find(|name| !valid_input_name(name)) {
        return sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            format!("invalid or reserved input name '{name}'"),
        );
    }
    if let Some(name) = request
        .inputs
        .iter()
        .find_map(|(name, value)| (!valid_input_value(value, 0)).then_some(name))
    {
        return sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            format!("input '{name}' contains an opaque or excessively nested value"),
        );
    }
    if let Some(name) = request
        .external_functions
        .iter()
        .find(|name| !valid_input_name(name))
    {
        return sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            format!("invalid or reserved external function name '{name}'"),
        );
    }
    let unique_external_functions = request.external_functions.iter().collect::<HashSet<_>>();
    if unique_external_functions.len() != request.external_functions.len() {
        return sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            "external function names must be unique".to_string(),
        );
    }
    if let Some(name) = request
        .inputs
        .keys()
        .find(|name| unique_external_functions.contains(name))
    {
        return sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            format!("input '{name}' conflicts with an external function"),
        );
    }

    let started = Instant::now();
    let mut result = match request.mode {
        SandboxMode::Exec => execute_program(
            &request,
            false,
            external_call_handler.clone(),
            session_state,
        ),
        SandboxMode::Eval => execute_program(&request, true, external_call_handler, session_state),
        SandboxMode::Check => execute_check(&request.source),
    };
    result.usage.wall_time_micros =
        u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
    result
}

fn execute_program(
    request: &WorkerRequest,
    eval: bool,
    external_call_handler: ExternalCallHandler,
    session_state: &mut Option<VmSessionState>,
) -> ExecutionResult {
    if let Err(error) = reject_too_complex_source(&request.source) {
        return ExecutionResult::execution_error(error, String::new(), ExecutionUsage::default());
    }
    let instructions = if eval {
        let tokens = match lex_for_parse(&request.source) {
            Ok(tokens) => tokens,
            Err(message) => {
                return ExecutionResult::execution_error(
                    format!("lex error: {message}"),
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        };
        let expression = match parse_eval(&tokens) {
            Ok(expression) => expression,
            Err(message) => {
                return ExecutionResult::execution_error(
                    format!("parse error: {message}"),
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        };
        match compile_eval(&expression) {
            Ok(instructions) => instructions,
            Err(message) => {
                return ExecutionResult::execution_error(
                    format!("compile error: {message}"),
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        }
    } else {
        let (spanned_tokens, _warnings) = match lex_with_spans_for_parse(&request.source) {
            Ok(result) => result,
            Err(error) => {
                return ExecutionResult::execution_error(
                    format!("lex error: {}", error.message),
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        };
        let tokens = spanned_tokens
            .iter()
            .map(|token| token.token.clone())
            .collect::<Vec<_>>();
        let program = match parse(&tokens) {
            Ok(program) => program,
            Err(message) => {
                return ExecutionResult::execution_error(
                    format!("parse error: {message}"),
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        };
        match compile_with_options(
            &program,
            compile_options_for_spanned_tokens(&spanned_tokens),
        ) {
            Ok(instructions) => instructions,
            Err(message) => {
                return ExecutionResult::execution_error(
                    format!("compile error: {message}"),
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        }
    };

    let modules = match &request.root {
        Some(root) => match virtual_modules_from_sandbox_dir(root) {
            Ok(modules) => modules,
            Err(error) => {
                return ExecutionResult::execution_error(
                    error,
                    String::new(),
                    ExecutionUsage::default(),
                );
            }
        },
        None => Vec::new(),
    };
    let module_sources = modules
        .into_iter()
        .map(|module| {
            (
                module.name,
                SourceModule {
                    source: module.source,
                    is_package: module.is_package,
                },
            )
        })
        .collect::<HashMap<_, _>>();
    let policy = policy_for_limits(&request.limits);
    let options = runtime_options_for_sandbox_policy(&policy);
    let mut inputs = request
        .inputs
        .iter()
        .map(|(name, value)| (name.clone(), sandbox_value_to_internal(value)))
        .collect::<Vec<_>>();
    inputs.extend(
        request
            .external_functions
            .iter()
            .map(|name| (name.clone(), external_function_value(name))),
    );
    let mut vm = Vm::new(instructions)
        .with_source_modules(Rc::new(module_sources))
        .with_runtime_options(options)
        .with_stdlib_import_policy(vm_stdlib_import_policy(policy))
        .with_external_call_handler(external_call_handler);
    if let Some(state) = session_state.clone() {
        vm = vm.with_session_state(state);
    }
    let mut vm = match vm.with_initial_globals(inputs) {
        Ok(vm) => vm,
        Err(error) => {
            return ExecutionResult::execution_error(
                format!("runtime error: {error}"),
                String::new(),
                ExecutionUsage::default(),
            );
        }
    };
    *session_state = Some(vm.session_state());
    let execution = if eval {
        vm.run_eval_captured()
    } else {
        vm.run_captured()
    };
    execution_result(execution, eval)
}

fn execute_check(source: &str) -> ExecutionResult {
    if let Err(error) = reject_too_complex_source(source) {
        return ExecutionResult::execution_error(error, String::new(), ExecutionUsage::default());
    }
    let (spanned_tokens, _warnings) = match lex_with_spans_for_parse(source) {
        Ok(result) => result,
        Err(error) => {
            return ExecutionResult::execution_error(
                format!("lex error: {}", error.message),
                String::new(),
                ExecutionUsage::default(),
            );
        }
    };
    let tokens = spanned_tokens
        .iter()
        .map(|token| token.token.clone())
        .collect::<Vec<_>>();
    let program = match parse(&tokens) {
        Ok(program) => program,
        Err(message) => {
            return ExecutionResult::execution_error(
                format!("parse error: {message}"),
                String::new(),
                ExecutionUsage::default(),
            );
        }
    };
    match compile_with_options(
        &program,
        compile_options_for_spanned_tokens(&spanned_tokens),
    ) {
        Ok(_) => ExecutionResult::success(
            Some(SandboxValue::None),
            None,
            String::new(),
            ExecutionUsage::default(),
        ),
        Err(message) => ExecutionResult::execution_error(
            format!("compile error: {message}"),
            String::new(),
            ExecutionUsage::default(),
        ),
    }
}

fn policy_for_limits(limits: &SandboxLimits) -> SandboxPolicy {
    SandboxPolicy::allow_stdlib_modules(SANDBOX_STDLIB_ALLOWLIST.iter().copied())
        .expect("built-in sandbox stdlib allowlist is valid")
        .with_bytes_warning(limits.bytes_warning)
        .with_max_instructions(limits.max_instructions)
        .with_max_call_depth(limits.max_call_depth)
        .with_max_output_bytes(limits.max_output_bytes)
        .with_max_allocated_bytes(limits.max_allocated_bytes)
}

fn execution_result(execution: VmExecution, eval: bool) -> ExecutionResult {
    let usage = ExecutionUsage {
        instructions: execution.usage.instructions,
        output_bytes: execution.usage.output_bytes,
        allocated_bytes: execution.usage.allocated_bytes,
        wall_time_micros: 0,
    };
    match execution.result {
        Ok(value) => {
            let display = eval.then(|| value.to_string());
            ExecutionResult::success(
                Some(if eval {
                    sandbox_value_from_internal(&value)
                } else {
                    SandboxValue::None
                }),
                display,
                execution.stdout,
                usage,
            )
        }
        Err(error) => ExecutionResult::execution_error(
            format!("runtime error: {error}"),
            execution.stdout,
            usage,
        ),
    }
}

fn sandbox_value_to_internal(value: &SandboxValue) -> Value {
    match value {
        SandboxValue::None => Value::None,
        SandboxValue::Bool(value) => Value::Bool(*value),
        SandboxValue::Integer(value) => value
            .try_into()
            .map(Value::Number)
            .unwrap_or_else(|_| Value::BigInt(value.clone())),
        SandboxValue::Float(value) => float_value(*value),
        SandboxValue::String(value) => Value::String(value.clone()),
        SandboxValue::Bytes(value) => bytes_value(value.clone()),
        SandboxValue::ByteArray(value) => byte_array_value(value.clone()),
        SandboxValue::List(values) => {
            list_value(values.iter().map(sandbox_value_to_internal).collect())
        }
        SandboxValue::Tuple(values) => {
            tuple_value(values.iter().map(sandbox_value_to_internal).collect())
        }
        SandboxValue::Dict(entries) => dict_value(
            entries
                .iter()
                .map(|(key, value)| {
                    (
                        sandbox_value_to_internal(key),
                        sandbox_value_to_internal(value),
                    )
                })
                .collect(),
        ),
        SandboxValue::Opaque { .. } => unreachable!("opaque values are rejected before execution"),
    }
}

fn sandbox_value_from_internal(value: &Value) -> SandboxValue {
    fn convert(
        value: &Value,
        active: &mut HashSet<(usize, u8)>,
        depth: usize,
    ) -> Option<SandboxValue> {
        if depth > 128 {
            return None;
        }
        match value {
            Value::None => Some(SandboxValue::None),
            Value::Bool(value) => Some(SandboxValue::Bool(*value)),
            Value::Number(value) => Some(SandboxValue::Integer((*value).into())),
            Value::BigInt(value) => Some(SandboxValue::Integer(value.clone())),
            Value::Float(value) => Some(SandboxValue::Float(**value)),
            Value::String(value) => Some(SandboxValue::String(value.clone())),
            Value::Bytes(value) => Some(SandboxValue::Bytes(value.as_ref().clone())),
            Value::ByteArray(value) => Some(SandboxValue::ByteArray(value.borrow().to_vec())),
            Value::List(values) => {
                let key = (Rc::as_ptr(values) as usize, 1);
                if !active.insert(key) {
                    return None;
                }
                let converted = values
                    .borrow()
                    .iter()
                    .map(|value| convert(value, active, depth + 1))
                    .collect::<Option<Vec<_>>>()
                    .map(SandboxValue::List);
                active.remove(&key);
                converted
            }
            Value::Tuple(values) => {
                let key = (Rc::as_ptr(values) as usize, 2);
                if !active.insert(key) {
                    return None;
                }
                let converted = values
                    .iter()
                    .map(|value| convert(value, active, depth + 1))
                    .collect::<Option<Vec<_>>>()
                    .map(SandboxValue::Tuple);
                active.remove(&key);
                converted
            }
            Value::Dict(entries) => {
                let key = (Rc::as_ptr(entries) as usize, 3);
                if !active.insert(key) {
                    return None;
                }
                let converted = entries
                    .borrow()
                    .iter()
                    .map(|(key, value)| {
                        Some((
                            convert(key, active, depth + 1)?,
                            convert(value, active, depth + 1)?,
                        ))
                    })
                    .collect::<Option<Vec<_>>>()
                    .map(SandboxValue::Dict);
                active.remove(&key);
                converted
            }
            _ => None,
        }
    }

    convert(value, &mut HashSet::new(), 0).unwrap_or_else(|| SandboxValue::Opaque {
        type_name: crate::vm::type_name(value).to_string(),
        display: value.to_string(),
    })
}

fn sandbox_value_from_internal_checked(value: &Value) -> Result<SandboxValue, String> {
    match sandbox_value_from_internal(value) {
        SandboxValue::Opaque { type_name, .. } => Err(format!(
            "TypeError: external function values cannot contain '{type_name}' objects"
        )),
        value => Ok(value),
    }
}

fn format_external_function_error(error: ExternalFunctionError) -> String {
    let type_name = if valid_exception_type_name(&error.type_name) {
        error.type_name
    } else {
        "ExternalFunctionError".to_string()
    };
    format!("{type_name}: {}", error.message)
}

fn valid_exception_type_name(name: &str) -> bool {
    valid_input_name(name) && (name.ends_with("Error") || name.ends_with("Exception"))
}

fn classify_execution_error(error: &str) -> ExecutionException {
    let phases = [
        ("decode error: ", ExecutionPhase::Decode, "UnicodeError"),
        ("lex error: ", ExecutionPhase::Lex, "SyntaxError"),
        ("parse error: ", ExecutionPhase::Parse, "SyntaxError"),
        ("compile error: ", ExecutionPhase::Compile, "SyntaxError"),
    ];
    for (prefix, phase, type_name) in phases {
        if let Some(message) = error.strip_prefix(prefix) {
            return ExecutionException {
                phase,
                type_name: type_name.to_string(),
                message: message.to_string(),
            };
        }
    }
    if let Some(message) = error.strip_prefix("sandbox error: ") {
        return ExecutionException {
            phase: ExecutionPhase::Sandbox,
            type_name: "SandboxError".to_string(),
            message: message.to_string(),
        };
    }
    let runtime = error.strip_prefix("runtime error: ").unwrap_or(error);
    if let Some(message) = runtime.strip_prefix("sandbox error: ") {
        return ExecutionException {
            phase: ExecutionPhase::Sandbox,
            type_name: "ResourceError".to_string(),
            message: message.to_string(),
        };
    }
    let (type_name, message) = runtime
        .split_once(": ")
        .filter(|(name, _)| name.ends_with("Error") || name.ends_with("Exception"))
        .unwrap_or(("RuntimeError", runtime));
    ExecutionException {
        phase: ExecutionPhase::Runtime,
        type_name: type_name.to_string(),
        message: message.to_string(),
    }
}

fn sandbox_error(status: ExecutionStatus, type_name: &str, message: String) -> ExecutionResult {
    ExecutionResult::error(
        status,
        ExecutionException {
            phase: ExecutionPhase::Sandbox,
            type_name: type_name.to_string(),
            message,
        },
    )
}

fn worker_error(message: String) -> ExecutionResult {
    ExecutionResult::error(
        ExecutionStatus::WorkerCrash,
        ExecutionException {
            phase: ExecutionPhase::Worker,
            type_name: "WorkerError".to_string(),
            message,
        },
    )
}

fn worker_exit_detail(status: ExitStatus, stderr: &str) -> String {
    if status.code().is_none() || status.code().is_some_and(|code| code >= 125) {
        "worker exceeded process limits or crashed".to_string()
    } else if stderr.is_empty() {
        status
            .code()
            .map(|code| format!("worker exited with status {code}"))
            .unwrap_or_else(|| "worker terminated without an exit status".to_string())
    } else {
        stderr.to_string()
    }
}

fn validate_execution_request(
    source: &str,
    inputs: &SandboxInputs,
    limits: &SandboxLimits,
    external_functions: &BTreeMap<String, ExternalFunction>,
) -> Option<ExecutionResult> {
    if source.len() > limits.max_source_bytes {
        return Some(sandbox_error(
            ExecutionStatus::Error,
            "ResourceError",
            format!("source exceeds {} byte limit", limits.max_source_bytes),
        ));
    }
    if let Some(name) = inputs.keys().find(|name| !valid_input_name(name)) {
        return Some(sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            format!("invalid or reserved input name '{name}'"),
        ));
    }
    if let Some(name) = inputs
        .iter()
        .find_map(|(name, value)| (!valid_input_value(value, 0)).then_some(name))
    {
        return Some(sandbox_error(
            ExecutionStatus::Error,
            "SandboxInputError",
            format!("input '{name}' contains an opaque or excessively nested value"),
        ));
    }
    inputs
        .keys()
        .find(|name| external_functions.contains_key(*name))
        .map(|name| {
            sandbox_error(
                ExecutionStatus::Error,
                "SandboxInputError",
                format!("input '{name}' conflicts with an external function"),
            )
        })
}

fn valid_input_name(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if first != '_' && !unicode_ident::is_xid_start(first) {
        return false;
    }
    if !chars.all(|character| character == '_' || unicode_ident::is_xid_continue(character)) {
        return false;
    }
    !matches!(
        name,
        "False"
            | "None"
            | "True"
            | "and"
            | "as"
            | "assert"
            | "async"
            | "await"
            | "break"
            | "class"
            | "continue"
            | "def"
            | "del"
            | "elif"
            | "else"
            | "except"
            | "finally"
            | "for"
            | "from"
            | "global"
            | "if"
            | "import"
            | "in"
            | "is"
            | "lambda"
            | "nonlocal"
            | "not"
            | "or"
            | "pass"
            | "raise"
            | "return"
            | "try"
            | "while"
            | "with"
            | "yield"
            | "__annotations__"
            | "__builtins__"
            | "__debug__"
            | "__doc__"
            | "__name__"
            | "__package__"
            | "__spec__"
    )
}

fn valid_input_value(value: &SandboxValue, depth: usize) -> bool {
    if depth > 128 {
        return false;
    }
    match value {
        SandboxValue::List(values) | SandboxValue::Tuple(values) => values
            .iter()
            .all(|value| valid_input_value(value, depth + 1)),
        SandboxValue::Dict(entries) => entries.iter().all(|(key, value)| {
            valid_input_value(key, depth + 1) && valid_input_value(value, depth + 1)
        }),
        SandboxValue::Opaque { .. } => false,
        SandboxValue::None
        | SandboxValue::Bool(_)
        | SandboxValue::Integer(_)
        | SandboxValue::Float(_)
        | SandboxValue::String(_)
        | SandboxValue::Bytes(_)
        | SandboxValue::ByteArray(_) => true,
    }
}

fn encode_frame<T: Serialize>(value: &T) -> Result<Vec<u8>, String> {
    let payload = rmp_serde::to_vec_named(value).map_err(|error| error.to_string())?;
    let length = u32::try_from(payload.len()).map_err(|_| "protocol frame is too large")?;
    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&length.to_be_bytes());
    frame.extend_from_slice(&payload);
    Ok(frame)
}

fn read_frame<R: Read, T: for<'de> Deserialize<'de>>(mut reader: R) -> Result<T, String> {
    let mut header = [0_u8; 4];
    reader
        .read_exact(&mut header)
        .map_err(|error| format!("cannot read frame header: {error}"))?;
    let length = u32::from_be_bytes(header) as usize;
    if length > MAX_PROTOCOL_FRAME_BYTES {
        return Err("protocol frame exceeds size limit".to_string());
    }
    let mut payload = vec![0; length];
    reader
        .read_exact(&mut payload)
        .map_err(|error| format!("cannot read frame body: {error}"))?;
    rmp_serde::from_slice(&payload).map_err(|error| error.to_string())
}

fn write_frame<W: Write, T: Serialize>(mut writer: W, value: &T) -> Result<(), String> {
    let frame = encode_frame(value)?;
    if frame.len() > MAX_PROTOCOL_FRAME_BYTES + 4 {
        return Err("protocol frame exceeds size limit".to_string());
    }
    writer
        .write_all(&frame)
        .map_err(|error| format!("cannot write frame: {error}"))?;
    writer
        .flush()
        .map_err(|error| format!("cannot flush frame: {error}"))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum WorkerTermination {
    Completed,
    MemoryLimit,
    TimeLimit,
}

struct WorkerOutcome {
    status: ExitStatus,
    termination: WorkerTermination,
    result: Option<ExecutionResult>,
}

enum SessionExecutionOutcome {
    Complete(ExecutionResult),
    MemoryLimit,
    TimeLimit,
    Exited(ExitStatus),
}

fn drive_session_execution(
    child: &mut process::Child,
    worker_stdin: &mut process::ChildStdin,
    messages: &mpsc::Receiver<Result<WorkerMessage, String>>,
    external_functions: &BTreeMap<String, ExternalFunction>,
    max_memory_bytes: u64,
    max_time_ms: u64,
) -> Result<SessionExecutionOutcome, String> {
    let started = Instant::now();
    let time_limit = Duration::from_millis(max_time_ms);
    loop {
        if started.elapsed() >= time_limit {
            child.kill().map_err(|error| {
                format!("failed to terminate timed-out session worker: {error}")
            })?;
            child
                .wait()
                .map_err(|error| format!("failed to reap timed-out session worker: {error}"))?;
            return Ok(SessionExecutionOutcome::TimeLimit);
        }
        if worker_memory_limit_exceeded(child, max_memory_bytes)? {
            child.kill().map_err(|error| {
                format!("failed to terminate oversized session worker: {error}")
            })?;
            child
                .wait()
                .map_err(|error| format!("failed to reap oversized session worker: {error}"))?;
            return Ok(SessionExecutionOutcome::MemoryLimit);
        }
        match messages.recv_timeout(Duration::from_millis(2)) {
            Ok(Ok(WorkerMessage::ExternalCall { call_id, call })) => {
                respond_to_external_call(worker_stdin, external_functions, call_id, call)?;
            }
            Ok(Ok(WorkerMessage::Complete(result))) => {
                return Ok(SessionExecutionOutcome::Complete(result));
            }
            Ok(Err(error)) => return Err(format!("invalid session worker response: {error}")),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll session worker: {error}"))?
        {
            loop {
                match messages.recv() {
                    Ok(Ok(WorkerMessage::ExternalCall { call_id, call })) => {
                        respond_to_external_call(worker_stdin, external_functions, call_id, call)?;
                    }
                    Ok(Ok(WorkerMessage::Complete(result))) => {
                        return Ok(SessionExecutionOutcome::Complete(result));
                    }
                    Ok(Err(_)) | Err(_) => return Ok(SessionExecutionOutcome::Exited(status)),
                }
            }
        }
    }
}

fn drive_worker(
    child: &mut process::Child,
    worker_stdin: &mut process::ChildStdin,
    messages: &mpsc::Receiver<Result<WorkerMessage, String>>,
    external_functions: &BTreeMap<String, ExternalFunction>,
    max_memory_bytes: u64,
    max_time_ms: u64,
) -> Result<WorkerOutcome, String> {
    let started = Instant::now();
    let time_limit = Duration::from_millis(max_time_ms);
    let mut result = None;
    loop {
        if started.elapsed() >= time_limit {
            child
                .kill()
                .map_err(|error| format!("failed to terminate timed-out worker: {error}"))?;
            let status = child
                .wait()
                .map_err(|error| format!("failed to reap timed-out worker: {error}"))?;
            return Ok(WorkerOutcome {
                status,
                termination: WorkerTermination::TimeLimit,
                result: None,
            });
        }
        if worker_memory_limit_exceeded(child, max_memory_bytes)? {
            child
                .kill()
                .map_err(|error| format!("failed to terminate oversized worker: {error}"))?;
            let status = child
                .wait()
                .map_err(|error| format!("failed to reap oversized worker: {error}"))?;
            return Ok(WorkerOutcome {
                status,
                termination: WorkerTermination::MemoryLimit,
                result: None,
            });
        }
        match messages.recv_timeout(Duration::from_millis(2)) {
            Ok(Ok(WorkerMessage::ExternalCall { call_id, call })) => {
                respond_to_external_call(worker_stdin, external_functions, call_id, call)?;
            }
            Ok(Ok(WorkerMessage::Complete(execution_result))) => {
                if result.replace(execution_result).is_some() {
                    return Err("worker sent more than one final result".to_string());
                }
            }
            Ok(Err(error)) => return Err(format!("invalid worker response: {error}")),
            Err(mpsc::RecvTimeoutError::Timeout | mpsc::RecvTimeoutError::Disconnected) => {}
        }
        if let Some(status) = child
            .try_wait()
            .map_err(|error| format!("failed to poll worker: {error}"))?
        {
            while result.is_none() {
                match messages.recv() {
                    Ok(Ok(WorkerMessage::ExternalCall { call_id, call })) => {
                        respond_to_external_call(worker_stdin, external_functions, call_id, call)?;
                    }
                    Ok(Ok(WorkerMessage::Complete(execution_result))) => {
                        result = Some(execution_result);
                    }
                    Ok(Err(error)) => {
                        return Err(format!("invalid worker response: {error}"));
                    }
                    Err(_) => break,
                }
            }
            return Ok(WorkerOutcome {
                status,
                termination: WorkerTermination::Completed,
                result,
            });
        }
    }
}

fn respond_to_external_call(
    worker_stdin: &mut process::ChildStdin,
    external_functions: &BTreeMap<String, ExternalFunction>,
    call_id: u64,
    call: ExternalCall,
) -> Result<(), String> {
    let result = match external_functions.get(&call.name) {
        Some(function) => match catch_unwind(AssertUnwindSafe(|| function(call))) {
            Ok(Ok(value)) if valid_input_value(&value, 0) => Ok(value),
            Ok(Ok(_)) => Err(ExternalFunctionError::new(
                "TypeError",
                "external function returned an opaque or excessively nested value",
            )),
            Ok(Err(error)) => Err(normalize_external_function_error(error)),
            Err(_) => Err(ExternalFunctionError::new(
                "RuntimeError",
                "external function panicked",
            )),
        },
        None => Err(ExternalFunctionError::new(
            "PermissionError",
            format!("external function '{}' is not authorized", call.name),
        )),
    };
    write_frame(worker_stdin, &HostCallResponse { call_id, result })
        .map_err(|error| format!("failed to send external function response: {error}"))
}

fn normalize_external_function_error(error: ExternalFunctionError) -> ExternalFunctionError {
    if valid_exception_type_name(&error.type_name) {
        error
    } else {
        ExternalFunctionError::new("ExternalFunctionError", error.message)
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
fn apply_process_memory_limit(_command: &mut Command, _bytes: u64) {}

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
fn worker_memory_limit_exceeded(
    child: &process::Child,
    max_memory_bytes: u64,
) -> Result<bool, String> {
    match process_memory_bytes(child) {
        Ok(bytes) => Ok(bytes > max_memory_bytes),
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(false),
        Err(error) => Err(format!("failed to inspect worker memory: {error}")),
    }
}

#[cfg(not(target_os = "macos"))]
fn worker_memory_limit_exceeded(
    _child: &process::Child,
    _max_memory_bytes: u64,
) -> Result<bool, String> {
    Ok(false)
}
