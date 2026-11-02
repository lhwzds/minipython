// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Modules/_sre/sre.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Backtracking matcher.
//!
//! Matching follows CPython's `re` semantics: leftmost-first alternation,
//! greedy/lazy/possessive repeats, and the same rules for which capture
//! values survive backtracking. A construct that may backtrack runs its
//! continuation as a nested *context*; contexts live on an explicit stack
//! (never the native stack), so pattern size and input length cannot exhaust
//! native recursion. Work is metered: every executed operation and every
//! scanned character counts as a step charged to the VM instruction budget,
//! and stack growth beyond a small allowance is charged to the allocation
//! budget.

use super::compiler::{At, Charset, Class, Fold, Op, Program, is_word};
use super::parser::{MAXREPEAT, RepeatKind};
use super::unicode::{ascii_lower, ascii_upper, lower};

/// An unset capture mark.
pub(super) const NONE: usize = usize::MAX;

/// Matcher steps per charged VM instruction.
pub(super) const STEPS_PER_INSTRUCTION: u64 = 8;
/// Steps accumulated before they are charged.
const STEP_BATCH: u64 = 4096;
/// Matcher stack memory that is not charged to the allocation budget.
const FREE_STACK_BYTES: usize = 64 * 1024;

/// Subject text: bytes, ASCII `str`, or `str` as code points.
pub(super) trait Text {
    fn at(&self, index: usize) -> u32;
}

impl Text for [u8] {
    #[inline(always)]
    fn at(&self, index: usize) -> u32 {
        u32::from(self[index])
    }
}

impl Text for [u32] {
    #[inline(always)]
    fn at(&self, index: usize) -> u32 {
        self[index]
    }
}

/// Charges matcher work to the VM budgets.
pub(super) trait Meter {
    fn charge_steps(&mut self, steps: u64) -> Result<(), String>;
    fn charge_memory(&mut self, bytes: usize) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resume {
    None,
    Branch,
    RepeatOne,
    MinRepeatOne,
    Repeat,
    MaxUntilMin,
    MaxUntilMore,
    MaxUntilTail,
    MinUntilMin,
    MinUntilTail,
    MinUntilMore,
    PossessiveMin,
    PossessiveMore,
    Atomic,
    Assert,
    AssertNot,
}

/// A matching context: where it is in the program and the input, plus the
/// state it restores when a nested context it started returns.
#[derive(Clone, Copy, Debug)]
struct Frame {
    resume: Resume,
    toplevel: bool,
    pushed: bool,
    pc: u32,
    ptr: usize,
    count: isize,
    lastmark: isize,
    lastindex: isize,
    /// Branch alternative, repeat context, or saved position.
    aux: usize,
    aux2: usize,
}

impl Frame {
    fn new(pc: u32, ptr: usize, toplevel: bool) -> Frame {
        Frame {
            resume: Resume::None,
            toplevel,
            pushed: false,
            pc,
            ptr,
            count: 0,
            lastmark: -1,
            lastindex: -1,
            aux: 0,
            aux2: 0,
        }
    }
}

/// A repeat in progress (`REPEAT` ... `UNTIL`).
#[derive(Clone, Copy, Debug)]
struct Rep {
    count: isize,
    /// The `Repeat` operation, or `u32::MAX` for a possessive repeat's
    /// placeholder.
    pc: u32,
    prev: Option<usize>,
    last_ptr: usize,
}

pub(super) struct Matcher<'a, T: Text + ?Sized> {
    prog: &'a Program,
    text: &'a T,
    /// End of the searched slice (`endpos`).
    pub end: usize,
    /// Start of the current attempt.
    pub start: usize,
    /// End of the last successful (sub)match.
    pub ptr: usize,
    pub marks: Vec<usize>,
    pub lastmark: isize,
    pub lastindex: isize,
    pub match_all: bool,
    pub must_advance: bool,
    repeat: Option<usize>,
    reps: Vec<Rep>,
    data: Vec<usize>,
    frames: Vec<Frame>,
    steps: u64,
    charged_bytes: usize,
}

enum Flow {
    /// Run the context in `ctx`.
    Dispatch,
    /// Return this result to the suspended parent context.
    Return(bool),
}

impl<'a, T: Text + ?Sized> Matcher<'a, T> {
    pub(super) fn new(prog: &'a Program, text: &'a T, end: usize) -> Self {
        Matcher {
            prog,
            text,
            end,
            start: 0,
            ptr: 0,
            marks: vec![NONE; prog.groups * 2],
            lastmark: -1,
            lastindex: -1,
            match_all: false,
            must_advance: false,
            repeat: None,
            reps: Vec::new(),
            data: Vec::new(),
            frames: Vec::new(),
            steps: 0,
            charged_bytes: 0,
        }
    }

    fn reset(&mut self) {
        self.lastmark = -1;
        self.lastindex = -1;
        self.repeat = None;
        self.reps.clear();
        self.data.clear();
        self.frames.clear();
    }

    /// Charges accumulated steps.
    pub(super) fn flush(&mut self, meter: &mut dyn Meter) -> Result<(), String> {
        let steps = self.steps;
        self.steps = 0;
        meter.charge_steps(steps.div_ceil(STEPS_PER_INSTRUCTION))
    }

    #[inline]
    fn tick(&mut self, steps: u64, meter: &mut dyn Meter) -> Result<(), String> {
        self.steps += steps;
        if self.steps >= STEP_BATCH {
            let charged = self.steps / STEPS_PER_INSTRUCTION;
            self.steps -= charged * STEPS_PER_INSTRUCTION;
            meter.charge_steps(charged)?;
        }
        Ok(())
    }

    fn stack_bytes(&self) -> usize {
        self.frames.capacity() * std::mem::size_of::<Frame>()
            + self.data.capacity() * std::mem::size_of::<usize>()
            + self.reps.capacity() * std::mem::size_of::<Rep>()
    }

    fn charge_stack(&mut self, meter: &mut dyn Meter) -> Result<(), String> {
        let bytes = self.stack_bytes().saturating_sub(FREE_STACK_BYTES);
        if bytes > self.charged_bytes {
            meter.charge_memory(bytes - self.charged_bytes)?;
            self.charged_bytes = bytes;
        }
        Ok(())
    }

    fn push_frame(&mut self, frame: Frame, meter: &mut dyn Meter) -> Result<(), String> {
        if self.frames.len() == self.frames.capacity() {
            self.frames.reserve(self.frames.len().max(16));
            self.charge_stack(meter)?;
        }
        self.frames.push(frame);
        Ok(())
    }

    fn push_rep(&mut self, rep: Rep, meter: &mut dyn Meter) -> Result<usize, String> {
        if self.reps.len() == self.reps.capacity() {
            self.reps.reserve(self.reps.len().max(8));
            self.charge_stack(meter)?;
        }
        self.reps.push(rep);
        Ok(self.reps.len() - 1)
    }

    fn mark_push(&mut self, lastmark: isize, meter: &mut dyn Meter) -> Result<(), String> {
        if lastmark >= 0 {
            let count = lastmark as usize + 1;
            if self.data.len() + count > self.data.capacity() {
                self.data.reserve((self.data.len() + count).max(64));
                self.charge_stack(meter)?;
            }
            self.data.extend_from_slice(&self.marks[..count]);
        }
        Ok(())
    }

    fn mark_pop(&mut self, lastmark: isize) {
        if lastmark >= 0 {
            let count = lastmark as usize + 1;
            let base = self.data.len() - count;
            self.marks[..count].copy_from_slice(&self.data[base..]);
            self.data.truncate(base);
        }
    }

    fn mark_pop_keep(&mut self, lastmark: isize) {
        if lastmark >= 0 {
            let count = lastmark as usize + 1;
            let base = self.data.len() - count;
            self.marks[..count].copy_from_slice(&self.data[base..]);
        }
    }

    fn mark_pop_discard(&mut self, lastmark: isize) {
        if lastmark >= 0 {
            let count = lastmark as usize + 1;
            self.data.truncate(self.data.len() - count);
        }
    }

    #[inline]
    fn word_at(&self, class: Class, index: usize) -> bool {
        is_word(class, self.text.at(index))
    }

    fn at(&self, code: At, ptr: usize) -> bool {
        let end = self.end;
        match code {
            At::Beginning | At::BeginningString => ptr == 0,
            At::BeginningLine => ptr == 0 || self.text.at(ptr - 1) == u32::from(b'\n'),
            At::End => (ptr + 1 == end && self.text.at(ptr) == u32::from(b'\n')) || ptr == end,
            At::EndLine => ptr == end || (ptr < end && self.text.at(ptr) == u32::from(b'\n')),
            At::EndString => ptr == end,
            At::Boundary(class) => {
                let before = ptr > 0 && self.word_at(class, ptr - 1);
                let here = ptr < end && self.word_at(class, ptr);
                before != here
            }
            At::NonBoundary(class) => {
                let before = ptr > 0 && self.word_at(class, ptr - 1);
                let here = ptr < end && self.word_at(class, ptr);
                before == here
            }
        }
    }

    #[inline]
    fn set_matches(&self, fold: Fold, set: &Charset, ch: u32) -> bool {
        match fold {
            Fold::Exact => set.contains_raw(ch) != set.negate,
            Fold::Ascii => set.contains_raw(ascii_lower(ch)) != set.negate,
            Fold::Unicode => set.contains_raw(lower(ch)) != set.negate,
            // The C locale: the lowercase form is tested, then a different
            // uppercase form, each against the whole set including its
            // negation (so `[^b-c]` accepts `c` through `C`).
            Fold::Locale => {
                let lo = ascii_lower(ch);
                let up = ascii_upper(ch);
                (set.contains_raw(lo) != set.negate)
                    || (up != lo && set.contains_raw(up) != set.negate)
            }
        }
    }

    /// Whether a single-character operation accepts `ch`.
    #[inline]
    fn char_matches(&self, op: &Op, ch: u32) -> bool {
        match *op {
            Op::Any => ch != u32::from(b'\n'),
            Op::AnyAll => true,
            Op::Literal(fold, lit) => fold_eq(fold, ch, lit),
            Op::NotLiteral(fold, lit) => !fold_eq(fold, ch, lit),
            Op::In(fold, set) => self.set_matches(fold, &self.prog.sets[set as usize], ch),
            _ => false,
        }
    }

    /// Number of repetitions of the single-character item at `item`
    /// starting at `ptr`, at most `maxcount`.
    fn count(
        &mut self,
        item: u32,
        ptr: usize,
        maxcount: u32,
        meter: &mut dyn Meter,
    ) -> Result<usize, String> {
        let limit = if maxcount == MAXREPEAT || (maxcount as usize) >= self.end - ptr {
            self.end
        } else {
            ptr + maxcount as usize
        };
        let op = &self.prog.ops[item as usize];
        let mut index = ptr;
        match *op {
            Op::AnyAll => index = limit,
            Op::Literal(Fold::Exact, lit) => {
                while index < limit && self.text.at(index) == lit {
                    index += 1;
                }
            }
            Op::Any => {
                while index < limit && self.text.at(index) != u32::from(b'\n') {
                    index += 1;
                }
            }
            _ => {
                while index < limit && self.char_matches(op, self.text.at(index)) {
                    index += 1;
                }
            }
        }
        self.tick((index - ptr) as u64 + 1, meter)?;
        Ok(index - ptr)
    }

    /// Compares the text at `*ptr` with a group's capture. `*ptr` ends past
    /// the compared characters, including on a mismatch, so the caller can
    /// charge every comparison.
    fn group_matches(&self, fold: Fold, group: u32, ptr: &mut usize) -> bool {
        let groupref = group as isize * 2;
        if groupref >= self.lastmark {
            return false;
        }
        let p = self.marks[groupref as usize];
        let e = self.marks[groupref as usize + 1];
        if p == NONE || e == NONE || e < p {
            return false;
        }
        for source in p..e {
            if *ptr >= self.end {
                return false;
            }
            let (a, b) = (self.text.at(*ptr), self.text.at(source));
            let equal = match fold {
                Fold::Exact => a == b,
                Fold::Ascii | Fold::Locale => ascii_lower(a) == ascii_lower(b),
                Fold::Unicode => lower(a) == lower(b),
            };
            *ptr += 1;
            if !equal {
                return false;
            }
        }
        true
    }

    fn group_exists(&self, group: u32) -> bool {
        let groupref = group as isize * 2;
        if groupref >= self.lastmark {
            return false;
        }
        let p = self.marks[groupref as usize];
        let e = self.marks[groupref as usize + 1];
        !(p == NONE || e == NONE || e < p)
    }

    /// Matches at `self.ptr` (the attempt starts at `self.start`).
    pub(super) fn run(&mut self, toplevel: bool, meter: &mut dyn Meter) -> Result<bool, String> {
        self.repeat = None;
        self.reps.clear();
        self.data.clear();
        self.frames.clear();
        let prog = self.prog;
        let ops = &prog.ops[..];
        let mut ctx = Frame::new(0, self.ptr, toplevel);
        let mut flow = Flow::Dispatch;
        loop {
            let ret = match flow {
                Flow::Dispatch => self.dispatch(&mut ctx, ops, meter)?,
                Flow::Return(ret) => Some(ret),
            };
            match ret {
                // `dispatch` started a nested context in `ctx`.
                None => {
                    flow = Flow::Dispatch;
                    continue;
                }
                Some(ret) => {
                    let Some(parent) = self.frames.pop() else {
                        return Ok(ret);
                    };
                    ctx = parent;
                    flow = self.resume(&mut ctx, ret, ops, meter)?;
                }
            }
        }
    }

    /// Suspends `ctx` (to resume at `resume`) and starts a nested context
    /// at `pc` from `self.ptr`.
    #[inline]
    fn call(
        &mut self,
        ctx: &mut Frame,
        resume: Resume,
        pc: u32,
        toplevel: bool,
        meter: &mut dyn Meter,
    ) -> Result<(), String> {
        ctx.resume = resume;
        self.push_frame(*ctx, meter)?;
        *ctx = Frame::new(pc, self.ptr, toplevel);
        Ok(())
    }

    /// Runs operations in `ctx` until it returns (`Some(result)`) or starts
    /// a nested context (`None`).
    fn dispatch(
        &mut self,
        ctx: &mut Frame,
        ops: &[Op],
        meter: &mut dyn Meter,
    ) -> Result<Option<bool>, String> {
        let end = self.end;
        loop {
            self.tick(1, meter)?;
            let pc = ctx.pc as usize;
            let ptr = ctx.ptr;
            match &ops[pc] {
                Op::Failure => return Ok(Some(false)),
                Op::Success => {
                    if ctx.toplevel
                        && ((self.match_all && ptr != end)
                            || (self.must_advance && ptr == self.start))
                    {
                        return Ok(Some(false));
                    }
                    self.ptr = ptr;
                    return Ok(Some(true));
                }
                Op::At(code) => {
                    if !self.at(*code, ptr) {
                        return Ok(Some(false));
                    }
                    ctx.pc += 1;
                }
                op @ (Op::Any | Op::AnyAll | Op::Literal(..) | Op::NotLiteral(..) | Op::In(..)) => {
                    if ptr >= end || !self.char_matches(op, self.text.at(ptr)) {
                        return Ok(Some(false));
                    }
                    ctx.ptr += 1;
                    ctx.pc += 1;
                }
                Op::Mark(index) => {
                    let index = *index as isize;
                    if index & 1 == 1 {
                        self.lastindex = index / 2 + 1;
                    }
                    if index > self.lastmark {
                        for j in (self.lastmark + 1)..index {
                            self.marks[j as usize] = NONE;
                        }
                        self.lastmark = index;
                    }
                    self.marks[index as usize] = ptr;
                    ctx.pc += 1;
                }
                Op::Jump(target) => ctx.pc = *target,
                Op::Branch(alternatives) => {
                    ctx.lastmark = self.lastmark;
                    ctx.lastindex = self.lastindex;
                    ctx.pushed = self.repeat.is_some();
                    if ctx.pushed {
                        self.mark_push(ctx.lastmark, meter)?;
                    }
                    let Some(index) = self.next_alternative(ops, alternatives, 0, ptr) else {
                        if ctx.pushed {
                            self.mark_pop_discard(ctx.lastmark);
                        }
                        return Ok(Some(false));
                    };
                    ctx.aux = index + 1;
                    self.ptr = ptr;
                    let target = alternatives[index];
                    let toplevel = ctx.toplevel;
                    self.call(ctx, Resume::Branch, target, toplevel, meter)?;
                    return Ok(None);
                }
                Op::RepeatOne {
                    kind,
                    min,
                    max,
                    tail,
                } => {
                    let (kind, min, max, tail) = (*kind, *min, *max, *tail);
                    if ptr > end || (min as usize) > end - ptr {
                        return Ok(Some(false));
                    }
                    self.ptr = ptr;
                    match kind {
                        RepeatKind::Greedy => {
                            let count = self.count(ctx.pc + 1, ptr, max, meter)?;
                            if count < min as usize {
                                return Ok(Some(false));
                            }
                            let ptr = ptr + count;
                            if matches!(ops[tail as usize], Op::Success)
                                && ptr == end
                                && !(ctx.toplevel && self.must_advance && ptr == self.start)
                            {
                                self.ptr = ptr;
                                return Ok(Some(true));
                            }
                            ctx.lastmark = self.lastmark;
                            ctx.lastindex = self.lastindex;
                            ctx.pushed = self.repeat.is_some();
                            if ctx.pushed {
                                self.mark_push(ctx.lastmark, meter)?;
                            }
                            ctx.ptr = ptr;
                            ctx.count = count as isize;
                            return self.repeat_one_next(ctx, ops, meter);
                        }
                        RepeatKind::Lazy => {
                            let mut ptr = ptr;
                            let mut count = 0;
                            if min > 0 {
                                count = self.count(ctx.pc + 1, ptr, min, meter)?;
                                if count < min as usize {
                                    return Ok(Some(false));
                                }
                                ptr += count;
                            }
                            if matches!(ops[tail as usize], Op::Success)
                                && !(ctx.toplevel
                                    && ((self.match_all && ptr != end)
                                        || (self.must_advance && ptr == self.start)))
                            {
                                self.ptr = ptr;
                                return Ok(Some(true));
                            }
                            ctx.lastmark = self.lastmark;
                            ctx.lastindex = self.lastindex;
                            ctx.pushed = self.repeat.is_some();
                            if ctx.pushed {
                                self.mark_push(ctx.lastmark, meter)?;
                            }
                            ctx.ptr = ptr;
                            ctx.count = count as isize;
                            return self.min_repeat_one_next(ctx, ops, meter);
                        }
                        RepeatKind::Possessive => {
                            let count = self.count(ctx.pc + 1, ptr, max, meter)?;
                            if count < min as usize {
                                return Ok(Some(false));
                            }
                            ctx.ptr = ptr + count;
                            ctx.pc = tail;
                        }
                    }
                }
                Op::Repeat { until, .. } => {
                    let until = *until;
                    let rep = Rep {
                        count: -1,
                        pc: ctx.pc,
                        prev: self.repeat,
                        last_ptr: NONE,
                    };
                    let index = self.push_rep(rep, meter)?;
                    self.repeat = Some(index);
                    self.ptr = ptr;
                    ctx.aux = index;
                    let toplevel = ctx.toplevel;
                    self.call(ctx, Resume::Repeat, until, toplevel, meter)?;
                    return Ok(None);
                }
                Op::Until { lazy } => {
                    let lazy = *lazy;
                    let Some(rep) = self.repeat else {
                        return Err(
                            "RuntimeError: internal error in regular expression engine".to_string()
                        );
                    };
                    self.ptr = ptr;
                    let count = self.reps[rep].count + 1;
                    ctx.count = count;
                    ctx.aux = rep;
                    let (min, max, body) = self.repeat_bounds(rep);
                    let toplevel = ctx.toplevel;
                    if count < min as isize {
                        self.reps[rep].count = count;
                        let resume = if lazy {
                            Resume::MinUntilMin
                        } else {
                            Resume::MaxUntilMin
                        };
                        self.call(ctx, resume, body, toplevel, meter)?;
                        return Ok(None);
                    }
                    if lazy {
                        // Try the tail first.
                        let prev = self.reps[rep].prev;
                        self.repeat = prev;
                        ctx.lastmark = self.lastmark;
                        ctx.lastindex = self.lastindex;
                        ctx.pushed = prev.is_some();
                        if ctx.pushed {
                            self.mark_push(ctx.lastmark, meter)?;
                        }
                        self.call(ctx, Resume::MinUntilTail, ctx.pc + 1, toplevel, meter)?;
                        return Ok(None);
                    }
                    if (count < max as isize || max == MAXREPEAT)
                        && self.ptr != self.reps[rep].last_ptr
                    {
                        self.reps[rep].count = count;
                        ctx.lastmark = self.lastmark;
                        ctx.lastindex = self.lastindex;
                        self.mark_push(ctx.lastmark, meter)?;
                        ctx.aux2 = self.reps[rep].last_ptr;
                        self.reps[rep].last_ptr = self.ptr;
                        self.call(ctx, Resume::MaxUntilMore, body, toplevel, meter)?;
                        return Ok(None);
                    }
                    return self.max_until_tail(ctx, meter).map(|()| None);
                }
                Op::PossessiveRepeat { .. } => {
                    self.ptr = ptr;
                    let rep = Rep {
                        count: -1,
                        pc: u32::MAX,
                        prev: self.repeat,
                        last_ptr: NONE,
                    };
                    let index = self.push_rep(rep, meter)?;
                    self.repeat = Some(index);
                    ctx.aux = index;
                    ctx.aux2 = NONE;
                    ctx.count = 0;
                    return self.possessive_next(ctx, ops, meter);
                }
                Op::Atomic { .. } => {
                    self.ptr = ptr;
                    let body = ctx.pc + 1;
                    self.call(ctx, Resume::Atomic, body, false, meter)?;
                    return Ok(None);
                }
                Op::Assert { negate, back, tail } => {
                    let (negate, back, tail) = (*negate, *back, *tail);
                    let body = ctx.pc + 1;
                    if !negate {
                        if ptr < back {
                            return Ok(Some(false));
                        }
                        self.ptr = ptr - back;
                        self.call(ctx, Resume::Assert, body, false, meter)?;
                        return Ok(None);
                    }
                    if ptr >= back {
                        self.ptr = ptr - back;
                        ctx.lastmark = self.lastmark;
                        ctx.lastindex = self.lastindex;
                        ctx.pushed = self.repeat.is_some();
                        if ctx.pushed {
                            self.mark_push(ctx.lastmark, meter)?;
                        }
                        self.call(ctx, Resume::AssertNot, body, false, meter)?;
                        return Ok(None);
                    }
                    ctx.pc = tail;
                }
                Op::GroupRef(fold, group) => {
                    let mut index = ptr;
                    let matched = self.group_matches(*fold, *group, &mut index);
                    // Failed comparisons cost as much as successful ones.
                    self.tick((index - ptr) as u64, meter)?;
                    if !matched {
                        return Ok(Some(false));
                    }
                    ctx.ptr = index;
                    ctx.pc += 1;
                }
                Op::GroupRefExists { group, no } => {
                    if self.group_exists(*group) {
                        ctx.pc += 1;
                    } else {
                        ctx.pc = *no;
                    }
                }
            }
        }
    }

    /// The first alternative from `from` whose first operation does not
    /// fail outright at `ptr`. Skipping the others saves starting a nested
    /// context that would only fail and restore the marks it saved.
    #[inline]
    fn next_alternative(
        &self,
        ops: &[Op],
        alternatives: &[u32],
        from: usize,
        ptr: usize,
    ) -> Option<usize> {
        (from..alternatives.len()).find(|&index| {
            let op = &ops[alternatives[index] as usize];
            match op {
                Op::Literal(..) | Op::In(..) => {
                    ptr < self.end && self.char_matches(op, self.text.at(ptr))
                }
                _ => true,
            }
        })
    }

    fn repeat_bounds(&self, rep: usize) -> (u32, u32, u32) {
        let pc = self.reps[rep].pc;
        match self.prog.ops[pc as usize] {
            Op::Repeat { min, max, .. } => (min, max, pc + 1),
            _ => unreachable!("repeat contexts point at their Repeat operation"),
        }
    }

    /// The next attempt of a greedy single-character repeat at `ctx.count`.
    fn repeat_one_next(
        &mut self,
        ctx: &mut Frame,
        ops: &[Op],
        meter: &mut dyn Meter,
    ) -> Result<Option<bool>, String> {
        let Op::RepeatOne { min, tail, .. } = ops[ctx.pc as usize] else {
            unreachable!();
        };
        let min = min as isize;
        if let Op::Literal(Fold::Exact, lit) = ops[tail as usize] {
            // Skip positions where the tail's first literal cannot match.
            while ctx.count >= min && (ctx.ptr >= self.end || self.text.at(ctx.ptr) != lit) {
                if ctx.count == min {
                    ctx.count -= 1;
                    break;
                }
                ctx.ptr -= 1;
                ctx.count -= 1;
            }
            self.tick(1, meter)?;
        }
        if ctx.count < min {
            if ctx.pushed {
                self.mark_pop_discard(ctx.lastmark);
            }
            return Ok(Some(false));
        }
        self.ptr = ctx.ptr;
        let toplevel = ctx.toplevel;
        self.call(ctx, Resume::RepeatOne, tail, toplevel, meter)?;
        Ok(None)
    }

    /// The next attempt of a lazy single-character repeat at `ctx.count`.
    fn min_repeat_one_next(
        &mut self,
        ctx: &mut Frame,
        ops: &[Op],
        meter: &mut dyn Meter,
    ) -> Result<Option<bool>, String> {
        let Op::RepeatOne { max, tail, .. } = ops[ctx.pc as usize] else {
            unreachable!();
        };
        if max == MAXREPEAT || ctx.count <= max as isize {
            self.ptr = ctx.ptr;
            let toplevel = ctx.toplevel;
            self.call(ctx, Resume::MinRepeatOne, tail, toplevel, meter)?;
            return Ok(None);
        }
        if ctx.pushed {
            self.mark_pop_discard(ctx.lastmark);
        }
        Ok(Some(false))
    }

    /// The tail of a greedy general repeat (`MAX_UNTIL` giving up on more
    /// iterations).
    fn max_until_tail(&mut self, ctx: &mut Frame, meter: &mut dyn Meter) -> Result<(), String> {
        let rep = ctx.aux;
        self.repeat = self.reps[rep].prev;
        let toplevel = ctx.toplevel;
        let tail = ctx.pc + 1;
        self.call(ctx, Resume::MaxUntilTail, tail, toplevel, meter)
    }

    /// The next iteration of a possessive repeat, or its tail.
    fn possessive_next(
        &mut self,
        ctx: &mut Frame,
        ops: &[Op],
        meter: &mut dyn Meter,
    ) -> Result<Option<bool>, String> {
        let Op::PossessiveRepeat { min, max, tail } = ops[ctx.pc as usize] else {
            unreachable!();
        };
        let body = ctx.pc + 1;
        if ctx.count < min as isize {
            self.call(ctx, Resume::PossessiveMin, body, false, meter)?;
            return Ok(None);
        }
        // `aux2` holds where the previous iteration started.
        if (ctx.count < max as isize || max == MAXREPEAT) && self.ptr != ctx.aux2 {
            ctx.lastmark = self.lastmark;
            ctx.lastindex = self.lastindex;
            self.mark_push(ctx.lastmark, meter)?;
            ctx.aux2 = self.ptr;
            self.call(ctx, Resume::PossessiveMore, body, false, meter)?;
            return Ok(None);
        }
        // Done: drop the placeholder repeat and match the tail here.
        self.repeat = self.reps[ctx.aux].prev;
        self.reps.pop();
        ctx.pc = tail;
        ctx.ptr = self.ptr;
        Ok(None)
    }

    /// Continues `ctx` after a nested context returned `ret`.
    fn resume(
        &mut self,
        ctx: &mut Frame,
        ret: bool,
        ops: &[Op],
        meter: &mut dyn Meter,
    ) -> Result<Flow, String> {
        let flow = |result: Option<bool>| match result {
            None => Flow::Dispatch,
            Some(ret) => Flow::Return(ret),
        };
        match ctx.resume {
            Resume::None => unreachable!("suspended contexts record where to resume"),
            Resume::Branch => {
                if ret {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(true));
                }
                if ctx.pushed {
                    self.mark_pop_keep(ctx.lastmark);
                }
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                let Op::Branch(alternatives) = &ops[ctx.pc as usize] else {
                    unreachable!();
                };
                if let Some(index) = self.next_alternative(ops, alternatives, ctx.aux, ctx.ptr) {
                    self.ptr = ctx.ptr;
                    let target = alternatives[index];
                    ctx.aux = index + 1;
                    let toplevel = ctx.toplevel;
                    self.call(ctx, Resume::Branch, target, toplevel, meter)?;
                    return Ok(Flow::Dispatch);
                }
                if ctx.pushed {
                    self.mark_pop_discard(ctx.lastmark);
                }
                Ok(Flow::Return(false))
            }
            Resume::RepeatOne => {
                if ret {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(true));
                }
                if ctx.pushed {
                    self.mark_pop_keep(ctx.lastmark);
                }
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                let Op::RepeatOne { min, .. } = ops[ctx.pc as usize] else {
                    unreachable!();
                };
                if ctx.count <= min as isize {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(false));
                }
                ctx.ptr -= 1;
                ctx.count -= 1;
                Ok(flow(self.repeat_one_next(ctx, ops, meter)?))
            }
            Resume::MinRepeatOne => {
                if ret {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(true));
                }
                if ctx.pushed {
                    self.mark_pop_keep(ctx.lastmark);
                }
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                self.ptr = ctx.ptr;
                let counted = if ctx.ptr < self.end {
                    self.count(ctx.pc + 1, ctx.ptr, 1, meter)?
                } else {
                    0
                };
                if counted == 0 {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(false));
                }
                ctx.ptr += 1;
                ctx.count += 1;
                Ok(flow(self.min_repeat_one_next(ctx, ops, meter)?))
            }
            Resume::Repeat => {
                let rep = ctx.aux;
                self.repeat = self.reps[rep].prev;
                self.reps.truncate(rep);
                Ok(Flow::Return(ret))
            }
            Resume::MaxUntilMin | Resume::MinUntilMin => {
                if ret {
                    return Ok(Flow::Return(true));
                }
                self.reps[ctx.aux].count = ctx.count - 1;
                self.ptr = ctx.ptr;
                Ok(Flow::Return(false))
            }
            Resume::MaxUntilMore => {
                let rep = ctx.aux;
                self.reps[rep].last_ptr = ctx.aux2;
                if ret {
                    self.mark_pop_discard(ctx.lastmark);
                    return Ok(Flow::Return(true));
                }
                self.mark_pop(ctx.lastmark);
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                self.reps[rep].count = ctx.count - 1;
                self.ptr = ctx.ptr;
                self.max_until_tail(ctx, meter)?;
                Ok(Flow::Dispatch)
            }
            Resume::MaxUntilTail => {
                self.repeat = Some(ctx.aux);
                if ret {
                    return Ok(Flow::Return(true));
                }
                self.ptr = ctx.ptr;
                Ok(Flow::Return(false))
            }
            Resume::MinUntilTail => {
                let rep = ctx.aux;
                self.repeat = Some(rep);
                if ret {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(true));
                }
                if ctx.pushed {
                    self.mark_pop(ctx.lastmark);
                }
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                self.ptr = ctx.ptr;
                let (_, max, body) = self.repeat_bounds(rep);
                if (ctx.count >= max as isize && max != MAXREPEAT)
                    || self.ptr == self.reps[rep].last_ptr
                {
                    return Ok(Flow::Return(false));
                }
                self.reps[rep].count = ctx.count;
                ctx.aux2 = self.reps[rep].last_ptr;
                self.reps[rep].last_ptr = self.ptr;
                let toplevel = ctx.toplevel;
                self.call(ctx, Resume::MinUntilMore, body, toplevel, meter)?;
                Ok(Flow::Dispatch)
            }
            Resume::MinUntilMore => {
                let rep = ctx.aux;
                self.reps[rep].last_ptr = ctx.aux2;
                if ret {
                    return Ok(Flow::Return(true));
                }
                self.reps[rep].count = ctx.count - 1;
                self.ptr = ctx.ptr;
                Ok(Flow::Return(false))
            }
            Resume::PossessiveMin => {
                if !ret {
                    self.repeat = self.reps[ctx.aux].prev;
                    self.reps.pop();
                    self.ptr = ctx.ptr;
                    return Ok(Flow::Return(false));
                }
                ctx.count += 1;
                ctx.aux2 = NONE;
                Ok(flow(self.possessive_next(ctx, ops, meter)?))
            }
            Resume::PossessiveMore => {
                if ret {
                    self.mark_pop_discard(ctx.lastmark);
                    ctx.count += 1;
                    return Ok(flow(self.possessive_next(ctx, ops, meter)?));
                }
                self.mark_pop(ctx.lastmark);
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                self.ptr = ctx.aux2;
                // Stop iterating: the tail follows.
                self.repeat = self.reps[ctx.aux].prev;
                self.reps.pop();
                let Op::PossessiveRepeat { tail, .. } = ops[ctx.pc as usize] else {
                    unreachable!();
                };
                ctx.pc = tail;
                ctx.ptr = self.ptr;
                Ok(Flow::Dispatch)
            }
            Resume::Atomic => {
                if !ret {
                    self.ptr = ctx.ptr;
                    return Ok(Flow::Return(false));
                }
                let Op::Atomic { tail } = ops[ctx.pc as usize] else {
                    unreachable!();
                };
                ctx.pc = tail;
                ctx.ptr = self.ptr;
                Ok(Flow::Dispatch)
            }
            Resume::Assert => {
                if !ret {
                    return Ok(Flow::Return(false));
                }
                let Op::Assert { tail, .. } = ops[ctx.pc as usize] else {
                    unreachable!();
                };
                ctx.pc = tail;
                Ok(Flow::Dispatch)
            }
            Resume::AssertNot => {
                if ret {
                    if ctx.pushed {
                        self.mark_pop_discard(ctx.lastmark);
                    }
                    return Ok(Flow::Return(false));
                }
                if ctx.pushed {
                    self.mark_pop(ctx.lastmark);
                }
                self.lastmark = ctx.lastmark;
                self.lastindex = ctx.lastindex;
                let Op::Assert { tail, .. } = ops[ctx.pc as usize] else {
                    unreachable!();
                };
                ctx.pc = tail;
                Ok(Flow::Dispatch)
            }
        }
    }

    /// `Pattern.match`/`fullmatch` at `pos`.
    pub(super) fn match_at(&mut self, pos: usize, meter: &mut dyn Meter) -> Result<bool, String> {
        self.reset();
        self.start = pos;
        self.ptr = pos;
        let result = self.run(true, meter);
        self.flush(meter)?;
        result
    }

    /// Searches from `pos` (`self.must_advance` applies to the first
    /// attempt only).
    pub(super) fn search(&mut self, pos: usize, meter: &mut dyn Meter) -> Result<bool, String> {
        self.reset();
        let end = self.end;
        let mut ptr = pos;
        if ptr > end {
            return Ok(false);
        }
        self.start = ptr;
        self.ptr = ptr;
        let mut status = self.run(true, meter)?;
        self.must_advance = false;
        if !status
            && matches!(
                self.prog.ops.first(),
                Some(Op::At(At::Beginning | At::BeginningString))
            )
        {
            self.start = end;
            self.ptr = end;
            self.flush(meter)?;
            return Ok(false);
        }
        let first = self.prog.first_chars.as_deref();
        while !status && ptr < end {
            ptr += 1;
            // Skip starts where no match can begin: every match starts with
            // a character accepted by one of `first`.
            if let Some(first) = first {
                let scan_from = ptr;
                while ptr < end {
                    let ch = self.text.at(ptr);
                    if first
                        .iter()
                        .any(|&pc| self.char_matches(&self.prog.ops[pc as usize], ch))
                    {
                        break;
                    }
                    ptr += 1;
                }
                self.tick((ptr - scan_from) as u64, meter)?;
                if ptr >= end {
                    break;
                }
            }
            self.lastmark = -1;
            self.lastindex = -1;
            self.start = ptr;
            self.ptr = ptr;
            status = self.run(false, meter)?;
        }
        self.flush(meter)?;
        Ok(status)
    }

    /// Spans of groups 1.. (`(NONE, NONE)` when unset) after a match.
    pub(super) fn group_spans(&self) -> Result<Vec<(usize, usize)>, String> {
        let mut spans = Vec::with_capacity(self.prog.groups);
        for group in 0..self.prog.groups {
            let j = group * 2;
            if (j as isize) + 1 <= self.lastmark
                && self.marks[j] != NONE
                && self.marks[j + 1] != NONE
            {
                let (start, end) = (self.marks[j], self.marks[j + 1]);
                if start > end {
                    return Err("SystemError: The span of capturing group is wrong, please report a bug for the re module.".to_string());
                }
                spans.push((start, end));
            } else {
                spans.push((NONE, NONE));
            }
        }
        Ok(spans)
    }
}

#[inline]
fn fold_eq(fold: Fold, ch: u32, lit: u32) -> bool {
    match fold {
        Fold::Exact => ch == lit,
        Fold::Ascii => ascii_lower(ch) == lit,
        Fold::Unicode => lower(ch) == lit,
        Fold::Locale => ch == lit || ascii_lower(ch) == lit || ascii_upper(ch) == lit,
    }
}
