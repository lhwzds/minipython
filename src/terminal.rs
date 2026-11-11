// SPDX-License-Identifier: MIT

//! Terminal sandbox conditions.
//!
//! A terminal condition ends the whole execution: the instruction,
//! allocation and output budgets, the hard native-stack ceiling, and (on the
//! parent side) the worker's wall-clock and process-memory limits and worker
//! failures. Unlike `RecursionError` or `MemoryError`, which CPython raises
//! as ordinary exceptions, no `except`, `finally`, `with` block, generator,
//! or native callback may observe or survive one.
//!
//! The VM threads errors through `Result<_, String>`, so a terminal
//! condition still travels up the Rust stack as a carrier string such as
//! `sandbox error: instruction limit exceeded`. The carrier is not the
//! authority, though. Tripping a condition latches its [`TerminalKind`] for
//! the current interpreter thread, and while the latch is set:
//!
//! - the frame loop's exception dispatch (`Vm::run_frame`), raising API
//!   (`Vm::raise_exception`), and error-to-exception conversion
//!   (`runtime_exception_from_message`) refuse to produce a Python exception
//!   and return the carrier instead, so wrapping the carrier in another
//!   message (`TypeError: ...`) cannot make it catchable;
//! - every instruction boundary stops, so native code that swallowed the
//!   carrier (an `Err(_) =>` fallback) cannot resume Python execution;
//! - every VM run returns the carrier, and the worker reports the latched
//!   kind, whatever the run itself returned.
//!
//! The latch is per thread because each execution owns its interpreter
//! thread; a top-level VM and every worker request reset it.

use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::{Deserialize, Serialize};

/// Why an execution was ended by the sandbox rather than by the program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TerminalKind {
    /// The VM instruction budget (`--max-steps`) ran out.
    InstructionLimit,
    /// The VM allocation budget (`--max-allocated-bytes`) ran out.
    AllocationLimit,
    /// The captured-output budget (`--max-output-bytes`) ran out.
    OutputLimit,
    /// Native recursion consumed the stack guard's headroom: the hard
    /// ceiling below the catchable `RecursionError` margins.
    NativeStackExhausted,
    /// The parent killed the worker at its wall-clock deadline.
    TimeLimit,
    /// The parent killed the worker over its process-memory limit.
    MemoryLimit,
    /// The worker could not be started, broke the protocol, or exited
    /// without a result.
    WorkerFailure,
}

impl TerminalKind {
    /// The `ExecutionException::message` reported for this kind. Worker
    /// failures carry their own detail instead.
    pub fn message(self) -> &'static str {
        match self {
            Self::InstructionLimit => "instruction limit exceeded",
            Self::AllocationLimit => "allocation limit exceeded",
            Self::OutputLimit => "output limit exceeded",
            Self::NativeStackExhausted => "native stack exhausted",
            Self::TimeLimit => "worker wall-clock limit exceeded",
            Self::MemoryLimit => "worker memory limit exceeded",
            Self::WorkerFailure => "worker failed",
        }
    }

    /// Whether the VM itself raises this kind (as opposed to the parent).
    pub fn is_vm_kind(self) -> bool {
        matches!(
            self,
            Self::InstructionLimit
                | Self::AllocationLimit
                | Self::OutputLimit
                | Self::NativeStackExhausted
        )
    }
}

/// The carrier string a terminal kind travels as through `Result<_, String>`.
pub(crate) fn carrier(kind: TerminalKind) -> String {
    format!("sandbox error: {}", kind.message())
}

/// Per-thread latch slot. Dropping a latched slot (thread exit) releases its
/// share of [`LATCHED_THREADS`].
struct LatchSlot {
    kind: Cell<Option<TerminalKind>>,
}

impl Drop for LatchSlot {
    fn drop(&mut self) {
        if self.kind.take().is_some() {
            LATCHED_THREADS.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

thread_local! {
    static LATCH: LatchSlot = const {
        LatchSlot {
            kind: Cell::new(None),
        }
    };
}

/// Number of threads holding a latched condition. The interpreter polls the
/// latch before every instruction, so the common case is one relaxed global
/// load; a nonzero value only means "look at your own slot".
static LATCHED_THREADS: AtomicUsize = AtomicUsize::new(0);

/// Latches `kind` for the current thread and returns the carrier to
/// propagate. The first condition wins: once latched, later trips return
/// the original carrier so the reported cause never changes.
#[cold]
pub(crate) fn trip(kind: TerminalKind) -> String {
    let latched = LATCH.with(|slot| match slot.kind.get() {
        Some(existing) => existing,
        None => {
            slot.kind.set(Some(kind));
            LATCHED_THREADS.fetch_add(1, Ordering::Relaxed);
            kind
        }
    });
    carrier(latched)
}

/// The condition latched on this thread, if any.
#[inline(always)]
pub(crate) fn latched() -> Option<TerminalKind> {
    if LATCHED_THREADS.load(Ordering::Relaxed) == 0 {
        return None;
    }
    latched_slow()
}

#[inline(never)]
fn latched_slow() -> Option<TerminalKind> {
    LATCH.with(|slot| slot.kind.get())
}

/// `Err(carrier)` while a condition is latched on this thread.
#[inline(always)]
pub(crate) fn check() -> Result<(), String> {
    match latched() {
        None => Ok(()),
        Some(kind) => Err(carrier(kind)),
    }
}

/// Clears this thread's latch at the start of a new execution.
pub(crate) fn reset() {
    if LATCHED_THREADS.load(Ordering::Relaxed) == 0 {
        return;
    }
    LATCH.with(|slot| {
        if slot.kind.take().is_some() {
            LATCHED_THREADS.fetch_sub(1, Ordering::Relaxed);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::compiler::compile_with_options;
    use crate::lexer::lex_with_spans_for_parse;
    use crate::parser::parse;
    use crate::value::external_function_value;
    use crate::vm::{RuntimeOptions, Vm, VmExecution};

    /// Places a terminal condition can arise in, each calling `T()` where
    /// the condition trips. Every one wraps the trigger in handlers that
    /// would swallow an ordinary exception.
    const CONTEXTS: &[(&str, &str)] = &[
        ("direct", "T()"),
        (
            "nested handlers",
            "try:\n    try:\n        T()\n    except Exception:\n        print('CAUGHT inner')\nexcept:\n    print('CAUGHT bare')",
        ),
        (
            "generator body",
            "def g():\n    try:\n        T()\n        yield 1\n    except BaseException:\n        yield 'CAUGHT in generator'\nprint(list(g()))",
        ),
        (
            "generator finally on close",
            "def g():\n    try:\n        yield 1\n    finally:\n        T()\nit = g()\nnext(it)\nit.close()",
        ),
        (
            "generator throw",
            "def g():\n    try:\n        yield 1\n    except ValueError:\n        T()\n    except BaseException:\n        yield 'CAUGHT in generator'\nit = g()\nnext(it)\nit.throw(ValueError)",
        ),
        (
            "__exit__",
            "class C:\n    def __enter__(self):\n        return self\n    def __exit__(self, *exc):\n        T()\nwith C():\n    pass",
        ),
        (
            "suppressing __exit__",
            "class C:\n    def __enter__(self):\n        return self\n    def __exit__(self, *exc):\n        print('CAUGHT by __exit__')\n        return True\nwith C():\n    T()",
        ),
        (
            "finally",
            "try:\n    T()\nfinally:\n    print('CAUGHT by finally')",
        ),
        (
            "raise in finally",
            "try:\n    T()\nfinally:\n    raise ValueError('replaced')",
        ),
        (
            "except star",
            "try:\n    T()\nexcept* BaseException:\n    print('CAUGHT by except*')",
        ),
        (
            "re.sub callable",
            "import re\nre.sub('a', lambda m: T(), 'aaa')",
        ),
        ("sort key", "sorted([3, 2, 1], key=lambda v: T())"),
        (
            "sort __lt__",
            "class A:\n    def __lt__(self, other):\n        return T()\nsorted([A(), A()])",
        ),
        (
            "json object_hook",
            "import json\njson.loads('{\"a\": 1}', object_hook=lambda d: T())",
        ),
        (
            "json default",
            "import json\njson.dumps(object(), default=lambda o: T())",
        ),
        (
            "__str__ from print",
            "class A:\n    def __str__(self):\n        return T()\nprint(A())",
        ),
        (
            "__str__ from str.format",
            "class A:\n    def __str__(self):\n        return T()\n'{}'.format(A())",
        ),
        (
            "__repr__ from repr",
            "class A:\n    def __repr__(self):\n        return T()\nrepr(A())",
        ),
        (
            "__getattr__ from hasattr",
            "class A:\n    def __getattr__(self, name):\n        return T()\nhasattr(A(), 'x')",
        ),
        (
            "__getattr__ from getattr default",
            "class A:\n    def __getattr__(self, name):\n        return T()\ngetattr(A(), 'x', None)",
        ),
        (
            "__hash__ from a set",
            "class A:\n    def __hash__(self):\n        return T()\n{A()}",
        ),
        (
            "next default",
            "def g():\n    T()\n    yield 1\nnext(g(), None)",
        ),
        ("map", "list(map(lambda v: T(), [1]))"),
        ("exec", "exec('T()')"),
        ("eval", "eval('T()')"),
        ("class body", "class X:\n    T()"),
        (
            "__init_subclass__",
            "class B:\n    def __init_subclass__(cls):\n        T()\nclass X(B):\n    pass",
        ),
        (
            "coroutine",
            "async def f():\n    try:\n        T()\n    except BaseException:\n        print('CAUGHT in coroutine')\nc = f()\ntry:\n    c.send(None)\nexcept StopIteration:\n    pass",
        ),
        (
            "copy.deepcopy hook",
            "import copy\nclass A:\n    def __deepcopy__(self, memo):\n        return T()\ncopy.deepcopy(A())",
        ),
    ];

    /// `T()` for each VM terminal kind, and the options that make it trip.
    fn trigger(kind: TerminalKind) -> (&'static str, RuntimeOptions) {
        let options = RuntimeOptions::default();
        match kind {
            TerminalKind::InstructionLimit => (
                "def T():\n    while True:\n        pass\n",
                options.with_max_instructions(50_000),
            ),
            TerminalKind::AllocationLimit => (
                "def T():\n    items = []\n    while True:\n        items.append('y' * 1000)\n",
                options
                    .with_max_instructions(50_000_000)
                    .with_max_allocated_bytes(400_000),
            ),
            TerminalKind::OutputLimit => (
                "def T():\n    while True:\n        print('z' * 100)\n",
                options
                    .with_max_instructions(50_000_000)
                    .with_max_output_bytes(4_000),
            ),
            // The hard native-stack ceiling cannot be reached from Python on
            // demand, so `T` is a native function that reports it exactly as
            // the stack guard does.
            TerminalKind::NativeStackExhausted => ("", options.with_max_instructions(50_000_000)),
            _ => unreachable!("not a VM kind"),
        }
    }

    /// What a test run reports, detached from VM values so it can leave the
    /// interpreter thread.
    struct Outcome {
        result: Result<(), String>,
        stdout: String,
        reported_exception: bool,
    }

    fn run(source: &str, options: RuntimeOptions) -> Outcome {
        let source = source.to_string();
        crate::stack::run_on_vm_stack(64 * 1024 * 1024, move || {
            let (tokens, _) = lex_with_spans_for_parse(&source).expect("test source lexes");
            let plain = tokens.iter().map(|t| t.token.clone()).collect::<Vec<_>>();
            let program = parse(&plain).expect("test source parses");
            let instructions =
                compile_with_options(&program, crate::compile_options_for_spanned_tokens(&tokens))
                    .expect("test source compiles");
            let handler: crate::vm::ExternalCallHandler =
                std::rc::Rc::new(|_, _, _| Err(crate::stack::stack_exhausted_error()));
            let mut vm = Vm::new(instructions)
                .with_runtime_options(options)
                .with_external_call_handler(handler)
                .with_initial_globals(vec![(
                    "exhaust".to_string(),
                    external_function_value("exhaust"),
                )])
                .expect("globals fit the budget");
            let execution: VmExecution = vm.run_captured();
            assert_eq!(
                latched().is_some(),
                execution
                    .result
                    .as_ref()
                    .is_err_and(|e| e.starts_with("sandbox error: ")),
                "the latch and the result agree"
            );
            Outcome {
                result: execution.result.map(|_| ()),
                reported_exception: execution.uncaught.is_some() || execution.system_exit.is_some(),
                stdout: execution.stdout,
            }
        })
        .expect("VM thread starts")
    }

    #[test]
    fn instruction_limit_after_exception_dispatch_stays_terminal() {
        let source = "while True:\n    try: 1 / 0\n    except ZeroDivisionError: pass\n";
        for limit in 1..=60 {
            let execution = run(
                source,
                RuntimeOptions::default().with_max_instructions(limit),
            );
            assert_eq!(
                execution.result,
                Err(carrier(TerminalKind::InstructionLimit)),
                "limit {limit}"
            );
            assert!(!execution.reported_exception, "limit {limit}");
        }
    }

    #[test]
    fn no_handler_survives_a_terminal_condition() {
        for kind in [
            TerminalKind::InstructionLimit,
            TerminalKind::AllocationLimit,
            TerminalKind::OutputLimit,
            TerminalKind::NativeStackExhausted,
        ] {
            let (definition, options) = trigger(kind);
            let definition = if definition.is_empty() {
                "def T():\n    return exhaust()\n"
            } else {
                definition
            };
            for (context, body) in CONTEXTS {
                let indented = body
                    .lines()
                    .map(|line| format!("    {line}\n"))
                    .collect::<String>();
                let source = format!(
                    "{definition}print('BEFORE')\ntry:\n{indented}except BaseException as error:\n    print('CAUGHT', type(error).__name__)\nprint('CONTINUED')\n"
                );
                let execution = run(&source, options);
                assert_eq!(
                    execution.result,
                    Err(carrier(kind)),
                    "{kind:?} in {context}: stdout={:?}",
                    execution.stdout
                );
                assert!(
                    !execution.reported_exception,
                    "{kind:?} in {context} reported a Python exception"
                );
                let stdout = execution.stdout;
                assert!(
                    stdout.starts_with("BEFORE\n"),
                    "{kind:?} in {context}: {stdout:?}"
                );
                assert!(
                    !stdout.contains("CAUGHT") && !stdout.contains("CONTINUED"),
                    "{kind:?} in {context} was observed: {stdout:?}"
                );
            }
        }
    }

    #[test]
    fn swallowing_the_carrier_in_native_code_does_not_resume_python() {
        // `exhaust` reports the condition, the native caller here discards
        // the error, and Python code still never runs again.
        let execution = run(
            "class A:\n    def __eq__(self, other):\n        exhaust()\nprint('BEFORE')\ntry:\n    [A()].count(1)\nexcept BaseException:\n    pass\nprint('CONTINUED')\n",
            RuntimeOptions::default(),
        );
        assert_eq!(
            execution.result,
            Err(carrier(TerminalKind::NativeStackExhausted))
        );
        assert_eq!(execution.stdout, "BEFORE\n");
    }

    #[test]
    fn a_new_top_level_vm_clears_an_earlier_latch() {
        crate::stack::run_on_vm_stack(8 * 1024 * 1024, || {
            trip(TerminalKind::OutputLimit);
            let _vm = Vm::new(Vec::new());
            assert_eq!(latched(), None);
        })
        .expect("VM thread starts");
    }

    #[test]
    fn the_first_trip_wins_until_reset() {
        std::thread::spawn(|| {
            assert_eq!(latched(), None);
            assert_eq!(check(), Ok(()));
            assert_eq!(
                trip(TerminalKind::OutputLimit),
                "sandbox error: output limit exceeded"
            );
            assert_eq!(
                trip(TerminalKind::InstructionLimit),
                "sandbox error: output limit exceeded"
            );
            assert_eq!(latched(), Some(TerminalKind::OutputLimit));
            assert_eq!(
                check(),
                Err("sandbox error: output limit exceeded".to_string())
            );
            reset();
            assert_eq!(latched(), None);
            assert_eq!(
                trip(TerminalKind::InstructionLimit),
                "sandbox error: instruction limit exceeded"
            );
        })
        .join()
        .expect("latch thread");
    }

    #[test]
    fn a_latch_is_private_to_its_thread() {
        let other = std::thread::spawn(|| {
            trip(TerminalKind::AllocationLimit);
            std::thread::spawn(latched).join().expect("inner thread")
        })
        .join()
        .expect("latch thread");
        assert_eq!(other, None);
    }

    #[test]
    fn vm_kinds_are_the_budget_and_stack_conditions() {
        let vm = [
            TerminalKind::InstructionLimit,
            TerminalKind::AllocationLimit,
            TerminalKind::OutputLimit,
            TerminalKind::NativeStackExhausted,
        ];
        for kind in vm {
            assert!(kind.is_vm_kind(), "{kind:?}");
        }
        for kind in [
            TerminalKind::TimeLimit,
            TerminalKind::MemoryLimit,
            TerminalKind::WorkerFailure,
        ] {
            assert!(!kind.is_vm_kind(), "{kind:?}");
        }
    }
}
