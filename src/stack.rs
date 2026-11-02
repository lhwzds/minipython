// SPDX-License-Identifier: MIT

//! Native stack ownership for Python execution.
//!
//! Every entry point that lexes, parses, compiles, or runs Python source does
//! so on a dedicated thread whose stack size is chosen here, never inherited
//! from the caller or from `RUST_MIN_STACK`. The thread records its stack
//! bounds when it starts, so recursive interpreter paths can ask how much
//! native stack remains and turn exhaustion into a Python exception instead
//! of a host stack overflow.
//!
//! The guard mirrors CPython 3.14, which checks the machine stack against a
//! soft limit (raise `RecursionError: Stack overflow (used N kB)` with a
//! context such as ` in comparison`) and a hard limit (fatal error), separately
//! from its Python-frame recursion limit. The margins are tiered so the most
//! specific error wins: native recursion that CPython guards with a context
//! (repr, str, comparison, JSON, compilation) trips first; entering a Python
//! frame trips next with the context-free message; paths that cannot fail
//! record the error for the next instruction; the space below is headroom in
//! which the error propagates and `except` handlers run; the hard margin ends
//! execution with an uncatchable sandbox error.

use std::cell::{Cell, RefCell};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

const MIB: usize = 1024 * 1024;

/// Stack reserved for in-process execution when no process memory limit
/// applies. Thread stacks are reserved address space; pages are committed only
/// when recursion actually reaches them.
pub(crate) const VM_STACK_SIZE: usize = 256 * MIB;

/// Smallest stack worth running Python on. Below this a refused reservation
/// is reported instead of retried.
const MIN_VM_STACK_SIZE: usize = 4 * MIB;

/// Share of a worker's process memory limit given to the interpreter stack.
/// Linux counts the whole reservation against `RLIMIT_AS`/`RLIMIT_DATA`, and
/// macOS counts touched stack pages in the polled physical footprint, so the
/// stack must leave most of the limit to the heap on both.
const VM_STACK_MEMORY_DIVISOR: u64 = 4;

/// Below this much remaining stack, native recursion that CPython guards with
/// a context raises `RecursionError: Stack overflow (used N kB)<context>`. It
/// must exceed the native stack used between two checks; debug builds use far
/// larger frames.
const NATIVE_MARGIN: usize = if cfg!(debug_assertions) { 4 * MIB } else { MIB };
/// Below this, entering a Python frame raises `RecursionError: Stack overflow
/// (used N kB)`, CPython's report when its C stack runs out.
const PYTHON_FRAME_MARGIN: usize = NATIVE_MARGIN / 8 * 7;
/// Below this, paths that cannot fail (`PartialEq`, `Display`) stop and record
/// the error for the next instruction boundary.
const DEFERRED_MARGIN: usize = NATIVE_MARGIN / 8 * 5;
/// Below this, execution stops with an uncatchable sandbox error.
const HARD_MARGIN: usize = NATIVE_MARGIN / 2;

/// Conservative allowance for the thread start-up frames above the point
/// where the stack bounds are recorded.
const ENTRY_SLACK: usize = 64 * 1024;

thread_local! {
    /// Highest usable stack address recorded at VM thread entry; zero when the
    /// current thread was not started by [`run_on_vm_stack`].
    static STACK_TOP: Cell<usize> = const { Cell::new(0) };
    /// Lowest usable stack address for the current VM thread.
    static STACK_LIMIT: Cell<usize> = const { Cell::new(0) };
    /// Set when a path that cannot fail stopped at the guard; the next
    /// instruction boundary turns it into an exception.
    static DEFERRED_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Heap memory held by the live Python frames of the current VM thread
    /// (their register files and per-frame structures), as estimated by the
    /// interpreter.
    static FRAME_HEAP: Cell<usize> = const { Cell::new(0) };
}

/// Number of threads holding a deferred error. The interpreter polls this
/// before every instruction, so it is a plain global load rather than a
/// thread-local access; a nonzero value only means "look at your own slot".
static DEFERRED_ERRORS: AtomicUsize = AtomicUsize::new(0);

#[inline(always)]
fn stack_pointer() -> usize {
    let marker = 0u8;
    std::ptr::addr_of!(marker) as usize
}

/// Remaining native stack and the amount used so far, or `None` on a thread
/// not started by [`run_on_vm_stack`].
#[inline]
fn stack_position() -> Option<(usize, usize)> {
    let remaining = remaining_stack()?;
    let used = STACK_TOP.with(Cell::get).saturating_sub(stack_pointer());
    Some((remaining, used))
}

/// Remaining native stack, or `None` on a thread not started by
/// [`run_on_vm_stack`].
#[inline(always)]
fn remaining_stack() -> Option<usize> {
    let limit = STACK_LIMIT.with(Cell::get);
    (limit != 0).then(|| stack_pointer().saturating_sub(limit))
}

/// The terminal error used when even the recursion headroom is consumed.
/// Reaching the hard ceiling latches the condition (see `crate::terminal`),
/// so the execution ends even if a caller discards this error.
pub(crate) fn stack_exhausted_error() -> String {
    crate::terminal::trip(crate::terminal::TerminalKind::NativeStackExhausted)
}

/// Whether native recursion should stop here, for best-effort walkers (such
/// as syntax warnings) that skip work instead of reporting an error.
#[inline]
pub(crate) fn stack_is_low() -> bool {
    remaining_stack().is_some_and(|remaining| remaining < NATIVE_MARGIN)
}

/// Guards entry into a Python frame. CPython 3.14 runs Python-to-Python
/// calls without C recursion, so only the recursion limit stops them there;
/// when the interpreter stack runs out first here, the error is CPython's
/// report for an exhausted C stack.
#[inline]
pub(crate) fn check_python_recursion() -> Result<(), String> {
    match native_recursion_error("", PYTHON_FRAME_MARGIN) {
        None => Ok(()),
        Some(error) => Err(report(error)),
    }
}

/// Guards one level of a recursive native path that CPython protects with its
/// C stack check. `context` is CPython's suffix, such as
/// `" while getting the repr of an object"`.
#[inline]
pub(crate) fn check_native_recursion(context: &str) -> Result<(), String> {
    match native_recursion_error(context, NATIVE_MARGIN) {
        None => Ok(()),
        Some(error) => Err(report(error)),
    }
}

/// CPython's error when its parser runs out of stack on deeply nested source.
pub(crate) const PARSER_STACK_OVERFLOW: &str =
    "MemoryError: Parser stack overflowed - Python source too complex to parse";

/// [`check_native_recursion`] for the parser, which CPython reports as a
/// `MemoryError` when its own stack overflows.
#[inline]
pub(crate) fn check_parser_recursion() -> Result<(), String> {
    match remaining_stack() {
        Some(remaining) if remaining < HARD_MARGIN => Err(report(stack_exhausted_error())),
        Some(remaining) if remaining < NATIVE_MARGIN => {
            Err(report(PARSER_STACK_OVERFLOW.to_string()))
        }
        _ => Ok(()),
    }
}

/// Guards one level of a recursive path that cannot return an error. Returns
/// `true` when the caller must stop recursing and return a placeholder; the
/// error is recorded for [`take_deferred_recursion_error`].
#[inline]
pub(crate) fn native_recursion_exhausted(context: &str) -> bool {
    let Some(error) = native_recursion_error(context, DEFERRED_MARGIN) else {
        return false;
    };
    DEFERRED_ERROR.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_none() {
            DEFERRED_ERRORS.fetch_add(1, Ordering::Relaxed);
            *slot = Some(error);
        }
    });
    true
}

/// Takes the error recorded by [`native_recursion_exhausted`], if any.
#[inline(always)]
pub(crate) fn take_deferred_recursion_error() -> Option<String> {
    if DEFERRED_ERRORS.load(Ordering::Relaxed) == 0 {
        return None;
    }
    take_own_deferred_error()
}

#[inline(never)]
fn take_own_deferred_error() -> Option<String> {
    let error = DEFERRED_ERROR.with(|slot| slot.borrow_mut().take());
    if error.is_some() {
        DEFERRED_ERRORS.fetch_sub(1, Ordering::Relaxed);
    }
    error
}

#[inline]
fn native_recursion_error(context: &str, margin: usize) -> Option<String> {
    match remaining_stack() {
        Some(remaining) if remaining < HARD_MARGIN => Some(stack_exhausted_error()),
        Some(remaining) if remaining < margin => Some(overflow_error(context)),
        _ => None,
    }
}

#[cold]
fn overflow_error(context: &str) -> String {
    let used = stack_position().map_or(0, |(_, used)| used);
    format!(
        "RecursionError: Stack overflow (used {} kB){context}",
        used / 1024
    )
}

/// A guard that reports its own error subsumes one recorded earlier in the
/// same overflow, so the program sees a single `RecursionError`.
fn report(error: String) -> String {
    take_deferred_recursion_error();
    error
}

/// Heap memory the live Python frames of a VM thread may hold, as a multiple
/// of its native stack. A worker's stack is a quarter of its process memory
/// limit, so frames get half of it; every frame also uses native stack, so
/// the two together stay well inside the limit.
const FRAME_HEAP_STACK_MULTIPLE: usize = 2;

/// The frame heap allowance of the current VM thread; unlimited on a thread
/// not started by [`run_on_vm_stack`].
fn frame_heap_allowance() -> usize {
    let limit = STACK_LIMIT.with(Cell::get);
    if limit == 0 {
        return usize::MAX;
    }
    STACK_TOP
        .with(Cell::get)
        .saturating_sub(limit)
        .saturating_mul(FRAME_HEAP_STACK_MULTIPLE)
}

/// Records `bytes` of heap for a Python frame being entered. Fails with
/// CPython's exhausted-stack `RecursionError` when the live frames would
/// exceed their allowance, so deep recursion through large frames is
/// catchable instead of exhausting the worker's memory.
pub(crate) fn enter_python_frame_heap(bytes: usize) -> Result<(), String> {
    let held = FRAME_HEAP.with(Cell::get);
    let total = held.saturating_add(bytes);
    if total > frame_heap_allowance() {
        let native = stack_position().map_or(0, |(_, used)| used);
        return Err(report(format!(
            "RecursionError: Stack overflow (used {} kB)",
            native.saturating_add(held) / 1024
        )));
    }
    FRAME_HEAP.with(|cell| cell.set(total));
    Ok(())
}

/// Records `bytes` of heap a running frame grew by. Returns `true` when the
/// live frames now exceed their allowance; the caller then charges the
/// growth to the allocation budget, since a running frame cannot raise.
pub(crate) fn grow_python_frame_heap(bytes: usize) -> bool {
    let total = FRAME_HEAP.with(Cell::get).saturating_add(bytes);
    FRAME_HEAP.with(|cell| cell.set(total));
    total > frame_heap_allowance()
}

/// Releases heap recorded for a frame that returned.
pub(crate) fn leave_python_frame_heap(bytes: usize) {
    FRAME_HEAP.with(|cell| cell.set(cell.get().saturating_sub(bytes)));
}

/// Stack size for a worker whose process memory is limited to `bytes`.
pub(crate) fn vm_stack_size_for_memory_limit(bytes: u64) -> usize {
    let share = usize::try_from(bytes / VM_STACK_MEMORY_DIVISOR).unwrap_or(usize::MAX);
    share.clamp(MIN_VM_STACK_SIZE, VM_STACK_SIZE)
}

/// Runs `body` on a thread with an explicitly sized stack and returns its
/// result, propagating panics. When the current thread is already a VM
/// thread, `body` runs in place. If the host refuses the reservation (for
/// example under a tight address-space limit), smaller stacks are tried down
/// to [`MIN_VM_STACK_SIZE`]; `body` never runs on an unmeasured stack.
pub(crate) fn run_on_vm_stack<F, R>(stack_size: usize, body: F) -> Result<R, String>
where
    F: FnOnce() -> R + Send,
    R: Send,
{
    if STACK_LIMIT.with(Cell::get) != 0 {
        return Ok(body());
    }
    // The body waits in a slot so a refused spawn leaves it available for the
    // next, smaller attempt.
    let slot = Mutex::new(Some(body));
    let mut size = stack_size.max(MIN_VM_STACK_SIZE);
    loop {
        let spawned = thread::scope(|scope| {
            let slot = &slot;
            let handle = thread::Builder::new()
                .name("minipython-vm".to_string())
                .stack_size(size)
                .spawn_scoped(scope, move || {
                    enter_vm_stack(size);
                    let body = slot
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .take()
                        .expect("VM body runs once");
                    body()
                })
                .ok()?;
            Some(match handle.join() {
                Ok(value) => value,
                Err(payload) => std::panic::resume_unwind(payload),
            })
        });
        if let Some(value) = spawned {
            return Ok(value);
        }
        if size <= MIN_VM_STACK_SIZE {
            return Err(format!(
                "sandbox error: cannot reserve a {size}-byte interpreter stack"
            ));
        }
        size = (size / 2).max(MIN_VM_STACK_SIZE);
    }
}

fn enter_vm_stack(size: usize) {
    let top = stack_pointer();
    let usable = size.saturating_sub(ENTRY_SLACK);
    STACK_TOP.with(|cell| cell.set(top));
    STACK_LIMIT.with(|cell| cell.set(top.saturating_sub(usable).max(1)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn margins_are_ordered_so_contextual_errors_trip_first() {
        assert!(NATIVE_MARGIN > PYTHON_FRAME_MARGIN);
        assert!(PYTHON_FRAME_MARGIN > DEFERRED_MARGIN);
        assert!(DEFERRED_MARGIN > HARD_MARGIN);
        assert!(HARD_MARGIN > 0);
    }

    #[test]
    fn worker_stack_is_a_quarter_of_the_memory_limit_within_bounds() {
        assert_eq!(vm_stack_size_for_memory_limit(256 * MIB as u64), 64 * MIB);
        assert_eq!(
            vm_stack_size_for_memory_limit(MIB as u64),
            MIN_VM_STACK_SIZE
        );
        assert_eq!(vm_stack_size_for_memory_limit(u64::MAX), VM_STACK_SIZE);
    }

    #[test]
    fn vm_threads_know_their_stack_and_nest_in_place() {
        assert!(stack_position().is_none());
        let depth = run_on_vm_stack(8 * MIB, || {
            let (remaining, used) = stack_position().expect("VM thread records its stack");
            assert!(remaining > 7 * MIB && used < MIB);
            run_on_vm_stack(8 * MIB, || stack_position().map(|(_, used)| used))
                .expect("nested call runs in place")
        })
        .expect("VM thread spawns");
        assert!(depth.is_some_and(|used| used < MIB));
    }

    #[test]
    fn deep_native_recursion_stops_at_the_guard() {
        fn recurse(depth: usize) -> Result<usize, String> {
            check_native_recursion(" in test")?;
            let padding = [depth as u8; 512];
            recurse(depth + 1).map(|reached| reached.max(padding[0] as usize))
        }
        let error = run_on_vm_stack(MIN_VM_STACK_SIZE, || recurse(0))
            .expect("VM thread spawns")
            .expect_err("recursion is unbounded");
        assert!(
            error.starts_with("RecursionError: Stack overflow (used ")
                && error.ends_with(" kB) in test"),
            "{error}"
        );
    }

    #[test]
    fn a_reported_error_discards_a_deferred_one() {
        run_on_vm_stack(MIN_VM_STACK_SIZE, || {
            DEFERRED_ERROR.with(|slot| *slot.borrow_mut() = Some("RecursionError: x".into()));
            DEFERRED_ERRORS.fetch_add(1, Ordering::Relaxed);
            assert_eq!(report("RecursionError: y".into()), "RecursionError: y");
            assert_eq!(take_deferred_recursion_error(), None);
        })
        .expect("VM thread spawns");
    }
}
