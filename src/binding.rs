use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use num_bigint::BigInt;
use serde::{Deserialize, Serialize};

use crate::{
    ExecutionException, ExecutionPhase, ExecutionResult, ExecutionStatus, ExecutionUsage,
    ExternalCall, ExternalFunctionError, Sandbox, SandboxInputs, SandboxLimits, SandboxMode,
    SandboxSession, SandboxValue,
};

const MAX_JSON_LINE_BYTES: usize = 32 * 1_048_576;
const MAX_VALUE_DEPTH: usize = 128;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingLimits {
    max_process_memory_bytes: Option<u64>,
    max_time_ms: Option<u64>,
    max_source_bytes: Option<usize>,
    max_instructions: Option<u64>,
    max_call_depth: Option<usize>,
    max_output_bytes: Option<usize>,
    max_allocated_bytes: Option<usize>,
    bytes_warning: Option<i64>,
}

impl BindingLimits {
    fn apply(self) -> SandboxLimits {
        let defaults = SandboxLimits::default();
        SandboxLimits {
            max_process_memory_bytes: self
                .max_process_memory_bytes
                .unwrap_or(defaults.max_process_memory_bytes),
            max_time_ms: self.max_time_ms.unwrap_or(defaults.max_time_ms),
            max_source_bytes: self.max_source_bytes.unwrap_or(defaults.max_source_bytes),
            max_instructions: self.max_instructions.unwrap_or(defaults.max_instructions),
            max_call_depth: self.max_call_depth.unwrap_or(defaults.max_call_depth),
            max_output_bytes: self.max_output_bytes.unwrap_or(defaults.max_output_bytes),
            max_allocated_bytes: self
                .max_allocated_bytes
                .unwrap_or(defaults.max_allocated_bytes),
            bytes_warning: self.bytes_warning.unwrap_or(defaults.bytes_warning),
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum BindingMode {
    Exec,
    Eval,
    Check,
}

impl From<BindingMode> for SandboxMode {
    fn from(mode: BindingMode) -> Self {
        match mode {
            BindingMode::Exec => Self::Exec,
            BindingMode::Eval => Self::Eval,
            BindingMode::Check => Self::Check,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BindingCommand {
    Configure {
        #[serde(default)]
        limits: BindingLimits,
        #[serde(default)]
        root: Option<String>,
        #[serde(default)]
        external_functions: Vec<String>,
    },
    Execute {
        mode: BindingMode,
        source: String,
        #[serde(default)]
        inputs: BTreeMap<String, BindingValue>,
    },
    StartSession,
    SessionExecute {
        mode: BindingMode,
        source: String,
        #[serde(default)]
        inputs: BTreeMap<String, BindingValue>,
    },
    CloseSession,
    ExternalResult {
        call_id: u64,
        #[serde(default)]
        value: Option<BindingValue>,
        #[serde(default)]
        error: Option<ExternalFunctionError>,
    },
    Close,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BindingResponse {
    Ok,
    ExecutionResult {
        result: BindingExecutionResult,
    },
    ExternalCall {
        call_id: u64,
        name: String,
        args: Vec<BindingValue>,
        keywords: Vec<(String, BindingValue)>,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
enum BindingValue {
    None,
    Bool(bool),
    Integer(String),
    Float(String),
    String(String),
    Bytes(Vec<u8>),
    ByteArray(Vec<u8>),
    List(Vec<BindingValue>),
    Tuple(Vec<BindingValue>),
    Dict(Vec<(BindingValue, BindingValue)>),
    Opaque(BindingOpaque),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BindingOpaque {
    type_name: String,
    display: String,
}

impl BindingValue {
    fn from_sandbox(value: SandboxValue) -> Self {
        match value {
            SandboxValue::None => Self::None,
            SandboxValue::Bool(value) => Self::Bool(value),
            SandboxValue::Integer(value) => Self::Integer(value.to_string()),
            SandboxValue::Float(value) => Self::Float(value.to_string()),
            SandboxValue::String(value) => Self::String(value),
            SandboxValue::Bytes(value) => Self::Bytes(value),
            SandboxValue::ByteArray(value) => Self::ByteArray(value),
            SandboxValue::List(values) => {
                Self::List(values.into_iter().map(Self::from_sandbox).collect())
            }
            SandboxValue::Tuple(values) => {
                Self::Tuple(values.into_iter().map(Self::from_sandbox).collect())
            }
            SandboxValue::Dict(entries) => Self::Dict(
                entries
                    .into_iter()
                    .map(|(key, value)| (Self::from_sandbox(key), Self::from_sandbox(value)))
                    .collect(),
            ),
            SandboxValue::Opaque { type_name, display } => {
                Self::Opaque(BindingOpaque { type_name, display })
            }
        }
    }

    fn into_sandbox(self, depth: usize) -> Result<SandboxValue, String> {
        if depth > MAX_VALUE_DEPTH {
            return Err(format!(
                "binding value exceeds maximum nesting depth of {MAX_VALUE_DEPTH}"
            ));
        }
        match self {
            Self::None => Ok(SandboxValue::None),
            Self::Bool(value) => Ok(SandboxValue::Bool(value)),
            Self::Integer(value) => value
                .parse::<BigInt>()
                .map(SandboxValue::Integer)
                .map_err(|_| "binding integer must be a base-10 integer string".to_string()),
            Self::Float(value) => value
                .parse::<f64>()
                .map(SandboxValue::Float)
                .map_err(|_| "binding float must be a valid floating-point string".to_string()),
            Self::String(value) => Ok(SandboxValue::String(value)),
            Self::Bytes(value) => Ok(SandboxValue::Bytes(value)),
            Self::ByteArray(value) => Ok(SandboxValue::ByteArray(value)),
            Self::List(values) => values
                .into_iter()
                .map(|value| value.into_sandbox(depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(SandboxValue::List),
            Self::Tuple(values) => values
                .into_iter()
                .map(|value| value.into_sandbox(depth + 1))
                .collect::<Result<Vec<_>, _>>()
                .map(SandboxValue::Tuple),
            Self::Dict(entries) => entries
                .into_iter()
                .map(|(key, value)| {
                    Ok((key.into_sandbox(depth + 1)?, value.into_sandbox(depth + 1)?))
                })
                .collect::<Result<Vec<_>, String>>()
                .map(SandboxValue::Dict),
            Self::Opaque(value) => Ok(SandboxValue::Opaque {
                type_name: value.type_name,
                display: value.display,
            }),
        }
    }
}

#[derive(Debug, Serialize)]
struct BindingExecutionResult {
    status: &'static str,
    value: Option<BindingValue>,
    value_display: Option<String>,
    stdout: String,
    stderr: String,
    exception: Option<BindingExecutionException>,
    usage: ExecutionUsage,
}

impl From<ExecutionResult> for BindingExecutionResult {
    fn from(result: ExecutionResult) -> Self {
        Self {
            status: status_name(result.status),
            value: result.value.map(BindingValue::from_sandbox),
            value_display: result.value_display,
            stdout: result.stdout,
            stderr: result.stderr,
            exception: result.exception.map(BindingExecutionException::from),
            usage: result.usage,
        }
    }
}

#[derive(Debug, Serialize)]
struct BindingExecutionException {
    phase: &'static str,
    type_name: String,
    message: String,
}

impl From<ExecutionException> for BindingExecutionException {
    fn from(exception: ExecutionException) -> Self {
        Self {
            phase: phase_name(exception.phase),
            type_name: exception.type_name,
            message: exception.message,
        }
    }
}

fn status_name(status: ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Success => "success",
        ExecutionStatus::Error => "error",
        ExecutionStatus::TimeLimit => "time_limit",
        ExecutionStatus::MemoryLimit => "memory_limit",
        ExecutionStatus::WorkerCrash => "worker_crash",
    }
}

fn phase_name(phase: ExecutionPhase) -> &'static str {
    match phase {
        ExecutionPhase::Decode => "decode",
        ExecutionPhase::Lex => "lex",
        ExecutionPhase::Parse => "parse",
        ExecutionPhase::Compile => "compile",
        ExecutionPhase::Runtime => "runtime",
        ExecutionPhase::Sandbox => "sandbox",
        ExecutionPhase::Worker => "worker",
    }
}

struct BindingChannel {
    input: Mutex<BufReader<io::Stdin>>,
    output: Mutex<BufWriter<io::Stdout>>,
}

impl BindingChannel {
    fn new() -> Self {
        Self {
            input: Mutex::new(BufReader::new(io::stdin())),
            output: Mutex::new(BufWriter::new(io::stdout())),
        }
    }

    fn read(&self) -> Result<Option<BindingCommand>, String> {
        let mut input = self
            .input
            .lock()
            .map_err(|_| "binding input lock is poisoned".to_string())?;
        let mut bytes = Vec::new();
        let count = (&mut *input)
            .take((MAX_JSON_LINE_BYTES + 1) as u64)
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("failed to read binding request: {error}"))?;
        if count == 0 {
            return Ok(None);
        }
        if bytes.len() > MAX_JSON_LINE_BYTES {
            return Err(format!(
                "binding request exceeds {MAX_JSON_LINE_BYTES} byte limit"
            ));
        }
        let line = std::str::from_utf8(&bytes)
            .map_err(|_| "binding request must be valid UTF-8".to_string())?;
        serde_json::from_str(line)
            .map(Some)
            .map_err(|error| format!("invalid binding request: {error}"))
    }

    fn write(&self, response: &BindingResponse) -> Result<(), String> {
        let mut output = self
            .output
            .lock()
            .map_err(|_| "binding output lock is poisoned".to_string())?;
        serde_json::to_writer(&mut *output, response)
            .map_err(|error| format!("failed to encode binding response: {error}"))?;
        output
            .write_all(b"\n")
            .and_then(|_| output.flush())
            .map_err(|error| format!("failed to write binding response: {error}"))
    }
}

fn binding_inputs(values: BTreeMap<String, BindingValue>) -> Result<SandboxInputs, String> {
    values
        .into_iter()
        .map(|(name, value)| Ok((name, value.into_sandbox(0)?)))
        .collect()
}

fn forward_external_call(
    channel: &BindingChannel,
    next_call_id: &AtomicU64,
    call: ExternalCall,
) -> Result<SandboxValue, ExternalFunctionError> {
    let call_id = next_call_id.fetch_add(1, Ordering::Relaxed);
    let response = BindingResponse::ExternalCall {
        call_id,
        name: call.name,
        args: call
            .args
            .into_iter()
            .map(BindingValue::from_sandbox)
            .collect(),
        keywords: call
            .keywords
            .into_iter()
            .map(|(name, value)| (name, BindingValue::from_sandbox(value)))
            .collect(),
    };
    channel.write(&response).map_err(|error| {
        ExternalFunctionError::new(
            "RuntimeError",
            format!("Python binding disconnected: {error}"),
        )
    })?;

    let command = channel.read().map_err(|error| {
        ExternalFunctionError::new(
            "RuntimeError",
            format!("invalid callback response: {error}"),
        )
    })?;
    let Some(BindingCommand::ExternalResult {
        call_id: response_id,
        value,
        error,
    }) = command
    else {
        return Err(ExternalFunctionError::new(
            "RuntimeError",
            "Python binding did not return an external_result response",
        ));
    };
    if response_id != call_id {
        return Err(ExternalFunctionError::new(
            "RuntimeError",
            "Python binding returned an unexpected external call id",
        ));
    }
    match (value, error) {
        (Some(value), None) => value.into_sandbox(0).map_err(|error| {
            ExternalFunctionError::new("TypeError", format!("invalid callback value: {error}"))
        }),
        (None, Some(error)) => Err(error),
        _ => Err(ExternalFunctionError::new(
            "RuntimeError",
            "Python binding callback response must contain exactly one of value or error",
        )),
    }
}

fn execute_once(
    sandbox: &Sandbox,
    mode: BindingMode,
    source: &str,
    inputs: SandboxInputs,
) -> ExecutionResult {
    match mode {
        BindingMode::Exec => sandbox.run_with_inputs(source, inputs),
        BindingMode::Eval => sandbox.eval_with_inputs(source, inputs),
        BindingMode::Check if inputs.is_empty() => sandbox.check(source),
        BindingMode::Check => invalid_check_inputs(),
    }
}

fn execute_session(
    session: &mut SandboxSession,
    mode: BindingMode,
    source: &str,
    inputs: SandboxInputs,
) -> ExecutionResult {
    match mode {
        BindingMode::Exec => session.run_with_inputs(source, inputs),
        BindingMode::Eval => session.eval_with_inputs(source, inputs),
        BindingMode::Check if inputs.is_empty() => session.check(source),
        BindingMode::Check => invalid_check_inputs(),
    }
}

fn invalid_check_inputs() -> ExecutionResult {
    ExecutionResult {
        status: ExecutionStatus::Error,
        value: None,
        value_display: None,
        stdout: String::new(),
        stderr: String::new(),
        exception: Some(ExecutionException {
            phase: ExecutionPhase::Sandbox,
            type_name: "SandboxInputError".to_string(),
            message: "syntax checks do not accept inputs".to_string(),
        }),
        usage: ExecutionUsage::default(),
    }
}

fn send_error(channel: &BindingChannel, message: impl Into<String>) -> bool {
    channel
        .write(&BindingResponse::Error {
            message: message.into(),
        })
        .is_ok()
}

pub fn serve_python_binding() -> i32 {
    let channel = Arc::new(BindingChannel::new());
    let next_call_id = Arc::new(AtomicU64::new(1));
    let worker_path = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            let _ = channel.write(&BindingResponse::Error {
                message: format!("cannot locate sandbox worker executable: {error}"),
            });
            return 1;
        }
    };
    let mut sandbox = None::<Sandbox>;
    let mut session = None::<SandboxSession>;

    loop {
        let command = match channel.read() {
            Ok(Some(command)) => command,
            Ok(None) => return 0,
            Err(error) => {
                let _ = send_error(&channel, error);
                return 1;
            }
        };
        match command {
            BindingCommand::Configure {
                limits,
                root,
                external_functions,
            } => {
                if sandbox.is_some() {
                    if !send_error(&channel, "binding is already configured") {
                        return 1;
                    }
                    continue;
                }
                let mut configured = Sandbox::new(&worker_path).with_limits(limits.apply());
                if let Some(root) = root {
                    configured = configured.with_root(PathBuf::from(root));
                }
                let mut registration_error = None;
                for name in external_functions {
                    let callback_channel = Arc::clone(&channel);
                    let callback_call_id = Arc::clone(&next_call_id);
                    if let Err(error) = configured.register_external_function(name, move |call| {
                        forward_external_call(&callback_channel, &callback_call_id, call)
                    }) {
                        registration_error = Some(error);
                        break;
                    }
                }
                if let Some(error) = registration_error {
                    if !send_error(&channel, error) {
                        return 1;
                    }
                    continue;
                }
                sandbox = Some(configured);
                if channel.write(&BindingResponse::Ok).is_err() {
                    return 1;
                }
            }
            BindingCommand::Execute {
                mode,
                source,
                inputs,
            } => {
                let Some(configured) = sandbox.as_ref() else {
                    if !send_error(&channel, "binding must be configured before execution") {
                        return 1;
                    }
                    continue;
                };
                let inputs = match binding_inputs(inputs) {
                    Ok(inputs) => inputs,
                    Err(error) => {
                        if !send_error(&channel, error) {
                            return 1;
                        }
                        continue;
                    }
                };
                let result = execute_once(configured, mode, &source, inputs);
                if channel
                    .write(&BindingResponse::ExecutionResult {
                        result: result.into(),
                    })
                    .is_err()
                {
                    return 1;
                }
            }
            BindingCommand::StartSession => {
                if session.is_some() {
                    if !send_error(&channel, "a binding session is already active") {
                        return 1;
                    }
                    continue;
                }
                let Some(configured) = sandbox.as_ref() else {
                    if !send_error(
                        &channel,
                        "binding must be configured before starting a session",
                    ) {
                        return 1;
                    }
                    continue;
                };
                match configured.session() {
                    Ok(started) => {
                        session = Some(started);
                        if channel.write(&BindingResponse::Ok).is_err() {
                            return 1;
                        }
                    }
                    Err(error) => {
                        if !send_error(&channel, error) {
                            return 1;
                        }
                    }
                }
            }
            BindingCommand::SessionExecute {
                mode,
                source,
                inputs,
            } => {
                let Some(active_session) = session.as_mut() else {
                    if !send_error(&channel, "no binding session is active") {
                        return 1;
                    }
                    continue;
                };
                let inputs = match binding_inputs(inputs) {
                    Ok(inputs) => inputs,
                    Err(error) => {
                        if !send_error(&channel, error) {
                            return 1;
                        }
                        continue;
                    }
                };
                let result = execute_session(active_session, mode, &source, inputs);
                if active_session.is_closed() {
                    session.take();
                }
                if channel
                    .write(&BindingResponse::ExecutionResult {
                        result: result.into(),
                    })
                    .is_err()
                {
                    return 1;
                }
            }
            BindingCommand::CloseSession => {
                let Some(active_session) = session.take() else {
                    if !send_error(&channel, "no binding session is active") {
                        return 1;
                    }
                    continue;
                };
                if let Err(error) = active_session.close() {
                    if !send_error(&channel, error) {
                        return 1;
                    }
                    continue;
                }
                if channel.write(&BindingResponse::Ok).is_err() {
                    return 1;
                }
            }
            BindingCommand::ExternalResult { .. } => {
                if !send_error(
                    &channel,
                    "external_result is only valid while handling an external call",
                ) {
                    return 1;
                }
            }
            BindingCommand::Close => {
                drop(session.take());
                return if channel.write(&BindingResponse::Ok).is_ok() {
                    0
                } else {
                    1
                };
            }
        }
    }
}
