// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_strptime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `strptime` for `date`, `time`, and `datetime` with CPython 3.14's
//! directive set, matching rules, and error messages in the C locale.
//!
//! The format becomes a pattern of named fields matched like a regular
//! expression: case-insensitively, with first-match backtracking, and with
//! every whitespace run matching one or more whitespace characters. Local
//! zone names (`%Z`) are those of the sandbox's UTC zone: `UTC` and `GMT`.

use super::*;
use unicode_general_category::{GeneralCategory, get_general_category};

/// A pattern node.
enum Node {
    /// One character satisfying the predicate.
    Class(fn(char) -> bool),
    /// A literal character, compared case-insensitively.
    Literal(char),
    /// A literal character compared exactly.
    Exact(char),
    Seq(Vec<Node>),
    Alt(Vec<Node>),
    /// Greedy `{min,max}`.
    Repeat(Box<Node>, usize, usize),
    /// A named capture.
    Group(&'static str, Box<Node>),
}

fn is_decimal(ch: char) -> bool {
    ch.is_ascii_digit()
        || (!ch.is_ascii() && get_general_category(ch) == GeneralCategory::DecimalNumber)
}

/// The value of a decimal digit (`\d` matches every Unicode decimal digit).
fn decimal_value(ch: char) -> u32 {
    if ch.is_ascii_digit() {
        return ch as u32 - '0' as u32;
    }
    let mut start = ch as u32;
    while let Some(previous) = start.checked_sub(1).and_then(char::from_u32)
        && get_general_category(previous) == GeneralCategory::DecimalNumber
    {
        start -= 1;
    }
    (ch as u32 - start) % 10
}

fn is_space(ch: char) -> bool {
    ch.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&ch)
}

fn d() -> Node {
    Node::Class(is_decimal)
}

fn range(low: char, high: char) -> Node {
    // Character classes over ASCII digits.
    match (low, high) {
        ('0', '1') => Node::Class(|ch| matches!(ch, '0'..='1')),
        ('0', '2') => Node::Class(|ch| matches!(ch, '0'..='2')),
        ('0', '3') => Node::Class(|ch| matches!(ch, '0'..='3')),
        ('0', '4') => Node::Class(|ch| matches!(ch, '0'..='4')),
        ('0', '5') => Node::Class(|ch| matches!(ch, '0'..='5')),
        ('0', '6') => Node::Class(|ch| matches!(ch, '0'..='6')),
        ('0', '9') => Node::Class(|ch| ch.is_ascii_digit()),
        ('1', '2') => Node::Class(|ch| matches!(ch, '1'..='2')),
        ('1', '4') => Node::Class(|ch| matches!(ch, '1'..='4')),
        ('1', '7') => Node::Class(|ch| matches!(ch, '1'..='7')),
        ('1', '9') => Node::Class(|ch| matches!(ch, '1'..='9')),
        _ => unreachable!("unused character class"),
    }
}

fn lit(text: &str) -> Node {
    let mut nodes = text.chars().map(Node::Literal).collect::<Vec<_>>();
    if nodes.len() == 1 {
        nodes.pop().expect("one node")
    } else {
        Node::Seq(nodes)
    }
}

fn seq(nodes: Vec<Node>) -> Node {
    Node::Seq(nodes)
}

fn alt(nodes: Vec<Node>) -> Node {
    Node::Alt(nodes)
}

fn space() -> Node {
    Node::Repeat(Box::new(Node::Class(is_space)), 1, usize::MAX)
}

fn names(list: &[&str]) -> Node {
    alt(list.iter().map(|name| lit(name)).collect())
}

fn group(name: &'static str, node: Node) -> Node {
    Node::Group(name, Box::new(node))
}

fn day_node() -> Node {
    group(
        "d",
        alt(vec![
            seq(vec![lit("3"), range('0', '1')]),
            seq(vec![range('1', '2'), d()]),
            seq(vec![lit("0"), range('1', '9')]),
            range('1', '9'),
            seq(vec![lit(" "), range('1', '9')]),
        ]),
    )
}

fn hour24_node() -> Node {
    group(
        "H",
        alt(vec![
            seq(vec![lit("2"), range('0', '3')]),
            seq(vec![range('0', '1'), d()]),
            d(),
            seq(vec![lit(" "), d()]),
        ]),
    )
}

fn hour12_node() -> Node {
    group(
        "I",
        alt(vec![
            seq(vec![lit("1"), range('0', '2')]),
            seq(vec![lit("0"), range('1', '9')]),
            range('1', '9'),
            seq(vec![lit(" "), range('1', '9')]),
        ]),
    )
}

fn minute_node() -> Node {
    group("M", alt(vec![seq(vec![range('0', '5'), d()]), d()]))
}

fn second_node() -> Node {
    group(
        "S",
        alt(vec![
            seq(vec![lit("6"), range('0', '1')]),
            seq(vec![range('0', '5'), d()]),
            d(),
        ]),
    )
}

fn month_node() -> Node {
    group(
        "m",
        alt(vec![
            seq(vec![lit("1"), range('0', '2')]),
            seq(vec![lit("0"), range('1', '9')]),
            range('1', '9'),
        ]),
    )
}

fn year4_node() -> Node {
    group("Y", seq(vec![d(), d(), d(), d()]))
}

fn year2_node() -> Node {
    group("y", seq(vec![d(), d()]))
}

const A_WEEKDAY: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
const F_WEEKDAY: [&str; 7] = [
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
    "sunday",
];
const A_MONTH: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
const F_MONTH: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Names sorted longest first (stable), as the alternation tries them.
fn by_length(list: &[&'static str]) -> Vec<&'static str> {
    let mut sorted = list.to_vec();
    sorted.sort_by_key(|name| std::cmp::Reverse(name.len()));
    sorted
}

fn ampm_node() -> Node {
    group("p", names(&["am", "pm"]))
}

/// The pattern of a directive (without `%`), or `None` for a bad
/// directive.
fn directive_node(directive: &str) -> Option<Node> {
    Some(match directive {
        "%" => lit("%"),
        "A" => group("A", names(&by_length(&F_WEEKDAY))),
        "a" => group("a", names(&by_length(&A_WEEKDAY))),
        "B" => group("B", names(&by_length(&F_MONTH))),
        "b" => group("b", names(&by_length(&A_MONTH))),
        "c" => seq(vec![
            directive_node("a")?,
            space(),
            directive_node("b")?,
            space(),
            day_node(),
            space(),
            hour24_node(),
            lit(":"),
            minute_node(),
            lit(":"),
            second_node(),
            space(),
            year4_node(),
        ]),
        "d" | "e" => day_node(),
        "f" => group(
            "f",
            Node::Repeat(Box::new(Node::Class(|ch| ch.is_ascii_digit())), 1, 6),
        ),
        "G" => group("G", seq(vec![d(), d(), d(), d()])),
        "H" | "k" => hour24_node(),
        "I" | "l" => hour12_node(),
        "j" => group(
            "j",
            alt(vec![
                seq(vec![lit("3"), lit("6"), range('0', '6')]),
                seq(vec![lit("3"), range('0', '5'), d()]),
                seq(vec![range('1', '2'), d(), d()]),
                seq(vec![lit("0"), range('1', '9'), d()]),
                seq(vec![lit("0"), lit("0"), range('1', '9')]),
                seq(vec![range('1', '9'), d()]),
                seq(vec![lit("0"), range('1', '9')]),
                range('1', '9'),
            ]),
        ),
        "M" => minute_node(),
        "m" => month_node(),
        "p" | "P" | "Op" => ampm_node(),
        "R" => seq(vec![hour24_node(), lit(":"), minute_node()]),
        "r" => seq(vec![
            hour12_node(),
            lit(":"),
            minute_node(),
            lit(":"),
            second_node(),
            space(),
            ampm_node(),
        ]),
        "S" => second_node(),
        "T" | "X" => seq(vec![
            hour24_node(),
            lit(":"),
            minute_node(),
            lit(":"),
            second_node(),
        ]),
        "U" | "W" => group(
            if directive == "U" { "U" } else { "W" },
            alt(vec![
                seq(vec![lit("5"), range('0', '3')]),
                seq(vec![range('0', '4'), d()]),
                d(),
            ]),
        ),
        "u" => group("u", range('1', '7')),
        "V" => group(
            "V",
            alt(vec![
                seq(vec![lit("5"), range('0', '3')]),
                seq(vec![lit("0"), range('1', '9')]),
                seq(vec![range('1', '4'), d()]),
                d(),
            ]),
        ),
        "w" => group("w", range('0', '6')),
        "x" => seq(vec![
            month_node(),
            lit("/"),
            day_node(),
            lit("/"),
            year2_node(),
        ]),
        "Y" => year4_node(),
        "y" => year2_node(),
        "Z" => group("Z", names(&["utc", "gmt"])),
        "z" => group(
            "z",
            alt(vec![
                seq(vec![
                    Node::Class(|ch| ch == '+' || ch == '-'),
                    d(),
                    d(),
                    Node::Repeat(Box::new(lit(":")), 0, 1),
                    range('0', '5'),
                    d(),
                    Node::Repeat(
                        Box::new(seq(vec![
                            Node::Repeat(Box::new(lit(":")), 0, 1),
                            range('0', '5'),
                            d(),
                            Node::Repeat(
                                Box::new(seq(vec![lit("."), Node::Repeat(Box::new(d()), 1, 6)])),
                                0,
                                1,
                            ),
                        ])),
                        0,
                        1,
                    ),
                ]),
                Node::Exact('Z'),
            ]),
        ),
        "OC" | "OH" | "OI" | "OM" | "OS" | "Od" | "Oe" | "Om" | "Ow" | "Oy" => seq(Vec::new()),
        _ => return None,
    })
}

fn chars_equal_ignore_case(a: char, b: char) -> bool {
    a == b || a.to_lowercase().eq(b.to_lowercase())
}

/// One character test.
#[derive(Clone, Copy)]
enum Single {
    Class(fn(char) -> bool),
    Literal(char),
    Exact(char),
}

impl Single {
    fn of(node: &Node) -> Option<Single> {
        match node {
            Node::Class(predicate) => Some(Single::Class(*predicate)),
            Node::Literal(ch) => Some(Single::Literal(*ch)),
            Node::Exact(ch) => Some(Single::Exact(*ch)),
            _ => None,
        }
    }

    fn matches(self, ch: char) -> bool {
        match self {
            Single::Class(predicate) => predicate(ch),
            Single::Literal(expected) => chars_equal_ignore_case(ch, expected),
            Single::Exact(expected) => ch == expected,
        }
    }
}

/// A matcher instruction. Patterns compile to a flat program run by a
/// backtracking loop over an explicit stack, so neither format nor input
/// length reaches native recursion.
#[derive(Clone, Copy)]
enum Inst {
    One(Single),
    /// A literal run of the format (byte range), compared
    /// case-insensitively.
    Run(u32, u32),
    /// A greedy `{min,max}` repeat of one character test.
    RepeatOne(Single, u32, u32),
    /// Try the first target, backtracking to the second.
    Split(u32, u32),
    Jump(u32),
    /// Record the position in a capture slot.
    Save(u32),
    Match,
}

/// A backtracking point.
#[derive(Clone, Copy)]
enum Backtrack {
    Retry {
        pc: u32,
        position: usize,
    },
    /// A greedy single-character repeat that can still give back
    /// characters down to `floor`.
    Repeat {
        pc: u32,
        floor: usize,
        position: usize,
    },
    Restore {
        slot: u32,
        old: usize,
    },
}

/// An unset capture slot.
const UNSET: usize = usize::MAX;
/// Matcher steps per charged VM instruction (as in `re`).
const STEPS_PER_INSTRUCTION: u64 = 8;
/// Steps accumulated before they are charged.
const STEP_BATCH: u64 = 4096;
/// Matcher memory (program and backtracking stack) not charged to the
/// allocation budget.
const FREE_MATCHER_BYTES: usize = 64 * 1024;
/// Repeat bound standing for "unbounded".
const UNBOUNDED: u32 = u32::MAX;

/// The compiled pattern of a format string.
struct Pattern {
    program: Vec<Inst>,
    /// Group names by capture index, in pattern order.
    groups: Vec<&'static str>,
    year_in_format: bool,
    day_in_format: bool,
}

fn index(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn compile_node(node: &Node, program: &mut Vec<Inst>, groups: &mut Vec<&'static str>) {
    match node {
        Node::Class(_) | Node::Literal(_) | Node::Exact(_) => {
            program.push(Inst::One(Single::of(node).expect("single")));
        }
        Node::Seq(nodes) => {
            for node in nodes {
                compile_node(node, program, groups);
            }
        }
        Node::Alt(options) => {
            let mut exits = Vec::new();
            for (position, option) in options.iter().enumerate() {
                if position + 1 == options.len() {
                    compile_node(option, program, groups);
                    break;
                }
                let split = program.len();
                program.push(Inst::Match);
                compile_node(option, program, groups);
                exits.push(program.len());
                program.push(Inst::Match);
                program[split] = Inst::Split(index(split + 1), index(program.len()));
            }
            let end = index(program.len());
            for exit in exits {
                program[exit] = Inst::Jump(end);
            }
        }
        Node::Repeat(inner, min, max) => {
            let bound = |value: usize| {
                if value == usize::MAX {
                    UNBOUNDED
                } else {
                    index(value)
                }
            };
            if let Some(single) = Single::of(inner) {
                program.push(Inst::RepeatOne(single, bound(*min), bound(*max)));
                return;
            }
            // Every repeated pattern consumes input, so loops terminate.
            for _ in 0..*min {
                compile_node(inner, program, groups);
            }
            if *max == usize::MAX {
                let start = program.len();
                program.push(Inst::Match);
                compile_node(inner, program, groups);
                program.push(Inst::Jump(index(start)));
                program[start] = Inst::Split(index(start + 1), index(program.len()));
            } else {
                let mut splits = Vec::new();
                for _ in *min..*max {
                    splits.push(program.len());
                    program.push(Inst::Match);
                    compile_node(inner, program, groups);
                }
                let end = index(program.len());
                for split in splits {
                    program[split] = Inst::Split(index(split + 1), end);
                }
            }
        }
        Node::Group(name, inner) => {
            let slot = index(groups.len() * 2);
            groups.push(name);
            program.push(Inst::Save(slot));
            compile_node(inner, program, groups);
            program.push(Inst::Save(slot + 1));
        }
    }
}

fn strptime_value_error(message: String) -> String {
    format!("ValueError: {message}")
}

fn build_pattern(format: &str) -> Result<Pattern, String> {
    let mut program = Vec::new();
    let mut groups = Vec::new();
    let mut year_in_format = false;
    let mut day_in_format = false;
    let mut chars = format.char_indices().peekable();
    // The pending literal run, as a byte range of the format.
    let mut run: Option<(usize, usize)> = None;
    let flush = |run: &mut Option<(usize, usize)>, program: &mut Vec<Inst>| {
        if let Some((start, end)) = run.take() {
            program.push(Inst::Run(index(start), index(end - start)));
        }
    };
    while let Some((offset, ch)) = chars.next() {
        if is_space(ch) {
            flush(&mut run, &mut program);
            while chars.next_if(|(_, ch)| is_space(*ch)).is_some() {}
            compile_node(&space(), &mut program, &mut groups);
            continue;
        }
        if ch != '%' {
            let end = offset + ch.len_utf8();
            run = Some(match run {
                Some((start, _)) => (start, end),
                None => (offset, end),
            });
            continue;
        }
        flush(&mut run, &mut program);
        let mut directive = String::new();
        if let Some((_, modifier @ ('O' | 'E'))) = chars.peek().copied() {
            directive.push(modifier);
            chars.next();
        }
        match chars.peek().copied() {
            Some((_, next)) if is_space(next) => {
                return Err(if directive.is_empty() {
                    strptime_value_error(format!("stray % in format {}", quoted(format)))
                } else {
                    strptime_value_error(format!(
                        "'{directive}' is a bad directive in format {}",
                        quoted(format)
                    ))
                });
            }
            Some((_, next)) => {
                directive.push(next);
                chars.next();
            }
            None => {}
        }
        if directive.is_empty() {
            return Err(strptime_value_error(format!(
                "stray % in format {}",
                quoted(format)
            )));
        }
        match directive.as_str() {
            "Y" | "y" | "G" => year_in_format = true,
            "d" => day_in_format = true,
            _ => {}
        }
        match directive_node(&directive) {
            Some(node) => compile_node(&node, &mut program, &mut groups),
            None => {
                return Err(strptime_value_error(format!(
                    "'{directive}' is a bad directive in format {}",
                    quoted(format)
                )));
            }
        }
    }
    flush(&mut run, &mut program);
    program.push(Inst::Match);
    for (position, name) in groups.iter().enumerate() {
        if groups[..position].contains(name) {
            return Err(format!(
                "ValueError: redefinition of group name {}",
                repr_string(name)
            ));
        }
    }
    Ok(Pattern {
        program,
        groups,
        year_in_format,
        day_in_format,
    })
}

/// The character starting at byte `position`.
fn char_at(text: &str, position: usize) -> Option<char> {
    text.get(position..)?.chars().next()
}

/// Matcher work charged to the VM budgets.
struct MatchMeter<'a> {
    vm: &'a Vm,
    steps: u64,
    charged_bytes: usize,
}

impl MatchMeter<'_> {
    fn tick(&mut self, steps: u64) -> Result<(), String> {
        self.steps += steps;
        if self.steps >= STEP_BATCH {
            let charged = self.steps / STEPS_PER_INSTRUCTION;
            self.steps -= charged * STEPS_PER_INSTRUCTION;
            self.vm.consume_instruction_steps(charged)?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        let steps = std::mem::take(&mut self.steps);
        self.vm
            .consume_instruction_steps(steps.div_ceil(STEPS_PER_INSTRUCTION))
    }

    /// Charges matcher memory beyond the free allowance.
    fn memory(&mut self, bytes: usize) -> Result<(), String> {
        let bytes = bytes.saturating_sub(FREE_MATCHER_BYTES);
        if bytes > self.charged_bytes {
            self.vm
                .consume_allocation_budget(bytes - self.charged_bytes)?;
            self.charged_bytes = bytes;
        }
        Ok(())
    }
}

/// Matches `pattern` at the start of `text` (`re.match` semantics: the
/// first success in backtracking order), returning the end and the capture
/// slots.
fn run_pattern(
    pattern: &Pattern,
    format: &str,
    text: &str,
    meter: &mut MatchMeter,
) -> Result<Option<(usize, Vec<usize>)>, String> {
    let program = &pattern.program;
    let program_bytes = program.capacity() * std::mem::size_of::<Inst>();
    meter.memory(program_bytes)?;
    let mut slots = vec![UNSET; pattern.groups.len() * 2];
    let mut stack: Vec<Backtrack> = Vec::new();
    let mut pc = 0usize;
    let mut position = 0usize;
    loop {
        meter.tick(1)?;
        let matched = match program[pc] {
            Inst::One(single) => match char_at(text, position) {
                Some(ch) if single.matches(ch) => {
                    position += ch.len_utf8();
                    pc += 1;
                    true
                }
                _ => false,
            },
            Inst::Run(start, length) => {
                let start = start as usize;
                let literal = &format[start..start + length as usize];
                let mut cursor = position;
                let mut matched = true;
                for expected in literal.chars() {
                    meter.tick(1)?;
                    match char_at(text, cursor) {
                        Some(ch) if chars_equal_ignore_case(ch, expected) => {
                            cursor += ch.len_utf8();
                        }
                        _ => {
                            matched = false;
                            break;
                        }
                    }
                }
                if matched {
                    position = cursor;
                    pc += 1;
                }
                matched
            }
            Inst::RepeatOne(single, min, max) => {
                let mut cursor = position;
                let mut count = 0u32;
                let mut floor = if min == 0 { Some(position) } else { None };
                while count < max {
                    meter.tick(1)?;
                    match char_at(text, cursor) {
                        Some(ch) if single.matches(ch) => {
                            cursor += ch.len_utf8();
                            count += 1;
                            if count == min {
                                floor = Some(cursor);
                            }
                        }
                        _ => break,
                    }
                }
                match floor {
                    Some(floor) => {
                        if cursor > floor {
                            stack.push(Backtrack::Repeat {
                                pc: index(pc + 1),
                                floor,
                                position: cursor,
                            });
                        }
                        position = cursor;
                        pc += 1;
                        true
                    }
                    None => false,
                }
            }
            Inst::Split(first, second) => {
                stack.push(Backtrack::Retry {
                    pc: second,
                    position,
                });
                pc = first as usize;
                true
            }
            Inst::Jump(target) => {
                pc = target as usize;
                true
            }
            Inst::Save(slot) => {
                stack.push(Backtrack::Restore {
                    slot,
                    old: slots[slot as usize],
                });
                slots[slot as usize] = position;
                pc += 1;
                true
            }
            Inst::Match => return Ok(Some((position, slots))),
        };
        if stack.len() == stack.capacity() && !stack.is_empty() {
            // The next push grows the stack: charge the growth first.
            let grown = stack.capacity() * 2 * std::mem::size_of::<Backtrack>();
            meter.memory(program_bytes + grown)?;
            stack.reserve(stack.len());
        }
        if matched {
            continue;
        }
        loop {
            meter.tick(1)?;
            match stack.pop() {
                None => return Ok(None),
                Some(Backtrack::Restore { slot, old }) => slots[slot as usize] = old,
                Some(Backtrack::Retry {
                    pc: next,
                    position: at,
                }) => {
                    pc = next as usize;
                    position = at;
                    break;
                }
                Some(Backtrack::Repeat {
                    pc: next,
                    floor,
                    position: at,
                }) => {
                    let previous = at - text[..at].chars().next_back().map_or(0, char::len_utf8);
                    if previous > floor {
                        stack.push(Backtrack::Repeat {
                            pc: next,
                            floor,
                            position: previous,
                        });
                    }
                    pc = next as usize;
                    position = previous;
                    break;
                }
            }
        }
    }
}

/// `'text'` as `%s` formatting of a quoted format (CPython quotes with
/// plain apostrophes, not `repr`).
fn quoted(text: &str) -> String {
    format!("'{text}'")
}

/// The parsed fields `_strptime` returns.
pub(super) struct Parsed {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub fraction: i64,
    pub tzname: Option<String>,
    pub gmtoff: Option<(i64, i64)>,
}

fn int_of(text: &[char]) -> i64 {
    text.iter()
        .fold(0i64, |total, ch| total * 10 + i64::from(decimal_value(*ch)))
}

/// `_calc_julian_from_U_or_W`.
fn julian_from_week(year: i64, week: i64, weekday: i64, monday_start: bool) -> i64 {
    let mut first_weekday = calendar::weekday(year, 1, 1);
    let mut weekday = weekday;
    if !monday_start {
        first_weekday = (first_weekday + 1) % 7;
        weekday = (weekday + 1) % 7;
    }
    let week0_length = (7 - first_weekday) % 7;
    if week == 0 {
        1 + weekday - first_weekday
    } else {
        1 + week0_length + 7 * (week - 1) + weekday
    }
}

/// Formats CPython keeps compiled before clearing its cache.
const STRPTIME_CACHE_MAX: usize = 5;
/// Hidden attribute of the `date` class holding the compiled-format cache.
const STRPTIME_CACHE: &str = "\0strptime_formats";

impl Vm {
    /// The per-interpreter list of formats `strptime` has compiled.
    fn dt_strptime_cache(&mut self, anchor: &Value) -> Result<ListRef, String> {
        let registry = self.dt_registry(anchor)?;
        let Value::Class { attrs, .. } = &registry.date else {
            return self.dt_error("TypeError", "datetime.date is unavailable");
        };
        let existing = attrs.borrow().get(STRPTIME_CACHE).cloned();
        if let Some(Value::List(items)) = existing {
            return Ok(items);
        }
        let items: ListRef = Rc::new(RefCell::new(Vec::new()));
        attrs
            .borrow_mut()
            .insert(STRPTIME_CACHE.to_string(), Value::List(items.clone()));
        Ok(items)
    }

    /// `_strptime`: the fields of `text` parsed with `format`.
    fn dt_strptime_fields(
        &mut self,
        class: &Value,
        text: &str,
        format: &str,
    ) -> Result<Parsed, String> {
        let pattern = match build_pattern(format) {
            Ok(pattern) => pattern,
            Err(message) => {
                // CPython raises format errors `from None`.
                // The suppressed context is the KeyError for the directive.
                let directive = message
                    .split_once("ValueError: '")
                    .and_then(|(_, rest)| rest.split_once("' is a bad directive"))
                    .map(|(directive, _)| directive.to_string())
                    .unwrap_or_default();
                return match runtime_exception_from_message(&message) {
                    Some(mut exception) => {
                        exception.suppress_context = true;
                        exception.context =
                            Some(Box::new(key_error_exception(Value::String(directive))));
                        Err(self.remember_runtime_exception_error(exception))
                    }
                    None => Err(message),
                };
            }
        };
        // CPython compiles each format once into a small cache (cleared
        // when it holds more than five), and only compiling warns.
        let cache = self.dt_strptime_cache(class)?;
        let cached = {
            let mut formats = cache.borrow_mut();
            if formats.len() > STRPTIME_CACHE_MAX {
                formats.clear();
            }
            formats
                .iter()
                .any(|item| matches!(item, Value::String(known) if known == format))
        };
        if !cached && pattern.day_in_format && !pattern.year_in_format {
            self.dt_deprecation(
                "Parsing dates involving a day of month without a year specified is ambiguous\nand fails to parse leap day. The default behavior will change in Python 3.15\nto either always raise an exception or to use a different default year (TBD).\nTo avoid trouble, add a specific year to the input & format.\nSee https://github.com/python/cpython/issues/70647.",
            )?;
        }
        if !cached {
            cache.borrow_mut().push(Value::String(format.to_string()));
        }
        let mut meter = MatchMeter {
            vm: self,
            steps: 0,
            charged_bytes: 0,
        };
        let found = run_pattern(&pattern, format, text, &mut meter)?;
        meter.flush()?;
        let Some((end, slots)) = found else {
            return Err(strptime_value_error(format!(
                "time data {} does not match format {}",
                repr_string(text),
                repr_string(format)
            )));
        };
        if end != text.len() {
            let rest = &text[end..];
            return Err(strptime_value_error(format!(
                "unconverted data remains: {rest}"
            )));
        }
        // Captured groups in pattern order (the order CPython's
        // `groupdict()` reports them).
        let ordered = pattern
            .groups
            .iter()
            .enumerate()
            .filter_map(|(group, name)| {
                let (start, end) = (slots[group * 2], slots[group * 2 + 1]);
                (start != UNSET && end != UNSET).then_some((*name, start, end))
            })
            .collect::<Vec<_>>();
        let value = |name: &str| {
            ordered
                .iter()
                .find(|(found, ..)| *found == name)
                .map(|(_, start, end)| text[*start..*end].chars().collect::<Vec<_>>())
        };

        let mut year: Option<i64> = None;
        let mut month = 1;
        let mut day = 1;
        let mut hour = 0;
        let mut minute = 0;
        let mut second = 0;
        let mut fraction = 0;
        let mut gmtoff: Option<(i64, i64)> = None;
        let mut iso_year: Option<i64> = None;
        let mut iso_week: Option<i64> = None;
        let mut week_of_year: Option<(i64, bool)> = None;
        let mut weekday: Option<i64> = None;
        let mut julian: Option<i64> = None;
        let lower = |chars: &[char]| chars.iter().collect::<String>().to_lowercase();
        let mut seen = Vec::new();
        for (name, ..) in &ordered {
            if seen.contains(name) {
                continue;
            }
            seen.push(*name);
            let field = value(name).expect("captured");
            match *name {
                "y" => {
                    let short = int_of(&field);
                    year = Some(if short <= 68 {
                        short + 2000
                    } else {
                        short + 1900
                    });
                }
                "Y" => year = Some(int_of(&field)),
                "G" => iso_year = Some(int_of(&field)),
                "m" => month = int_of(&field),
                "B" => {
                    month = F_MONTH
                        .iter()
                        .position(|name| *name == lower(&field))
                        .unwrap_or(0) as i64
                        + 1;
                }
                "b" => {
                    month = A_MONTH
                        .iter()
                        .position(|name| *name == lower(&field))
                        .unwrap_or(0) as i64
                        + 1;
                }
                "d" => day = int_of(&field),
                "H" => hour = int_of(&field),
                "I" => {
                    hour = int_of(&field);
                    let ampm = value("p").map(|field| lower(&field)).unwrap_or_default();
                    if ampm.is_empty() || ampm == "am" {
                        if hour == 12 {
                            hour = 0;
                        }
                    } else if ampm == "pm" && hour != 12 {
                        hour += 12;
                    }
                }
                "M" => minute = int_of(&field),
                "S" => second = int_of(&field),
                "f" => {
                    let mut digits = field.iter().collect::<String>();
                    while digits.len() < 6 {
                        digits.push('0');
                    }
                    fraction = digits.parse::<i64>().unwrap_or(0);
                }
                "A" => {
                    weekday = F_WEEKDAY
                        .iter()
                        .position(|name| *name == lower(&field))
                        .map(|index| index as i64);
                }
                "a" => {
                    weekday = A_WEEKDAY
                        .iter()
                        .position(|name| *name == lower(&field))
                        .map(|index| index as i64);
                }
                "u" => weekday = Some(int_of(&field) - 1),
                "w" => {
                    let value = int_of(&field);
                    weekday = Some(if value == 0 { 6 } else { value - 1 });
                }
                "j" => julian = Some(int_of(&field)),
                "U" => week_of_year = Some((int_of(&field), false)),
                "W" => week_of_year = Some((int_of(&field), true)),
                "V" => iso_week = Some(int_of(&field)),
                "z" => gmtoff = Some(parse_strptime_offset(&field)?),
                _ => {}
            }
        }

        if iso_year.is_some() {
            if julian.is_some() {
                return Err(strptime_value_error(
                    "Day of the year directive '%j' is not compatible with ISO year directive '%G'. Use '%Y' instead.".to_string(),
                ));
            }
            if iso_week.is_none() || weekday.is_none() {
                return Err(strptime_value_error(
                    "ISO year directive '%G' must be used with the ISO week directive '%V' and a weekday directive ('%A', '%a', '%w', or '%u').".to_string(),
                ));
            }
        } else if iso_week.is_some() {
            if year.is_none() || weekday.is_none() {
                return Err(strptime_value_error(
                    "ISO week directive '%V' must be used with the ISO year directive '%G' and a weekday directive ('%A', '%a', '%w', or '%u').".to_string(),
                ));
            }
            return Err(strptime_value_error(
                "ISO week directive '%V' is incompatible with the year directive '%Y'. Use the ISO year '%G' instead.".to_string(),
            ));
        }

        let mut leap_year_fix = false;
        let mut year = match year {
            Some(year) => year,
            None if month == 2 && day == 29 => {
                leap_year_fix = true;
                1904
            }
            None => 1900,
        };

        if julian.is_none()
            && let Some(weekday) = weekday
        {
            if let Some((week, monday_start)) = week_of_year {
                check_date(year, 1, 1)?;
                julian = Some(julian_from_week(year, week, weekday, monday_start));
            } else if let (Some(iso_year), Some(iso_week)) = (iso_year, iso_week) {
                let (y, m, dd) = iso_to_ymd(iso_year, iso_week, weekday + 1)?;
                check_date(y, m, dd)?;
                year = y;
                month = m;
                day = dd;
            }
            if let Some(value) = julian
                && value <= 0
            {
                year -= 1;
                julian = Some(value + if is_leap(year) { 366 } else { 365 });
            }
        }

        match julian {
            None => {
                check_date(year, month, day)?;
            }
            Some(julian) => {
                check_date(year, 1, 1)?;
                let ordinal = julian - 1 + ymd_to_ordinal(year, 1, 1);
                if ordinal < 1 {
                    return Err("ValueError: ordinal must be >= 1".to_string());
                }
                let ymd = Ymd::from_ordinal(ordinal);
                check_date(ymd.year, ymd.month, ymd.day)?;
                year = ymd.year;
                month = ymd.month;
                day = ymd.day;
            }
        }
        if leap_year_fix {
            year = 1900;
        }
        Ok(Parsed {
            year,
            month,
            day,
            hour,
            minute,
            second,
            fraction,
            tzname: value("Z").map(|field| field.iter().collect()),
            gmtoff,
        })
    }

    /// `date.strptime`, `time.strptime`, and `datetime.strptime`.
    pub(super) fn dt_strptime(
        &mut self,
        class: &Value,
        kind: Kind,
        text: &str,
        format: &str,
    ) -> Result<Value, String> {
        let parsed = self.dt_strptime_fields(class, text, format)?;
        let mut args = Vec::new();
        if kind != Kind::Time {
            args.extend([num(parsed.year), num(parsed.month), num(parsed.day)]);
        }
        if kind != Kind::Date {
            args.extend([
                num(parsed.hour),
                num(parsed.minute),
                num(parsed.second),
                num(parsed.fraction),
            ]);
            if let Some((seconds, us)) = parsed.gmtoff {
                let total = i128::from(seconds) * US_PER_SECOND + i128::from(us);
                let offset = Delta::from_us(total)?;
                if !tz::offset_in_range(offset) {
                    return self.dt_error(
                        "ValueError",
                        format!(
                            "offset must be a timedelta strictly between -timedelta(hours=24) and timedelta(hours=24), not {}",
                            delta::delta_repr("datetime.timedelta", offset)
                        ),
                    );
                }
                let name = parsed.tzname.filter(|name| !name.is_empty());
                let tz = self.dt_make_timezone(class, offset, name)?;
                args.push(tz);
            }
        }
        self.call_value(class.clone(), args)
    }
}

/// The `%z` field: `Z`, or `±HH[:]MM[[:]SS[.ffffff]]`, as seconds and
/// microseconds with the sign applied to both.
fn parse_strptime_offset(field: &[char]) -> Result<(i64, i64), String> {
    if field == ['Z'] {
        return Ok((0, 0));
    }
    let original = field.iter().collect::<String>();
    let mut z = field.to_vec();
    if z.get(3) == Some(&':') {
        z.remove(3);
        if z.len() > 5 {
            if z[5] != ':' {
                return Err(format!("ValueError: Inconsistent use of : in {original}"));
            }
            z.remove(5);
        }
    }
    let int_field = |start: usize, end: usize| -> Result<i64, String> {
        let slice = z.get(start..end.min(z.len())).unwrap_or(&[]);
        if slice.is_empty() {
            return Ok(0);
        }
        if slice.iter().all(|ch| is_decimal(*ch)) {
            Ok(int_of(slice))
        } else {
            Err(format!(
                "ValueError: invalid literal for int() with base 10: {}",
                repr_string(&slice.iter().collect::<String>())
            ))
        }
    };
    let hours = int_field(1, 3)?;
    let minutes = int_field(3, 5)?;
    let seconds = int_field(5, 7)?;
    let mut fraction_text = z.get(8..).unwrap_or(&[]).iter().collect::<String>();
    while fraction_text.chars().count() < 6 {
        fraction_text.push('0');
    }
    let fraction = int_of(&fraction_text.chars().collect::<Vec<_>>());
    let mut offset = hours * 3600 + minutes * 60 + seconds;
    let mut fraction = fraction;
    if z.first() == Some(&'-') {
        offset = -offset;
        fraction = -fraction;
    }
    Ok((offset, fraction))
}
