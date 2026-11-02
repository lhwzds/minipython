// SPDX-License-Identifier: MIT

//! Native `io.StringIO`.
//!
//! `io.StringIO` is an ordinary class (`_io.StringIO`) whose methods are Rust
//! builtins; each instance keeps its text buffer in a hidden field holding a
//! `Value::StringIOBuffer`. CPython 3.14.6 is the behavior oracle for the
//! newline modes, positions (code-point indexes, which is what `tell()`
//! returns for a `StringIO`), the seek rules, and the error messages, which
//! differ between `StringIO`'s own methods and the ones it inherits from
//! `_io._IOBase` (those add a trailing period).

use super::native_class::{
    NativeClass, c_no_args, c_one_arg, c_slot_no_args, c_unpack, hidden_field, native_property,
    set_hidden_field,
};
use super::*;

/// Prefix of every `io.StringIO` method builtin.
pub(crate) const STRINGIO_PREFIX: &str = "_io.StringIO.";
/// Hidden instance field holding the buffer.
const STATE_FIELD: &str = "\0stringio";
/// Hidden class attribute marking the `io.StringIO` class itself (not its
/// subclasses, whose own attribute scope lacks it).
const CLASS_MARKER: &str = "\0stringio_class";

const CLOSED: &str = "ValueError: I/O operation on closed file";
const IOBASE_CLOSED: &str = "ValueError: I/O operation on closed file.";
const UNINITIALIZED: &str = "ValueError: I/O operation on uninitialized object";

const SEEN_CR: u8 = 1;
const SEEN_LF: u8 = 2;
const SEEN_CRLF: u8 = 4;

const DOC: &str = "Text stream kept in memory.";

const METHODS: &[&str] = &[
    "__init__",
    "__getattr__",
    "__iter__",
    "__next__",
    "__enter__",
    "__exit__",
    "__del__",
    "__getstate__",
    "__setstate__",
    "_checkClosed",
    "_checkReadable",
    "_checkSeekable",
    "_checkWritable",
    "close",
    "detach",
    "fileno",
    "flush",
    "getvalue",
    "isatty",
    "read",
    "readable",
    "readline",
    "readlines",
    "seek",
    "seekable",
    "tell",
    "truncate",
    "writable",
    "write",
    "writelines",
];

const PROPERTIES: &[&str] = &["closed", "line_buffering", "newlines", "encoding", "errors"];

/// The `newline` argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Newline {
    /// `None`: universal newlines, translated to `\n` on write.
    Translate,
    /// `''`: universal newlines, kept as written.
    Universal,
    Lf,
    Cr,
    CrLf,
}

impl Newline {
    fn universal(self) -> bool {
        matches!(self, Newline::Translate | Newline::Universal)
    }

    fn value(self) -> Value {
        match self {
            Newline::Translate => Value::None,
            Newline::Universal => Value::String(String::new()),
            Newline::Lf => Value::String("\n".to_string()),
            Newline::Cr => Value::String("\r".to_string()),
            Newline::CrLf => Value::String("\r\n".to_string()),
        }
    }
}

pub(crate) type StringIOBufferRef = Rc<RefCell<StringIOBuffer>>;

/// The state of one `StringIO`.
#[derive(Debug, Clone)]
pub(crate) struct StringIOBuffer {
    text: String,
    /// Number of code points in `text`.
    chars: usize,
    /// Current position in code points; may be past the end.
    pos: usize,
    closed: bool,
    newline: Newline,
    /// Newline kinds seen by the universal-newline decoder.
    seen: u8,
    /// A known `(code point, byte)` offset pair, so sequential access to
    /// non-ASCII text does not rescan from the start.
    cursor: (usize, usize),
}

impl StringIOBuffer {
    fn new(newline: Newline) -> Self {
        Self {
            text: String::new(),
            chars: 0,
            pos: 0,
            closed: false,
            newline,
            seen: 0,
            cursor: (0, 0),
        }
    }

    pub(crate) fn estimated_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.text.capacity()
    }

    /// Byte offset of code point `index` (`index <= chars`).
    fn byte_offset(&mut self, index: usize) -> usize {
        if self.text.len() == self.chars {
            return index;
        }
        if index >= self.chars {
            return self.text.len();
        }
        let (mut char_index, mut byte_index) = if index >= self.cursor.0 {
            self.cursor
        } else {
            (0, 0)
        };
        let bytes = self.text.as_bytes();
        while char_index < index {
            byte_index += utf8_width(bytes[byte_index]);
            char_index += 1;
        }
        self.cursor = (char_index, byte_index);
        byte_index
    }

    /// The text written for `text`: universal-newline decoding (recording
    /// the newline kinds seen) and `\n` translation for `newline='\r'` or
    /// `'\r\n'`.
    fn translate(&mut self, text: &str) -> String {
        if self.newline.universal() {
            let bytes = text.as_bytes();
            let mut index = 0;
            while index < bytes.len() {
                match bytes[index] {
                    b'\n' => self.seen |= SEEN_LF,
                    b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                        self.seen |= SEEN_CRLF;
                        index += 1;
                    }
                    b'\r' => self.seen |= SEEN_CR,
                    _ => {}
                }
                index += 1;
            }
            if self.newline == Newline::Translate && text.contains('\r') {
                return text.replace("\r\n", "\n").replace('\r', "\n");
            }
            return text.to_string();
        }
        match self.newline {
            Newline::Cr if text.contains('\n') => text.replace('\n', "\r"),
            Newline::CrLf if text.contains('\n') => text.replace('\n', "\r\n"),
            _ => text.to_string(),
        }
    }

    /// Writes already translated text at the current position, padding an
    /// overseek with NUL characters. Returns the number of bytes the buffer
    /// grew by.
    fn write_translated(&mut self, text: &str) -> usize {
        let before = self.text.len();
        let count = text.chars().count();
        if self.pos >= self.chars {
            if self.pos > self.chars {
                let padding = self.pos - self.chars;
                self.text.extend(std::iter::repeat_n('\0', padding));
                self.chars = self.pos;
            }
            self.text.push_str(text);
            self.chars += count;
        } else {
            let start = self.byte_offset(self.pos);
            let end_char = (self.pos + count).min(self.chars);
            let end = self.byte_offset(end_char);
            self.text.replace_range(start..end, text);
            self.chars = self.chars - (end_char - self.pos) + count;
            self.cursor = (self.pos, start);
        }
        self.pos += count;
        self.text.len().saturating_sub(before)
    }

    /// Reads up to `size` code points (`None`: to the end).
    fn read(&mut self, size: Option<usize>) -> String {
        if self.pos >= self.chars {
            return String::new();
        }
        let available = self.chars - self.pos;
        let count = size.map_or(available, |size| size.min(available));
        let start = self.byte_offset(self.pos);
        let end = self.byte_offset(self.pos + count);
        self.pos += count;
        self.text[start..end].to_string()
    }

    /// Reads one line of at most `limit` code points.
    fn readline(&mut self, limit: Option<usize>) -> String {
        if self.pos >= self.chars {
            return String::new();
        }
        let available = self.chars - self.pos;
        let window = limit.map_or(available, |limit| limit.min(available));
        let start = self.byte_offset(self.pos);
        let window_end = self.byte_offset(self.pos + window);
        let bytes = &self.text.as_bytes()[start..window_end];
        let line_end = match self.newline {
            Newline::Translate | Newline::Lf => bytes
                .iter()
                .position(|&byte| byte == b'\n')
                .map(|at| at + 1),
            Newline::Cr => bytes
                .iter()
                .position(|&byte| byte == b'\r')
                .map(|at| at + 1),
            Newline::Universal => bytes
                .iter()
                .position(|&byte| byte == b'\n' || byte == b'\r')
                .map(|at| {
                    if bytes[at] == b'\r' && bytes.get(at + 1) == Some(&b'\n') {
                        at + 2
                    } else {
                        at + 1
                    }
                }),
            Newline::CrLf => bytes
                .windows(2)
                .position(|pair| pair == b"\r\n")
                .map(|at| at + 2),
        };
        let end = line_end.map_or(window_end, |line_end| start + line_end);
        let line = self.text[start..end].to_string();
        let count = if line_end.is_some() {
            line.chars().count()
        } else {
            window
        };
        self.pos += count;
        self.cursor = (self.pos, end);
        line
    }

    fn truncate(&mut self, size: usize) {
        if size < self.chars {
            let end = self.byte_offset(size);
            self.text.truncate(end);
            self.chars = size;
            if self.cursor.0 > size {
                self.cursor = (0, 0);
            }
        }
    }

    fn newlines(&self) -> Value {
        if !self.newline.universal() {
            return Value::None;
        }
        let one = |text: &str| Value::String(text.to_string());
        match self.seen {
            0 => Value::None,
            SEEN_CR => one("\r"),
            SEEN_LF => one("\n"),
            SEEN_CRLF => one("\r\n"),
            seen => tuple_value(
                [(SEEN_CR, "\r"), (SEEN_LF, "\n"), (SEEN_CRLF, "\r\n")]
                    .into_iter()
                    .filter(|(flag, _)| seen & flag != 0)
                    .map(|(_, text)| one(text))
                    .collect(),
            ),
        }
    }
}

/// An independent copy of a buffer (`copy.copy`/`copy.deepcopy` of a
/// `StringIO` copy its value and position, as CPython's `__getstate__`
/// round trip does).
pub(crate) fn copied_buffer(buffer: &StringIOBufferRef) -> Value {
    Value::StringIOBuffer(Rc::new(RefCell::new(buffer.borrow().clone())))
}

/// The fields of a shallow instance copy, with a fresh buffer.
pub(crate) fn copied_fields(fields: &HashMap<String, Value>) -> HashMap<String, Value> {
    let mut copied = fields.clone();
    if let Some(Value::StringIOBuffer(buffer)) = copied.get(STATE_FIELD) {
        let buffer = copied_buffer(buffer);
        copied.insert(STATE_FIELD.to_string(), buffer);
    }
    copied
}

fn utf8_width(lead: u8) -> usize {
    match lead {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        _ => 4,
    }
}

fn method(name: &str) -> Value {
    Value::Builtin(format!("{STRINGIO_PREFIX}{name}"))
}

/// Builds the `io.StringIO` class.
pub(crate) fn string_io_class() -> Value {
    let mut attrs: Vec<(String, Value)> = METHODS
        .iter()
        .map(|name| ((*name).to_string(), method(name)))
        .collect();
    for property in PROPERTIES {
        attrs.push((
            (*property).to_string(),
            native_property(property, format!("{STRINGIO_PREFIX}{property}"), None),
        ));
    }
    NativeClass {
        name: "StringIO",
        module: "_io",
        doc: Some(DOC),
        bases: vec![builtin_type_value("object")],
        attrs,
        hidden: vec![(CLASS_MARKER.to_string(), Value::Bool(true))],
    }
    .build()
}

/// Whether `value` is an instance of `io.StringIO` itself (not a subclass).
fn is_exact_string_io(value: &Value) -> bool {
    matches!(value, Value::Instance { class_attrs, .. } if class_attrs.borrow().contains_key(CLASS_MARKER))
}

/// Whether `value` is an instance of `io.StringIO` or a subclass.
fn is_string_io(value: &Value) -> bool {
    match value {
        Value::Instance {
            class_attrs,
            class_bases,
            ..
        } => find_class_attr(class_attrs, class_bases, CLASS_MARKER).is_some(),
        _ => false,
    }
}

fn buffer_of(receiver: &Value) -> Option<StringIOBufferRef> {
    match hidden_field(receiver, STATE_FIELD) {
        Some(Value::StringIOBuffer(buffer)) => Some(buffer),
        _ => None,
    }
}

impl Vm {
    pub(super) fn stringio_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if args.is_empty() {
            return Err(format!(
                "TypeError: unbound method StringIO.{method}() needs an argument"
            ));
        }
        let receiver = args.remove(0);
        if !is_string_io(&receiver) {
            return Err(format!(
                "TypeError: descriptor '{method}' for '_io.StringIO' objects doesn't apply to a '{}' object",
                type_name(&receiver)
            ));
        }
        let qualname = format!("StringIO.{method}");
        match method {
            "__init__" => self.stringio_init(&receiver, args, keywords),
            "__getattr__" => {
                let name = match args.first() {
                    Some(Value::String(name)) => name.clone(),
                    _ => String::new(),
                };
                let shown = if is_exact_string_io(&receiver) {
                    "_io.StringIO"
                } else {
                    type_name(&receiver)
                };
                Err(format!(
                    "AttributeError: '{shown}' object has no attribute '{name}'"
                ))
            }
            "closed" => {
                let buffer = initialized(&receiver)?;
                Ok(Value::Bool(buffer.borrow().closed))
            }
            "line_buffering" => {
                open_buffer(&receiver)?;
                Ok(Value::Bool(false))
            }
            "newlines" => Ok(open_buffer(&receiver)?.borrow().newlines()),
            "encoding" | "errors" => Ok(Value::None),
            "getvalue" => {
                c_no_args(&qualname, &args, &keywords)?;
                let buffer = open_buffer(&receiver)?;
                Ok(Value::String(buffer.borrow().text.clone()))
            }
            "tell" => {
                c_no_args(&qualname, &args, &keywords)?;
                let buffer = open_buffer(&receiver)?;
                Ok(Value::Number(buffer.borrow().pos as i64))
            }
            "read" => {
                let [size] = c_unpack(&qualname, "read", args, &keywords, 0, 1)?
                    .try_into()
                    .expect("one slot");
                let size = self.native_optional_ssize(size)?;
                let buffer = open_buffer(&receiver)?;
                let size = size.and_then(|size| usize::try_from(size).ok());
                Ok(Value::String(buffer.borrow_mut().read(size)))
            }
            "readline" => {
                let [size] = c_unpack(&qualname, "readline", args, &keywords, 0, 1)?
                    .try_into()
                    .expect("one slot");
                let size = self.native_optional_ssize(size)?;
                let buffer = open_buffer(&receiver)?;
                let size = size.and_then(|size| usize::try_from(size).ok());
                Ok(Value::String(buffer.borrow_mut().readline(size)))
            }
            "readlines" => {
                let [hint] = c_unpack(&qualname, "readlines", args, &keywords, 0, 1)?
                    .try_into()
                    .expect("one slot");
                let hint = self.native_optional_ssize(hint)?.unwrap_or(-1);
                self.stringio_readlines(&receiver, hint)
            }
            "write" => {
                let text = c_one_arg(&qualname, args, &keywords)?;
                self.stringio_write(&receiver, text)
            }
            "writelines" => {
                // CPython names the inherited method when no argument is
                // given.
                let name = if args.is_empty() && keywords.is_empty() {
                    "_IOBase.writelines"
                } else {
                    qualname.as_str()
                };
                let lines = c_one_arg(name, args, &keywords)?;
                self.stringio_check_closed_attr(&receiver)?;
                let exact = is_exact_string_io(&receiver);
                self.for_each_iterable_value(lines, |vm, line| {
                    if exact {
                        vm.stringio_write(&receiver, line)?;
                    } else {
                        let write = vm.load_attribute_value(receiver.clone(), "write")?;
                        vm.call_value(write, vec![line])?;
                    }
                    Ok(())
                })?;
                Ok(Value::None)
            }
            "seek" => {
                let [pos, whence] = c_unpack(&qualname, "seek", args, &keywords, 1, 2)?
                    .try_into()
                    .expect("two slots");
                let pos = self.native_ssize(pos.expect("required"))?;
                let whence = match whence {
                    Some(whence) => self.native_c_int(whence)?,
                    None => 0,
                };
                let buffer = open_buffer(&receiver)?;
                if !(0..=2).contains(&whence) {
                    return Err(format!(
                        "ValueError: Invalid whence ({whence}, should be 0, 1 or 2)"
                    ));
                }
                if pos < 0 && whence == 0 {
                    return Err(format!("ValueError: Negative seek position {pos}"));
                }
                if whence != 0 && pos != 0 {
                    return Err("OSError: Can't do nonzero cur-relative seeks".to_string());
                }
                let mut buffer = buffer.borrow_mut();
                match whence {
                    0 => buffer.pos = pos as usize,
                    2 => buffer.pos = buffer.chars,
                    _ => {}
                }
                Ok(Value::Number(buffer.pos as i64))
            }
            "truncate" => {
                let [size] = c_unpack(&qualname, "truncate", args, &keywords, 0, 1)?
                    .try_into()
                    .expect("one slot");
                let size = self.native_optional_ssize(size)?;
                let buffer = open_buffer(&receiver)?;
                let mut buffer = buffer.borrow_mut();
                let size = size.unwrap_or(buffer.pos as i64);
                if size < 0 {
                    return Err(format!("ValueError: Negative size value {size}"));
                }
                buffer.truncate(size as usize);
                Ok(Value::Number(size))
            }
            "readable" | "writable" | "seekable" => {
                c_no_args(&qualname, &args, &keywords)?;
                open_buffer(&receiver)?;
                Ok(Value::Bool(true))
            }
            "isatty" => {
                c_no_args(&qualname, &args, &keywords)?;
                self.stringio_check_closed_attr(&receiver)?;
                Ok(Value::Bool(false))
            }
            "flush" => {
                c_no_args(&qualname, &args, &keywords)?;
                Ok(Value::None)
            }
            "close" => {
                c_no_args(&qualname, &args, &keywords)?;
                if let Some(buffer) = buffer_of(&receiver) {
                    let mut buffer = buffer.borrow_mut();
                    buffer.closed = true;
                    buffer.text = String::new();
                    buffer.chars = 0;
                    buffer.cursor = (0, 0);
                }
                Ok(Value::None)
            }
            "__del__" => {
                c_slot_no_args(&args, &keywords)?;
                self.call_method_by_name(&receiver, "close", Vec::new())?;
                Ok(Value::None)
            }
            "fileno" | "detach" => {
                if !args.is_empty() || !keywords.is_empty() {
                    return Err(format!("TypeError: {method}() takes no arguments"));
                }
                Err(format!("io.UnsupportedOperation: {method}"))
            }
            "_checkClosed" => {
                c_no_args(&qualname, &args, &keywords)?;
                self.stringio_check_closed_attr(&receiver)?;
                Ok(Value::None)
            }
            "_checkReadable" | "_checkWritable" | "_checkSeekable" => {
                c_no_args(&qualname, &args, &keywords)?;
                let (probe, what) = match method {
                    "_checkReadable" => ("readable", "readable"),
                    "_checkWritable" => ("writable", "writable"),
                    _ => ("seekable", "seekable"),
                };
                let result = self.call_method_by_name(&receiver, probe, Vec::new())?;
                if !matches!(result, Value::Bool(true)) {
                    return Err(format!(
                        "io.UnsupportedOperation: File or stream is not {what}."
                    ));
                }
                Ok(Value::Bool(true))
            }
            "__iter__" => {
                c_slot_no_args(&args, &keywords)?;
                self.stringio_check_closed_attr(&receiver)?;
                Ok(receiver)
            }
            "__next__" => {
                c_slot_no_args(&args, &keywords)?;
                let line = if is_exact_string_io(&receiver) {
                    let buffer = open_buffer(&receiver)?;
                    Value::String(buffer.borrow_mut().readline(None))
                } else {
                    open_buffer(&receiver)?;
                    let line = self.call_method_by_name(&receiver, "readline", Vec::new())?;
                    if !matches!(line, Value::String(_)) {
                        return Err(format!(
                            "OSError: readline() should have returned a str object, not '{}'",
                            type_name(&line)
                        ));
                    }
                    line
                };
                match &line {
                    Value::String(text) if text.is_empty() => Err("StopIteration".to_string()),
                    _ => Ok(line),
                }
            }
            "__enter__" => {
                c_no_args(&qualname, &args, &keywords)?;
                self.stringio_check_closed_attr(&receiver)?;
                Ok(receiver)
            }
            "__exit__" => {
                if !keywords.is_empty() {
                    return Err(format!(
                        "TypeError: {qualname}() takes no keyword arguments"
                    ));
                }
                self.call_method_by_name(&receiver, "close", Vec::new())
            }
            "__getstate__" => {
                c_no_args(&qualname, &args, &keywords)?;
                let buffer = open_buffer(&receiver)?;
                let buffer = buffer.borrow();
                let dict = match &receiver {
                    Value::Instance { fields, .. } => {
                        let entries = scope_dict_entries(fields);
                        if entries.is_empty() {
                            Value::None
                        } else {
                            dict_value(entries)
                        }
                    }
                    _ => Value::None,
                };
                Ok(tuple_value(vec![
                    Value::String(buffer.text.clone()),
                    buffer.newline.value(),
                    Value::Number(buffer.pos as i64),
                    dict,
                ]))
            }
            "__setstate__" => {
                let state = c_one_arg(&qualname, args, &keywords)?;
                self.stringio_setstate(&receiver, state)
            }
            _ => Err(format!("unknown builtin: {STRINGIO_PREFIX}{method}")),
        }
    }

    fn call_method_by_name(
        &mut self,
        receiver: &Value,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let method = self.load_attribute_value(receiver.clone(), name)?;
        self.call_value(method, args)
    }

    /// `_io._IOBase`'s closed check: reads the `closed` attribute and
    /// raises with the trailing period.
    fn stringio_check_closed_attr(&mut self, receiver: &Value) -> Result<(), String> {
        let closed = self.load_attribute_value(receiver.clone(), "closed")?;
        if self.truth_value(closed)? {
            return Err(IOBASE_CLOSED.to_string());
        }
        Ok(())
    }

    fn stringio_init(
        &mut self,
        receiver: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let [value, newline] = super::native_class::c_clinic(
            "StringIO",
            &["initial_value", "newline"],
            0,
            args,
            keywords,
        )?
        .try_into()
        .expect("two slots");
        let newline = match newline {
            None => Newline::Lf,
            Some(Value::None) => Newline::Translate,
            Some(Value::String(text)) => match text.as_str() {
                "" => Newline::Universal,
                "\n" => Newline::Lf,
                "\r" => Newline::Cr,
                "\r\n" => Newline::CrLf,
                _ => {
                    return Err(format!(
                        "ValueError: illegal newline value: {}",
                        crate::value::repr_string(&text)
                    ));
                }
            },
            Some(other) => {
                return Err(format!(
                    "TypeError: newline must be str or None, not {}",
                    type_name(&other)
                ));
            }
        };
        let initial = match value {
            None | Some(Value::None) => String::new(),
            Some(Value::String(text)) => text,
            Some(other) => match str_subclass_string(&other) {
                Some(text) => text,
                None => {
                    return Err(format!(
                        "TypeError: initial_value must be str or None, not {}",
                        type_name(&other)
                    ));
                }
            },
        };
        let mut buffer = StringIOBuffer::new(newline);
        if !initial.is_empty() {
            let translated = buffer.translate(&initial);
            self.consume_allocation_budget(translated.len())?;
            buffer.write_translated(&translated);
            buffer.pos = 0;
            buffer.cursor = (0, 0);
        }
        set_hidden_field(
            receiver,
            STATE_FIELD,
            Value::StringIOBuffer(Rc::new(RefCell::new(buffer))),
        );
        Ok(Value::None)
    }

    fn stringio_write(&mut self, receiver: &Value, text: Value) -> Result<Value, String> {
        let buffer = initialized(receiver)?;
        let text = match text {
            Value::String(text) => text,
            other => match str_subclass_string(&other) {
                Some(text) => text,
                None => {
                    return Err(format!(
                        "TypeError: string argument expected, got '{}'",
                        type_name(&other)
                    ));
                }
            },
        };
        if buffer.borrow().closed {
            return Err(CLOSED.to_string());
        }
        let size = text.chars().count();
        if size > 0 {
            let mut buffer = buffer.borrow_mut();
            let translated = buffer.translate(&text);
            let padding = buffer.pos.saturating_sub(buffer.chars);
            self.consume_allocation_budget(translated.len().saturating_add(padding))?;
            buffer.write_translated(&translated);
        }
        Ok(Value::Number(size as i64))
    }

    fn stringio_readlines(&mut self, receiver: &Value, hint: i64) -> Result<Value, String> {
        self.stringio_check_closed_attr(receiver)?;
        let mut lines = Vec::new();
        let mut total: i64 = 0;
        let exact = is_exact_string_io(receiver);
        loop {
            let line = if exact {
                let buffer = open_buffer(receiver)?;
                Value::String(buffer.borrow_mut().readline(None))
            } else {
                match self.call_with_exception_capture(|vm| {
                    vm.call_method_by_name(receiver, "__next__", Vec::new())
                })? {
                    Ok(line) => line,
                    Err(exception) if exception.type_name == "StopIteration" => break,
                    Err(exception) => return Err(self.propagate_captured_exception(exception)),
                }
            };
            if exact && matches!(&line, Value::String(text) if text.is_empty()) {
                break;
            }
            let length = match &line {
                Value::String(text) => text.chars().count() as i64,
                other => self.len_value(other.clone())? as i64,
            };
            self.consume_allocation_budget(std::mem::size_of::<Value>())?;
            lines.push(line);
            if hint > 0 {
                if length > hint - total {
                    break;
                }
                total += length;
            }
        }
        Ok(list_value(lines))
    }

    fn stringio_setstate(&mut self, receiver: &Value, state: Value) -> Result<Value, String> {
        let items = match &state {
            Value::Tuple(items) if items.len() == 4 => items.as_ref().clone(),
            other => {
                return Err(format!(
                    "TypeError: _io.StringIO.__setstate__ argument should be 4-tuple, got {}",
                    type_name(other)
                ));
            }
        };
        if buffer_of(receiver).is_some_and(|buffer| buffer.borrow().closed) {
            return Err(CLOSED.to_string());
        }
        // Validate the value and newline by re-running `__init__`, as
        // CPython does, then restore the raw text and position.
        self.stringio_init(receiver, vec![Value::None, items[1].clone()], Vec::new())?;
        let text = match &items[0] {
            Value::String(text) => text.clone(),
            other => {
                return Err(format!(
                    "TypeError: initial_value must be str or None, not {}",
                    type_name(other)
                ));
            }
        };
        let buffer = initialized(receiver)?;
        {
            let mut buffer = buffer.borrow_mut();
            self.consume_allocation_budget(text.len())?;
            buffer.chars = text.chars().count();
            buffer.text = text;
            buffer.cursor = (0, 0);
        }
        let pos = match &items[2] {
            Value::Number(pos) => *pos,
            Value::Bool(flag) => i64::from(*flag),
            other => {
                return Err(format!(
                    "TypeError: third item of state must be an integer, got {}",
                    type_name(other)
                ));
            }
        };
        if pos < 0 {
            return Err("ValueError: position value cannot be negative".to_string());
        }
        buffer.borrow_mut().pos = pos as usize;
        match &items[3] {
            Value::None => {}
            Value::Dict(entries) => {
                for (key, value) in entries.borrow().entries.iter() {
                    if let Value::String(key) = key {
                        set_hidden_field(receiver, key, value.clone());
                    }
                }
            }
            other => {
                return Err(format!(
                    "TypeError: fourth item of state should be a dict, got a {}",
                    type_name(other)
                ));
            }
        }
        Ok(Value::None)
    }
}

/// The buffer of an initialized `StringIO`.
fn initialized(receiver: &Value) -> Result<StringIOBufferRef, String> {
    buffer_of(receiver).ok_or_else(|| UNINITIALIZED.to_string())
}

/// The buffer of an initialized, open `StringIO`.
fn open_buffer(receiver: &Value) -> Result<StringIOBufferRef, String> {
    let buffer = initialized(receiver)?;
    if buffer.borrow().closed {
        return Err(CLOSED.to_string());
    }
    Ok(buffer)
}
