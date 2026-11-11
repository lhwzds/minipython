// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/re/__init__.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Native `re` module.
//!
//! Patterns are parsed (`parser.rs`) and compiled (`compiler.rs`) natively,
//! and matched by a metered backtracking engine (`engine.rs`) that keeps its
//! backtracking state on an explicit stack. CPython 3.14.6 is the behavior
//! oracle for syntax, error messages, matching results, and the Python-level
//! API. `Pattern`, `Match`, and the `finditer` scanner are `Value::Re`
//! objects; `re.PatternError` is an exception class and `re.RegexFlag` an
//! `int` subclass.

use super::*;
use std::fmt;

mod compiler;
mod engine;
mod parser;
mod unicode;

use compiler::Program;
use engine::{Matcher, Meter, NONE, Text};
use parser::{ReError, code_points_to_string};

pub(super) mod flags {
    pub const IGNORECASE: u32 = 2;
    pub const LOCALE: u32 = 4;
    pub const MULTILINE: u32 = 8;
    pub const DOTALL: u32 = 16;
    pub const UNICODE: u32 = 32;
    pub const VERBOSE: u32 = 64;
    pub const DEBUG: u32 = 128;
    pub const ASCII: u32 = 256;
    pub const TYPE_FLAGS: u32 = ASCII | LOCALE | UNICODE;
    pub const GLOBAL_FLAGS: u32 = DEBUG;
}

/// Prefix of every native `re` builtin.
const RE_PREFIX: &str = "re.";
/// Hidden class attribute marking `re.PatternError` (and subclasses) so the
/// VM builds its instances natively.
pub(crate) const RE_PATTERN_ERROR_MARKER: &str = "\0re_pattern_error";
/// Hidden class attribute marking `re.RegexFlag`.
const RE_FLAG_MARKER: &str = "\0re_regex_flag";
/// Number of compiled patterns kept by `re`'s cache (`re._MAXCACHE`).
const MAXCACHE: usize = 512;

const FLAG_NAMES: &[(&str, u32)] = &[
    ("ASCII", flags::ASCII),
    ("IGNORECASE", flags::IGNORECASE),
    ("LOCALE", flags::LOCALE),
    ("UNICODE", flags::UNICODE),
    ("MULTILINE", flags::MULTILINE),
    ("DOTALL", flags::DOTALL),
    ("VERBOSE", flags::VERBOSE),
    ("DEBUG", flags::DEBUG),
];

/// One-letter aliases of the flags (`re.I`, ...).
const FLAG_LETTERS: &[(&str, u32)] = &[
    ("A", flags::ASCII),
    ("I", flags::IGNORECASE),
    ("L", flags::LOCALE),
    ("U", flags::UNICODE),
    ("M", flags::MULTILINE),
    ("S", flags::DOTALL),
    ("X", flags::VERBOSE),
];

const MODULE_ALL: &[&str] = &[
    "match",
    "fullmatch",
    "search",
    "sub",
    "subn",
    "split",
    "findall",
    "finditer",
    "compile",
    "purge",
    "escape",
    "error",
    "Pattern",
    "Match",
    "A",
    "I",
    "L",
    "M",
    "S",
    "X",
    "U",
    "ASCII",
    "IGNORECASE",
    "LOCALE",
    "MULTILINE",
    "DOTALL",
    "VERBOSE",
    "UNICODE",
    "NOFLAG",
    "RegexFlag",
    "PatternError",
];

const MODULE_FUNCTIONS: &[&str] = &[
    "match",
    "fullmatch",
    "search",
    "sub",
    "subn",
    "split",
    "findall",
    "finditer",
    "compile",
    "purge",
    "escape",
];

const PATTERN_METHODS: &[&str] = &[
    "match",
    "fullmatch",
    "search",
    "sub",
    "subn",
    "split",
    "findall",
    "finditer",
    "scanner",
    "__copy__",
    "__deepcopy__",
];

const MATCH_METHODS: &[&str] = &[
    "group",
    "groups",
    "groupdict",
    "start",
    "end",
    "span",
    "expand",
    "__getitem__",
    "__copy__",
    "__deepcopy__",
];

const PATTERN_DIR: &[&str] = &[
    "__class__",
    "__class_getitem__",
    "__copy__",
    "__deepcopy__",
    "__delattr__",
    "__dir__",
    "__doc__",
    "__eq__",
    "__format__",
    "__ge__",
    "__getattribute__",
    "__getstate__",
    "__gt__",
    "__hash__",
    "__init__",
    "__init_subclass__",
    "__le__",
    "__lt__",
    "__module__",
    "__ne__",
    "__new__",
    "__reduce__",
    "__reduce_ex__",
    "__repr__",
    "__setattr__",
    "__sizeof__",
    "__str__",
    "__subclasshook__",
    "findall",
    "finditer",
    "flags",
    "fullmatch",
    "groupindex",
    "groups",
    "match",
    "pattern",
    "scanner",
    "search",
    "split",
    "sub",
    "subn",
];

const MATCH_DIR: &[&str] = &[
    "__class__",
    "__class_getitem__",
    "__copy__",
    "__deepcopy__",
    "__delattr__",
    "__dir__",
    "__doc__",
    "__eq__",
    "__format__",
    "__ge__",
    "__getattribute__",
    "__getitem__",
    "__getstate__",
    "__gt__",
    "__hash__",
    "__init__",
    "__init_subclass__",
    "__le__",
    "__lt__",
    "__module__",
    "__ne__",
    "__new__",
    "__reduce__",
    "__reduce_ex__",
    "__repr__",
    "__setattr__",
    "__sizeof__",
    "__str__",
    "__subclasshook__",
    "end",
    "endpos",
    "expand",
    "group",
    "groupdict",
    "groups",
    "lastgroup",
    "lastindex",
    "pos",
    "re",
    "regs",
    "span",
    "start",
    "string",
];

const MODULE_DOC: &str = "Regular expression matching for str and bytes: compile, search, match, sub, split and related helpers.";

const PATTERN_ERROR_DOC: &str = "Raised when a regular expression cannot be compiled; msg, pattern, pos, lineno and colno say where.";

// ---------------------------------------------------------------------------
// Objects
// ---------------------------------------------------------------------------

/// Pattern source as given to `re.compile`.
#[derive(Clone, PartialEq, Eq, Hash)]
enum PatternText {
    Str(String),
    Bytes(Vec<u8>),
}

pub(crate) struct PatternObject {
    /// The pattern object as passed (`Pattern.pattern`).
    source: Value,
    text: PatternText,
    flags: i64,
    program: Program,
    groupindex: Vec<(String, usize)>,
}

/// The string being matched.
enum SubjectText {
    Bytes(Rc<Vec<u8>>),
    Ascii(String),
    Wide(Vec<u32>),
}

pub(crate) struct Subject {
    /// The object as passed (`Match.string`).
    original: Value,
    text: SubjectText,
}

impl Subject {
    fn len(&self) -> usize {
        match &self.text {
            SubjectText::Bytes(bytes) => bytes.len(),
            SubjectText::Ascii(text) => text.len(),
            SubjectText::Wide(chars) => chars.len(),
        }
    }

    fn is_bytes(&self) -> bool {
        matches!(self.text, SubjectText::Bytes(_))
    }

    /// The exact `str`/`bytes` slice `[start, end)`.
    fn slice(&self, start: usize, end: usize) -> Value {
        match &self.text {
            SubjectText::Bytes(bytes) => {
                if start == 0
                    && end == bytes.len()
                    && let Value::Bytes(original) = &self.original
                {
                    return Value::Bytes(original.clone());
                }
                bytes_value(bytes[start..end].to_vec())
            }
            SubjectText::Ascii(text) => Value::String(text[start..end].to_string()),
            SubjectText::Wide(chars) => Value::String(code_points_to_string(&chars[start..end])),
        }
    }

    fn empty(&self) -> Value {
        if self.is_bytes() {
            bytes_value(Vec::new())
        } else {
            Value::String(String::new())
        }
    }
}

pub(crate) struct MatchObject {
    pattern: Rc<PatternObject>,
    subject: Rc<Subject>,
    pos: usize,
    endpos: usize,
    /// Spans of group 0 and every group (`(NONE, NONE)` when unset).
    spans: Vec<(usize, usize)>,
    lastindex: isize,
}

pub(crate) struct ScannerObject {
    pattern: Rc<PatternObject>,
    subject: Rc<Subject>,
    pos: usize,
    endpos: usize,
    /// Where the next search starts; `None` once exhausted.
    start: Cell<Option<usize>>,
    must_advance: Cell<bool>,
}

/// A `re` runtime object.
#[derive(Clone)]
pub(crate) enum ReObject {
    Pattern(Rc<PatternObject>),
    Match(Rc<MatchObject>),
    Scanner(Rc<ScannerObject>),
}

impl fmt::Debug for ReObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.repr())
    }
}

/// Python's `%.<limit>R`: a repr cut to `limit` characters.
fn truncate_chars(text: String, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        Some((index, _)) => text[..index].to_string(),
        None => text,
    }
}

impl ReObject {
    pub(crate) fn type_name(&self) -> &'static str {
        match self {
            ReObject::Pattern(_) => "re.Pattern",
            ReObject::Match(_) => "re.Match",
            ReObject::Scanner(_) => "_sre.SRE_Scanner",
        }
    }

    pub(crate) fn type_object(&self) -> Value {
        Value::Builtin(self.type_name().to_string())
    }

    pub(crate) fn identity(&self) -> usize {
        match self {
            ReObject::Pattern(object) => Rc::as_ptr(object) as *const u8 as usize,
            ReObject::Match(object) => Rc::as_ptr(object) as *const u8 as usize,
            ReObject::Scanner(object) => Rc::as_ptr(object) as *const u8 as usize,
        }
    }

    pub(crate) fn repr(&self) -> String {
        match self {
            ReObject::Pattern(pattern) => pattern_repr(pattern),
            ReObject::Match(m) => {
                let (start, end) = m.spans[0];
                let text = match m.subject.slice(start, end) {
                    Value::String(text) => crate::value::repr_string(&text),
                    Value::Bytes(bytes) => crate::value::repr_bytes(&bytes),
                    _ => String::new(),
                };
                format!(
                    "<re.Match object; span=({start}, {end}), match={}>",
                    truncate_chars(text, 50)
                )
            }
            ReObject::Scanner(scanner) => format!(
                "<_sre.SRE_Scanner object at {:#x}>",
                Rc::as_ptr(scanner) as *const u8 as usize
            ),
        }
    }

    /// `==`: patterns compare by source, flags, and type; other objects by
    /// identity.
    pub(crate) fn equals(&self, other: &ReObject) -> bool {
        match (self, other) {
            (ReObject::Pattern(left), ReObject::Pattern(right)) => {
                Rc::ptr_eq(left, right) || (left.text == right.text && left.flags == right.flags)
            }
            _ => self.identity() == other.identity(),
        }
    }

    pub(crate) fn hash_into(&self, hasher: &mut impl std::hash::Hasher) {
        match self {
            ReObject::Pattern(pattern) => {
                "re.Pattern".hash(hasher);
                pattern.text.hash(hasher);
                pattern.flags.hash(hasher);
            }
            other => other.identity().hash(hasher),
        }
    }

    pub(crate) fn estimated_size(&self) -> usize {
        match self {
            ReObject::Pattern(_) => std::mem::size_of::<PatternObject>(),
            ReObject::Match(m) => std::mem::size_of::<MatchObject>() + m.spans.len() * 16,
            ReObject::Scanner(_) => std::mem::size_of::<ScannerObject>(),
        }
    }

    pub(crate) fn dir_names(&self) -> Vec<String> {
        let names = match self {
            ReObject::Pattern(_) => PATTERN_DIR,
            ReObject::Match(_) => MATCH_DIR,
            ReObject::Scanner(_) => &["match", "pattern", "search"][..],
        };
        names.iter().map(|name| (*name).to_string()).collect()
    }

    /// Attribute lookup on a `re` object.
    pub(crate) fn attribute(&self, name: &str) -> Result<Value, String> {
        let receiver = || Value::Re(self.clone());
        let method = |qualname: &str| {
            Ok(Value::BoundMethod {
                function: Box::new(Value::Builtin(format!("{RE_PREFIX}{qualname}"))),
                receiver: Box::new(receiver()),
                identity: Rc::new(()),
            })
        };
        match self {
            ReObject::Pattern(pattern) => match name {
                "pattern" => Ok(pattern.source.clone()),
                "flags" => Ok(Value::Number(pattern.flags)),
                "groups" => Ok(Value::Number(pattern.program.groups as i64)),
                "groupindex" if pattern.groupindex.is_empty() => Ok(dict_value(Vec::new())),
                "groupindex" => Ok(mapping_proxy_value(Rc::new(RefCell::new(
                    DictStorage::new(
                        pattern
                            .groupindex
                            .iter()
                            .map(|(name, index)| {
                                (Value::String(name.clone()), Value::Number(*index as i64))
                            })
                            .collect(),
                    ),
                )))),
                "__class__" => Ok(self.type_object()),
                "__doc__" => Ok(Value::String(PATTERN_DOC.to_string())),
                "__module__" => Ok(Value::String("re".to_string())),
                name if PATTERN_METHODS.contains(&name) => method(&format!("Pattern.{name}")),
                _ => Err(format!(
                    "AttributeError: 're.Pattern' object has no attribute '{name}'"
                )),
            },
            ReObject::Match(m) => match name {
                "string" => Ok(m.subject.original.clone()),
                "re" => Ok(Value::Re(ReObject::Pattern(m.pattern.clone()))),
                "pos" => Ok(Value::Number(m.pos as i64)),
                "endpos" => Ok(Value::Number(m.endpos as i64)),
                "lastindex" => Ok(if m.lastindex >= 0 {
                    Value::Number(m.lastindex as i64)
                } else {
                    Value::None
                }),
                "lastgroup" => Ok(m
                    .pattern
                    .groupindex
                    .iter()
                    .find(|(_, index)| m.lastindex >= 0 && *index == m.lastindex as usize)
                    .map(|(name, _)| Value::String(name.clone()))
                    .unwrap_or(Value::None)),
                "regs" => Ok(tuple_value(
                    m.spans
                        .iter()
                        .map(|&(start, end)| span_value(start, end))
                        .collect(),
                )),
                "__class__" => Ok(self.type_object()),
                "__doc__" => Ok(Value::String(MATCH_DOC.to_string())),
                "__module__" => Ok(Value::String("re".to_string())),
                name if MATCH_METHODS.contains(&name) => method(&format!("Match.{name}")),
                _ => Err(format!(
                    "AttributeError: 're.Match' object has no attribute '{name}'"
                )),
            },
            ReObject::Scanner(scanner) => match name {
                "pattern" => Ok(Value::Re(ReObject::Pattern(scanner.pattern.clone()))),
                "search" | "match" => method(&format!("Scanner.{name}")),
                "__class__" => Ok(self.type_object()),
                _ => Err(format!(
                    "AttributeError: '_sre.SRE_Scanner' object has no attribute '{name}'"
                )),
            },
        }
    }

    /// The error for `obj.name = value` and `del obj.name`.
    pub(crate) fn set_attribute_error(&self, name: &str) -> String {
        let known = match self {
            ReObject::Pattern(_) => {
                matches!(name, "pattern" | "flags" | "groups" | "groupindex")
            }
            ReObject::Match(_) => matches!(
                name,
                "string" | "re" | "pos" | "endpos" | "lastindex" | "lastgroup" | "regs"
            ),
            ReObject::Scanner(_) => name == "pattern",
        };
        if known {
            "AttributeError: readonly attribute".to_string()
        } else if PATTERN_METHODS.contains(&name) || MATCH_METHODS.contains(&name) {
            format!(
                "AttributeError: '{}' object attribute '{name}' is read-only",
                self.type_name()
            )
        } else {
            format!(
                "AttributeError: '{}' object has no attribute '{name}' and no __dict__ for setting new attributes",
                self.type_name()
            )
        }
    }
}

const PATTERN_DOC: &str = "A compiled regular expression.";
const MATCH_DOC: &str =
    "The outcome of a successful match, with the matched groups and their positions.";

fn span_value(start: usize, end: usize) -> Value {
    if start == NONE {
        tuple_value(vec![Value::Number(-1), Value::Number(-1)])
    } else {
        tuple_value(vec![Value::Number(start as i64), Value::Number(end as i64)])
    }
}

fn pattern_repr(pattern: &PatternObject) -> String {
    let source = match &pattern.text {
        PatternText::Str(text) => crate::value::repr_string(text),
        PatternText::Bytes(bytes) => crate::value::repr_bytes(bytes),
    };
    let mut flags = pattern.flags;
    if matches!(pattern.text, PatternText::Str(_))
        && flags & i64::from(flags::LOCALE | flags::UNICODE | flags::ASCII)
            == i64::from(flags::UNICODE)
    {
        flags &= !i64::from(flags::UNICODE);
    }
    let mut items = Vec::new();
    for (name, value) in [
        ("re.IGNORECASE", flags::IGNORECASE),
        ("re.LOCALE", flags::LOCALE),
        ("re.MULTILINE", flags::MULTILINE),
        ("re.DOTALL", flags::DOTALL),
        ("re.UNICODE", flags::UNICODE),
        ("re.VERBOSE", flags::VERBOSE),
        ("re.DEBUG", flags::DEBUG),
        ("re.ASCII", flags::ASCII),
    ] {
        if flags & i64::from(value) != 0 {
            items.push(name.to_string());
            flags &= !i64::from(value);
        }
    }
    if flags != 0 {
        items.push(format!("{:#x}", flags as i32));
    }
    let source = truncate_chars(source, 200);
    if items.is_empty() {
        format!("re.compile({source})")
    } else {
        format!("re.compile({source}, {})", items.join("|"))
    }
}

// ---------------------------------------------------------------------------
// Compiled pattern cache
// ---------------------------------------------------------------------------

type CacheKey = (PatternText, i64);

thread_local! {
    static PATTERN_CACHE: RefCell<Vec<(CacheKey, Rc<PatternObject>)>> = const { RefCell::new(Vec::new()) };
}

fn cache_lookup(key: &CacheKey) -> Option<Rc<PatternObject>> {
    PATTERN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|(entry, _)| entry == key)?;
        // Most recently used entries live at the end.
        let entry = cache.remove(index);
        let pattern = entry.1.clone();
        cache.push(entry);
        Some(pattern)
    })
}

fn cache_store(key: CacheKey, pattern: Rc<PatternObject>) {
    PATTERN_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= MAXCACHE {
            cache.remove(0);
        }
        cache.push((key, pattern));
    });
}

fn cache_purge() {
    PATTERN_CACHE.with(|cache| cache.borrow_mut().clear());
}

// ---------------------------------------------------------------------------
// Budgets
// ---------------------------------------------------------------------------

/// Charges matcher work to the shared VM budgets without borrowing the VM,
/// so callbacks into Python can run between matches.
struct BudgetMeter {
    instructions: Rc<Cell<Option<u64>>>,
    allocation: Rc<RefCell<AllocationBudgetState>>,
}

impl Meter for BudgetMeter {
    fn charge_steps(&mut self, steps: u64) -> Result<(), String> {
        let Some(remaining) = self.instructions.get() else {
            return Ok(());
        };
        if steps > remaining {
            self.instructions.set(Some(0));
            return Err(crate::terminal::trip(
                crate::terminal::TerminalKind::InstructionLimit,
            ));
        }
        self.instructions.set(Some(remaining - steps));
        Ok(())
    }

    fn charge_memory(&mut self, bytes: usize) -> Result<(), String> {
        let mut budget = self.allocation.borrow_mut();
        let Some(remaining) = budget.remaining else {
            return Ok(());
        };
        if bytes > remaining {
            return Err(crate::terminal::trip(
                crate::terminal::TerminalKind::AllocationLimit,
            ));
        }
        budget.remaining = Some(remaining - bytes);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Argument binding for the C-level methods
// ---------------------------------------------------------------------------

/// Binds arguments the way CPython's Argument Clinic does for `Pattern` and
/// `Match` methods.
fn bind_clinic(
    name: &str,
    params: &[&str],
    required: usize,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Vec<Option<Value>>, String> {
    let total = params.len();
    let given = args.len() + keywords.len();
    if args.len() > total || given > total {
        if keywords.is_empty() || args.len() > total {
            return Err(format!(
                "TypeError: {name}() takes at most {total} argument{} ({} given)",
                if total == 1 { "" } else { "s" },
                given
            ));
        }
    }
    let mut values: Vec<Option<Value>> = vec![None; total];
    for (index, value) in args.into_iter().enumerate() {
        values[index] = Some(value);
    }
    for (keyword, value) in keywords {
        let Some(index) = params.iter().position(|param| *param == keyword) else {
            return Err(format!(
                "TypeError: {name}() got an unexpected keyword argument '{keyword}'"
            ));
        };
        if values[index].is_some() {
            return Err(format!(
                "TypeError: argument for {name}() given by name ('{keyword}') and position ({})",
                index + 1
            ));
        }
        values[index] = Some(value);
    }
    for (index, param) in params.iter().enumerate().take(required) {
        if values[index].is_none() {
            return Err(format!(
                "TypeError: {name}() missing required argument '{param}' (pos {})",
                index + 1
            ));
        }
    }
    Ok(values)
}

fn no_keywords(qualname: &str, keywords: &[(String, Value)]) -> Result<(), String> {
    if keywords.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "TypeError: {qualname}() takes no keyword arguments"
        ))
    }
}

// ---------------------------------------------------------------------------
// Module object
// ---------------------------------------------------------------------------

/// `re.RegexFlag` methods, which bind to flag values like Python methods.
pub(crate) fn is_re_flag_method(name: &str) -> bool {
    name.starts_with("re.RegexFlag.")
}

pub(crate) fn is_re_builtin(name: &str) -> bool {
    name.starts_with(RE_PREFIX)
}

/// `re.Pattern` and `re.Match` type objects.
pub(crate) fn is_re_type_name(name: &str) -> bool {
    matches!(name, "re.Pattern" | "re.Match" | "_sre.SRE_Scanner")
}

/// Attributes of the `re.Pattern` and `re.Match` type objects.
pub(crate) fn type_attribute(type_name: &str, name: &str) -> Option<Value> {
    if !is_re_type_name(type_name) {
        return None;
    }
    let short = type_name.rsplit('.').next().unwrap_or(type_name);
    match name {
        "__name__" | "__qualname__" => {
            let short = if type_name == "_sre.SRE_Scanner" {
                "SRE_Scanner"
            } else {
                short
            };
            Some(Value::String(short.to_string()))
        }
        "__module__" => Some(Value::String(
            if type_name == "_sre.SRE_Scanner" {
                "_sre"
            } else {
                "re"
            }
            .to_string(),
        )),
        "__doc__" => Some(Value::String(
            match type_name {
                "re.Pattern" => PATTERN_DOC,
                "re.Match" => MATCH_DOC,
                _ => "",
            }
            .to_string(),
        )),
        "__class_getitem__" => Some(Value::BoundMethod {
            function: Box::new(Value::Builtin(format!(
                "{RE_PREFIX}{short}.__class_getitem__"
            ))),
            receiver: Box::new(Value::Builtin(type_name.to_string())),
            identity: Rc::new(()),
        }),
        name if type_name == "re.Pattern" && PATTERN_METHODS.contains(&name) => {
            Some(Value::Builtin(format!("{RE_PREFIX}Pattern.{name}")))
        }
        name if type_name == "re.Match" && MATCH_METHODS.contains(&name) => {
            Some(Value::Builtin(format!("{RE_PREFIX}Match.{name}")))
        }
        _ => None,
    }
}

/// `isinstance(value, re.Pattern)` and friends.
pub(crate) fn value_is_instance(type_name: &str, value: &Value) -> bool {
    match (type_name, value) {
        ("re.Pattern", Value::Re(ReObject::Pattern(_))) => true,
        ("re.Match", Value::Re(ReObject::Match(_))) => true,
        ("_sre.SRE_Scanner", Value::Re(ReObject::Scanner(_))) => true,
        _ => false,
    }
}

fn flag_value(flag_class: &Value, value: i64) -> Value {
    let Value::Class {
        name, attrs, bases, ..
    } = flag_class
    else {
        return Value::Number(value);
    };
    let fields = new_scope();
    fields
        .borrow_mut()
        .insert(INT_SUBCLASS_STORAGE_FIELD.to_string(), Value::Number(value));
    Value::Instance {
        class_name: name.clone(),
        fields,
        class_attrs: attrs.clone(),
        class_bases: bases.clone(),
    }
}

fn regex_flag_class() -> Value {
    let attrs = new_scope();
    {
        let mut values = attrs.borrow_mut();
        let mut order = Vec::new();
        let mut insert = |key: &str, value: Value| {
            order.push(Value::String(key.to_string()));
            values.insert(key.to_string(), value);
        };
        insert("__module__", Value::String("re".to_string()));
        insert("__qualname__", Value::String("RegexFlag".to_string()));
        insert("__doc__", Value::None);
        for method in [
            "__repr__",
            "__str__",
            "__format__",
            "__or__",
            "__ror__",
            "__and__",
            "__rand__",
            "__xor__",
            "__rxor__",
            "__invert__",
        ] {
            insert(
                method,
                Value::Builtin(format!("{RE_PREFIX}RegexFlag.{method}")),
            );
        }
        for property in ["name", "value"] {
            insert(
                property,
                Value::Property {
                    fget: Some(Rc::new(Value::Builtin(format!(
                        "{RE_PREFIX}RegexFlag.{property}"
                    )))),
                    fset: None,
                    fdel: None,
                    doc: Rc::new(RefCell::new(None)),
                    doc_from_getter: false,
                    name: Rc::new(RefCell::new(Some(Value::String(property.to_string())))),
                    identity: Rc::new(()),
                },
            );
        }
        values.insert(RE_FLAG_MARKER.to_string(), Value::Bool(true));
        values.insert(
            CLASS_QUALNAME_ATTR.to_string(),
            Value::String("RegexFlag".to_string()),
        );
        values.insert(CLASS_ATTR_ORDER_ATTR.to_string(), tuple_value(order));
    }
    Value::Class {
        name: "RegexFlag".to_string(),
        type_params: Vec::new(),
        metaclass: None,
        bases: vec![builtin_type_value("int")],
        attrs,
    }
}

fn pattern_error_class() -> Value {
    let attrs = new_scope();
    {
        let mut values = attrs.borrow_mut();
        let order = vec![
            Value::String("__module__".to_string()),
            Value::String("__doc__".to_string()),
        ];
        values.insert("__module__".to_string(), Value::String("re".to_string()));
        values.insert(
            "__doc__".to_string(),
            Value::String(PATTERN_ERROR_DOC.to_string()),
        );
        values.insert(
            CLASS_QUALNAME_ATTR.to_string(),
            Value::String("PatternError".to_string()),
        );
        values.insert(RE_PATTERN_ERROR_MARKER.to_string(), Value::Bool(true));
        values.insert(CLASS_ATTR_ORDER_ATTR.to_string(), tuple_value(order));
    }
    Value::Class {
        name: "PatternError".to_string(),
        type_params: Vec::new(),
        metaclass: None,
        bases: vec![builtin_type_value("Exception")],
        attrs,
    }
}

/// Builds the `re` module value.
pub(crate) fn re_module_value() -> Value {
    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        let mut insert = |key: &str, value: Value| {
            values.insert(key.to_string(), value);
        };
        insert("__name__", Value::String("re".to_string()));
        insert("__doc__", Value::String(MODULE_DOC.to_string()));
        insert("__package__", Value::String("re".to_string()));
        insert("__version__", Value::String("2.2.1".to_string()));
        insert(
            "__all__",
            list_value(
                MODULE_ALL
                    .iter()
                    .map(|name| Value::String((*name).to_string()))
                    .collect(),
            ),
        );
        for function in MODULE_FUNCTIONS {
            insert(function, Value::Builtin(format!("{RE_PREFIX}{function}")));
        }
        insert("Pattern", Value::Builtin("re.Pattern".to_string()));
        insert("Match", Value::Builtin("re.Match".to_string()));
        let error = pattern_error_class();
        insert("PatternError", error.clone());
        insert("error", error);
        let flag_class = regex_flag_class();
        insert("NOFLAG", flag_value(&flag_class, 0));
        // `re.I is re.IGNORECASE`: one object per flag.
        let mut members: Vec<(u32, Value)> = Vec::new();
        for (name, value) in FLAG_NAMES.iter().chain(FLAG_LETTERS) {
            let member = match members.iter().find(|(bits, _)| bits == value) {
                Some((_, member)) => member.clone(),
                None => {
                    let member = flag_value(&flag_class, i64::from(*value));
                    members.push((*value, member.clone()));
                    member
                }
            };
            insert(name, member);
        }
        insert("RegexFlag", flag_class);
        insert("_MAXCACHE", Value::Number(MAXCACHE as i64));
        insert("_MAXCACHE2", Value::Number(256));
    }
    Value::Module {
        name: "re".to_string(),
        attrs: scope,
    }
}

/// `~flag` for a `RegexFlag` value (`None` for other values).
pub(crate) fn invert_flag(value: &Value) -> Option<Value> {
    let Value::Instance {
        class_name,
        class_attrs,
        class_bases,
        ..
    } = value
    else {
        return None;
    };
    if !class_attrs.borrow().contains_key(RE_FLAG_MARKER) {
        return None;
    }
    let Some(Value::Number(receiver)) = int_subclass_integer(value) else {
        return None;
    };
    let all_bits = (1i64 << 9) - 1;
    let inverted = if receiver >= 0 {
        all_bits ^ receiver
    } else {
        !receiver
    };
    let class = Value::Class {
        name: class_name.clone(),
        type_params: Vec::new(),
        metaclass: None,
        bases: class_bases.clone(),
        attrs: class_attrs.clone(),
    };
    Some(flag_value(&class, inverted))
}

/// `str(flag)`/`repr(flag)` of a `RegexFlag` value.
fn regex_flag_repr(value: i64) -> String {
    if value == 0 {
        return "re.NOFLAG".to_string();
    }
    if value < 0 {
        return format!("re.RegexFlag({value})");
    }
    let mut rest = value;
    let mut names = Vec::new();
    for (name, bit) in FLAG_NAMES {
        let bit = i64::from(*bit);
        if rest & bit != 0 {
            names.push(format!("re.{name}"));
            rest &= !bit;
        }
    }
    if names.is_empty() {
        return format!("re.RegexFlag({value})");
    }
    if rest != 0 {
        names.push(format!("{rest:#x}"));
    }
    names.join("|")
}

fn regex_flag_name(value: i64) -> Value {
    if value == 0 {
        return Value::String("NOFLAG".to_string());
    }
    let mut rest = value;
    let mut names = Vec::new();
    for (name, bit) in FLAG_NAMES {
        let bit = i64::from(*bit);
        if rest & bit != 0 {
            names.push((*name).to_string());
            rest &= !bit;
        }
    }
    if names.is_empty() || rest != 0 {
        return Value::None;
    }
    Value::String(names.join("|"))
}

// ---------------------------------------------------------------------------
// Template (replacement string) parsing
// ---------------------------------------------------------------------------

/// A parsed replacement template: literals between group references.
struct Template {
    /// The first literal, then (group, literal) pairs.
    first: Value,
    items: Vec<(usize, Option<Value>)>,
}

enum TemplateError {
    Pattern { msg: String, pos: usize },
    Other(String),
}

fn literal_value(points: &[u32], is_text: bool) -> Value {
    if is_text {
        Value::String(code_points_to_string(points))
    } else {
        bytes_value(points.iter().map(|&point| point as u8).collect())
    }
}

/// Parses `\1`, `\g<name>`, and escapes in a replacement template given as
/// code points (`parse_template`).
fn parse_template(
    chars: &[u32],
    is_text: bool,
    groups: usize,
    groupindex: &[(String, usize)],
) -> Result<Template, TemplateError> {
    let mut index = 0;
    let mut literal: Vec<u32> = Vec::new();
    let mut first: Option<Value> = None;
    let mut items: Vec<(usize, Option<Value>)> = Vec::new();
    let error_text = |msg: String| {
        if is_text {
            msg
        } else {
            msg.chars()
                .map(|ch| {
                    if (ch as u32) < 0x80 {
                        ch.to_string()
                    } else {
                        format!("\\x{:02x}", ch as u32)
                    }
                })
                .collect()
        }
    };
    let digit =
        |point: Option<&u32>| point.is_some_and(|&p| (b'0' as u32..=b'9' as u32).contains(&p));
    let octdigit =
        |point: Option<&u32>| point.is_some_and(|&p| (b'0' as u32..=b'7' as u32).contains(&p));
    let push_group = |group: usize,
                      literal: &mut Vec<u32>,
                      first: &mut Option<Value>,
                      items: &mut Vec<(usize, Option<Value>)>| {
        let value = literal_value(literal, is_text);
        literal.clear();
        match items.last_mut() {
            None if first.is_none() => *first = Some(value),
            Some(last) => {
                last.1 = Some(value);
            }
            None => {}
        }
        items.push((group, None));
    };
    if chars.last() == Some(&(b'\\' as u32)) {
        // A trailing backslash is reported before any other escape.
        let mut probe = 0;
        while probe < chars.len() {
            if chars[probe] == b'\\' as u32 {
                probe += 2;
            } else {
                probe += 1;
            }
        }
        if probe == chars.len() + 1 {
            return Err(TemplateError::Pattern {
                msg: "bad escape (end of pattern)".to_string(),
                pos: chars.len() - 1,
            });
        }
    }
    while index < chars.len() {
        let ch = chars[index];
        if ch != b'\\' as u32 {
            literal.push(ch);
            index += 1;
            continue;
        }
        let Some(&c) = chars.get(index + 1) else {
            return Err(TemplateError::Pattern {
                msg: "bad escape (end of pattern)".to_string(),
                pos: chars.len() - 1,
            });
        };
        let escape_start = index;
        index += 2;
        if c == b'g' as u32 {
            if chars.get(index) != Some(&(b'<' as u32)) {
                return Err(TemplateError::Pattern {
                    msg: "missing <".to_string(),
                    pos: index,
                });
            }
            index += 1;
            let name_start = index;
            loop {
                match chars.get(index) {
                    None => {
                        if index == name_start {
                            return Err(TemplateError::Pattern {
                                msg: "missing group name".to_string(),
                                pos: index,
                            });
                        }
                        return Err(TemplateError::Pattern {
                            msg: "missing >, unterminated name".to_string(),
                            pos: name_start,
                        });
                    }
                    Some(&p) if p == b'>' as u32 => break,
                    Some(&p) if p == b'\\' as u32 && index + 1 < chars.len() => index += 2,
                    Some(_) => index += 1,
                }
            }
            let name = &chars[name_start..index];
            index += 1;
            if name.is_empty() {
                return Err(TemplateError::Pattern {
                    msg: "missing group name".to_string(),
                    pos: index - 1,
                });
            }
            let group = if name
                .iter()
                .all(|&p| (b'0' as u32..=b'9' as u32).contains(&p))
            {
                let value = name.iter().fold(0u128, |value, &digit| {
                    value
                        .saturating_mul(10)
                        .saturating_add(u128::from(digit - b'0' as u32))
                });
                if value >= parser::MAXGROUPS as u128 {
                    return Err(TemplateError::Pattern {
                        msg: format!("invalid group reference {value}"),
                        pos: name_start,
                    });
                }
                value as usize
            } else {
                let text = code_points_to_string(name);
                if !is_text && !name.iter().all(|&p| p < 0x80) {
                    return Err(TemplateError::Pattern {
                        msg: error_text(format!(
                            "bad character in group name {}",
                            ascii_repr(&text)
                        )),
                        pos: name_start,
                    });
                }
                if !is_python_identifier(&text) {
                    return Err(TemplateError::Pattern {
                        msg: error_text(format!(
                            "bad character in group name {}",
                            crate::value::repr_string(&text)
                        )),
                        pos: name_start,
                    });
                }
                match groupindex.iter().find(|(key, _)| *key == text) {
                    Some((_, group)) => *group,
                    None => {
                        return Err(TemplateError::Other(format!(
                            "IndexError: unknown group name {}",
                            crate::value::repr_string(&text)
                        )));
                    }
                }
            };
            if group > groups {
                return Err(TemplateError::Pattern {
                    msg: format!("invalid group reference {group}"),
                    pos: name_start,
                });
            }
            push_group(group, &mut literal, &mut first, &mut items);
        } else if c == b'0' as u32 {
            let mut value = 0u32;
            let mut digits = 0;
            while digits < 2 && octdigit(chars.get(index)) {
                value = value * 8 + (chars[index] - b'0' as u32);
                index += 1;
                digits += 1;
            }
            literal.push(value & 0xff);
        } else if (b'1' as u32..=b'9' as u32).contains(&c) {
            let mut text = vec![c];
            let mut octal = false;
            if digit(chars.get(index)) {
                text.push(chars[index]);
                index += 1;
                if (b'0' as u32..=b'7' as u32).contains(&c)
                    && (b'0' as u32..=b'7' as u32).contains(&text[1])
                    && octdigit(chars.get(index))
                {
                    text.push(chars[index]);
                    index += 1;
                    octal = true;
                    let value = text
                        .iter()
                        .fold(0u32, |value, &digit| value * 8 + (digit - b'0' as u32));
                    if value > 0o377 {
                        return Err(TemplateError::Pattern {
                            msg: format!(
                                "octal escape value \\{} outside of range 0-0o377",
                                code_points_to_string(&text)
                            ),
                            pos: escape_start,
                        });
                    }
                    literal.push(value);
                }
            }
            if !octal {
                let group = text.iter().fold(0usize, |value, &digit| {
                    value * 10 + (digit - b'0' as u32) as usize
                });
                if group > groups {
                    return Err(TemplateError::Pattern {
                        msg: format!("invalid group reference {group}"),
                        pos: escape_start + 1,
                    });
                }
                push_group(group, &mut literal, &mut first, &mut items);
            }
        } else {
            let simple = match char::from_u32(c) {
                Some('a') => Some(7),
                Some('b') => Some(8),
                Some('f') => Some(12),
                Some('n') => Some(10),
                Some('r') => Some(13),
                Some('t') => Some(9),
                Some('v') => Some(11),
                Some('\\') => Some(92),
                _ => None,
            };
            match simple {
                Some(value) => literal.push(value),
                None => {
                    if char::from_u32(c).is_some_and(|ch| ch.is_ascii_alphabetic()) {
                        let text = format!("\\{}", char::from_u32(c).expect("ascii letter"));
                        return Err(TemplateError::Pattern {
                            msg: format!("bad escape {text}"),
                            pos: escape_start,
                        });
                    }
                    literal.push(b'\\' as u32);
                    literal.push(c);
                }
            }
        }
    }
    let value = literal_value(&literal, is_text);
    match items.last_mut() {
        Some(last) => last.1 = Some(value),
        None => first = Some(value),
    }
    let first = first.unwrap_or_else(|| literal_value(&[], is_text));
    // Empty literals between groups are dropped, as CPython's template does.
    let items = items
        .into_iter()
        .map(|(group, literal)| {
            let literal = literal.filter(|value| match value {
                Value::String(text) => !text.is_empty(),
                Value::Bytes(bytes) => !bytes.is_empty(),
                _ => true,
            });
            (group, literal)
        })
        .collect();
    Ok(Template { first, items })
}

fn ascii_repr(text: &str) -> String {
    let repr = crate::value::repr_string(text);
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

fn is_python_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first == '_' || unicode_ident::is_xid_start(first))
        && chars.all(|ch| ch == '_' || unicode_ident::is_xid_continue(ch))
}

/// How `sub` produces replacements.
enum Replacement {
    /// A template without backslashes, inserted as is.
    Literal(Value),
    Template(Template),
    Callable(Value),
}

// ---------------------------------------------------------------------------
// VM integration
// ---------------------------------------------------------------------------

/// A search outcome: span and group spans.
struct Found {
    start: usize,
    end: usize,
    groups: Vec<(usize, usize)>,
    lastindex: isize,
}

macro_rules! with_text {
    ($subject:expr, $text:ident => $body:expr) => {
        match &$subject.text {
            SubjectText::Bytes(bytes) => {
                let $text: &[u8] = bytes.as_slice();
                $body
            }
            SubjectText::Ascii(string) => {
                let $text: &[u8] = string.as_bytes();
                $body
            }
            SubjectText::Wide(chars) => {
                let $text: &[u32] = chars.as_slice();
                $body
            }
        }
    };
}

fn found_from<T: Text + ?Sized>(matcher: &Matcher<'_, T>) -> Result<Found, String> {
    Ok(Found {
        start: matcher.start,
        end: matcher.ptr,
        groups: matcher.group_spans()?,
        lastindex: matcher.lastindex,
    })
}

#[derive(Clone, Copy, PartialEq)]
enum MatchMode {
    Match,
    FullMatch,
    Search,
}

impl Vm {
    fn re_meter(&self) -> BudgetMeter {
        BudgetMeter {
            instructions: self.instruction_budget.clone(),
            allocation: self.allocation_budget.clone(),
        }
    }

    /// `obj[index]` for `re` objects and `re.Pattern[...]`/`re.Match[...]`.
    pub(super) fn re_subscript(
        &mut self,
        object: &Value,
        index: &Value,
    ) -> Option<Result<Value, String>> {
        match object {
            Value::Re(ReObject::Match(_)) => Some(self.re_match_method(
                "__getitem__",
                vec![object.clone(), index.clone()],
                Vec::new(),
            )),
            Value::Re(other) => Some(Err(format!(
                "TypeError: '{}' object is not subscriptable",
                other.type_name()
            ))),
            Value::Builtin(name) if matches!(name.as_str(), "re.Pattern" | "re.Match") => {
                Some(self.re_class_getitem(vec![object.clone(), index.clone()], Vec::new()))
            }
            _ => None,
        }
    }

    pub(super) fn call_re_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if is_re_type_name(name) {
            return Err(format!("TypeError: cannot create '{name}' instances"));
        }
        let rest = name.strip_prefix(RE_PREFIX).unwrap_or(name);
        match rest.split_once('.') {
            None => self.re_module_function(rest, args, keywords),
            Some(("Pattern", method)) => self.re_pattern_method(method, args, keywords),
            Some(("Match", method)) => self.re_match_method(method, args, keywords),
            Some(("Scanner", method)) => self.re_scanner_method(method, args, keywords),
            Some(("RegexFlag", method)) => self.re_flag_method(method, args, keywords),
            _ => Err(format!("unknown builtin: {name}")),
        }
    }

    /// The live `re.PatternError` class.
    fn re_error_class(&mut self) -> Result<Value, String> {
        let module = match lookup_string_key(&self.module_cache, "re") {
            Some(module @ Value::Module { .. }) => module,
            _ => self.load_stdlib_module("re")?,
        };
        self.load_attribute_value(module, "PatternError")
    }

    fn re_flag_class(&mut self) -> Result<Value, String> {
        let module = match lookup_string_key(&self.module_cache, "re") {
            Some(module @ Value::Module { .. }) => module,
            _ => self.load_stdlib_module("re")?,
        };
        self.load_attribute_value(module, "RegexFlag")
    }

    /// Raises `re.PatternError(msg, pattern, pos)`.
    fn re_raise_pattern_error<T>(
        &mut self,
        msg: String,
        pattern: Value,
        pos: Option<usize>,
    ) -> Result<T, String> {
        let class = self.re_error_class()?;
        let error = build_pattern_error(
            class,
            Value::String(msg),
            pattern,
            pos.map(|pos| Value::Number(pos as i64)),
        )?;
        let exception = exception_from_value(error)?;
        self.raise_exception(exception, true)
    }

    fn re_raise_error<T>(&mut self, error: ReError, pattern: &Value) -> Result<T, String> {
        match error {
            // Errors found after parsing (look-behind widths) carry no
            // pattern or position, as in CPython's compiler.
            ReError::Pattern { msg, pos: None } => {
                self.re_raise_pattern_error(msg, Value::None, None)
            }
            ReError::Pattern { msg, pos } => self.re_raise_pattern_error(msg, pattern.clone(), pos),
            ReError::Other(message) => Err(message),
        }
    }

    /// `re.PatternError(...)` called from Python.
    pub(super) fn call_re_pattern_error(
        &mut self,
        class: Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let bound = super::argparse::bind_args(
            &super::argparse::Sig {
                qualname: "PatternError.__init__",
                params: &["msg", "pattern", "pos"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let msg = bound.arg(0);
        let pattern = bound.arg(1);
        let pos = bound.arg(2);
        let pos = if matches!(pos, Value::None) {
            None
        } else {
            Some(pos)
        };
        build_pattern_error_checked(self, class, msg, pattern, pos)
    }

    /// Converts the `flags` argument to the flag bits.
    fn re_flags(&mut self, flags: &Value) -> Result<i64, String> {
        let value = match flags {
            Value::Bool(value) => i64::from(*value),
            Value::Number(value) => *value,
            Value::BigInt(_) => {
                return Err("OverflowError: Python int too large to convert to C int".to_string());
            }
            value => match int_subclass_integer(value) {
                Some(Value::Number(value)) => value,
                Some(_) => {
                    return Err(
                        "OverflowError: Python int too large to convert to C int".to_string()
                    );
                }
                None => {
                    return Err(format!(
                        "TypeError: unsupported operand type(s) for &: '{}' and 'int'",
                        type_name(value)
                    ));
                }
            },
        };
        Ok(value)
    }

    /// `re._compile(pattern, flags)`.
    fn re_compile(&mut self, pattern: Value, flags: Value) -> Result<Rc<PatternObject>, String> {
        if let Value::Re(ReObject::Pattern(compiled)) = &pattern {
            if self.truth_value(flags)? {
                return Err(
                    "ValueError: cannot process flags argument with a compiled pattern".to_string(),
                );
            }
            return Ok(compiled.clone());
        }
        for key in [&pattern, &flags] {
            if !is_hashable_key(key) {
                return Err(format!(
                    "TypeError: cannot use 'tuple' as a dict key (unhashable type: '{}')",
                    type_name(key)
                ));
            }
        }
        let (text, exact) = match &pattern {
            Value::String(text) | Value::IdentityString { value: text, .. } => {
                (PatternText::Str(text.clone()), true)
            }
            Value::Bytes(bytes) => (PatternText::Bytes(bytes.as_ref().clone()), true),
            value => match str_subclass_string(value) {
                Some(text) => (PatternText::Str(text), false),
                None => match bytes_subclass_bytes(value) {
                    Some(bytes) => (PatternText::Bytes(bytes), false),
                    None => {
                        return Err(
                            "TypeError: first argument must be string or compiled pattern"
                                .to_string(),
                        );
                    }
                },
            },
        };
        let flag_bits = self.re_flags(&flags)?;
        let key = (text.clone(), flag_bits);
        if exact && let Some(cached) = cache_lookup(&key) {
            return Ok(cached);
        }
        let (chars, is_text): (Vec<u32>, bool) = match &text {
            PatternText::Str(text) => (text.chars().map(u32::from).collect(), true),
            PatternText::Bytes(bytes) => (bytes.iter().map(|&b| u32::from(b)).collect(), false),
        };
        let call_depth = self.frame_stack.borrow().len();
        let recursion_limit = self.frame_stack.recursion_limit.get();
        let mut warnings = Vec::new();
        let parsed = parser::parse(
            &chars,
            is_text,
            flag_bits as u32,
            call_depth,
            recursion_limit,
            &mut warnings,
        );
        // CPython warns while parsing, before any later syntax error.
        for warning in warnings {
            self.emit_warning(warning, builtin_type_value("FutureWarning"))?;
        }
        let parsed = match parsed {
            Ok(parsed) => parsed,
            Err(error) => return self.re_raise_error(error, &pattern),
        };
        let mut meter = self.re_meter();
        let program = match compiler::compile(
            &parsed.seq,
            parsed.flags,
            parsed.groups,
            &parsed.group_widths,
            &mut meter,
        ) {
            Ok(program) => program,
            Err(error) => return self.re_raise_error(error, &pattern),
        };
        let final_flags = flag_bits | i64::from(parsed.flags);
        if final_flags > i64::from(i32::MAX) || final_flags < i64::from(i32::MIN) {
            return Err("OverflowError: Python int too large to convert to C int".to_string());
        }
        self.consume_allocation_budget(program.heap_size())?;
        let compiled = Rc::new(PatternObject {
            source: pattern,
            text,
            flags: final_flags,
            program,
            groupindex: parsed.groupindex,
        });
        if exact && final_flags & i64::from(flags::DEBUG) == 0 {
            cache_store(key, compiled.clone());
        }
        Ok(compiled)
    }

    /// The subject of a match: checks its type against the pattern's.
    fn re_subject(&mut self, pattern: &PatternObject, string: Value) -> Result<Subject, String> {
        let pattern_is_text = matches!(pattern.text, PatternText::Str(_));
        let text = match &string {
            Value::String(text) | Value::IdentityString { value: text, .. } => Some(text.clone()),
            value => str_subclass_string(value),
        };
        if let Some(text) = text {
            if !pattern_is_text {
                return Err(
                    "TypeError: cannot use a bytes pattern on a string-like object".to_string(),
                );
            }
            let text = if text.is_ascii() {
                SubjectText::Ascii(text)
            } else {
                SubjectText::Wide(text.chars().map(u32::from).collect())
            };
            return Ok(Subject {
                original: string,
                text,
            });
        }
        let bytes = match &string {
            Value::Bytes(bytes) => Some(bytes.clone()),
            value => bytes_buffer_value_bytes(value)?.map(Rc::new),
        };
        let Some(bytes) = bytes else {
            return Err(format!(
                "TypeError: expected string or bytes-like object, got '{}'",
                type_name(&string)
            ));
        };
        if pattern_is_text {
            return Err(
                "TypeError: cannot use a string pattern on a bytes-like object".to_string(),
            );
        }
        Ok(Subject {
            original: string,
            text: SubjectText::Bytes(bytes),
        })
    }

    /// A `pos`/`endpos` argument (a signed machine-size integer).
    fn re_index(&mut self, value: Value) -> Result<i64, String> {
        match self.index_integer_value(value)? {
            Value::Number(value) => Ok(value),
            _ => Err("OverflowError: Python int too large to convert to C ssize_t".to_string()),
        }
    }

    fn re_bounds(
        &mut self,
        subject: &Subject,
        pos: Option<Value>,
        endpos: Option<Value>,
    ) -> Result<(usize, usize), String> {
        let length = subject.len() as i64;
        let pos = match pos {
            Some(value) => self.re_index(value)?,
            None => 0,
        };
        let endpos = match endpos {
            Some(value) => self.re_index(value)?,
            None => i64::MAX,
        };
        Ok((
            pos.clamp(0, length) as usize,
            endpos.clamp(0, length) as usize,
        ))
    }

    fn re_run(
        &mut self,
        pattern: &PatternObject,
        subject: &Subject,
        pos: usize,
        endpos: usize,
        mode: MatchMode,
    ) -> Result<Option<Found>, String> {
        let mut meter = self.re_meter();
        with_text!(subject, text => {
            let mut matcher = Matcher::new(&pattern.program, text, endpos);
            let matched = match mode {
                MatchMode::Search => matcher.search(pos, &mut meter)?,
                MatchMode::Match => matcher.match_at(pos, &mut meter)?,
                MatchMode::FullMatch => {
                    matcher.match_all = true;
                    matcher.match_at(pos, &mut meter)?
                }
            };
            if matched { Ok(Some(found_from(&matcher)?)) } else { Ok(None) }
        })
    }

    fn re_match_value(
        &mut self,
        pattern: Rc<PatternObject>,
        subject: Rc<Subject>,
        pos: usize,
        endpos: usize,
        found: Found,
    ) -> Value {
        let mut spans = Vec::with_capacity(found.groups.len() + 1);
        spans.push((found.start, found.end));
        spans.extend(found.groups);
        Value::Re(ReObject::Match(Rc::new(MatchObject {
            pattern,
            subject,
            pos,
            endpos,
            spans,
            lastindex: found.lastindex,
        })))
    }

    fn re_module_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        use super::argparse::{Sig, bind_args};
        let sig = |qualname: &'static str, params: &'static [&'static str], required: usize| Sig {
            qualname,
            params,
            required,
            kwonly: &[],
            varargs: false,
            varkw: false,
        };
        match name {
            "compile" => {
                let bound = bind_args(&sig("compile", &["pattern", "flags"], 1), args, keywords)?;
                let pattern = self.re_compile(bound.arg(0), bound.or(1, Value::Number(0)))?;
                Ok(Value::Re(ReObject::Pattern(pattern)))
            }
            "search" | "match" | "fullmatch" | "findall" | "finditer" => {
                let qualname: &'static str = match name {
                    "search" => "search",
                    "match" => "match",
                    "fullmatch" => "fullmatch",
                    "findall" => "findall",
                    _ => "finditer",
                };
                let bound = bind_args(
                    &sig(qualname, &["pattern", "string", "flags"], 2),
                    args,
                    keywords,
                )?;
                let pattern = self.re_compile(bound.arg(0), bound.or(2, Value::Number(0)))?;
                let receiver = Value::Re(ReObject::Pattern(pattern));
                self.re_pattern_method(name, vec![receiver, bound.arg(1)], Vec::new())
            }
            "sub" | "subn" => {
                let qualname: &'static str = if name == "sub" { "sub" } else { "subn" };
                let bound = bind_args(
                    &Sig {
                        qualname,
                        params: &["pattern", "repl", "string"],
                        required: 3,
                        kwonly: &["count", "flags"],
                        varargs: true,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                let mut count = bound.get(3);
                let mut flags = bound.get(4);
                let extra = bound.extra_args.clone();
                if !extra.is_empty() {
                    let mut extra = extra.into_iter();
                    if count.is_some() {
                        return Err(format!(
                            "TypeError: {qualname}() got multiple values for argument 'count'"
                        ));
                    }
                    count = extra.next();
                    if let Some(value) = extra.next() {
                        if flags.is_some() {
                            return Err(format!(
                                "TypeError: {qualname}() got multiple values for argument 'flags'"
                            ));
                        }
                        flags = Some(value);
                        let more = extra.count();
                        if more > 0 {
                            return Err(format!(
                                "TypeError: {qualname}() takes from 3 to 5 positional arguments but {} were given",
                                5 + more
                            ));
                        }
                    }
                    self.emit_deprecation_warning(
                        "'count' is passed as positional argument".to_string(),
                    )?;
                }
                let pattern = self.re_compile(bound.arg(0), flags.unwrap_or(Value::Number(0)))?;
                let receiver = Value::Re(ReObject::Pattern(pattern));
                self.re_pattern_method(
                    name,
                    vec![
                        receiver,
                        bound.arg(1),
                        bound.arg(2),
                        count.unwrap_or(Value::Number(0)),
                    ],
                    Vec::new(),
                )
            }
            "split" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "split",
                        params: &["pattern", "string"],
                        required: 2,
                        kwonly: &["maxsplit", "flags"],
                        varargs: true,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                let mut maxsplit = bound.get(2);
                let mut flags = bound.get(3);
                let extra = bound.extra_args.clone();
                if !extra.is_empty() {
                    let mut extra = extra.into_iter();
                    if maxsplit.is_some() {
                        return Err(
                            "TypeError: split() got multiple values for argument 'maxsplit'"
                                .to_string(),
                        );
                    }
                    maxsplit = extra.next();
                    if let Some(value) = extra.next() {
                        if flags.is_some() {
                            return Err(
                                "TypeError: split() got multiple values for argument 'flags'"
                                    .to_string(),
                            );
                        }
                        flags = Some(value);
                        let more = extra.count();
                        if more > 0 {
                            return Err(format!(
                                "TypeError: split() takes from 2 to 4 positional arguments but {} were given",
                                4 + more
                            ));
                        }
                    }
                    self.emit_deprecation_warning(
                        "'maxsplit' is passed as positional argument".to_string(),
                    )?;
                }
                let pattern = self.re_compile(bound.arg(0), flags.unwrap_or(Value::Number(0)))?;
                let receiver = Value::Re(ReObject::Pattern(pattern));
                self.re_pattern_method(
                    "split",
                    vec![receiver, bound.arg(1), maxsplit.unwrap_or(Value::Number(0))],
                    Vec::new(),
                )
            }
            "escape" => {
                let bound = bind_args(&sig("escape", &["pattern"], 1), args, keywords)?;
                re_escape(bound.arg(0))
            }
            "purge" => {
                bind_args(&sig("purge", &[], 0), args, keywords)?;
                cache_purge();
                Ok(Value::None)
            }
            _ => Err(format!(
                "AttributeError: module 're' has no attribute '{name}'"
            )),
        }
    }

    fn re_pattern_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if method == "__class_getitem__" {
            return self.re_class_getitem(args, keywords);
        }
        let receiver = if args.is_empty() {
            Value::None
        } else {
            args.remove(0)
        };
        let Value::Re(ReObject::Pattern(pattern)) = receiver else {
            return Err(format!(
                "TypeError: descriptor '{method}' for 're.Pattern' objects doesn't apply to a '{}' object",
                type_name(&receiver)
            ));
        };
        match method {
            "match" | "fullmatch" | "search" => {
                let values = bind_clinic(method, &["string", "pos", "endpos"], 1, args, keywords)?;
                let mut values = values.into_iter();
                let string = values.next().flatten().expect("required");
                let (pos, endpos) = (values.next().flatten(), values.next().flatten());
                let subject = self.re_subject(&pattern, string)?;
                let (pos, endpos) = self.re_bounds(&subject, pos, endpos)?;
                let mode = match method {
                    "match" => MatchMode::Match,
                    "fullmatch" => MatchMode::FullMatch,
                    _ => MatchMode::Search,
                };
                let subject = Rc::new(subject);
                match self.re_run(&pattern, &subject, pos, endpos, mode)? {
                    Some(found) => Ok(self.re_match_value(pattern, subject, pos, endpos, found)),
                    None => Ok(Value::None),
                }
            }
            "findall" => {
                let values = bind_clinic(method, &["string", "pos", "endpos"], 1, args, keywords)?;
                let mut values = values.into_iter();
                let string = values.next().flatten().expect("required");
                let (pos, endpos) = (values.next().flatten(), values.next().flatten());
                let subject = self.re_subject(&pattern, string)?;
                let (pos, endpos) = self.re_bounds(&subject, pos, endpos)?;
                self.re_findall(&pattern, &subject, pos, endpos)
            }
            "finditer" | "scanner" => {
                let values = bind_clinic(method, &["string", "pos", "endpos"], 1, args, keywords)?;
                let mut values = values.into_iter();
                let string = values.next().flatten().expect("required");
                let (pos, endpos) = (values.next().flatten(), values.next().flatten());
                let subject = self.re_subject(&pattern, string)?;
                let (pos, endpos) = self.re_bounds(&subject, pos, endpos)?;
                let scanner = Value::Re(ReObject::Scanner(Rc::new(ScannerObject {
                    pattern,
                    subject: Rc::new(subject),
                    pos,
                    endpos,
                    start: Cell::new(Some(pos)),
                    must_advance: Cell::new(false),
                })));
                if method == "scanner" {
                    return Ok(scanner);
                }
                Ok(shared_iterator(Value::CallIterator {
                    callable: Box::new(Value::BoundMethod {
                        function: Box::new(Value::Builtin(format!("{RE_PREFIX}Scanner.search"))),
                        receiver: Box::new(scanner),
                        identity: Rc::new(()),
                    }),
                    sentinel: Box::new(Value::None),
                    done: false,
                }))
            }
            "split" => {
                let values = bind_clinic(method, &["string", "maxsplit"], 1, args, keywords)?;
                let mut values = values.into_iter();
                let string = values.next().flatten().expect("required");
                let maxsplit = match values.next().flatten() {
                    Some(value) => self.re_index(value)?,
                    None => 0,
                };
                let subject = self.re_subject(&pattern, string)?;
                self.re_split(&pattern, &subject, maxsplit)
            }
            "sub" | "subn" => {
                let values = bind_clinic(method, &["repl", "string", "count"], 2, args, keywords)?;
                let mut values = values.into_iter();
                let repl = values.next().flatten().expect("required");
                let string = values.next().flatten().expect("required");
                let count = match values.next().flatten() {
                    Some(value) => self.re_index(value)?,
                    None => 0,
                };
                self.re_sub(pattern, repl, string, count, method == "subn")
            }
            "__copy__" => {
                if !args.is_empty() {
                    return Err(format!(
                        "TypeError: Pattern.__copy__() takes no arguments ({} given)",
                        args.len()
                    ));
                }
                no_keywords("Pattern.__copy__", &keywords)?;
                Ok(Value::Re(ReObject::Pattern(pattern)))
            }
            "__deepcopy__" => {
                no_keywords("Pattern.__deepcopy__", &keywords)?;
                if args.len() != 1 {
                    return Err(format!(
                        "TypeError: Pattern.__deepcopy__() takes exactly one argument ({} given)",
                        args.len()
                    ));
                }
                Ok(Value::Re(ReObject::Pattern(pattern)))
            }
            _ => Err(format!(
                "AttributeError: 're.Pattern' object has no attribute '{method}'"
            )),
        }
    }

    fn re_class_getitem(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        no_keywords("__class_getitem__", &keywords)?;
        let [owner, item] = <[Value; 2]>::try_from(args).map_err(|args| {
            format!(
                "TypeError: __class_getitem__() takes exactly one argument ({} given)",
                args.len().saturating_sub(1)
            )
        })?;
        let items = match item {
            Value::Tuple(items) => items.as_ref().clone(),
            item => vec![item],
        };
        Ok(generic_alias_value(owner, items))
    }

    fn re_findall(
        &mut self,
        pattern: &PatternObject,
        subject: &Subject,
        pos: usize,
        endpos: usize,
    ) -> Result<Value, String> {
        let groups = pattern.program.groups;
        let mut results = Vec::new();
        let mut meter = self.re_meter();
        with_text!(subject, text => {
            let mut matcher = Matcher::new(&pattern.program, text, endpos);
            let mut start = pos;
            while start <= endpos {
                if !matcher.search(start, &mut meter)? {
                    break;
                }
                let found = found_from(&matcher)?;
                let item = match groups {
                    0 => subject.slice(found.start, found.end),
                    1 => group_or_empty(subject, found.groups[0]),
                    _ => tuple_value(
                        found
                            .groups
                            .iter()
                            .map(|&span| group_or_empty(subject, span))
                            .collect(),
                    ),
                };
                results.push(item);
                matcher.must_advance = found.end == found.start;
                start = found.end;
            }
        });
        Ok(list_value(results))
    }

    fn re_split(
        &mut self,
        pattern: &PatternObject,
        subject: &Subject,
        maxsplit: i64,
    ) -> Result<Value, String> {
        let mut results = Vec::new();
        let mut meter = self.re_meter();
        let endpos = subject.len();
        with_text!(subject, text => {
            let mut matcher = Matcher::new(&pattern.program, text, endpos);
            let mut last = 0;
            let mut start = 0;
            let mut count: i64 = 0;
            while maxsplit == 0 || count < maxsplit {
                if !matcher.search(start, &mut meter)? {
                    break;
                }
                let found = found_from(&matcher)?;
                results.push(subject.slice(last, found.start));
                for &span in &found.groups {
                    results.push(group_or_none(subject, span));
                }
                count += 1;
                matcher.must_advance = found.end == found.start;
                last = found.end;
                start = found.end;
            }
            results.push(subject.slice(last, endpos));
        });
        Ok(list_value(results))
    }

    fn re_replacement(
        &mut self,
        pattern: &PatternObject,
        repl: Value,
    ) -> Result<Replacement, String> {
        if self.re_is_callable(&repl)? {
            return Ok(Replacement::Callable(repl));
        }
        let text = match &repl {
            Value::String(text) | Value::IdentityString { value: text, .. } => Some(text.clone()),
            value => str_subclass_string(value),
        };
        if let Some(text) = &text
            && !text.contains('\\')
        {
            return Ok(Replacement::Literal(repl));
        }
        if text.is_none()
            && let Some(bytes) = bytes_buffer_value_bytes(&repl)?
            && !bytes.contains(&b'\\')
        {
            return Ok(Replacement::Literal(repl));
        }
        self.re_compile_template(pattern, &repl)
            .map(Replacement::Template)
    }

    fn re_is_callable(&mut self, value: &Value) -> Result<bool, String> {
        let result =
            self.call_value(Value::Builtin("callable".to_string()), vec![value.clone()])?;
        self.truth_value(result)
    }

    /// Parses a replacement template for `pattern`.
    fn re_compile_template(
        &mut self,
        pattern: &PatternObject,
        repl: &Value,
    ) -> Result<Template, String> {
        let text = match repl {
            Value::String(text) | Value::IdentityString { value: text, .. } => Some(text.clone()),
            value => str_subclass_string(value),
        };
        let (chars, is_text): (Vec<u32>, bool) = match text {
            Some(text) => (text.chars().map(u32::from).collect(), true),
            None => match bytes_buffer_value_bytes(repl)? {
                Some(bytes) => (bytes.iter().map(|&b| u32::from(b)).collect(), false),
                None => {
                    return Err(format!(
                        "TypeError: decoding to str: need a bytes-like object, {} found",
                        type_name(repl)
                    ));
                }
            },
        };
        match parse_template(&chars, is_text, pattern.program.groups, &pattern.groupindex) {
            Ok(template) => Ok(template),
            Err(TemplateError::Pattern { msg, pos }) => {
                self.re_raise_pattern_error(msg, repl.clone(), Some(pos))
            }
            Err(TemplateError::Other(message)) => Err(message),
        }
    }

    /// Expands a template for one match.
    fn re_expand(&mut self, template: &Template, m: &MatchObject) -> Result<Value, String> {
        expand_template(template, &m.subject, &m.spans)
    }

    fn re_sub(
        &mut self,
        pattern: Rc<PatternObject>,
        repl: Value,
        string: Value,
        count: i64,
        subn: bool,
    ) -> Result<Value, String> {
        let replacement = self.re_replacement(&pattern, repl)?;
        let subject = Rc::new(self.re_subject(&pattern, string)?);
        let endpos = subject.len();
        let mut out = SubOutput::new(subject.is_bytes());
        let mut n: i64 = 0;
        let mut meter = self.re_meter();
        with_text!(subject, text => {
            let mut matcher = Matcher::new(&pattern.program, text, endpos);
            let mut last = 0;
            let mut start = 0;
            while (count == 0 || n < count) && start <= endpos {
                if !matcher.search(start, &mut meter)? {
                    break;
                }
                let (b, e) = (matcher.start, matcher.ptr);
                if last < b {
                    out.push_slice(&subject, last, b);
                }
                match &replacement {
                    Replacement::Literal(value) => out.push_item(value)?,
                    Replacement::Template(template) => {
                        let found = found_from(&matcher)?;
                        if template.is_bytes() == subject.is_bytes() {
                            out.push_template(template, &subject, (b, e), &found.groups);
                        } else {
                            let spans: Vec<(usize, usize)> =
                                std::iter::once((b, e)).chain(found.groups).collect();
                            let item = expand_template(template, &subject, &spans)?;
                            out.push_item(&item)?;
                        }
                    }
                    Replacement::Callable(callable) => {
                        let found = found_from(&matcher)?;
                        let m = self.re_match_value(pattern.clone(), subject.clone(), 0, endpos, found);
                        let result = self.call_value(callable.clone(), vec![m])?;
                        if !matches!(result, Value::None) {
                            out.push_item(&result)?;
                        }
                    }
                }
                last = e;
                n += 1;
                matcher.must_advance = e == b;
                start = e;
            }
            if last < endpos {
                out.push_slice(&subject, last, endpos);
            }
        });
        let joined = out.finish()?;
        if subn {
            Ok(tuple_value(vec![joined, Value::Number(n)]))
        } else {
            Ok(joined)
        }
    }

    fn re_match_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if method == "__class_getitem__" {
            return self.re_class_getitem(args, keywords);
        }
        let receiver = if args.is_empty() {
            Value::None
        } else {
            args.remove(0)
        };
        let Value::Re(ReObject::Match(m)) = receiver else {
            return Err(format!(
                "TypeError: descriptor '{method}' for 're.Match' objects doesn't apply to a '{}' object",
                type_name(&receiver)
            ));
        };
        match method {
            "group" => {
                no_keywords("Match.group", &keywords)?;
                match args.len() {
                    0 => Ok(self.re_group_value(&m, 0, Value::None)),
                    1 => {
                        let index = self.re_group_index(&m, args.pop().expect("one"))?;
                        Ok(self.re_group_value(&m, index, Value::None))
                    }
                    _ => {
                        let mut items = Vec::with_capacity(args.len());
                        for arg in args {
                            let index = self.re_group_index(&m, arg)?;
                            items.push(self.re_group_value(&m, index, Value::None));
                        }
                        Ok(tuple_value(items))
                    }
                }
            }
            "__getitem__" => {
                no_keywords("Match.__getitem__", &keywords)?;
                if args.len() != 1 {
                    return Err(format!(
                        "TypeError: expected 1 argument, got {}",
                        args.len()
                    ));
                }
                let index = self.re_group_index(&m, args.pop().expect("one"))?;
                Ok(self.re_group_value(&m, index, Value::None))
            }
            "groups" | "groupdict" => {
                if args.len() + keywords.len() > 1 {
                    if keywords.is_empty() {
                        return Err(format!(
                            "TypeError: {method}() takes at most 1 argument ({} given)",
                            args.len()
                        ));
                    }
                    return Err(format!(
                        "TypeError: {method}() takes at most 1 keyword argument ({} given)",
                        args.len() + keywords.len()
                    ));
                }
                let values = bind_clinic(method, &["default"], 0, args, keywords)?;
                let default = values.into_iter().next().flatten().unwrap_or(Value::None);
                if method == "groups" {
                    let items = (1..m.spans.len())
                        .map(|index| self.re_group_value(&m, index, default.clone()))
                        .collect();
                    Ok(tuple_value(items))
                } else {
                    let entries = m
                        .pattern
                        .groupindex
                        .iter()
                        .map(|(name, index)| {
                            (
                                Value::String(name.clone()),
                                self.re_group_value(&m, *index, default.clone()),
                            )
                        })
                        .collect();
                    Ok(dict_value(entries))
                }
            }
            "start" | "end" | "span" => {
                no_keywords(&format!("Match.{method}"), &keywords)?;
                if args.len() > 1 {
                    return Err(format!(
                        "TypeError: {method} expected at most 1 argument, got {}",
                        args.len()
                    ));
                }
                let index = match args.pop() {
                    Some(arg) => self.re_group_index(&m, arg)?,
                    None => 0,
                };
                let (start, end) = m.spans[index];
                let as_int = |value: usize| {
                    if value == NONE {
                        Value::Number(-1)
                    } else {
                        Value::Number(value as i64)
                    }
                };
                Ok(match method {
                    "start" => as_int(start),
                    "end" => as_int(end),
                    _ => span_value(start, end),
                })
            }
            "expand" => {
                let values = bind_clinic("expand", &["template"], 1, args, keywords)?;
                let template_value = values.into_iter().next().flatten().expect("required");
                let template = self.re_compile_template(&m.pattern, &template_value)?;
                self.re_expand(&template, &m)
            }
            "__copy__" => {
                if !args.is_empty() {
                    return Err(format!(
                        "TypeError: Match.__copy__() takes no arguments ({} given)",
                        args.len()
                    ));
                }
                no_keywords("Match.__copy__", &keywords)?;
                Ok(Value::Re(ReObject::Match(m)))
            }
            "__deepcopy__" => {
                no_keywords("Match.__deepcopy__", &keywords)?;
                if args.len() != 1 {
                    return Err(format!(
                        "TypeError: Match.__deepcopy__() takes exactly one argument ({} given)",
                        args.len()
                    ));
                }
                Ok(Value::Re(ReObject::Match(m)))
            }
            _ => Err(format!(
                "AttributeError: 're.Match' object has no attribute '{method}'"
            )),
        }
    }

    /// A group given by number or name.
    fn re_group_index(&mut self, m: &MatchObject, group: Value) -> Result<usize, String> {
        let index = match &group {
            Value::Number(_) | Value::Bool(_) | Value::BigInt(_) => Some(group.clone()),
            value if int_subclass_integer(value).is_some() => int_subclass_integer(value),
            value if instance_special_method(value, "__index__").is_some() => {
                Some(self.index_integer_value(group.clone())?)
            }
            _ => None,
        };
        if let Some(index) = index {
            let index = match index {
                Value::Bool(value) => i64::from(value),
                Value::Number(value) => value,
                _ => return Err("IndexError: no such group".to_string()),
            };
            if index < 0 || index as usize >= m.spans.len() {
                return Err("IndexError: no such group".to_string());
            }
            return Ok(index as usize);
        }
        let name = match &group {
            Value::String(name) | Value::IdentityString { value: name, .. } => Some(name.clone()),
            value => str_subclass_string(value),
        };
        if let Some(name) = name
            && let Some((_, index)) = m.pattern.groupindex.iter().find(|(key, _)| *key == name)
        {
            return Ok(*index);
        }
        // Unhashable keys fail the group-name lookup with a TypeError.
        if !is_hashable_key(&group) {
            return Err(format!(
                "TypeError: cannot use '{0}' as a dict key (unhashable type: '{0}')",
                type_name(&group)
            ));
        }
        Err("IndexError: no such group".to_string())
    }

    fn re_group_value(&self, m: &MatchObject, index: usize, default: Value) -> Value {
        let (start, end) = m.spans[index];
        if start == NONE {
            default
        } else {
            m.subject.slice(start, end)
        }
    }

    fn re_scanner_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        no_keywords(&format!("SRE_Scanner.{method}"), &keywords)?;
        let receiver = if args.is_empty() {
            Value::None
        } else {
            args.remove(0)
        };
        let Value::Re(ReObject::Scanner(scanner)) = receiver else {
            return Err("TypeError: expected a scanner".to_string());
        };
        if !args.is_empty() {
            return Err(format!(
                "TypeError: SRE_Scanner.{method}() takes no arguments ({} given)",
                args.len()
            ));
        }
        let Some(start) = scanner.start.get() else {
            return Ok(Value::None);
        };
        let pattern = scanner.pattern.clone();
        let subject = scanner.subject.clone();
        let endpos = scanner.endpos;
        let found = {
            let mut meter = self.re_meter();
            with_text!(subject, text => {
                let mut matcher = Matcher::new(&pattern.program, text, endpos);
                matcher.must_advance = scanner.must_advance.get();
                let matched = if method == "match" {
                    matcher.match_at(start, &mut meter)?
                } else {
                    matcher.search(start, &mut meter)?
                };
                if matched { Some(found_from(&matcher)?) } else { None }
            })
        };
        match found {
            None => {
                scanner.start.set(None);
                Ok(Value::None)
            }
            Some(found) => {
                scanner.must_advance.set(found.end == found.start);
                scanner.start.set(Some(found.end));
                Ok(self.re_match_value(pattern, subject, scanner.pos, endpos, found))
            }
        }
    }

    fn re_flag_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        no_keywords(&format!("RegexFlag.{method}"), &keywords)?;
        let value_of = |value: &Value| -> Option<i64> {
            match value {
                Value::Number(value) => Some(*value),
                Value::Bool(value) => Some(i64::from(*value)),
                value => match int_subclass_integer(value) {
                    Some(Value::Number(value)) => Some(value),
                    _ => None,
                },
            }
        };
        let Some(receiver) = args.first().and_then(value_of) else {
            return Err("TypeError: RegexFlag method requires a RegexFlag receiver".to_string());
        };
        match method {
            "__repr__" | "__str__" => Ok(Value::String(regex_flag_repr(receiver))),
            "__format__" => {
                let spec = args.get(1).cloned().unwrap_or(Value::String(String::new()));
                if matches!(&spec, Value::String(text) if text.is_empty()) {
                    return Ok(Value::String(regex_flag_repr(receiver)));
                }
                self.call_value(
                    Value::Builtin("format".to_string()),
                    vec![Value::Number(receiver), spec],
                )
            }
            "name" => Ok(regex_flag_name(receiver)),
            "value" => Ok(Value::Number(receiver)),
            "__invert__" => {
                let class = self.re_flag_class()?;
                let all_bits = (1i64 << 9) - 1;
                let value = if receiver >= 0 {
                    all_bits ^ receiver
                } else {
                    !receiver
                };
                Ok(flag_value(&class, value))
            }
            "__or__" | "__ror__" | "__and__" | "__rand__" | "__xor__" | "__rxor__" => {
                let Some(other) = args.get(1).and_then(value_of) else {
                    return Ok(Value::NotImplemented);
                };
                let value = match method {
                    "__or__" | "__ror__" => receiver | other,
                    "__and__" | "__rand__" => receiver & other,
                    _ => receiver ^ other,
                };
                let class = self.re_flag_class()?;
                Ok(flag_value(&class, value))
            }
            _ => Err(format!("unknown builtin: re.RegexFlag.{method}")),
        }
    }
}

fn group_or_empty(subject: &Subject, (start, end): (usize, usize)) -> Value {
    if start == NONE {
        subject.empty()
    } else {
        subject.slice(start, end)
    }
}

fn group_or_none(subject: &Subject, (start, end): (usize, usize)) -> Value {
    if start == NONE {
        Value::None
    } else {
        subject.slice(start, end)
    }
}

/// Expands a template given the spans of group 0 and every group.
fn expand_template(
    template: &Template,
    subject: &Subject,
    spans: &[(usize, usize)],
) -> Result<Value, String> {
    if template.items.is_empty() {
        return Ok(template.first.clone());
    }
    let mut parts = vec![template.first.clone()];
    for (group, literal) in &template.items {
        let Some(&(start, end)) = spans.get(*group) else {
            return Err("IndexError: no such group".to_string());
        };
        if start != NONE {
            parts.push(subject.slice(start, end));
        }
        if let Some(literal) = literal {
            parts.push(literal.clone());
        }
    }
    let is_text = matches!(template.first, Value::String(_));
    join_parts(parts, is_text)
}

impl Template {
    fn is_bytes(&self) -> bool {
        matches!(self.first, Value::Bytes(_))
    }
}

/// The result of `sub`, built as it goes: the joined text plus how many
/// items CPython's final `join` would have seen, so a replacement of the
/// wrong type fails with the same message (after every replacement was
/// computed, as CPython joins at the end).
struct SubOutput {
    is_bytes: bool,
    text: String,
    bytes: Vec<u8>,
    items: usize,
    error: Option<String>,
}

impl SubOutput {
    fn new(is_bytes: bool) -> Self {
        SubOutput {
            is_bytes,
            text: String::new(),
            bytes: Vec::new(),
            items: 0,
            error: None,
        }
    }

    fn push_slice(&mut self, subject: &Subject, start: usize, end: usize) {
        self.items += 1;
        match &subject.text {
            SubjectText::Bytes(bytes) => self.bytes.extend_from_slice(&bytes[start..end]),
            SubjectText::Ascii(text) => self.text.push_str(&text[start..end]),
            SubjectText::Wide(chars) => self.text.extend(
                chars[start..end]
                    .iter()
                    .filter_map(|&point| char::from_u32(point)),
            ),
        }
    }

    fn push_literal(&mut self, value: &Value) {
        match value {
            Value::String(text) => self.text.push_str(text),
            Value::Bytes(bytes) => self.bytes.extend_from_slice(bytes),
            _ => {}
        }
    }

    /// A template whose literals already have the subject's type.
    fn push_template(
        &mut self,
        template: &Template,
        subject: &Subject,
        span: (usize, usize),
        groups: &[(usize, usize)],
    ) {
        self.items += 1;
        self.push_literal(&template.first);
        for (group, literal) in &template.items {
            let (start, end) = if *group == 0 {
                span
            } else {
                groups[*group - 1]
            };
            if start != NONE {
                let items = self.items;
                self.push_slice(subject, start, end);
                self.items = items;
            }
            if let Some(literal) = literal {
                self.push_literal(literal);
            }
        }
    }

    /// One replacement value (a literal or a callable's result).
    fn push_item(&mut self, value: &Value) -> Result<(), String> {
        let index = self.items;
        self.items += 1;
        if self.error.is_some() {
            return Ok(());
        }
        if self.is_bytes {
            match value {
                Value::Bytes(bytes) => self.bytes.extend_from_slice(bytes),
                value => match bytes_buffer_value_bytes(value)? {
                    Some(bytes) => self.bytes.extend_from_slice(&bytes),
                    None => {
                        self.error = Some(format!(
                            "TypeError: sequence item {index}: expected a bytes-like object, {} found",
                            type_name(value)
                        ));
                    }
                },
            }
        } else {
            match value {
                Value::String(text) | Value::IdentityString { value: text, .. } => {
                    self.text.push_str(text)
                }
                value => match str_subclass_string(value) {
                    Some(text) => self.text.push_str(&text),
                    None => {
                        self.error = Some(format!(
                            "TypeError: sequence item {index}: expected str instance, {} found",
                            type_name(value)
                        ));
                    }
                },
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Value, String> {
        if let Some(error) = self.error {
            return Err(error);
        }
        Ok(if self.is_bytes {
            bytes_value(self.bytes)
        } else {
            Value::String(self.text)
        })
    }
}

/// `''.join(parts)` or `b''.join(parts)` with CPython's item errors.
fn join_parts(parts: Vec<Value>, is_text: bool) -> Result<Value, String> {
    if is_text {
        let mut out = String::new();
        for (index, part) in parts.iter().enumerate() {
            match part {
                Value::String(text) | Value::IdentityString { value: text, .. } => {
                    out.push_str(text)
                }
                value => match str_subclass_string(value) {
                    Some(text) => out.push_str(&text),
                    None => {
                        return Err(format!(
                            "TypeError: sequence item {index}: expected str instance, {} found",
                            type_name(value)
                        ));
                    }
                },
            }
        }
        Ok(Value::String(out))
    } else {
        let mut out = Vec::new();
        for (index, part) in parts.iter().enumerate() {
            match part {
                Value::Bytes(bytes) => out.extend_from_slice(bytes),
                value => match bytes_buffer_value_bytes(value)? {
                    Some(bytes) => out.extend_from_slice(&bytes),
                    None => {
                        return Err(format!(
                            "TypeError: sequence item {index}: expected a bytes-like object, {} found",
                            type_name(value)
                        ));
                    }
                },
            }
        }
        Ok(bytes_value(out))
    }
}

/// `re.escape`.
fn re_escape(pattern: Value) -> Result<Value, String> {
    const SPECIAL: &[u8] = b"()[]{}?*+-|^$\\.&~# \t\n\r\x0b\x0c";
    let text = match &pattern {
        Value::String(text) | Value::IdentityString { value: text, .. } => Some(text.clone()),
        value => str_subclass_string(value),
    };
    if let Some(text) = text {
        let mut out = String::with_capacity(text.len());
        for ch in text.chars() {
            if (ch as u32) < 128 && SPECIAL.contains(&(ch as u8)) {
                out.push('\\');
            }
            out.push(ch);
        }
        return Ok(Value::String(out));
    }
    let Some(bytes) = bytes_buffer_value_bytes(&pattern)? else {
        return Err(format!(
            "TypeError: decoding to str: need a bytes-like object, {} found",
            type_name(&pattern)
        ));
    };
    let mut out = Vec::with_capacity(bytes.len());
    for byte in bytes {
        if SPECIAL.contains(&byte) {
            out.push(b'\\');
        }
        out.push(byte);
    }
    Ok(bytes_value(out))
}

/// Builds a `re.PatternError` instance from already-validated parts.
fn build_pattern_error(
    class: Value,
    msg: Value,
    pattern: Value,
    pos: Option<Value>,
) -> Result<Value, String> {
    let Value::Class { name, bases, .. } = &class else {
        return Err("TypeError: re.PatternError is not a class".to_string());
    };
    let type_hierarchy = exception_type_hierarchy(name, bases);
    let msg_text = match &msg {
        Value::String(text) => text.clone(),
        value => value.to_string(),
    };
    let (display, lineno, colno) = match (&pattern, &pos) {
        (pattern, Some(Value::Number(pos))) if !matches!(pattern, Value::None) => {
            let pos = *pos;
            let mut display = format!("{msg_text} at position {pos}");
            let (lineno, colno, has_newline) = line_and_column(pattern, pos);
            if has_newline {
                display = format!("{display} (line {lineno}, column {colno})");
            }
            (display, Value::Number(lineno), Value::Number(colno))
        }
        _ => (msg_text, Value::None, Value::None),
    };
    Ok(Value::Exception {
        type_name: name.clone(),
        type_hierarchy,
        type_object: Some(Box::new(class.clone())),
        message: Some(display.clone()),
        args: vec![Value::String(display)],
        attrs: vec![
            ("msg".to_string(), msg),
            ("pattern".to_string(), pattern),
            ("pos".to_string(), pos.unwrap_or(Value::None)),
            ("lineno".to_string(), lineno),
            ("colno".to_string(), colno),
        ],
        exceptions: None,
        cause: None,
        context: None,
        suppress_context: false,
        identity: Rc::new(crate::value::ExceptionIdentity::default()),
    })
}

/// `PatternError(msg, pattern, pos)` constructed from Python, where the
/// arguments may be arbitrary objects.
fn build_pattern_error_checked(
    vm: &mut Vm,
    class: Value,
    msg: Value,
    pattern: Value,
    pos: Option<Value>,
) -> Result<Value, String> {
    if let (false, Some(pos_value)) = (matches!(pattern, Value::None), &pos) {
        let msg_text = vm.str_value(&msg)?;
        let pos_int = vm.index_i64(pos_value.clone(), "pos")?;
        let error = build_pattern_error(
            class,
            Value::String(msg_text),
            pattern,
            Some(Value::Number(pos_int)),
        )?;
        // `msg` keeps the object passed in.
        if let Value::Exception {
            type_name,
            type_hierarchy,
            type_object,
            message,
            args,
            mut attrs,
            exceptions,
            cause,
            context,
            suppress_context,
            identity,
        } = error
        {
            attrs[0].1 = msg;
            attrs[2].1 = pos_value.clone();
            return Ok(Value::Exception {
                type_name,
                type_hierarchy,
                type_object,
                message,
                args,
                attrs,
                exceptions,
                cause,
                context,
                suppress_context,
                identity,
            });
        }
        unreachable!("build_pattern_error returns an exception");
    }
    let display = match &msg {
        Value::String(text) => text.clone(),
        value => vm.str_value(value)?,
    };
    let Value::Class { name, bases, .. } = &class else {
        return Err("TypeError: re.PatternError is not a class".to_string());
    };
    Ok(Value::Exception {
        type_name: name.clone(),
        type_hierarchy: exception_type_hierarchy(name, bases),
        type_object: Some(Box::new(class.clone())),
        message: Some(display),
        args: vec![msg.clone()],
        attrs: vec![
            ("msg".to_string(), msg),
            ("pattern".to_string(), pattern),
            ("pos".to_string(), pos.unwrap_or(Value::None)),
            ("lineno".to_string(), Value::None),
            ("colno".to_string(), Value::None),
        ],
        exceptions: None,
        cause: None,
        context: None,
        suppress_context: false,
        identity: Rc::new(crate::value::ExceptionIdentity::default()),
    })
}

/// 1-based line and column of `pos` in a str or bytes pattern, and whether
/// the pattern contains a newline at all.
fn line_and_column(pattern: &Value, pos: i64) -> (i64, i64, bool) {
    let units: Vec<u32> = match pattern {
        Value::String(text) | Value::IdentityString { value: text, .. } => {
            text.chars().map(u32::from).collect()
        }
        Value::Bytes(bytes) => bytes.iter().map(|&b| u32::from(b)).collect(),
        value => match str_subclass_string(value) {
            Some(text) => text.chars().map(u32::from).collect(),
            None => bytes_subclass_bytes(value)
                .map(|bytes| bytes.iter().map(|&b| u32::from(b)).collect())
                .unwrap_or_default(),
        },
    };
    let len = units.len() as i64;
    // Python slicing semantics for `count(newline, 0, pos)`/`rfind`.
    let end = if pos < 0 {
        (len + pos).max(0)
    } else {
        pos.min(len)
    } as usize;
    let newline = u32::from(b'\n');
    let lineno = units[..end].iter().filter(|&&u| u == newline).count() as i64 + 1;
    let last = units[..end]
        .iter()
        .rposition(|&u| u == newline)
        .map_or(-1, |index| index as i64);
    let colno = pos - last;
    (lineno, colno, units.contains(&newline))
}
