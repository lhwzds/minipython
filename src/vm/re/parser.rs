// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/re/_parser.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Regular expression syntax: pattern source to a syntax tree.
//!
//! The accepted syntax, the error messages, and the error positions follow
//! CPython 3.14's `re` module, which is the behavior oracle. Patterns are
//! parsed as sequences of code points: `str` patterns by character, `bytes`
//! patterns by byte (each byte is one code point below 256).

use std::collections::HashMap;

use super::flags::{
    ASCII, DEBUG, DOTALL, GLOBAL_FLAGS, IGNORECASE, LOCALE, MULTILINE, TYPE_FLAGS, UNICODE, VERBOSE,
};

/// Largest repeat count (`_sre.MAXREPEAT`); counts reaching it overflow.
pub(super) const MAXREPEAT: u32 = u32::MAX;
/// Largest number of groups (`_sre.MAXGROUPS`).
pub(super) const MAXGROUPS: usize = 1_073_741_823;
/// Width of an unbounded subpattern (`_parser.MAXWIDTH`).
pub(super) const MAXWIDTH: u128 = 1 << 64;

/// Frames CPython's `re.compile` stacks before the parser's first
/// alternation (`compile`, `_compile`, `_compiler.compile`, `parse`).
const PARSE_ENTRY_FRAMES: usize = 4;

/// An error raised while parsing or compiling a pattern.
#[derive(Debug, Clone)]
pub(crate) enum ReError {
    /// `re.PatternError(msg, pattern, pos)`.
    Pattern { msg: String, pos: Option<usize> },
    /// Any other exception, as `"TypeName: message"`.
    Other(String),
}

impl ReError {
    pub(super) fn pattern(msg: impl Into<String>, pos: usize) -> Self {
        ReError::Pattern {
            msg: msg.into(),
            pos: Some(pos),
        }
    }
}

type ParseResult<T> = Result<T, ReError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum AtCode {
    Beginning,
    BeginningString,
    Boundary,
    NonBoundary,
    End,
    EndString,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Category {
    Digit,
    NotDigit,
    Space,
    NotSpace,
    Word,
    NotWord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum SetItem {
    Negate,
    Literal(u32),
    Range(u32, u32),
    Category(Category),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RepeatKind {
    Greedy,
    Lazy,
    Possessive,
}

/// One parsed item. A sequence of items is a [`Seq`].
#[derive(Clone, Debug)]
pub(super) enum Node {
    Literal(u32),
    NotLiteral(u32),
    Any,
    In(Vec<SetItem>),
    At(AtCode),
    Branch(Vec<Seq>),
    Subpattern {
        group: Option<usize>,
        add_flags: u32,
        del_flags: u32,
        body: Seq,
    },
    Atomic(Seq),
    Repeat {
        kind: RepeatKind,
        min: u32,
        max: u32,
        body: Seq,
    },
    GroupRef(usize),
    GroupRefExists {
        group: usize,
        yes: Seq,
        no: Option<Seq>,
    },
    Assert {
        behind: bool,
        negate: bool,
        body: Seq,
    },
    Failure,
}

impl Node {
    /// Whether two items are interchangeable as a common branch prefix.
    /// Items holding subpatterns never compare equal, as in CPython where
    /// they compare by identity.
    fn same_prefix(&self, other: &Node) -> bool {
        match (self, other) {
            (Node::Literal(a), Node::Literal(b)) | (Node::NotLiteral(a), Node::NotLiteral(b)) => {
                a == b
            }
            (Node::Any, Node::Any) | (Node::Failure, Node::Failure) => true,
            (Node::In(a), Node::In(b)) => a == b,
            (Node::At(a), Node::At(b)) => a == b,
            (Node::GroupRef(a), Node::GroupRef(b)) => a == b,
            _ => false,
        }
    }

    fn is_repeat(&self) -> bool {
        matches!(self, Node::Repeat { .. })
    }
}

pub(super) type Seq = Vec<Node>;

/// The parsed pattern.
pub(super) struct Parsed {
    pub seq: Seq,
    /// Flags after inline global flags and the str/bytes fix-up.
    pub flags: u32,
    /// Number of groups plus one (group 0).
    pub groups: usize,
    /// Group names in definition order.
    pub groupindex: Vec<(String, usize)>,
    /// Widths of closed groups, used for back-reference widths.
    pub group_widths: Vec<Option<(u128, u128)>>,
}

#[derive(Clone, Copy, PartialEq)]
struct Tok {
    ch: u32,
    /// The escaped character when `ch` is a backslash.
    esc: Option<u32>,
}

impl Tok {
    fn len(self) -> usize {
        if self.esc.is_some() { 2 } else { 1 }
    }

    fn is(self, ch: char) -> bool {
        self.esc.is_none() && self.ch == ch as u32
    }

    fn is_one_of(self, chars: &str) -> bool {
        self.esc.is_none() && char::from_u32(self.ch).is_some_and(|ch| chars.contains(ch))
    }

    fn push_to(self, out: &mut Vec<u32>) {
        out.push(self.ch);
        if let Some(esc) = self.esc {
            out.push(esc);
        }
    }

    fn text(self) -> String {
        let mut out = Vec::new();
        self.push_to(&mut out);
        code_points_to_string(&out)
    }
}

pub(super) fn code_points_to_string(points: &[u32]) -> String {
    points
        .iter()
        .map(|&point| char::from_u32(point).unwrap_or('\u{fffd}'))
        .collect()
}

fn is_digit(tok: Option<Tok>) -> bool {
    tok.is_some_and(|tok| tok.esc.is_none() && (b'0' as u32..=b'9' as u32).contains(&tok.ch))
}

fn is_octdigit(tok: Option<Tok>) -> bool {
    tok.is_some_and(|tok| tok.esc.is_none() && (b'0' as u32..=b'7' as u32).contains(&tok.ch))
}

fn is_hexdigit(tok: Option<Tok>) -> bool {
    tok.is_some_and(|tok| {
        tok.esc.is_none() && char::from_u32(tok.ch).is_some_and(|ch| ch.is_ascii_hexdigit())
    })
}

fn is_ascii_letter(ch: u32) -> bool {
    char::from_u32(ch).is_some_and(|ch| ch.is_ascii_alphabetic())
}

fn flag_for(ch: u32) -> Option<u32> {
    match char::from_u32(ch)? {
        'i' => Some(IGNORECASE),
        'L' => Some(LOCALE),
        'm' => Some(MULTILINE),
        's' => Some(DOTALL),
        'x' => Some(VERBOSE),
        'a' => Some(ASCII),
        'u' => Some(UNICODE),
        _ => None,
    }
}

/// Python's `str.isidentifier()` on a group name.
fn is_identifier(name: &[u32]) -> bool {
    let mut chars = name.iter().map(|&point| char::from_u32(point));
    let Some(Some(first)) = chars.next() else {
        return false;
    };
    if first != '_' && !unicode_ident::is_xid_start(first) {
        return false;
    }
    chars.all(|ch| ch.is_some_and(|ch| ch == '_' || unicode_ident::is_xid_continue(ch)))
}

/// `name.isdecimal() and name.isascii()`.
fn is_ascii_decimal(name: &[u32]) -> bool {
    !name.is_empty()
        && name
            .iter()
            .all(|&point| (b'0' as u32..=b'9' as u32).contains(&point))
}

fn parse_decimal(digits: &[u32]) -> u128 {
    digits.iter().fold(0u128, |value, &digit| {
        value
            .saturating_mul(10)
            .saturating_add(u128::from(digit - b'0' as u32))
    })
}

/// Python's `%r` of a group name.
fn name_repr(name: &[u32]) -> String {
    crate::value::repr_string(&code_points_to_string(name))
}

/// Python's `%a` of a group name.
fn name_ascii(name: &[u32]) -> String {
    let repr = name_repr(name);
    let mut out = String::new();
    for ch in repr.chars() {
        let point = ch as u32;
        if point < 0x80 {
            out.push(ch);
        } else if point <= 0xff {
            out.push_str(&format!("\\x{point:02x}"));
        } else if point <= 0xffff {
            out.push_str(&format!("\\u{point:04x}"));
        } else {
            out.push_str(&format!("\\U{point:08x}"));
        }
    }
    out
}

/// Python's `msg.encode('ascii', 'backslashreplace').decode('ascii')`.
fn backslashreplace(msg: &str) -> String {
    let mut out = String::new();
    for ch in msg.chars() {
        let point = ch as u32;
        if point < 0x80 {
            out.push(ch);
        } else if point <= 0xff {
            out.push_str(&format!("\\x{point:02x}"));
        } else if point <= 0xffff {
            out.push_str(&format!("\\u{point:04x}"));
        } else {
            out.push_str(&format!("\\U{point:08x}"));
        }
    }
    out
}

/// Looks up `\N{name}` the way `unicodedata.lookup` does.
fn lookup_character_name(name: &[u32]) -> Option<u32> {
    let name = code_points_to_string(name);
    crate::lexer::lookup_unicode_name(&name).map(u32::from)
}

pub(super) struct Tokenizer<'a> {
    chars: &'a [u32],
    is_text: bool,
    index: usize,
    next: Option<Tok>,
}

impl<'a> Tokenizer<'a> {
    pub(super) fn new(chars: &'a [u32], is_text: bool) -> ParseResult<Self> {
        let mut tokenizer = Tokenizer {
            chars,
            is_text,
            index: 0,
            next: None,
        };
        tokenizer.advance()?;
        Ok(tokenizer)
    }

    fn advance(&mut self) -> ParseResult<()> {
        let index = self.index;
        let Some(&ch) = self.chars.get(index) else {
            self.next = None;
            return Ok(());
        };
        if ch == b'\\' as u32 {
            let Some(&esc) = self.chars.get(index + 1) else {
                return Err(ReError::pattern(
                    "bad escape (end of pattern)",
                    self.chars.len() - 1,
                ));
            };
            self.next = Some(Tok { ch, esc: Some(esc) });
            self.index = index + 2;
        } else {
            self.next = Some(Tok { ch, esc: None });
            self.index = index + 1;
        }
        Ok(())
    }

    fn get(&mut self) -> ParseResult<Option<Tok>> {
        let this = self.next;
        self.advance()?;
        Ok(this)
    }

    fn matches(&mut self, ch: char) -> ParseResult<bool> {
        if self.next.is_some_and(|tok| tok.is(ch)) {
            self.advance()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(super) fn tell(&self) -> usize {
        self.index - self.next.map_or(0, Tok::len)
    }

    fn seek(&mut self, index: usize) -> ParseResult<()> {
        self.index = index;
        self.advance()
    }

    fn error(&self, msg: impl Into<String>, offset: usize) -> ReError {
        let msg = msg.into();
        let msg = if self.is_text {
            msg
        } else {
            backslashreplace(&msg)
        };
        ReError::pattern(msg, self.tell() - offset)
    }

    /// Up to `n` tokens from `accept`, appended to `out`.
    fn get_while(
        &mut self,
        n: usize,
        accept: fn(Option<Tok>) -> bool,
        out: &mut Vec<u32>,
    ) -> ParseResult<()> {
        for _ in 0..n {
            let tok = self.next;
            if !accept(tok) {
                break;
            }
            tok.expect("accepted token exists").push_to(out);
            self.advance()?;
        }
        Ok(())
    }

    fn get_until(&mut self, terminator: char, name: &str) -> ParseResult<Vec<u32>> {
        let mut result = Vec::new();
        loop {
            let tok = self.next;
            self.advance()?;
            let Some(tok) = tok else {
                if result.is_empty() {
                    return Err(self.error(format!("missing {name}"), 0));
                }
                return Err(self.error(
                    format!("missing {terminator}, unterminated name"),
                    result.len(),
                ));
            };
            if tok.is(terminator) {
                if result.is_empty() {
                    return Err(self.error(format!("missing {name}"), 1));
                }
                break;
            }
            tok.push_to(&mut result);
        }
        Ok(result)
    }

    fn check_group_name(&self, name: &[u32], offset: usize) -> ParseResult<()> {
        if !self.is_text && !name.iter().all(|&point| point < 0x80) {
            return Err(self.error(
                format!("bad character in group name {}", name_ascii(name)),
                name.len() + offset,
            ));
        }
        if !is_identifier(name) {
            return Err(self.error(
                format!("bad character in group name {}", name_repr(name)),
                name.len() + offset,
            ));
        }
        Ok(())
    }
}

struct State {
    flags: u32,
    group_names: HashMap<String, usize>,
    groupindex: Vec<(String, usize)>,
    group_widths: Vec<Option<(u128, u128)>>,
    lookbehind_groups: Option<usize>,
    /// Conditional group references by number, with their first position.
    groupref_pos: Vec<(usize, usize)>,
    groupref_seen: std::collections::HashSet<usize>,
    /// Python frames on the caller's stack plus the parser's entry frames,
    /// and the recursion limit, to raise CPython's `RecursionError` at the
    /// same nesting depth.
    base_frames: usize,
    recursion_limit: usize,
    warnings: Vec<String>,
}

impl State {
    fn groups(&self) -> usize {
        self.group_widths.len()
    }

    fn open_group(&mut self, name: Option<&[u32]>) -> Result<usize, String> {
        let gid = self.groups();
        self.group_widths.push(None);
        if self.groups() > MAXGROUPS {
            return Err("too many groups".to_string());
        }
        if let Some(name) = name {
            let key = code_points_to_string(name);
            if let Some(&old) = self.group_names.get(&key) {
                return Err(format!(
                    "redefinition of group name {} as group {gid}; was group {old}",
                    name_repr(name)
                ));
            }
            self.group_names.insert(key.clone(), gid);
            self.groupindex.push((key, gid));
        }
        Ok(gid)
    }

    fn close_group(&mut self, gid: usize, body: &Seq) -> ParseResult<()> {
        let width = seq_width(body, &self.group_widths)?;
        self.group_widths[gid] = Some(width);
        Ok(())
    }

    fn check_group(&self, gid: usize) -> bool {
        gid < self.groups() && self.group_widths[gid].is_some()
    }

    fn check_lookbehind_group(&self, gid: usize, source: &Tokenizer) -> ParseResult<()> {
        if let Some(lookbehind_groups) = self.lookbehind_groups {
            if !self.check_group(gid) {
                return Err(source.error("cannot refer to an open group", 0));
            }
            if gid >= lookbehind_groups {
                return Err(source.error(
                    "cannot refer to group defined in the same lookbehind subpattern",
                    0,
                ));
            }
        }
        Ok(())
    }

    /// Raises CPython's `RecursionError` when the parser's own Python frames
    /// at `nested` levels plus `extra` calls would exceed the limit.
    fn check_depth(&self, nested: usize, extra: usize) -> ParseResult<()> {
        let frames = self
            .base_frames
            .saturating_add(1)
            .saturating_add(nested)
            .saturating_add(extra);
        if frames > self.recursion_limit {
            return Err(ReError::Other(
                "RecursionError: maximum recursion depth exceeded".to_string(),
            ));
        }
        crate::stack::check_native_recursion(" while compiling a regular expression")
            .map_err(ReError::Other)
    }
}

/// Width bounds of a sequence (`SubPattern.getwidth`).
pub(super) fn seq_width(
    seq: &Seq,
    group_widths: &[Option<(u128, u128)>],
) -> ParseResult<(u128, u128)> {
    crate::stack::check_native_recursion(" while compiling a regular expression")
        .map_err(ReError::Other)?;
    let mut lo: u128 = 0;
    let mut hi: u128 = 0;
    for node in seq {
        match node {
            Node::Branch(items) => {
                let mut i = MAXWIDTH;
                let mut j = 0;
                for item in items {
                    let (l, h) = seq_width(item, group_widths)?;
                    i = i.min(l);
                    j = j.max(h);
                }
                lo += i;
                hi += j;
            }
            Node::Atomic(body) | Node::Subpattern { body, .. } => {
                let (i, j) = seq_width(body, group_widths)?;
                lo += i;
                hi += j;
            }
            Node::Repeat { min, max, body, .. } => {
                let (i, j) = seq_width(body, group_widths)?;
                lo += i * u128::from(*min);
                if *max == MAXREPEAT && j != 0 {
                    hi = MAXWIDTH;
                } else {
                    hi += j * u128::from(*max);
                }
            }
            Node::Any | Node::In(_) | Node::Literal(_) | Node::NotLiteral(_) => {
                lo += 1;
                hi += 1;
            }
            Node::GroupRef(group) => {
                let (i, j) = group_widths
                    .get(*group)
                    .copied()
                    .flatten()
                    .unwrap_or((0, 0));
                lo += i;
                hi += j;
            }
            Node::GroupRefExists { yes, no, .. } => {
                let (mut i, mut j) = seq_width(yes, group_widths)?;
                if let Some(no) = no {
                    let (l, h) = seq_width(no, group_widths)?;
                    i = i.min(l);
                    j = j.max(h);
                } else {
                    i = 0;
                }
                lo += i;
                hi += j;
            }
            Node::At(_) | Node::Assert { .. } | Node::Failure => {}
        }
        lo = lo.min(MAXWIDTH);
        hi = hi.min(MAXWIDTH);
    }
    Ok((lo.min(MAXWIDTH), hi.min(MAXWIDTH)))
}

/// Parses a pattern given as code points. `call_depth` is the number of
/// Python frames active at the `re` call and `recursion_limit` the current
/// `sys.getrecursionlimit()`.
pub(super) fn parse(
    chars: &[u32],
    is_text: bool,
    flags: u32,
    call_depth: usize,
    recursion_limit: usize,
    warnings: &mut Vec<String>,
) -> ParseResult<Parsed> {
    let mut source = Tokenizer::new(chars, is_text)?;
    let mut state = State {
        flags,
        group_names: HashMap::new(),
        groupindex: Vec::new(),
        group_widths: vec![None],
        lookbehind_groups: None,
        groupref_pos: Vec::new(),
        groupref_seen: std::collections::HashSet::new(),
        base_frames: call_depth + PARSE_ENTRY_FRAMES,
        recursion_limit,
        warnings: Vec::new(),
    };
    let seq = parse_sub(&mut source, &mut state, flags & VERBOSE != 0, 0);
    warnings.append(&mut state.warnings);
    let seq = seq?;
    let flags = fix_flags(is_text, state.flags)?;
    if source.next.is_some() {
        return Err(source.error("unbalanced parenthesis", 0));
    }
    for &(group, pos) in &state.groupref_pos {
        if group >= state.groups() {
            return Err(ReError::pattern(
                format!("invalid group reference {group}"),
                pos,
            ));
        }
    }
    Ok(Parsed {
        seq,
        flags,
        groups: state.groups(),
        groupindex: state.groupindex,
        group_widths: state.group_widths,
    })
}

/// Checks and completes flags for the pattern type (`fix_flags`).
pub(super) fn fix_flags(is_text: bool, mut flags: u32) -> ParseResult<u32> {
    if is_text {
        if flags & LOCALE != 0 {
            return Err(ReError::Other(
                "ValueError: cannot use LOCALE flag with a str pattern".to_string(),
            ));
        }
        if flags & ASCII == 0 {
            flags |= UNICODE;
        } else if flags & UNICODE != 0 {
            return Err(ReError::Other(
                "ValueError: ASCII and UNICODE flags are incompatible".to_string(),
            ));
        }
    } else {
        if flags & UNICODE != 0 {
            return Err(ReError::Other(
                "ValueError: cannot use UNICODE flag with a bytes pattern".to_string(),
            ));
        }
        if flags & LOCALE != 0 && flags & ASCII != 0 {
            return Err(ReError::Other(
                "ValueError: ASCII and LOCALE flags are incompatible".to_string(),
            ));
        }
    }
    Ok(flags)
}

/// An alternation `a|b|c`.
fn parse_sub(
    source: &mut Tokenizer,
    state: &mut State,
    mut verbose: bool,
    nested: usize,
) -> ParseResult<Seq> {
    state.check_depth(nested, 1)?;
    let mut items: Vec<Seq> = Vec::new();
    loop {
        let first = nested == 0 && items.is_empty();
        items.push(parse_seq(source, state, verbose, nested + 1, first)?);
        if !source.matches('|')? {
            break;
        }
        if nested == 0 {
            verbose = state.flags & VERBOSE != 0;
        }
    }
    if items.len() == 1 {
        return Ok(items.pop().expect("one item"));
    }

    let mut result = Seq::new();
    // Move a prefix shared by every alternative out of the branch.
    let shared = (0..)
        .take_while(|&index| {
            let Some(head) = items[0].get(index) else {
                return false;
            };
            items[1..]
                .iter()
                .all(|item| item.get(index).is_some_and(|other| head.same_prefix(other)))
        })
        .count();
    if shared > 0 {
        result.extend(items[0].drain(..shared));
        for item in &mut items[1..] {
            item.drain(..shared);
        }
    }

    // A branch of single characters is a character set.
    let mut set = Vec::new();
    let mut all_chars = true;
    for item in &items {
        if item.len() != 1 {
            all_chars = false;
            break;
        }
        match &item[0] {
            Node::Literal(ch) => set.push(SetItem::Literal(*ch)),
            Node::In(items) if items.first() != Some(&SetItem::Negate) => {
                set.extend(items.iter().copied())
            }
            _ => {
                all_chars = false;
                break;
            }
        }
    }
    if all_chars {
        result.push(Node::In(uniq(set)));
        return Ok(result);
    }
    result.push(Node::Branch(items));
    Ok(result)
}

/// Drops repeated set items, keeping the first of each.
fn uniq(items: Vec<SetItem>) -> Vec<SetItem> {
    let mut seen = std::collections::HashSet::with_capacity(items.len());
    items
        .into_iter()
        .filter(|item| seen.insert(*item))
        .collect()
}

fn category_escape(esc: u32) -> Option<Node> {
    let category = |category| Some(Node::In(vec![SetItem::Category(category)]));
    match char::from_u32(esc)? {
        'A' => Some(Node::At(AtCode::BeginningString)),
        'b' => Some(Node::At(AtCode::Boundary)),
        'B' => Some(Node::At(AtCode::NonBoundary)),
        'd' => category(Category::Digit),
        'D' => category(Category::NotDigit),
        's' => category(Category::Space),
        'S' => category(Category::NotSpace),
        'w' => category(Category::Word),
        'W' => category(Category::NotWord),
        'Z' | 'z' => Some(Node::At(AtCode::EndString)),
        _ => None,
    }
}

fn simple_escape(esc: u32) -> Option<u32> {
    match char::from_u32(esc)? {
        'a' => Some(7),
        'b' => Some(8),
        'f' => Some(12),
        'n' => Some(10),
        'r' => Some(13),
        't' => Some(9),
        'v' => Some(11),
        '\\' => Some(92),
        _ => None,
    }
}

/// `\x`, `\u`, `\U`, and `\N` escapes shared by both escape contexts.
/// `Ok(None)` means the escape is not one of them.
fn code_escape(
    source: &mut Tokenizer,
    esc: u32,
    state: &State,
    nested: usize,
) -> ParseResult<Option<u32>> {
    let mut escape = vec![b'\\' as u32, esc];
    let digits = match char::from_u32(esc) {
        Some('x') => 2,
        Some('u') if source.is_text => 4,
        Some('U') if source.is_text => 8,
        Some('N') if source.is_text => {
            state.check_depth(nested, 3)?;
            if !source.matches('{')? {
                return Err(source.error("missing {", 0));
            }
            let name = source.get_until('}', "character name")?;
            return match lookup_character_name(&name) {
                Some(ch) => Ok(Some(ch)),
                None => Err(source.error(
                    format!("undefined character name {}", name_repr(&name)),
                    name.len() + 4,
                )),
            };
        }
        _ => return Ok(None),
    };
    state.check_depth(nested, 3)?;
    source.get_while(digits, is_hexdigit, &mut escape)?;
    if escape.len() != digits + 2 {
        let text = code_points_to_string(&escape);
        return Err(source.error(format!("incomplete escape {text}"), escape.len()));
    }
    let value = escape[2..].iter().fold(0u64, |value, &digit| {
        value * 16
            + u64::from(
                char::from_u32(digit)
                    .and_then(|d| d.to_digit(16))
                    .unwrap_or(0),
            )
    });
    if value > 0x10ffff {
        let text = code_points_to_string(&escape);
        return Err(source.error(format!("bad escape {text}"), escape.len()));
    }
    Ok(Some(value as u32))
}

/// An escape inside a character set.
fn class_escape(
    source: &mut Tokenizer,
    esc: u32,
    state: &State,
    nested: usize,
) -> ParseResult<SetItem> {
    if let Some(ch) = simple_escape(esc) {
        return Ok(SetItem::Literal(ch));
    }
    if let Some(Node::In(items)) = category_escape(esc) {
        return Ok(items[0]);
    }
    if let Some(ch) = code_escape(source, esc, state, nested)? {
        return Ok(SetItem::Literal(ch));
    }
    let mut escape = vec![b'\\' as u32, esc];
    if (b'0' as u32..=b'7' as u32).contains(&esc) {
        state.check_depth(nested, 3)?;
        source.get_while(2, is_octdigit, &mut escape)?;
        let value = escape[1..]
            .iter()
            .fold(0u32, |value, &digit| value * 8 + (digit - b'0' as u32));
        if value > 0o377 {
            let text = code_points_to_string(&escape);
            return Err(source.error(
                format!("octal escape value {text} outside of range 0-0o377"),
                escape.len(),
            ));
        }
        return Ok(SetItem::Literal(value));
    }
    if (b'0' as u32..=b'9' as u32).contains(&esc) || is_ascii_letter(esc) {
        let text = code_points_to_string(&escape);
        return Err(source.error(format!("bad escape {text}"), escape.len()));
    }
    Ok(SetItem::Literal(esc))
}

/// An escape outside a character set.
fn escape(source: &mut Tokenizer, esc: u32, state: &mut State, nested: usize) -> ParseResult<Node> {
    if let Some(node) = category_escape(esc) {
        return Ok(node);
    }
    if let Some(ch) = simple_escape(esc) {
        return Ok(Node::Literal(ch));
    }
    if let Some(ch) = code_escape(source, esc, state, nested)? {
        return Ok(Node::Literal(ch));
    }
    let mut escape = vec![b'\\' as u32, esc];
    if esc == b'0' as u32 {
        state.check_depth(nested, 3)?;
        source.get_while(2, is_octdigit, &mut escape)?;
        let value = escape[1..]
            .iter()
            .fold(0u32, |value, &digit| value * 8 + (digit - b'0' as u32));
        return Ok(Node::Literal(value));
    }
    if (b'1' as u32..=b'9' as u32).contains(&esc) {
        // An octal escape or a decimal group reference.
        if is_digit(source.next) {
            state.check_depth(nested, 2)?;
            source.get()?.expect("digit").push_to(&mut escape);
            if (b'0' as u32..=b'7' as u32).contains(&escape[1])
                && (b'0' as u32..=b'7' as u32).contains(&escape[2])
                && is_octdigit(source.next)
            {
                source.get()?.expect("digit").push_to(&mut escape);
                let value = escape[1..]
                    .iter()
                    .fold(0u32, |value, &digit| value * 8 + (digit - b'0' as u32));
                if value > 0o377 {
                    let text = code_points_to_string(&escape);
                    return Err(source.error(
                        format!("octal escape value {text} outside of range 0-0o377"),
                        escape.len(),
                    ));
                }
                return Ok(Node::Literal(value));
            }
        }
        let group = parse_decimal(&escape[1..]) as usize;
        if group < state.groups() {
            if !state.check_group(group) {
                return Err(source.error("cannot refer to an open group", escape.len()));
            }
            state.check_lookbehind_group(group, source)?;
            return Ok(Node::GroupRef(group));
        }
        return Err(source.error(format!("invalid group reference {group}"), escape.len() - 1));
    }
    if is_ascii_letter(esc) {
        let text = code_points_to_string(&escape);
        return Err(source.error(format!("bad escape {text}"), escape.len()));
    }
    Ok(Node::Literal(esc))
}

/// A sequence of items up to `|`, `)`, or the end.
fn parse_seq(
    source: &mut Tokenizer,
    state: &mut State,
    mut verbose: bool,
    nested: usize,
    first: bool,
) -> ParseResult<Seq> {
    state.check_depth(nested, 1)?;
    let mut seq = Seq::new();
    loop {
        let Some(this) = source.next else {
            break;
        };
        if this.is('|') || this.is(')') {
            break;
        }
        state.check_depth(nested, 2)?;
        source.get()?;
        if verbose {
            if this.is_one_of(" \t\n\r\u{b}\u{c}") {
                continue;
            }
            if this.is('#') {
                loop {
                    match source.get()? {
                        None => break,
                        Some(tok) if tok.is('\n') => break,
                        Some(_) => {}
                    }
                }
                continue;
            }
        }
        if let Some(esc) = this.esc {
            state.check_depth(nested, 1)?;
            let node = escape(source, esc, state, nested)?;
            seq.push(node);
        } else if !this.is_one_of(".\\[{()*+?^$|") {
            seq.push(Node::Literal(this.ch));
        } else if this.is('[') {
            parse_set(source, state, nested, &mut seq)?;
        } else if this.is_one_of("*+?{") {
            parse_repeat(source, this, &mut seq, state, nested)?;
        } else if this.is('.') {
            seq.push(Node::Any);
        } else if this.is('(') {
            if let Some(new_verbose) = parse_group(source, state, verbose, nested, first, &mut seq)?
            {
                verbose = new_verbose;
            }
        } else if this.is('^') {
            seq.push(Node::At(AtCode::Beginning));
        } else if this.is('$') {
            seq.push(Node::At(AtCode::End));
        } else {
            unreachable!("every special character is handled");
        }
    }

    // Unpack non-capturing groups without flags.
    if seq.iter().any(is_plain_group) {
        let mut unpacked = Seq::with_capacity(seq.len());
        for node in seq {
            if is_plain_group(&node) {
                let Node::Subpattern { body, .. } = node else {
                    unreachable!("checked above");
                };
                unpacked.extend(body);
            } else {
                unpacked.push(node);
            }
        }
        return Ok(unpacked);
    }
    Ok(seq)
}

/// A non-capturing group without flags, which is unpacked into its parent.
fn is_plain_group(node: &Node) -> bool {
    matches!(
        node,
        Node::Subpattern {
            group: None,
            add_flags: 0,
            del_flags: 0,
            ..
        }
    )
}

fn parse_set(
    source: &mut Tokenizer,
    state: &mut State,
    nested: usize,
    seq: &mut Seq,
) -> ParseResult<()> {
    let here = source.tell() - 1;
    let mut set: Vec<SetItem> = Vec::new();
    if source.next.is_some_and(|tok| tok.is('[')) {
        state
            .warnings
            .push(format!("Possible nested set at position {}", source.tell()));
    }
    let negate = source.matches('^')?;
    loop {
        let Some(this) = source.get()? else {
            return Err(source.error("unterminated character set", source.tell() - here));
        };
        if this.is(']') && !set.is_empty() {
            break;
        }
        let code1 = if let Some(esc) = this.esc {
            class_escape(source, esc, state, nested)?
        } else {
            if !set.is_empty()
                && this.is_one_of("-&~|")
                && source.next.is_some_and(|next| next == this)
            {
                let kind = match char::from_u32(this.ch) {
                    Some('-') => "difference",
                    Some('&') => "intersection",
                    Some('~') => "symmetric difference",
                    _ => "union",
                };
                state.warnings.push(format!(
                    "Possible set {kind} at position {}",
                    source.tell() - 1
                ));
            }
            SetItem::Literal(this.ch)
        };
        if source.matches('-')? {
            let Some(that) = source.get()? else {
                return Err(source.error("unterminated character set", source.tell() - here));
            };
            if that.is(']') {
                set.push(code1);
                set.push(SetItem::Literal(b'-' as u32));
                break;
            }
            let code2 = if let Some(esc) = that.esc {
                class_escape(source, esc, state, nested)?
            } else {
                if that.is('-') {
                    state.warnings.push(format!(
                        "Possible set difference at position {}",
                        source.tell() - 2
                    ));
                }
                SetItem::Literal(that.ch)
            };
            let (SetItem::Literal(lo), SetItem::Literal(hi)) = (code1, code2) else {
                let msg = format!("bad character range {}-{}", this.text(), that.text());
                return Err(source.error(msg, this.len() + 1 + that.len()));
            };
            if hi < lo {
                let msg = format!("bad character range {}-{}", this.text(), that.text());
                return Err(source.error(msg, this.len() + 1 + that.len()));
            }
            set.push(SetItem::Range(lo, hi));
        } else {
            set.push(code1);
        }
    }
    let mut set = uniq(set);
    if set.len() == 1
        && let SetItem::Literal(ch) = set[0]
    {
        seq.push(if negate {
            Node::NotLiteral(ch)
        } else {
            Node::Literal(ch)
        });
        return Ok(());
    }
    if negate {
        set.insert(0, SetItem::Negate);
    }
    seq.push(Node::In(set));
    Ok(())
}

fn parse_repeat(
    source: &mut Tokenizer,
    this: Tok,
    seq: &mut Seq,
    state: &State,
    nested: usize,
) -> ParseResult<()> {
    let here = source.tell();
    let (min, max) = match char::from_u32(this.ch) {
        Some('?') => (0, 1),
        Some('*') => (0, MAXREPEAT),
        Some('+') => (1, MAXREPEAT),
        _ => {
            // '{'
            if source.next.is_some_and(|tok| tok.is('}')) {
                seq.push(Node::Literal(this.ch));
                return Ok(());
            }
            state.check_depth(nested, 2)?;
            let mut lo = Vec::new();
            let mut hi = Vec::new();
            while is_digit(source.next) {
                source.get()?.expect("digit").push_to(&mut lo);
            }
            if source.matches(',')? {
                while is_digit(source.next) {
                    source.get()?.expect("digit").push_to(&mut hi);
                }
            } else {
                hi = lo.clone();
            }
            if !source.matches('}')? {
                seq.push(Node::Literal(this.ch));
                source.seek(here)?;
                return Ok(());
            }
            let mut min = 0u32;
            let mut max = MAXREPEAT;
            if !lo.is_empty() {
                let value = parse_decimal(&lo);
                if value >= u128::from(MAXREPEAT) {
                    return Err(ReError::Other(
                        "OverflowError: the repetition number is too large".to_string(),
                    ));
                }
                min = value as u32;
            }
            if !hi.is_empty() {
                let value = parse_decimal(&hi);
                if value >= u128::from(MAXREPEAT) {
                    return Err(ReError::Other(
                        "OverflowError: the repetition number is too large".to_string(),
                    ));
                }
                max = value as u32;
                if max < min {
                    return Err(
                        source.error("min repeat greater than max repeat", source.tell() - here)
                    );
                }
            }
            (min, max)
        }
    };
    let offset = source.tell() - here + this.len();
    let item = match seq.last() {
        None | Some(Node::At(_)) => {
            return Err(source.error("nothing to repeat", offset));
        }
        Some(node) if node.is_repeat() => {
            return Err(source.error("multiple repeat", offset));
        }
        Some(_) => seq.pop().expect("checked above"),
    };
    let body = match item {
        Node::Subpattern {
            group: None,
            add_flags: 0,
            del_flags: 0,
            body,
        } => body,
        item => vec![item],
    };
    let kind = if source.matches('?')? {
        RepeatKind::Lazy
    } else if source.matches('+')? {
        RepeatKind::Possessive
    } else {
        RepeatKind::Greedy
    };
    seq.push(Node::Repeat {
        kind,
        min,
        max,
        body,
    });
    Ok(())
}

/// A group after its `(`. Returns the new verbose setting when the group
/// was a global flags group.
fn parse_group(
    source: &mut Tokenizer,
    state: &mut State,
    verbose: bool,
    nested: usize,
    first: bool,
    seq: &mut Seq,
) -> ParseResult<Option<bool>> {
    let start = source.tell() - 1;
    let mut capture = true;
    let mut atomic = false;
    let mut name: Option<Vec<u32>> = None;
    let mut add_flags = 0;
    let mut del_flags = 0;
    if source.matches('?')? {
        let Some(ch) = source.get()? else {
            return Err(source.error("unexpected end of pattern", 0));
        };
        if ch.is('P') {
            if source.matches('<')? {
                state.check_depth(nested, 3)?;
                let group_name = source.get_until('>', "group name")?;
                source.check_group_name(&group_name, 1)?;
                name = Some(group_name);
            } else if source.matches('=')? {
                state.check_depth(nested, 3)?;
                let group_name = source.get_until(')', "group name")?;
                source.check_group_name(&group_name, 1)?;
                let key = code_points_to_string(&group_name);
                let Some(&gid) = state.group_names.get(&key) else {
                    return Err(source.error(
                        format!("unknown group name {}", name_repr(&group_name)),
                        group_name.len() + 1,
                    ));
                };
                if !state.check_group(gid) {
                    return Err(source.error("cannot refer to an open group", group_name.len() + 1));
                }
                state.check_lookbehind_group(gid, source)?;
                seq.push(Node::GroupRef(gid));
                return Ok(None);
            } else {
                let Some(ch) = source.get()? else {
                    return Err(source.error("unexpected end of pattern", 0));
                };
                return Err(
                    source.error(format!("unknown extension ?P{}", ch.text()), ch.len() + 2)
                );
            }
        } else if ch.is(':') {
            capture = false;
        } else if ch.is('#') {
            loop {
                if source.next.is_none() {
                    return Err(
                        source.error("missing ), unterminated comment", source.tell() - start)
                    );
                }
                if source.get()?.is_some_and(|tok| tok.is(')')) {
                    break;
                }
            }
            return Ok(None);
        } else if ch.is_one_of("=!<") {
            let mut kind = ch;
            let mut behind = false;
            let mut saved_lookbehind = None;
            if ch.is('<') {
                let Some(next) = source.get()? else {
                    return Err(source.error("unexpected end of pattern", 0));
                };
                if !next.is_one_of("=!") {
                    return Err(source.error(
                        format!("unknown extension ?<{}", next.text()),
                        next.len() + 2,
                    ));
                }
                kind = next;
                behind = true;
                saved_lookbehind = Some(state.lookbehind_groups);
                if state.lookbehind_groups.is_none() {
                    state.lookbehind_groups = Some(state.groups());
                }
            }
            let body = parse_sub(source, state, verbose, nested + 1)?;
            if let Some(None) = saved_lookbehind {
                state.lookbehind_groups = None;
            }
            if !source.matches(')')? {
                return Err(
                    source.error("missing ), unterminated subpattern", source.tell() - start)
                );
            }
            if kind.is('=') {
                seq.push(Node::Assert {
                    behind,
                    negate: false,
                    body,
                });
            } else if !body.is_empty() {
                seq.push(Node::Assert {
                    behind,
                    negate: true,
                    body,
                });
            } else {
                seq.push(Node::Failure);
            }
            return Ok(None);
        } else if ch.is('(') {
            state.check_depth(nested, 3)?;
            let cond_name = source.get_until(')', "group name")?;
            let cond_group;
            if !is_ascii_decimal(&cond_name) {
                source.check_group_name(&cond_name, 1)?;
                let key = code_points_to_string(&cond_name);
                let Some(&gid) = state.group_names.get(&key) else {
                    return Err(source.error(
                        format!("unknown group name {}", name_repr(&cond_name)),
                        cond_name.len() + 1,
                    ));
                };
                cond_group = gid;
            } else {
                let value = parse_decimal(&cond_name);
                if value == 0 {
                    return Err(source.error("bad group number", cond_name.len() + 1));
                }
                if value >= MAXGROUPS as u128 {
                    return Err(source.error(
                        format!("invalid group reference {value}"),
                        cond_name.len() + 1,
                    ));
                }
                cond_group = value as usize;
                if state.groupref_seen.insert(cond_group) {
                    state
                        .groupref_pos
                        .push((cond_group, source.tell() - cond_name.len() - 1));
                }
            }
            state.check_lookbehind_group(cond_group, source)?;
            let yes = parse_seq(source, state, verbose, nested + 1, false)?;
            let no = if source.matches('|')? {
                let no = parse_seq(source, state, verbose, nested + 1, false)?;
                if source.next.is_some_and(|tok| tok.is('|')) {
                    return Err(source.error("conditional backref with more than two branches", 0));
                }
                Some(no)
            } else {
                None
            };
            if !source.matches(')')? {
                return Err(
                    source.error("missing ), unterminated subpattern", source.tell() - start)
                );
            }
            seq.push(Node::GroupRefExists {
                group: cond_group,
                yes,
                no,
            });
            return Ok(None);
        } else if ch.is('>') {
            capture = false;
            atomic = true;
        } else if ch.esc.is_none() && (flag_for(ch.ch).is_some() || ch.is('-')) {
            match parse_flags(source, state, ch)? {
                None => {
                    if !first || !seq.is_empty() {
                        return Err(source.error(
                            "global flags not at the start of the expression",
                            source.tell() - start,
                        ));
                    }
                    return Ok(Some(state.flags & VERBOSE != 0));
                }
                Some((add, del)) => {
                    add_flags = add;
                    del_flags = del;
                    capture = false;
                }
            }
        } else {
            return Err(source.error(format!("unknown extension ?{}", ch.text()), ch.len() + 1));
        }
    }

    let group = if capture {
        match state.open_group(name.as_deref()) {
            Ok(gid) => Some(gid),
            Err(msg) => {
                let offset = name.as_ref().map_or(0, Vec::len) + 1;
                return Err(source.error(msg, offset));
            }
        }
    } else {
        None
    };
    let sub_verbose = (verbose || add_flags & VERBOSE != 0) && del_flags & VERBOSE == 0;
    let body = parse_sub(source, state, sub_verbose, nested + 1)?;
    if !source.matches(')')? {
        return Err(source.error("missing ), unterminated subpattern", source.tell() - start));
    }
    if let Some(gid) = group {
        state.close_group(gid, &body)?;
    }
    if atomic {
        seq.push(Node::Atomic(body));
    } else {
        seq.push(Node::Subpattern {
            group,
            add_flags,
            del_flags,
            body,
        });
    }
    Ok(None)
}

/// Inline flags after `(?`. `None` means global flags `(?flags)`.
fn parse_flags(
    source: &mut Tokenizer,
    state: &mut State,
    first: Tok,
) -> ParseResult<Option<(u32, u32)>> {
    let mut add_flags = 0;
    let mut del_flags = 0;
    let mut ch = first;
    if !ch.is('-') {
        loop {
            let flag = flag_for(ch.ch).expect("caller checked the flag");
            if source.is_text {
                if ch.is('L') {
                    return Err(source.error(
                        "bad inline flags: cannot use 'L' flag with a str pattern",
                        0,
                    ));
                }
            } else if ch.is('u') {
                return Err(source.error(
                    "bad inline flags: cannot use 'u' flag with a bytes pattern",
                    0,
                ));
            }
            add_flags |= flag;
            if flag & TYPE_FLAGS != 0 && add_flags & TYPE_FLAGS != flag {
                return Err(source.error(
                    "bad inline flags: flags 'a', 'u' and 'L' are incompatible",
                    0,
                ));
            }
            let Some(next) = source.get()? else {
                return Err(source.error("missing -, : or )", 0));
            };
            ch = next;
            if ch.is_one_of(")-:") {
                break;
            }
            if ch.esc.is_some() || flag_for(ch.ch).is_none() {
                let msg = if ch.esc.is_none() && is_alpha(ch.ch) {
                    "unknown flag"
                } else {
                    "missing -, : or )"
                };
                return Err(source.error(msg, ch.len()));
            }
        }
    }
    if ch.is(')') {
        state.flags |= add_flags;
        return Ok(None);
    }
    if add_flags & GLOBAL_FLAGS != 0 {
        return Err(source.error("bad inline flags: cannot turn on global flag", 1));
    }
    if ch.is('-') {
        let Some(next) = source.get()? else {
            return Err(source.error("missing flag", 0));
        };
        ch = next;
        if ch.esc.is_some() || flag_for(ch.ch).is_none() {
            let msg = if ch.esc.is_none() && is_alpha(ch.ch) {
                "unknown flag"
            } else {
                "missing flag"
            };
            return Err(source.error(msg, ch.len()));
        }
        loop {
            let flag = flag_for(ch.ch).expect("checked above");
            if flag & TYPE_FLAGS != 0 {
                return Err(source.error(
                    "bad inline flags: cannot turn off flags 'a', 'u' and 'L'",
                    0,
                ));
            }
            del_flags |= flag;
            let Some(next) = source.get()? else {
                return Err(source.error("missing :", 0));
            };
            ch = next;
            if ch.is(':') {
                break;
            }
            if ch.esc.is_some() || flag_for(ch.ch).is_none() {
                let msg = if ch.esc.is_none() && is_alpha(ch.ch) {
                    "unknown flag"
                } else {
                    "missing :"
                };
                return Err(source.error(msg, ch.len()));
            }
        }
    }
    if del_flags & GLOBAL_FLAGS != 0 {
        return Err(source.error("bad inline flags: cannot turn off global flag", 1));
    }
    if add_flags & del_flags != 0 {
        return Err(source.error("bad inline flags: flag turned on and off", 1));
    }
    let _ = DEBUG;
    Ok(Some((add_flags, del_flags)))
}

/// Python's `str.isalpha()` on one character.
fn is_alpha(ch: u32) -> bool {
    char::from_u32(ch).is_some_and(|ch| {
        use unicode_general_category::{GeneralCategory as G, get_general_category};
        matches!(
            get_general_category(ch),
            G::UppercaseLetter
                | G::LowercaseLetter
                | G::TitlecaseLetter
                | G::ModifierLetter
                | G::OtherLetter
        )
    })
}
