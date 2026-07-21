"""Pure Python client for the isolated MiniPython sandbox executable."""

from __future__ import annotations

import json
import os
import subprocess
import threading
from dataclasses import dataclass
from typing import Any, Callable, Dict, Mapping, Optional, Sequence


class SandboxBindingError(RuntimeError):
    """The binding transport or lifecycle contract failed."""


@dataclass(frozen=True)
class Limits:
    max_process_memory_bytes: Optional[int] = None
    max_time_ms: Optional[int] = None
    max_source_bytes: Optional[int] = None
    max_instructions: Optional[int] = None
    max_call_depth: Optional[int] = None
    max_output_bytes: Optional[int] = None
    max_allocated_bytes: Optional[int] = None
    bytes_warning: Optional[int] = None

    def _wire(self) -> Dict[str, int]:
        return {
            name: value
            for name, value in vars(self).items()
            if value is not None
        }


@dataclass(frozen=True)
class OpaqueValue:
    type_name: str
    display: str


@dataclass(frozen=True)
class ExecutionException:
    phase: str
    type_name: str
    message: str


@dataclass(frozen=True)
class ExecutionUsage:
    instructions: int
    output_bytes: int
    allocated_bytes: int
    wall_time_micros: int


@dataclass(frozen=True)
class ExecutionResult:
    status: str
    value: Any
    value_display: Optional[str]
    stdout: str
    stderr: str
    exception: Optional[ExecutionException]
    usage: ExecutionUsage

    @property
    def is_success(self) -> bool:
        return self.status == "success"


def _encode_value(value: Any, depth: int = 0, active: Optional[set[int]] = None) -> Any:
    if depth > 128:
        raise TypeError("binding value exceeds maximum nesting depth of 128")
    if value is None:
        return {"kind": "none"}
    if isinstance(value, bool):
        return {"kind": "bool", "value": value}
    if isinstance(value, int):
        return {"kind": "integer", "value": str(value)}
    if isinstance(value, float):
        return {"kind": "float", "value": repr(value)}
    if isinstance(value, str):
        return {"kind": "string", "value": value}
    if isinstance(value, bytes):
        return {"kind": "bytes", "value": list(value)}
    if isinstance(value, bytearray):
        return {"kind": "byte_array", "value": list(value)}
    if isinstance(value, OpaqueValue):
        return {
            "kind": "opaque",
            "value": {"type_name": value.type_name, "display": value.display},
        }
    if active is None:
        active = set()
    if isinstance(value, (list, tuple, dict)):
        identity = id(value)
        if identity in active:
            raise TypeError("binding values cannot contain cycles")
        active.add(identity)
        try:
            if isinstance(value, list):
                return {
                    "kind": "list",
                    "value": [_encode_value(item, depth + 1, active) for item in value],
                }
            if isinstance(value, tuple):
                return {
                    "kind": "tuple",
                    "value": [_encode_value(item, depth + 1, active) for item in value],
                }
            return {
                "kind": "dict",
                "value": [
                    [
                        _encode_value(key, depth + 1, active),
                        _encode_value(item, depth + 1, active),
                    ]
                    for key, item in value.items()
                ],
            }
        finally:
            active.remove(identity)
    raise TypeError(f"unsupported binding value type: {type(value).__name__}")


def _decode_value(payload: Any, depth: int = 0) -> Any:
    if depth > 128 or not isinstance(payload, dict):
        raise SandboxBindingError("invalid nested value in binding response")
    kind = payload.get("kind")
    value = payload.get("value")
    if kind == "none":
        return None
    if kind == "bool":
        return bool(value)
    if kind == "integer":
        return int(value)
    if kind == "float":
        return float(value)
    if kind == "string":
        return str(value)
    if kind == "bytes":
        return bytes(value)
    if kind == "byte_array":
        return bytearray(value)
    if kind == "list":
        return [_decode_value(item, depth + 1) for item in value]
    if kind == "tuple":
        return tuple(_decode_value(item, depth + 1) for item in value)
    if kind == "dict":
        decoded = {}
        for key, item in value:
            decoded[_decode_value(key, depth + 1)] = _decode_value(item, depth + 1)
        return decoded
    if kind == "opaque":
        return OpaqueValue(type_name=value["type_name"], display=value["display"])
    raise SandboxBindingError(f"unknown binding value kind: {kind!r}")


def _decode_result(payload: Mapping[str, Any]) -> ExecutionResult:
    exception_payload = payload.get("exception")
    exception = (
        None
        if exception_payload is None
        else ExecutionException(
            phase=exception_payload["phase"],
            type_name=exception_payload["type_name"],
            message=exception_payload["message"],
        )
    )
    usage = payload["usage"]
    return ExecutionResult(
        status=payload["status"],
        value=None if payload.get("value") is None else _decode_value(payload["value"]),
        value_display=payload.get("value_display"),
        stdout=payload["stdout"],
        stderr=payload["stderr"],
        exception=exception,
        usage=ExecutionUsage(
            instructions=usage["instructions"],
            output_bytes=usage["output_bytes"],
            allocated_bytes=usage["allocated_bytes"],
            wall_time_micros=usage["wall_time_micros"],
        ),
    )


class Sandbox:
    """A process-isolated MiniPython sandbox controlled through one broker."""

    def __init__(
        self,
        executable: Optional[os.PathLike[str] | str] = None,
        *,
        limits: Optional[Limits] = None,
        root: Optional[os.PathLike[str] | str] = None,
        external_functions: Optional[Mapping[str, Callable[..., Any]]] = None,
    ) -> None:
        executable = executable or os.environ.get("MINIPYTHON_EXECUTABLE", "mnpy")
        self._callbacks = dict(external_functions or {})
        for name, callback in self._callbacks.items():
            if not isinstance(name, str):
                raise TypeError("external function names must be strings")
            if not callable(callback):
                raise TypeError(f"external function {name!r} is not callable")
        self._lock = threading.RLock()
        self._busy = False
        self._closed = False
        self._session: Optional[Session] = None
        self._stderr: list[str] = []
        environment = os.environ.copy()
        environment["MINIPYTHON_INTERNAL_WORKER"] = "1"
        self._process = subprocess.Popen(
            [os.fspath(executable), "--python-binding"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            encoding="utf-8",
            bufsize=1,
            env=environment,
        )
        assert self._process.stdin is not None
        assert self._process.stdout is not None
        assert self._process.stderr is not None
        self._stderr_thread = threading.Thread(
            target=self._collect_stderr,
            args=(self._process.stderr,),
            daemon=True,
        )
        self._stderr_thread.start()
        try:
            self._request(
                {
                    "type": "configure",
                    "limits": (limits or Limits())._wire(),
                    "root": None if root is None else os.fspath(root),
                    "external_functions": list(self._callbacks),
                },
                "ok",
            )
        except BaseException:
            self._closed = True
            self._terminate()
            raise

    def _collect_stderr(self, stream: Any) -> None:
        for line in stream:
            self._stderr.append(line)

    def _send(self, payload: Mapping[str, Any]) -> None:
        if self._process.stdin is None:
            raise SandboxBindingError("binding input is closed")
        try:
            self._process.stdin.write(json.dumps(payload, separators=(",", ":")) + "\n")
            self._process.stdin.flush()
        except (BrokenPipeError, OSError) as error:
            raise SandboxBindingError(f"binding disconnected: {error}") from error

    def _read(self) -> Mapping[str, Any]:
        if self._process.stdout is None:
            raise SandboxBindingError("binding output is closed")
        line = self._process.stdout.readline()
        if not line:
            detail = "".join(self._stderr).strip()
            suffix = f": {detail}" if detail else ""
            raise SandboxBindingError(f"binding exited unexpectedly{suffix}")
        try:
            response = json.loads(line)
        except json.JSONDecodeError as error:
            raise SandboxBindingError(f"binding returned invalid JSON: {error}") from error
        if not isinstance(response, dict) or not isinstance(response.get("type"), str):
            raise SandboxBindingError("binding returned an invalid response")
        return response

    def _handle_external_call(self, response: Mapping[str, Any]) -> None:
        call_id = response["call_id"]
        name = response["name"]
        try:
            callback = self._callbacks[name]
            args = [_decode_value(value) for value in response["args"]]
            keywords = {
                key: _decode_value(value) for key, value in response["keywords"]
            }
            value = _encode_value(callback(*args, **keywords))
            callback_response = {
                "type": "external_result",
                "call_id": call_id,
                "value": value,
            }
        except BaseException as error:
            callback_response = {
                "type": "external_result",
                "call_id": call_id,
                "error": {
                    "type_name": type(error).__name__,
                    "message": str(error),
                },
            }
        self._send(callback_response)

    def _request(self, payload: Mapping[str, Any], expected: str) -> Mapping[str, Any]:
        with self._lock:
            if self._closed:
                raise SandboxBindingError("binding is closed")
            if self._busy:
                raise SandboxBindingError("binding calls cannot be reentrant")
            self._busy = True
            try:
                self._send(payload)
                while True:
                    response = self._read()
                    response_type = response["type"]
                    if response_type == "external_call":
                        self._handle_external_call(response)
                        continue
                    if response_type == "error":
                        raise SandboxBindingError(response.get("message", "binding request failed"))
                    if response_type != expected:
                        raise SandboxBindingError(
                            f"expected {expected!r}, received {response_type!r}"
                        )
                    return response
            finally:
                self._busy = False

    def _execute(
        self,
        command: str,
        mode: str,
        source: str,
        inputs: Optional[Mapping[str, Any]],
    ) -> ExecutionResult:
        encoded_inputs = {
            name: _encode_value(value) for name, value in (inputs or {}).items()
        }
        response = self._request(
            {
                "type": command,
                "mode": mode,
                "source": source,
                "inputs": encoded_inputs,
            },
            "execution_result",
        )
        return _decode_result(response["result"])

    def run(
        self, source: str, inputs: Optional[Mapping[str, Any]] = None
    ) -> ExecutionResult:
        return self._execute("execute", "exec", source, inputs)

    def eval(
        self, source: str, inputs: Optional[Mapping[str, Any]] = None
    ) -> ExecutionResult:
        return self._execute("execute", "eval", source, inputs)

    def check(self, source: str) -> ExecutionResult:
        return self._execute("execute", "check", source, None)

    def session(self) -> "Session":
        with self._lock:
            if self._session is not None and not self._session.closed:
                raise SandboxBindingError("a binding session is already active")
            self._request({"type": "start_session"}, "ok")
            self._session = Session(self)
            return self._session

    def close(self) -> None:
        with self._lock:
            if self._closed:
                return
            try:
                self._request({"type": "close"}, "ok")
            finally:
                self._closed = True
                if self._session is not None:
                    self._session._closed = True
                self._terminate()

    def _terminate(self) -> None:
        if self._process.stdin is not None and not self._process.stdin.closed:
            self._process.stdin.close()
        try:
            self._process.wait(timeout=2)
        except subprocess.TimeoutExpired:
            self._process.kill()
            self._process.wait()
        if self._process.stdout is not None:
            self._process.stdout.close()
        if self._process.stderr is not None:
            self._process.stderr.close()

    def __enter__(self) -> "Sandbox":
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()

    def __del__(self) -> None:
        if not getattr(self, "_closed", True):
            try:
                self.close()
            except Exception:
                self._terminate()


class Session:
    def __init__(self, sandbox: Sandbox) -> None:
        self._sandbox = sandbox
        self._closed = False

    @property
    def closed(self) -> bool:
        return self._closed

    def _execute(
        self,
        mode: str,
        source: str,
        inputs: Optional[Mapping[str, Any]],
    ) -> ExecutionResult:
        if self._closed:
            raise SandboxBindingError("session is closed")
        result = self._sandbox._execute("session_execute", mode, source, inputs)
        if result.status in {"time_limit", "memory_limit", "worker_crash"}:
            self._closed = True
            self._sandbox._session = None
        return result

    def run(
        self, source: str, inputs: Optional[Mapping[str, Any]] = None
    ) -> ExecutionResult:
        return self._execute("exec", source, inputs)

    def eval(
        self, source: str, inputs: Optional[Mapping[str, Any]] = None
    ) -> ExecutionResult:
        return self._execute("eval", source, inputs)

    def check(self, source: str) -> ExecutionResult:
        return self._execute("check", source, None)

    def close(self) -> None:
        if self._closed:
            return
        self._sandbox._request({"type": "close_session"}, "ok")
        self._closed = True

    def __enter__(self) -> "Session":
        return self

    def __exit__(self, exc_type: Any, exc: Any, traceback: Any) -> None:
        self.close()


__all__: Sequence[str] = (
    "ExecutionException",
    "ExecutionResult",
    "ExecutionUsage",
    "Limits",
    "OpaqueValue",
    "Sandbox",
    "SandboxBindingError",
    "Session",
)
