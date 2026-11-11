// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/csv.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Native `csv` module.
//!
//! CPython splits `csv` between the C accelerator `_csv` (the reader, the
//! writer, `Dialect` validation, the registry, `Error`, `field_size_limit`)
//! and the Python-level half of the module (`Dialect` and its subclasses,
//! `DictReader`, `DictWriter`, and `Sniffer`). Both halves are implemented
//! here in Rust, with CPython 3.14.6 as the black-box behavior oracle:
//!
//! - every type is an ordinary class whose methods are Rust builtins
//!   (`native_class.rs`), so `type()`, `isinstance`, subclassing, and
//!   attribute lookup behave as for CPython's types and classes; the C types
//!   keep their `_csv` module name and reject new attributes;
//! - the Python-level classes reach their collaborators through attribute
//!   lookup (`self.reader`, `self.fieldnames`, `self.writer`,
//!   `self._dict_to_list`, `self.sniff`, ...), so subclass overrides apply
//!   where CPython applies them;
//! - the dialect registry and the field size limit are per-interpreter
//!   module state, kept in hidden attributes of the module object.
//!
//! `reader.rs` holds the parsing state machine, `writer.rs` the quoting
//! rules, `dict.rs` `DictReader`/`DictWriter`, and `sniffer.rs` the
//! `Sniffer` heuristics.

use super::argparse::{Sig, bind_args};
use super::native_class::{
    NativeClass, c_clinic, c_no_args, c_unpack_positional, hidden_field, native_instance,
    native_property,
};
use super::*;

mod dict;
mod reader;
mod sniffer;
mod writer;

/// Prefix of the builtins of the Python-level csv classes (`csv.DictReader.*`).
pub(crate) const CSV_PREFIX: &str = "csv.";
/// Prefix of the builtins of the C accelerator (`_csv.reader`,
/// `_csv.reader.__next__`, ...).
pub(crate) const CSV_C_PREFIX: &str = "_csv.";

pub(super) const QUOTE_MINIMAL: i64 = 0;
pub(super) const QUOTE_ALL: i64 = 1;
pub(super) const QUOTE_NONNUMERIC: i64 = 2;
pub(super) const QUOTE_NONE: i64 = 3;
pub(super) const QUOTE_STRINGS: i64 = 4;
pub(super) const QUOTE_NOTNULL: i64 = 5;

/// CPython's default `field_size_limit()` (128 KiB).
const DEFAULT_FIELD_LIMIT: i64 = 128 * 1024;

// Hidden module state.
const STATE_ERROR: &str = "\0csv_error";
const STATE_DIALECT_TYPE: &str = "\0csv_dialect_type";
pub(super) const STATE_READER_TYPE: &str = "\0csv_reader_type";
pub(super) const STATE_WRITER_TYPE: &str = "\0csv_writer_type";
const STATE_DIALECTS: &str = "\0csv_dialects";
const STATE_FIELD_LIMIT: &str = "\0csv_field_limit";
pub(super) const STATE_PY_DIALECT: &str = "\0csv_py_dialect";

/// Hidden field of a `_csv.Dialect` instance: the validated settings.
const CONFIG_FIELD: &str = "\0csv_config";
/// Hidden class attribute marking `_csv.Dialect` (and subclasses).
const DIALECT_MARKER: &str = "\0csv_dialect_class";

const MODULE_DOC: &str =
    "Reading and writing of comma-separated values, with configurable dialects.";

const DIALECT_PY_DOC: &str =
    "Base class for CSV dialect descriptions; subclasses set the formatting attributes.";
const SNIFFER_DOC: &str =
    "Guesses the dialect of CSV text and whether it starts with a header row.";
const C_DIALECT_DOC: &str = "Validated CSV formatting settings.";
const READER_DOC: &str = "Iterator that parses CSV lines into lists of fields.";
const WRITER_DOC: &str = "Formats rows as CSV and writes them to a file-like object.";

const MODULE_ALL: &[&str] = &[
    "QUOTE_MINIMAL",
    "QUOTE_ALL",
    "QUOTE_NONNUMERIC",
    "QUOTE_NONE",
    "QUOTE_STRINGS",
    "QUOTE_NOTNULL",
    "Error",
    "Dialect",
    "excel",
    "excel_tab",
    "field_size_limit",
    "reader",
    "writer",
    "register_dialect",
    "get_dialect",
    "list_dialects",
    "Sniffer",
    "unregister_dialect",
    "DictReader",
    "DictWriter",
    "unix_dialect",
];

const MODULE_FUNCTIONS: &[&str] = &[
    "reader",
    "writer",
    "register_dialect",
    "get_dialect",
    "list_dialects",
    "unregister_dialect",
    "field_size_limit",
];

/// `_csv.Dialect` parameters in CPython's keyword order.
const DIALECT_KEYWORDS: &[&str] = &[
    "dialect",
    "delimiter",
    "doublequote",
    "escapechar",
    "lineterminator",
    "quotechar",
    "quoting",
    "skipinitialspace",
    "strict",
];

/// Validated dialect settings.
#[derive(Debug, Clone)]
pub(super) struct Config {
    pub delimiter: char,
    pub quotechar: Option<char>,
    pub escapechar: Option<char>,
    pub doublequote: bool,
    pub skipinitialspace: bool,
    /// The `lineterminator` object as given (a `str` or `str` subclass).
    pub lineterminator_value: Value,
    pub lineterminator: String,
    pub quoting: i64,
    pub strict: bool,
}

fn char_or_none(c: Option<char>) -> Value {
    match c {
        Some(c) => Value::String(c.to_string()),
        None => Value::None,
    }
}

impl Config {
    fn to_value(&self) -> Value {
        tuple_value(vec![
            Value::String(self.delimiter.to_string()),
            char_or_none(self.quotechar),
            char_or_none(self.escapechar),
            Value::Bool(self.doublequote),
            Value::Bool(self.skipinitialspace),
            self.lineterminator_value.clone(),
            Value::Number(self.quoting),
            Value::Bool(self.strict),
        ])
    }

    fn from_value(value: &Value) -> Option<Config> {
        let Value::Tuple(items) = value else {
            return None;
        };
        let [
            delimiter,
            quotechar,
            escapechar,
            doublequote,
            skipinitialspace,
            lineterminator,
            quoting,
            strict,
        ] = items.as_slice()
        else {
            return None;
        };
        let one_char = |value: &Value| match value {
            Value::String(text) => text.chars().next(),
            _ => None,
        };
        Some(Config {
            delimiter: one_char(delimiter)?,
            quotechar: one_char(quotechar),
            escapechar: one_char(escapechar),
            doublequote: matches!(doublequote, Value::Bool(true)),
            skipinitialspace: matches!(skipinitialspace, Value::Bool(true)),
            lineterminator: string_of(lineterminator).unwrap_or_default(),
            lineterminator_value: lineterminator.clone(),
            quoting: match quoting {
                Value::Number(quoting) => *quoting,
                _ => QUOTE_MINIMAL,
            },
            strict: matches!(strict, Value::Bool(true)),
        })
    }

    /// The setting `name` as a Python value (the `_csv.Dialect` getters).
    fn attribute(&self, name: &str) -> Value {
        match name {
            "delimiter" => Value::String(self.delimiter.to_string()),
            "quotechar" => char_or_none(self.quotechar),
            "escapechar" => char_or_none(self.escapechar),
            "doublequote" => Value::Bool(self.doublequote),
            "skipinitialspace" => Value::Bool(self.skipinitialspace),
            "lineterminator" => self.lineterminator_value.clone(),
            "quoting" => Value::Number(self.quoting),
            "strict" => Value::Bool(self.strict),
            _ => Value::None,
        }
    }
}

/// The settings of a `_csv.Dialect` instance.
pub(super) fn dialect_config(dialect: &Value) -> Option<Config> {
    hidden_field(dialect, CONFIG_FIELD).and_then(|value| Config::from_value(&value))
}

fn methods(prefix: &str, class: &str, names: &[&str]) -> Vec<(String, Value)> {
    names
        .iter()
        .map(|name| {
            (
                (*name).to_string(),
                Value::Builtin(format!("{prefix}{class}.{name}")),
            )
        })
        .collect()
}

fn class_getitem() -> (String, Value) {
    (
        "__class_getitem__".to_string(),
        Value::ClassMethod {
            function: Rc::new(Value::Builtin("GenericAlias".to_string())),
            identity: Rc::new(()),
        },
    )
}

fn object_base() -> Vec<Value> {
    vec![builtin_type_value("object")]
}

fn text(value: &str) -> Value {
    Value::String(value.to_string())
}

fn error_class() -> Value {
    NativeClass {
        name: "Error",
        module: "_csv",
        doc: None,
        bases: vec![builtin_type_value("Exception")],
        attrs: Vec::new(),
        hidden: Vec::new(),
    }
    .build()
}

/// `_csv.Dialect`, the validated settings type.
fn c_dialect_class() -> Value {
    let mut attrs = methods(
        CSV_C_PREFIX,
        "Dialect",
        &[
            "__new__",
            "__init__",
            "__setattr__",
            "__delattr__",
            "__getattr__",
            "__reduce_ex__",
        ],
    );
    for name in &DIALECT_KEYWORDS[1..] {
        attrs.push((
            (*name).to_string(),
            native_property(name, format!("{CSV_C_PREFIX}Dialect.{name}"), None),
        ));
    }
    NativeClass {
        name: "Dialect",
        module: "_csv",
        doc: Some(C_DIALECT_DOC),
        bases: object_base(),
        attrs,
        hidden: vec![(DIALECT_MARKER.to_string(), Value::Bool(true))],
    }
    .build()
}

fn reader_class() -> Value {
    let mut attrs = methods(
        CSV_C_PREFIX,
        "reader",
        &[
            "__new__",
            "__iter__",
            "__next__",
            "__setattr__",
            "__delattr__",
            "__getattr__",
            "__reduce_ex__",
        ],
    );
    for name in ["dialect", "line_num"] {
        attrs.push((
            name.to_string(),
            native_property(name, format!("{CSV_C_PREFIX}reader.{name}"), None),
        ));
    }
    NativeClass {
        name: "reader",
        module: "_csv",
        doc: Some(READER_DOC),
        bases: object_base(),
        attrs,
        hidden: Vec::new(),
    }
    .build()
}

fn writer_class() -> Value {
    let mut attrs = methods(
        CSV_C_PREFIX,
        "writer",
        &[
            "__new__",
            "writerow",
            "writerows",
            "__setattr__",
            "__delattr__",
            "__getattr__",
            "__reduce_ex__",
        ],
    );
    attrs.push((
        "dialect".to_string(),
        native_property("dialect", format!("{CSV_C_PREFIX}writer.dialect"), None),
    ));
    NativeClass {
        name: "writer",
        module: "_csv",
        doc: Some(WRITER_DOC),
        bases: object_base(),
        attrs,
        hidden: Vec::new(),
    }
    .build()
}

/// `csv.Dialect`, the Python-level base class of dialect descriptions.
fn py_dialect_class() -> Value {
    let mut attrs = vec![
        ("_name".to_string(), text("")),
        ("_valid".to_string(), Value::Bool(false)),
    ];
    for name in [
        "delimiter",
        "quotechar",
        "escapechar",
        "doublequote",
        "skipinitialspace",
        "lineterminator",
        "quoting",
    ] {
        attrs.push((name.to_string(), Value::None));
    }
    attrs.extend(methods(CSV_PREFIX, "Dialect", &["__init__", "_validate"]));
    NativeClass {
        name: "Dialect",
        module: "csv",
        doc: Some(DIALECT_PY_DOC),
        bases: object_base(),
        attrs,
        hidden: Vec::new(),
    }
    .build()
}

fn dialect_subclass(name: &str, doc: &str, base: &Value, attrs: Vec<(&str, Value)>) -> Value {
    NativeClass {
        name,
        module: "csv",
        doc: Some(doc),
        bases: vec![base.clone()],
        attrs: attrs
            .into_iter()
            .map(|(key, value)| (key.to_string(), value))
            .collect(),
        hidden: Vec::new(),
    }
    .build()
}

/// Builds the `csv` module. `string_io` is `io.StringIO` (`csv.StringIO`)
/// and `types` the `types` module, both imported as CPython's csv module imports
/// them.
pub(crate) fn csv_module_value(string_io: Value, types: Value) -> Result<Value, String> {
    let error = error_class();
    let c_dialect = c_dialect_class();
    let py_dialect = py_dialect_class();
    let excel = dialect_subclass(
        "excel",
        "Dialect for comma-separated files as Excel writes them.",
        &py_dialect,
        vec![
            ("delimiter", text(",")),
            ("quotechar", text("\"")),
            ("doublequote", Value::Bool(true)),
            ("skipinitialspace", Value::Bool(false)),
            ("lineterminator", text("\r\n")),
            ("quoting", Value::Number(QUOTE_MINIMAL)),
        ],
    );
    let excel_tab = dialect_subclass(
        "excel_tab",
        "Dialect for tab-separated files as Excel writes them.",
        &excel,
        vec![("delimiter", text("\t"))],
    );
    let unix = dialect_subclass(
        "unix_dialect",
        "Dialect with newline line endings and every field quoted.",
        &py_dialect,
        vec![
            ("delimiter", text(",")),
            ("quotechar", text("\"")),
            ("doublequote", Value::Bool(true)),
            ("skipinitialspace", Value::Bool(false)),
            ("lineterminator", text("\n")),
            ("quoting", Value::Number(QUOTE_ALL)),
        ],
    );
    let dict_reader = NativeClass {
        name: "DictReader",
        module: "csv",
        doc: None,
        bases: object_base(),
        attrs: {
            let mut attrs = methods(CSV_PREFIX, "DictReader", &["__init__", "__iter__"]);
            attrs.push((
                "fieldnames".to_string(),
                native_property(
                    "fieldnames",
                    format!("{CSV_PREFIX}DictReader.fieldnames"),
                    Some(format!("{CSV_PREFIX}DictReader.fieldnames_setter")),
                ),
            ));
            attrs.extend(methods(CSV_PREFIX, "DictReader", &["__next__"]));
            attrs.push(class_getitem());
            attrs
        },
        hidden: Vec::new(),
    }
    .build();
    let dict_writer = NativeClass {
        name: "DictWriter",
        module: "csv",
        doc: None,
        bases: object_base(),
        attrs: {
            let mut attrs = methods(
                CSV_PREFIX,
                "DictWriter",
                &[
                    "__init__",
                    "writeheader",
                    "_dict_to_list",
                    "writerow",
                    "writerows",
                ],
            );
            attrs.push(class_getitem());
            attrs
        },
        hidden: Vec::new(),
    }
    .build();
    let sniffer = NativeClass {
        name: "Sniffer",
        module: "csv",
        doc: Some(SNIFFER_DOC),
        bases: object_base(),
        attrs: methods(
            CSV_PREFIX,
            "Sniffer",
            &[
                "__init__",
                "sniff",
                "_guess_quote_and_delimiter",
                "_guess_delimiter",
                "has_header",
            ],
        ),
        hidden: Vec::new(),
    }
    .build();

    // The registry starts with the three built-in dialects.
    let mut registry = Vec::new();
    for (name, class) in [
        ("excel", &excel),
        ("excel-tab", &excel_tab),
        ("unix", &unix),
    ] {
        let config = config_from_class_attrs(class)?;
        let dialect = native_instance(
            &c_dialect,
            vec![(CONFIG_FIELD.to_string(), config.to_value())],
        )?;
        registry.push((text(name), dialect));
    }

    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        let mut insert = |key: &str, value: Value| {
            values.insert(key.to_string(), value);
        };
        insert("__name__", text("csv"));
        insert("__doc__", text(MODULE_DOC));
        insert("__package__", text(""));
        insert("__version__", text("1.0"));
        insert(
            "__all__",
            list_value(MODULE_ALL.iter().map(|name| text(name)).collect()),
        );
        for (name, value) in [
            ("QUOTE_MINIMAL", QUOTE_MINIMAL),
            ("QUOTE_ALL", QUOTE_ALL),
            ("QUOTE_NONNUMERIC", QUOTE_NONNUMERIC),
            ("QUOTE_NONE", QUOTE_NONE),
            ("QUOTE_STRINGS", QUOTE_STRINGS),
            ("QUOTE_NOTNULL", QUOTE_NOTNULL),
        ] {
            insert(name, Value::Number(value));
        }
        for function in MODULE_FUNCTIONS {
            insert(
                function,
                Value::Builtin(format!("{CSV_C_PREFIX}{function}")),
            );
        }
        insert("Error", error.clone());
        insert("_Dialect", c_dialect.clone());
        insert("Dialect", py_dialect.clone());
        insert("excel", excel);
        insert("excel_tab", excel_tab);
        insert("unix_dialect", unix);
        insert("DictReader", dict_reader);
        insert("DictWriter", dict_writer);
        insert("Sniffer", sniffer);
        insert("StringIO", string_io);
        insert("types", types);
        insert(STATE_ERROR, error);
        insert(STATE_DIALECT_TYPE, c_dialect);
        insert(STATE_READER_TYPE, reader_class());
        insert(STATE_WRITER_TYPE, writer_class());
        insert(STATE_PY_DIALECT, py_dialect);
        insert(STATE_DIALECTS, dict_value(registry));
        insert(STATE_FIELD_LIMIT, Value::Number(DEFAULT_FIELD_LIMIT));
    }
    Ok(Value::Module {
        name: "csv".to_string(),
        attrs: scope,
    })
}

/// The settings of one of the built-in dialect classes, read from its own
/// and inherited class attributes.
fn config_from_class_attrs(class: &Value) -> Result<Config, String> {
    let Value::Class { attrs, bases, .. } = class else {
        return Err("TypeError: dialect class expected".to_string());
    };
    let get = |name: &str| find_class_attr(attrs, bases, name).unwrap_or(Value::None);
    let one = |name: &str| match get(name) {
        Value::String(text) => text.chars().next(),
        _ => None,
    };
    Ok(Config {
        delimiter: one("delimiter").unwrap_or(','),
        quotechar: one("quotechar"),
        escapechar: one("escapechar"),
        doublequote: matches!(get("doublequote"), Value::Bool(true)),
        skipinitialspace: matches!(get("skipinitialspace"), Value::Bool(true)),
        lineterminator: string_of(&get("lineterminator")).unwrap_or_default(),
        lineterminator_value: get("lineterminator"),
        quoting: match get("quoting") {
            Value::Number(quoting) => quoting,
            _ => QUOTE_MINIMAL,
        },
        strict: false,
    })
}

/// The text of a `str` (or `str` subclass) value.
pub(super) fn string_of(value: &Value) -> Option<String> {
    match value {
        Value::String(text) | Value::IdentityString { value: text, .. } => Some(text.clone()),
        other => str_subclass_string(other),
    }
}

/// Whether `value` is a `_csv.Dialect` instance (or of a subclass).
pub(super) fn is_c_dialect(value: &Value) -> bool {
    match value {
        Value::Instance {
            class_attrs,
            class_bases,
            ..
        } => find_class_attr(class_attrs, class_bases, DIALECT_MARKER).is_some(),
        _ => false,
    }
}

/// The attribute-error message for a new attribute on a C type without
/// `__dict__`.
pub(super) fn no_new_attribute(type_name: &str, name: &str, deleting: bool) -> String {
    if deleting {
        format!("AttributeError: '{type_name}' object has no attribute '{name}'")
    } else {
        format!(
            "AttributeError: '{type_name}' object has no attribute '{name}' and no __dict__ for setting new attributes"
        )
    }
}

/// The error `copy.copy`/`copy.deepcopy` raise for `_csv` objects, which
/// cannot be pickled (`None` for other values).
pub(crate) fn copy_error(value: &Value) -> Option<String> {
    let Value::Instance {
        class_attrs,
        class_bases,
        ..
    } = value
    else {
        return None;
    };
    let Some(Value::Builtin(reduce)) = find_class_attr(class_attrs, class_bases, "__reduce_ex__")
    else {
        return None;
    };
    let type_name = reduce
        .strip_prefix(CSV_C_PREFIX)?
        .strip_suffix(".__reduce_ex__")?;
    Some(pickle_error(type_name, value))
}

/// `__reduce_ex__` of the `_csv` types: a `Dialect` (whose constructor
/// takes arguments) reports its class name, the reader and writer their
/// qualified type name.
fn pickle_error(type_name: &str, value: &Value) -> String {
    if type_name == "Dialect" {
        return format!(
            "TypeError: cannot pickle '{}' instances",
            super::type_name(value)
        );
    }
    let shown = match value {
        Value::Instance { class_attrs, .. }
            if class_attrs.borrow().contains_key("__reduce_ex__") =>
        {
            format!("_csv.{type_name}")
        }
        other => super::type_name(other).to_string(),
    };
    format!("TypeError: cannot pickle '{shown}' object")
}

/// The `AttributeError` CPython raises for a missing attribute of a C type,
/// which names the type with its module.
pub(super) fn csv_pickle_error(type_name: &str, value: &Value) -> String {
    pickle_error(type_name, value)
}

/// The `cannot create` error of `_csv.reader`/`_csv.writer` (or of a
/// subclass, named by its own name).
pub(super) fn cannot_create(type_name: &str, class: Option<&Value>) -> String {
    let shown = match class {
        Some(Value::Class { name, attrs, .. }) if !attrs.borrow().contains_key("__new__") => {
            name.clone()
        }
        _ => format!("_csv.{type_name}"),
    };
    format!("TypeError: cannot create '{shown}' instances")
}

pub(super) fn missing_attribute(type_name: &str, args: &[Value]) -> String {
    let name = args.first().and_then(string_of).unwrap_or_default();
    format!("AttributeError: '{type_name}' object has no attribute '{name}'")
}

/// `_set_char` / `_set_char_or_none`: a one-character `str` (or `None`
/// when allowed).
fn csv_char(name: &str, value: &Value, allow_none: bool) -> Result<Option<char>, String> {
    let or_none = if allow_none { " or None" } else { "" };
    if allow_none && matches!(value, Value::None) {
        return Ok(None);
    }
    let Some(text) = string_of(value) else {
        return Err(format!(
            "TypeError: \"{name}\" must be a unicode character{or_none}, not {}",
            type_name(value)
        ));
    };
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => Ok(Some(c)),
        _ => Err(format!(
            "TypeError: \"{name}\" must be a unicode character{or_none}, not a string of length {}",
            text.chars().count()
        )),
    }
}

pub(super) use super::native_class::hidden_field as field;
pub(super) use super::native_class::set_hidden_field as set_field;

impl Vm {
    // ------------------------------------------------------------------
    // Module state
    // ------------------------------------------------------------------

    fn csv_module(&mut self) -> Result<Value, String> {
        if let Some(module @ Value::Module { .. }) = lookup_string_key(&self.module_cache, "csv") {
            return Ok(module);
        }
        self.load_stdlib_module("csv")
    }

    pub(super) fn csv_state(&mut self, key: &str) -> Result<Value, String> {
        let module = self.csv_module()?;
        let Value::Module { attrs, .. } = &module else {
            return Err("SystemError: csv module state is missing".to_string());
        };
        attrs
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| "SystemError: csv module state is missing".to_string())
    }

    fn csv_set_state(&mut self, key: &str, value: Value) -> Result<(), String> {
        let module = self.csv_module()?;
        if let Value::Module { attrs, .. } = &module {
            attrs.borrow_mut().insert(key.to_string(), value);
        }
        Ok(())
    }

    /// Raises `_csv.Error(message)`.
    pub(super) fn csv_error<T>(&mut self, message: impl Into<String>) -> Result<T, String> {
        let class = self.csv_state(STATE_ERROR)?;
        self.native_raise(class, message.into())
    }

    pub(super) fn csv_field_limit(&mut self) -> Result<i64, String> {
        match self.csv_state(STATE_FIELD_LIMIT)? {
            Value::Number(limit) => Ok(limit),
            _ => Ok(DEFAULT_FIELD_LIMIT),
        }
    }

    fn csv_registry(&mut self) -> Result<Value, String> {
        self.csv_state(STATE_DIALECTS)
    }

    /// `registry.<method>(key, sentinel)`, telling a missing key apart.
    fn csv_registry_call(&mut self, method: &str, name: &Value) -> Result<Option<Value>, String> {
        let registry = self.csv_registry()?;
        let function = self.load_attribute_value(registry, method)?;
        let missing = Value::Builtin("\0csv_missing".to_string());
        // `str` subclasses hash and compare like their text.
        let key = string_of(name).map_or_else(|| name.clone(), Value::String);
        let found = self.call_value(function, vec![key, missing.clone()])?;
        Ok(if found == missing { None } else { Some(found) })
    }

    /// The registered dialect `name`, or `_csv.Error("unknown dialect")`.
    fn csv_registry_get(&mut self, name: &Value) -> Result<Value, String> {
        match self.csv_registry_call("get", name)? {
            Some(dialect) => Ok(dialect),
            None => self.csv_error("unknown dialect"),
        }
    }

    // ------------------------------------------------------------------
    // Dispatch
    // ------------------------------------------------------------------

    pub(super) fn csv_dispatch(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if let Some(rest) = name.strip_prefix(CSV_C_PREFIX) {
            return match rest.split_once('.') {
                None => self.csv_module_function(rest, args, keywords),
                Some(("Dialect", method)) => self.csv_c_dialect_method(method, args, keywords),
                Some(("reader", method)) => self.csv_reader_method(method, args, keywords),
                Some(("writer", method)) => self.csv_writer_method(method, args, keywords),
                _ => Err(format!("unknown builtin: {name}")),
            };
        }
        let rest = name.strip_prefix(CSV_PREFIX).unwrap_or(name);
        match rest.split_once('.') {
            Some(("Dialect", method)) => self.csv_py_dialect_method(method, args, keywords),
            Some(("DictReader", method)) => self.csv_dict_reader_method(method, args, keywords),
            Some(("DictWriter", method)) => self.csv_dict_writer_method(method, args, keywords),
            Some(("Sniffer", method)) => self.csv_sniffer_method(method, args, keywords),
            _ => Err(format!("unknown builtin: {name}")),
        }
    }

    fn csv_module_function(
        &mut self,
        function: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match function {
            "reader" => self.csv_new_reader(args, keywords),
            "writer" => self.csv_new_writer(args, keywords),
            "register_dialect" => {
                let [name, dialect] = c_unpack_positional("register_dialect", args, 1, 2)?
                    .try_into()
                    .expect("two slots");
                let name = name.expect("required");
                if string_of(&name).is_none() {
                    return Err("TypeError: dialect name must be a string".to_string());
                }
                let dialect = self.csv_call_dialect(dialect, keywords)?;
                let registry = self.csv_registry()?;
                let setitem = self.load_attribute_value(registry, "__setitem__")?;
                let key = string_of(&name).map_or(name, Value::String);
                self.call_value(setitem, vec![key, dialect])?;
                Ok(Value::None)
            }
            "get_dialect" => {
                let [name] = c_clinic("get_dialect", &["name"], 1, args, keywords)?
                    .try_into()
                    .expect("one slot");
                self.csv_registry_get(&name.expect("required"))
            }
            "unregister_dialect" => {
                let [name] = c_clinic("unregister_dialect", &["name"], 1, args, keywords)?
                    .try_into()
                    .expect("one slot");
                match self.csv_registry_call("pop", &name.expect("required"))? {
                    Some(_) => Ok(Value::None),
                    None => self.csv_error("unknown dialect"),
                }
            }
            "list_dialects" => {
                c_no_args("_csv.list_dialects", &args, &keywords)?;
                let registry = self.csv_registry()?;
                self.call_value(Value::Builtin("list".to_string()), vec![registry])
            }
            "field_size_limit" => {
                let [limit] = c_clinic("field_size_limit", &["new_limit"], 0, args, keywords)?
                    .try_into()
                    .expect("one slot");
                let old = self.csv_field_limit()?;
                if let Some(limit) = limit {
                    let limit = match limit {
                        Value::Number(limit) => limit,
                        Value::BigInt(_) => {
                            return Err(
                                "OverflowError: Python int too large to convert to C ssize_t"
                                    .to_string(),
                            );
                        }
                        _ => return Err("TypeError: limit must be an integer".to_string()),
                    };
                    self.csv_set_state(STATE_FIELD_LIMIT, Value::Number(limit))?;
                }
                Ok(Value::Number(old))
            }
            _ => Err(format!("unknown builtin: {CSV_C_PREFIX}{function}")),
        }
    }

    // ------------------------------------------------------------------
    // _csv.Dialect
    // ------------------------------------------------------------------

    /// `_csv.Dialect(dialect, **kwargs)`, as the reader, the writer, and
    /// `register_dialect` call it.
    pub(super) fn csv_call_dialect(
        &mut self,
        dialect: Option<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let class = self.csv_state(STATE_DIALECT_TYPE)?;
        let args = dialect.into_iter().collect();
        self.csv_dialect_new(class, args, keywords)
    }

    /// `_csv.Dialect.__new__`: CPython's argument binding with
    /// an unnamed function, then the setting conversions and checks.
    fn csv_dialect_new(
        &mut self,
        class: Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let total = DIALECT_KEYWORDS.len();
        if args.len() + keywords.len() > total {
            return Err(format!(
                "TypeError: function takes at most {total} arguments ({} given)",
                args.len() + keywords.len()
            ));
        }
        let mut slots: Vec<Option<Value>> = vec![None; total];
        let positional = args.len();
        for (index, value) in args.into_iter().enumerate() {
            slots[index] = Some(value);
        }
        for (keyword, value) in keywords {
            let Some(index) = DIALECT_KEYWORDS.iter().position(|name| *name == keyword) else {
                return Err(format!(
                    "TypeError: this function got an unexpected keyword argument '{keyword}'"
                ));
            };
            if index < positional {
                return Err(format!(
                    "TypeError: argument for function given by name ('{keyword}') and position ({})",
                    index + 1
                ));
            }
            slots[index] = Some(value);
        }
        let mut dialect = slots[0].take();
        if let Some(name) = &dialect
            && string_of(name).is_some()
        {
            dialect = Some(self.csv_registry_get(name)?);
        }
        if let Some(existing) = &dialect
            && is_c_dialect(existing)
            && slots[1..].iter().all(Option::is_none)
        {
            return Ok(existing.clone());
        }
        if let Some(source) = &dialect {
            for (index, name) in DIALECT_KEYWORDS.iter().enumerate().skip(1) {
                if slots[index].is_none() {
                    // CPython ignores any error reading a setting from the
                    // dialect object and uses the default instead.
                    slots[index] = self.load_attribute_catching(source.clone(), name)?.ok();
                }
            }
        }
        let config = self.csv_validate(&slots)?;
        native_instance(&class, vec![(CONFIG_FIELD.to_string(), config.to_value())])
    }

    /// The conversions and checks of `_csv.Dialect.__new__`, in CPython's
    /// order. `slots` follows `DIALECT_KEYWORDS`.
    fn csv_validate(&mut self, slots: &[Option<Value>]) -> Result<Config, String> {
        let slot = |index: usize| slots[index].clone();
        let delimiter = match slot(1) {
            None => ',',
            Some(value) => csv_char("delimiter", &value, false)?.unwrap_or(','),
        };
        let doublequote = match slot(2) {
            None => true,
            Some(value) => self.truth_value(value)?,
        };
        let escapechar = match slot(3) {
            None => None,
            Some(value) => csv_char("escapechar", &value, true)?,
        };
        let lineterminator_value = match slot(4) {
            None => text("\r\n"),
            Some(value) => {
                if string_of(&value).is_none() {
                    return Err(format!(
                        "TypeError: \"lineterminator\" must be a string, not {}",
                        type_name(&value)
                    ));
                }
                value
            }
        };
        let quotechar_value = slot(5);
        let quotechar = match &quotechar_value {
            None => Some('"'),
            Some(value) => csv_char("quotechar", value, true)?,
        };
        let quoting_value = slot(6);
        let mut quoting = match &quoting_value {
            None => QUOTE_MINIMAL,
            Some(Value::Number(quoting)) => {
                if i32::try_from(*quoting).is_err() {
                    return Err(
                        "OverflowError: Python int too large to convert to C int".to_string()
                    );
                }
                *quoting
            }
            Some(Value::BigInt(_)) => {
                return Err("OverflowError: Python int too large to convert to C int".to_string());
            }
            Some(value) => {
                return Err(format!(
                    "TypeError: \"quoting\" must be an integer, not {}",
                    type_name(value)
                ));
            }
        };
        let skipinitialspace = match slot(7) {
            None => false,
            Some(value) => self.truth_value(value)?,
        };
        let strict = match slot(8) {
            None => false,
            Some(value) => self.truth_value(value)?,
        };
        if !(QUOTE_MINIMAL..=QUOTE_NOTNULL).contains(&quoting) {
            return Err("TypeError: bad \"quoting\" value".to_string());
        }
        if matches!(quotechar_value, Some(Value::None)) && quoting_value.is_none() {
            quoting = QUOTE_NONE;
        }
        if quoting != QUOTE_NONE && quotechar.is_none() {
            return Err("TypeError: quotechar must be set if quoting enabled".to_string());
        }
        let lineterminator = string_of(&lineterminator_value).unwrap_or_default();
        let bad_char = |name: &str, c: Option<char>, allow_space: bool| -> Result<(), String> {
            match c {
                Some('\r' | '\n') => Err(format!("ValueError: bad {name} value")),
                Some(' ') if !allow_space => Err(format!("ValueError: bad {name} value")),
                Some(c) if lineterminator.contains(c) => {
                    Err(format!("ValueError: bad {name} or lineterminator value"))
                }
                _ => Ok(()),
            }
        };
        bad_char("delimiter", Some(delimiter), true)?;
        bad_char("escapechar", escapechar, !skipinitialspace)?;
        bad_char("quotechar", quotechar, !skipinitialspace)?;
        for (first, second, a, b) in [
            ("delimiter", "escapechar", Some(delimiter), escapechar),
            ("delimiter", "quotechar", Some(delimiter), quotechar),
            ("escapechar", "quotechar", escapechar, quotechar),
        ] {
            if a.is_some() && a == b {
                return Err(format!("ValueError: bad {first} or {second} value"));
            }
        }
        Ok(Config {
            delimiter,
            quotechar,
            escapechar,
            doublequote,
            skipinitialspace,
            lineterminator_value,
            lineterminator,
            quoting,
            strict,
        })
    }

    fn csv_c_dialect_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if args.is_empty() {
            return Err(format!(
                "TypeError: descriptor '{method}' of '_csv.Dialect' object needs an argument"
            ));
        }
        let receiver = args.remove(0);
        match method {
            "__new__" => self.csv_dialect_new(receiver, args, keywords),
            "__init__" => Ok(Value::None),
            "__setattr__" | "__delattr__" => {
                let name = args.first().and_then(string_of).unwrap_or_default();
                if DIALECT_KEYWORDS[1..].contains(&name.as_str()) {
                    return Err(format!(
                        "AttributeError: attribute '{name}' of '_csv.Dialect' objects is not writable"
                    ));
                }
                Err(no_new_attribute(
                    "_csv.Dialect",
                    &name,
                    method == "__delattr__",
                ))
            }
            "__reduce_ex__" => Err(pickle_error("Dialect", &receiver)),
            "__getattr__" => {
                let exact = matches!(&receiver, Value::Instance { class_attrs, .. } if class_attrs.borrow().contains_key(DIALECT_MARKER));
                let shown = if exact {
                    "_csv.Dialect"
                } else {
                    type_name(&receiver)
                };
                Err(missing_attribute(shown, &args))
            }
            name => Ok(dialect_config(&receiver)
                .map(|config| config.attribute(name))
                .unwrap_or(Value::None)),
        }
    }

    // ------------------------------------------------------------------
    // csv.Dialect (the Python-level base class)
    // ------------------------------------------------------------------

    fn csv_py_dialect_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let qualname = match method {
            "__init__" => "Dialect.__init__",
            _ => "Dialect._validate",
        };
        let bound = bind_args(
            &Sig {
                qualname,
                params: &["self"],
                required: 1,
                kwonly: &[],
                varargs: false,
                varkw: false,
            },
            args,
            keywords,
        )?;
        let receiver = bound.arg(0);
        if method == "__init__" {
            // `if self.__class__ != Dialect: self._valid = True`
            let class = self.load_attribute_value(receiver.clone(), "__class__")?;
            let base = self.csv_state(STATE_PY_DIALECT)?;
            if !self.equal_values(class, base)? {
                self.store_attribute_value(receiver.clone(), "_valid", Value::Bool(true))?;
            }
            let validate = self.load_attribute_value(receiver, "_validate")?;
            self.call_value(validate, Vec::new())?;
            return Ok(Value::None);
        }
        // `_Dialect(self)`, with a TypeError re-raised as Error from None.
        match self
            .call_with_exception_capture(|vm| vm.csv_call_dialect(Some(receiver), Vec::new()))?
        {
            Ok(_) => Ok(Value::None),
            Err(exception) if exception.type_name == "TypeError" => {
                let message = exception.message.clone().unwrap_or_default();
                let class = self.csv_state(STATE_ERROR)?;
                let error = self.call_value(class, vec![Value::String(message)])?;
                let mut error = exception_from_value(error)?;
                error.context = Some(Box::new(exception));
                error.suppress_context = true;
                self.raise_exception(error, true)
            }
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }
}
