// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/re/_compiler.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Compiles a parsed pattern into a matcher program.
//!
//! The program is a flat list of operations addressed by index. Constructs
//! that backtrack (branches, repeats, lookarounds) carry the indexes of
//! their bodies and tails; the matcher in `engine.rs` runs them with an
//! explicit stack.

use super::engine::{Meter, STEPS_PER_INSTRUCTION};
use super::flags::{ASCII, DOTALL, IGNORECASE, LOCALE, MULTILINE, TYPE_FLAGS, UNICODE};
use super::parser::{
    AtCode, Category, MAXREPEAT, Node, ReError, RepeatKind, Seq, SetItem, seq_width,
};
use super::unicode::{ascii_is_cased, ascii_lower, extra_cases, lower, unicode_is_cased};

/// Largest look-behind width (`_compiler.MAXCODE`).
const MAXCODE: u128 = (1 << 32) - 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum At {
    Beginning,
    BeginningLine,
    BeginningString,
    End,
    EndLine,
    EndString,
    Boundary(Class),
    NonBoundary(Class),
}

/// Which definition of `\d`, `\s`, `\w`, and word boundaries applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Class {
    Ascii,
    Unicode,
    /// The C locale: ASCII semantics for bytes.
    Locale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Cat {
    Digit(Class),
    NotDigit(Class),
    Space(Class),
    NotSpace(Class),
    Word(Class),
    NotWord(Class),
}

/// How a character is folded before a literal, set, or back-reference
/// comparison.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Fold {
    Exact,
    /// ASCII lowercase (bytes, or `str` with `re.ASCII`).
    Ascii,
    /// Unicode lowercase.
    Unicode,
    /// The C locale for bytes with `re.LOCALE`: a character matches when it,
    /// its lowercase, or its uppercase does.
    Locale,
}

/// A character set: code points (as sorted, disjoint ranges plus a bitmap
/// for Latin-1), categories, and ranges compared case-insensitively.
#[derive(Clone, Debug, Default)]
pub(super) struct Charset {
    pub negate: bool,
    pub bitmap: [u64; 4],
    pub ranges: Vec<(u32, u32)>,
    pub categories: Vec<Cat>,
    /// Ranges that also accept a character whose uppercase is inside.
    pub uni_ignore_ranges: Vec<(u32, u32)>,
}

impl Charset {
    #[inline]
    pub(super) fn contains_raw(&self, ch: u32) -> bool {
        if ch < 256 {
            if self.bitmap[(ch >> 6) as usize] & (1 << (ch & 63)) != 0 {
                return true;
            }
        } else if self.ranges_contain(ch) {
            return true;
        }
        if !self.categories.is_empty()
            && self.categories.iter().any(|&cat| category_matches(cat, ch))
        {
            return true;
        }
        if !self.uni_ignore_ranges.is_empty() {
            let upper = super::unicode::upper(ch);
            return self
                .uni_ignore_ranges
                .iter()
                .any(|&(lo, hi)| (lo..=hi).contains(&ch) || (lo..=hi).contains(&upper));
        }
        false
    }

    fn ranges_contain(&self, ch: u32) -> bool {
        let index = self.ranges.partition_point(|&(_, hi)| hi < ch);
        self.ranges.get(index).is_some_and(|&(lo, _)| lo <= ch)
    }

    fn finish(&mut self, mut points: Vec<(u32, u32)>) {
        points.sort_unstable();
        let mut merged: Vec<(u32, u32)> = Vec::with_capacity(points.len());
        for (lo, hi) in points {
            if let Some(last) = merged.last_mut()
                && lo <= last.1.saturating_add(1)
            {
                last.1 = last.1.max(hi);
                continue;
            }
            merged.push((lo, hi));
        }
        for &(lo, hi) in &merged {
            for ch in lo..=hi.min(255) {
                self.bitmap[(ch >> 6) as usize] |= 1 << (ch & 63);
            }
        }
        self.ranges = merged;
    }
}

pub(super) fn category_matches(cat: Cat, ch: u32) -> bool {
    match cat {
        Cat::Digit(class) => is_digit(class, ch),
        Cat::NotDigit(class) => !is_digit(class, ch),
        Cat::Space(class) => is_space(class, ch),
        Cat::NotSpace(class) => !is_space(class, ch),
        Cat::Word(class) => is_word(class, ch),
        Cat::NotWord(class) => !is_word(class, ch),
    }
}

#[inline]
fn is_digit(class: Class, ch: u32) -> bool {
    match class {
        Class::Unicode => super::unicode::uni_is_digit(ch),
        Class::Ascii | Class::Locale => super::unicode::ascii_is_digit(ch),
    }
}

#[inline]
fn is_space(class: Class, ch: u32) -> bool {
    match class {
        Class::Unicode => super::unicode::uni_is_space(ch),
        Class::Ascii | Class::Locale => super::unicode::ascii_is_space(ch),
    }
}

#[inline]
pub(super) fn is_word(class: Class, ch: u32) -> bool {
    match class {
        Class::Unicode => super::unicode::uni_is_word(ch),
        Class::Ascii | Class::Locale => super::unicode::ascii_is_word(ch),
    }
}

#[derive(Clone, Debug)]
pub(super) enum Op {
    Failure,
    Success,
    Any,
    AnyAll,
    At(At),
    Literal(Fold, u32),
    NotLiteral(Fold, u32),
    In(Fold, u32),
    Mark(u32),
    Jump(u32),
    /// Alternatives start at the listed indexes; each ends with a jump past
    /// the branch.
    Branch(Box<[u32]>),
    /// A repeat of one single-character item at `pc + 1` (followed by
    /// `Success`); the tail starts at `tail`.
    RepeatOne {
        kind: RepeatKind,
        min: u32,
        max: u32,
        tail: u32,
    },
    /// A general repeat whose body starts at `pc + 1` and ends at the
    /// `Until` operation at `until`.
    Repeat {
        min: u32,
        max: u32,
        until: u32,
    },
    Until {
        lazy: bool,
    },
    /// A possessive repeat whose body starts at `pc + 1` and ends with
    /// `Success`; the tail starts at `tail`.
    PossessiveRepeat {
        min: u32,
        max: u32,
        tail: u32,
    },
    Atomic {
        tail: u32,
    },
    Assert {
        negate: bool,
        back: usize,
        tail: u32,
    },
    GroupRef(Fold, u32),
    /// Continues at `pc + 1` when the group matched, else at `no`.
    GroupRefExists {
        group: u32,
        no: u32,
    },
}

pub(super) struct Program {
    pub ops: Vec<Op>,
    pub sets: Vec<Charset>,
    /// Number of capturing groups (excluding group 0).
    pub groups: usize,
    /// Single-character operations one of which accepts the first
    /// character of every match, when that is known; searches skip other
    /// start positions.
    pub first_chars: Option<Vec<u32>>,
}

impl Program {
    /// Approximate heap size, charged to the allocation budget.
    pub(super) fn heap_size(&self) -> usize {
        let sets: usize = self
            .sets
            .iter()
            .map(|set| {
                std::mem::size_of::<Charset>()
                    + set.ranges.len() * 8
                    + set.categories.len() * 2
                    + set.uni_ignore_ranges.len() * 8
            })
            .sum();
        self.ops.len() * std::mem::size_of::<Op>() + sets
    }
}

struct Compiler<'a> {
    ops: Vec<Op>,
    sets: Vec<Charset>,
    group_widths: &'a [Option<(u128, u128)>],
    meter: &'a mut dyn Meter,
    /// Compilation work not yet charged, in matcher steps.
    work: u64,
}

/// Compiles parsed `seq` with the final `flags`.
pub(super) fn compile(
    seq: &Seq,
    flags: u32,
    groups: usize,
    group_widths: &[Option<(u128, u128)>],
    meter: &mut dyn Meter,
) -> Result<Program, ReError> {
    let mut compiler = Compiler {
        ops: Vec::new(),
        sets: Vec::new(),
        group_widths,
        meter,
        work: 0,
    };
    compiler.compile_seq(seq, flags)?;
    compiler.charge(0, true)?;
    compiler.ops.push(Op::Success);
    let mut first = Vec::new();
    let first_chars = first_char_ops(&compiler.ops, 0, 0, &mut first).then_some(first);
    Ok(Program {
        ops: compiler.ops,
        sets: compiler.sets,
        groups: groups - 1,
        first_chars,
    })
}

/// Collects the single-character operations that can consume the first
/// character of any match starting at `pc`; false when a match may start
/// some other way (empty, anchored, or through a back-reference).
fn first_char_ops(ops: &[Op], pc: u32, depth: usize, out: &mut Vec<u32>) -> bool {
    if depth > 8 || out.len() > 16 {
        return false;
    }
    match &ops[pc as usize] {
        Op::Mark(_) => first_char_ops(ops, pc + 1, depth + 1, out),
        Op::Any | Op::AnyAll | Op::Literal(..) | Op::NotLiteral(..) | Op::In(..) => {
            out.push(pc);
            true
        }
        Op::Branch(alternatives) => alternatives
            .iter()
            .all(|&start| first_char_ops(ops, start, depth + 1, out)),
        Op::RepeatOne { min, .. } if *min > 0 => first_char_ops(ops, pc + 1, depth + 1, out),
        Op::Repeat { min, .. } | Op::PossessiveRepeat { min, .. } if *min > 0 => {
            first_char_ops(ops, pc + 1, depth + 1, out)
        }
        Op::Atomic { .. } => first_char_ops(ops, pc + 1, depth + 1, out),
        _ => false,
    }
}

fn combine_flags(flags: u32, add: u32, del: u32) -> u32 {
    let flags = if add & TYPE_FLAGS != 0 {
        flags & !TYPE_FLAGS
    } else {
        flags
    };
    (flags | add) & !del
}

/// A repeat body of one single-character item.
fn is_simple(seq: &Seq) -> bool {
    if seq.len() != 1 {
        return false;
    }
    match &seq[0] {
        Node::Subpattern {
            group: None, body, ..
        } => is_simple(body),
        Node::Literal(_) | Node::NotLiteral(_) | Node::Any | Node::In(_) => true,
        _ => false,
    }
}

fn class_for(flags: u32) -> Class {
    if flags & LOCALE != 0 {
        Class::Locale
    } else if flags & UNICODE != 0 {
        Class::Unicode
    } else {
        Class::Ascii
    }
}

/// The case folding of `flags` without `re.LOCALE` (`None` when case
/// sensitive), with its `iscased`, `tolower`, and extra cases.
#[derive(Clone, Copy, PartialEq)]
enum Fixup {
    None,
    Ascii,
    Unicode,
}

fn fixup_for(flags: u32) -> Fixup {
    if flags & IGNORECASE == 0 || flags & LOCALE != 0 {
        Fixup::None
    } else if flags & UNICODE != 0 {
        Fixup::Unicode
    } else {
        Fixup::Ascii
    }
}

impl Fixup {
    fn is_cased(self, ch: u32) -> bool {
        match self {
            Fixup::None => false,
            Fixup::Ascii => ascii_is_cased(ch),
            Fixup::Unicode => unicode_is_cased(ch),
        }
    }

    fn lower(self, ch: u32) -> u32 {
        match self {
            Fixup::None => ch,
            Fixup::Ascii => ascii_lower(ch),
            Fixup::Unicode => lower(ch),
        }
    }

    fn extra(self, ch: u32) -> &'static [u32] {
        match self {
            Fixup::Unicode => extra_cases(ch),
            _ => &[],
        }
    }
}

fn map_category(category: Category, class: Class) -> Cat {
    match category {
        Category::Digit => Cat::Digit(class),
        Category::NotDigit => Cat::NotDigit(class),
        Category::Space => Cat::Space(class),
        Category::NotSpace => Cat::NotSpace(class),
        Category::Word => Cat::Word(class),
        Category::NotWord => Cat::NotWord(class),
    }
}

impl Compiler<'_> {
    /// Charges compilation work (set expansion is proportional to range
    /// sizes, not pattern length) to the instruction budget like matching.
    fn charge(&mut self, steps: u64, flush: bool) -> Result<(), ReError> {
        self.work += steps;
        if self.work >= 4096 || flush {
            let instructions = self.work.div_ceil(STEPS_PER_INSTRUCTION);
            self.work = 0;
            self.meter
                .charge_steps(instructions)
                .map_err(ReError::Other)?;
        }
        Ok(())
    }

    fn pc(&self) -> u32 {
        self.ops.len() as u32
    }

    fn add_set(&mut self, set: Charset) -> u32 {
        self.sets.push(set);
        (self.sets.len() - 1) as u32
    }

    fn compile_seq(&mut self, seq: &Seq, flags: u32) -> Result<(), ReError> {
        crate::stack::check_native_recursion(" while compiling a regular expression")
            .map_err(ReError::Other)?;
        for node in seq {
            self.charge(1, false)?;
            self.compile_node(node, flags)?;
        }
        Ok(())
    }

    fn compile_literal(&mut self, ch: u32, negate: bool, flags: u32) {
        let fixup = fixup_for(flags);
        let op = if flags & IGNORECASE == 0 {
            (Fold::Exact, ch)
        } else if flags & LOCALE != 0 {
            (Fold::Locale, ch)
        } else if !fixup.is_cased(ch) {
            (Fold::Exact, ch)
        } else {
            let lo = fixup.lower(ch);
            let extra = fixup.extra(lo);
            if fixup == Fixup::Ascii {
                (Fold::Ascii, lo)
            } else if extra.is_empty() {
                (Fold::Unicode, lo)
            } else {
                let mut set = Charset {
                    negate,
                    ..Charset::default()
                };
                let mut points = vec![(lo, lo)];
                points.extend(extra.iter().map(|&k| (k, k)));
                set.finish(points);
                let id = self.add_set(set);
                self.ops.push(Op::In(Fold::Unicode, id));
                return;
            }
        };
        self.ops.push(if negate {
            Op::NotLiteral(op.0, op.1)
        } else {
            Op::Literal(op.0, op.1)
        });
    }

    fn compile_set(&mut self, items: &[SetItem], flags: u32) -> Result<(), ReError> {
        let fixup = fixup_for(flags);
        let class = class_for(flags);
        let mut set = Charset::default();
        let mut points: Vec<(u32, u32)> = Vec::new();
        let mut has_cased = false;
        for item in items {
            match *item {
                SetItem::Negate => set.negate = true,
                SetItem::Category(category) => set.categories.push(map_category(category, class)),
                SetItem::Literal(ch) => {
                    if fixup == Fixup::None {
                        points.push((ch, ch));
                        continue;
                    }
                    let lo = fixup.lower(ch);
                    points.push((lo, lo));
                    points.extend(fixup.extra(lo).iter().map(|&k| (k, k)));
                    if fixup.is_cased(ch) || lo > 0xffff {
                        has_cased = true;
                    }
                }
                SetItem::Range(lo, hi) => {
                    if fixup == Fixup::None {
                        points.push((lo, hi));
                        continue;
                    }
                    self.charge(u64::from(hi.min(0xffff).saturating_sub(lo)), false)?;
                    for ch in lo..=hi.min(0xffff) {
                        let folded = fixup.lower(ch);
                        points.push((folded, folded));
                        points.extend(fixup.extra(folded).iter().map(|&k| (k, k)));
                        if !has_cased && fixup.is_cased(ch) {
                            has_cased = true;
                        }
                    }
                    if hi > 0xffff {
                        has_cased = true;
                        match fixup {
                            Fixup::Unicode => set.uni_ignore_ranges.push((lo, hi)),
                            _ => points.push((lo, hi)),
                        }
                    }
                }
            }
        }
        set.finish(points);
        let fold = if flags & IGNORECASE != 0 && flags & LOCALE != 0 {
            Fold::Locale
        } else if !has_cased {
            Fold::Exact
        } else if fixup == Fixup::Ascii {
            Fold::Ascii
        } else {
            Fold::Unicode
        };
        let id = self.add_set(set);
        self.ops.push(Op::In(fold, id));
        Ok(())
    }

    fn compile_node(&mut self, node: &Node, flags: u32) -> Result<(), ReError> {
        match node {
            Node::Literal(ch) => self.compile_literal(*ch, false, flags),
            Node::NotLiteral(ch) => self.compile_literal(*ch, true, flags),
            Node::In(items) => self.compile_set(items, flags)?,
            Node::Any => self.ops.push(if flags & DOTALL != 0 {
                Op::AnyAll
            } else {
                Op::Any
            }),
            Node::At(code) => {
                let class = class_for(flags);
                let multiline = flags & MULTILINE != 0;
                self.ops.push(Op::At(match code {
                    AtCode::Beginning if multiline => At::BeginningLine,
                    AtCode::Beginning => At::Beginning,
                    AtCode::BeginningString => At::BeginningString,
                    AtCode::End if multiline => At::EndLine,
                    AtCode::End => At::End,
                    AtCode::EndString => At::EndString,
                    AtCode::Boundary => At::Boundary(class),
                    AtCode::NonBoundary => At::NonBoundary(class),
                }));
            }
            Node::Repeat {
                kind,
                min,
                max,
                body,
            } => {
                if is_simple(body) {
                    let at = self.ops.len();
                    self.ops.push(Op::Failure);
                    self.compile_seq(body, flags)?;
                    self.ops.push(Op::Success);
                    self.ops[at] = Op::RepeatOne {
                        kind: *kind,
                        min: *min,
                        max: *max,
                        tail: self.pc(),
                    };
                } else if *kind == RepeatKind::Possessive {
                    let at = self.ops.len();
                    self.ops.push(Op::Failure);
                    self.compile_seq(body, flags)?;
                    self.ops.push(Op::Success);
                    self.ops[at] = Op::PossessiveRepeat {
                        min: *min,
                        max: *max,
                        tail: self.pc(),
                    };
                } else {
                    let at = self.ops.len();
                    self.ops.push(Op::Failure);
                    self.compile_seq(body, flags)?;
                    self.ops[at] = Op::Repeat {
                        min: *min,
                        max: *max,
                        until: self.pc(),
                    };
                    self.ops.push(Op::Until {
                        lazy: *kind == RepeatKind::Lazy,
                    });
                }
            }
            Node::Subpattern {
                group,
                add_flags,
                del_flags,
                body,
            } => {
                if let Some(group) = group {
                    self.ops.push(Op::Mark(((group - 1) * 2) as u32));
                }
                self.compile_seq(body, combine_flags(flags, *add_flags, *del_flags))?;
                if let Some(group) = group {
                    self.ops.push(Op::Mark(((group - 1) * 2 + 1) as u32));
                }
            }
            Node::Atomic(body) => {
                let at = self.ops.len();
                self.ops.push(Op::Failure);
                self.compile_seq(body, flags)?;
                self.ops.push(Op::Success);
                self.ops[at] = Op::Atomic { tail: self.pc() };
            }
            Node::Assert {
                behind,
                negate,
                body,
            } => {
                let back = if *behind {
                    let (lo, hi) = seq_width(body, self.group_widths)?;
                    if lo > MAXCODE {
                        return Err(ReError::Pattern {
                            msg: "looks too much behind".to_string(),
                            pos: None,
                        });
                    }
                    if lo != hi {
                        return Err(ReError::Pattern {
                            msg: "look-behind requires fixed-width pattern".to_string(),
                            pos: None,
                        });
                    }
                    lo as usize
                } else {
                    0
                };
                let at = self.ops.len();
                self.ops.push(Op::Failure);
                self.compile_seq(body, flags)?;
                self.ops.push(Op::Success);
                self.ops[at] = Op::Assert {
                    negate: *negate,
                    back,
                    tail: self.pc(),
                };
            }
            Node::Branch(items) => {
                let at = self.ops.len();
                self.ops.push(Op::Failure);
                let mut starts = Vec::with_capacity(items.len());
                let mut jumps = Vec::with_capacity(items.len());
                for item in items {
                    starts.push(self.pc());
                    self.compile_seq(item, flags)?;
                    jumps.push(self.ops.len());
                    self.ops.push(Op::Jump(0));
                }
                let end = self.pc();
                for jump in jumps {
                    self.ops[jump] = Op::Jump(end);
                }
                self.ops[at] = Op::Branch(starts.into_boxed_slice());
            }
            Node::GroupRef(group) => {
                let fold = if flags & IGNORECASE == 0 {
                    Fold::Exact
                } else if flags & LOCALE != 0 {
                    Fold::Locale
                } else if flags & UNICODE == 0 {
                    Fold::Ascii
                } else {
                    Fold::Unicode
                };
                self.ops.push(Op::GroupRef(fold, (*group - 1) as u32));
            }
            Node::GroupRefExists { group, yes, no } => {
                let at = self.ops.len();
                self.ops.push(Op::Failure);
                self.compile_seq(yes, flags)?;
                if let Some(no) = no {
                    let jump = self.ops.len();
                    self.ops.push(Op::Jump(0));
                    let no_start = self.pc();
                    self.compile_seq(no, flags)?;
                    let end = self.pc();
                    self.ops[jump] = Op::Jump(end);
                    self.ops[at] = Op::GroupRefExists {
                        group: (*group - 1) as u32,
                        no: no_start,
                    };
                } else {
                    self.ops[at] = Op::GroupRefExists {
                        group: (*group - 1) as u32,
                        no: self.pc(),
                    };
                }
            }
            Node::Failure => self.ops.push(Op::Failure),
        }
        let _ = (ASCII, MAXREPEAT);
        Ok(())
    }
}
