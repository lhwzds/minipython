// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/argparse.py, Python/suggestions.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Native `argparse` module.
//!
//! The module exposes real classes (`ArgumentParser`, `Namespace`, the
//! action classes, the help formatters, ...) whose methods are Rust builtins
//! named `argparse.<qualname>.<method>`. Every collaboration point CPython
//! reaches through attribute lookup (`self.error`, `action(...)`,
//! `formatter._format_action`, `type_func(...)`, ...) is also reached through
//! attribute lookup here, so user subclasses override behavior exactly where
//! CPython 3.14 lets them. All parser state lives in ordinary instance
//! attributes (`_actions`, `_option_string_actions`, `_defaults`, ...).
//!
//! Each method body is its own `#[inline(never)]` function and calls between
//! this module's own methods skip the VM's generic call path, so parsers
//! nested through subcommands need little native stack.

use super::*;

mod actions;
mod container;
mod formatter;
mod parser;
mod textwrap;

/// Prefix of every native `argparse` builtin.
pub(crate) const ARGPARSE_PREFIX: &str = "argparse.";
/// Hidden class attribute marking `argparse.ArgumentError` (and subclasses
/// through the MRO) so the VM can build its exception instances natively.
pub(crate) const ARGPARSE_ARGUMENT_ERROR_MARKER: &str = "\0argparse_argument_error";

pub(super) const SUPPRESS: &str = "==SUPPRESS==";
pub(super) const OPTIONAL: &str = "?";
pub(super) const ZERO_OR_MORE: &str = "*";
pub(super) const ONE_OR_MORE: &str = "+";
pub(super) const PARSER: &str = "A...";
pub(super) const REMAINDER: &str = "...";
pub(super) const UNRECOGNIZED_ARGS_ATTR: &str = "_unrecognized_args";

const MODULE_DOC: &str =
    "Parse command-line arguments into a namespace and generate usage and help text.";

const MODULE_ALL: &[&str] = &[
    "ArgumentParser",
    "ArgumentError",
    "ArgumentTypeError",
    "BooleanOptionalAction",
    "FileType",
    "HelpFormatter",
    "ArgumentDefaultsHelpFormatter",
    "RawDescriptionHelpFormatter",
    "RawTextHelpFormatter",
    "MetavarTypeHelpFormatter",
    "Namespace",
    "Action",
    "ONE_OR_MORE",
    "OPTIONAL",
    "PARSER",
    "REMAINDER",
    "SUPPRESS",
    "ZERO_OR_MORE",
];

/// One native class: its qualified name, bases (argparse class qualnames,
/// or `object` / `Exception`), and the methods implemented in Rust.
struct ClassSpec {
    qualname: &'static str,
    bases: &'static [&'static str],
    methods: &'static [&'static str],
    doc: Option<&'static str>,
}

const ACTION_DOC: &str =
    "Base class for argument actions; it holds the settings given to add_argument().";

const CLASS_SPECS: &[ClassSpec] = &[
    ClassSpec {
        qualname: "_AttributeHolder",
        bases: &["object"],
        methods: &["__repr__", "_get_kwargs", "_get_args"],
        doc: Some("Base class giving a repr built from the instance attributes."),
    },
    ClassSpec {
        qualname: "HelpFormatter",
        bases: &["object"],
        methods: &[
            "__init__",
            "_set_color",
            "_indent",
            "_dedent",
            "_add_item",
            "start_section",
            "end_section",
            "add_text",
            "add_usage",
            "add_argument",
            "add_arguments",
            "format_help",
            "_join_parts",
            "_format_usage",
            "_is_long_option",
            "_get_actions_usage_parts",
            "_format_text",
            "_format_action",
            "_format_action_invocation",
            "_metavar_formatter",
            "_format_args",
            "_expand_help",
            "_iter_indented_subactions",
            "_split_lines",
            "_fill_text",
            "_get_help_string",
            "_get_default_metavar_for_optional",
            "_get_default_metavar_for_positional",
        ],
        doc: Some("Default formatter for usage and help text."),
    },
    ClassSpec {
        qualname: "HelpFormatter._Section",
        bases: &["object"],
        methods: &["__init__", "format_help"],
        doc: None,
    },
    ClassSpec {
        qualname: "RawDescriptionHelpFormatter",
        bases: &["HelpFormatter"],
        methods: &["_fill_text"],
        doc: Some("Help formatter that keeps description and epilog text as written."),
    },
    ClassSpec {
        qualname: "RawTextHelpFormatter",
        bases: &["RawDescriptionHelpFormatter"],
        methods: &["_split_lines"],
        doc: Some("Help formatter that keeps all help text as written."),
    },
    ClassSpec {
        qualname: "ArgumentDefaultsHelpFormatter",
        bases: &["HelpFormatter"],
        methods: &["_get_help_string"],
        doc: Some("Help formatter that appends each argument's default to its help."),
    },
    ClassSpec {
        qualname: "MetavarTypeHelpFormatter",
        bases: &["HelpFormatter"],
        methods: &[
            "_get_default_metavar_for_optional",
            "_get_default_metavar_for_positional",
        ],
        doc: Some("Help formatter that names argument values after their type."),
    },
    ClassSpec {
        qualname: "ArgumentError",
        bases: &["Exception"],
        methods: &[],
        doc: Some("Raised for a bad argument definition or value; the message names the argument."),
    },
    ClassSpec {
        qualname: "ArgumentTypeError",
        bases: &["Exception"],
        methods: &[],
        doc: Some("Raised by a type converter to reject a command-line string."),
    },
    ClassSpec {
        qualname: "Action",
        bases: &["_AttributeHolder"],
        methods: &["__init__", "_get_kwargs", "format_usage", "__call__"],
        doc: Some(ACTION_DOC),
    },
    ClassSpec {
        qualname: "BooleanOptionalAction",
        bases: &["Action"],
        methods: &["__init__", "__call__", "format_usage"],
        doc: None,
    },
    ClassSpec {
        qualname: "_StoreAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_StoreConstAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_StoreTrueAction",
        bases: &["_StoreConstAction"],
        methods: &["__init__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_StoreFalseAction",
        bases: &["_StoreConstAction"],
        methods: &["__init__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_AppendAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_AppendConstAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_CountAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_HelpAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_VersionAction",
        bases: &["Action"],
        methods: &["__init__", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_SubParsersAction",
        bases: &["Action"],
        methods: &["__init__", "add_parser", "_get_subactions", "__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_SubParsersAction._ChoicesPseudoAction",
        bases: &["Action"],
        methods: &["__init__"],
        doc: None,
    },
    ClassSpec {
        qualname: "_ExtendAction",
        bases: &["_AppendAction"],
        methods: &["__call__"],
        doc: None,
    },
    ClassSpec {
        qualname: "FileType",
        bases: &["object"],
        methods: &["__init__", "__call__", "__repr__"],
        doc: Some("Deprecated converter that opens the named file with the given mode."),
    },
    ClassSpec {
        qualname: "Namespace",
        bases: &["_AttributeHolder"],
        methods: &["__init__", "__eq__", "__contains__"],
        doc: Some(
            "Plain attribute container returned by parse_args(), compared by its attributes.",
        ),
    },
    ClassSpec {
        qualname: "_ActionsContainer",
        bases: &["object"],
        methods: &[
            "__init__",
            "register",
            "_registry_get",
            "set_defaults",
            "get_default",
            "add_argument",
            "add_argument_group",
            "add_mutually_exclusive_group",
            "_add_action",
            "_remove_action",
            "_add_container_actions",
            "_get_positional_kwargs",
            "_get_optional_kwargs",
            "_pop_action_class",
            "_get_handler",
            "_check_conflict",
            "_handle_conflict_error",
            "_handle_conflict_resolve",
            "_check_help",
        ],
        doc: None,
    },
    ClassSpec {
        qualname: "_ArgumentGroup",
        bases: &["_ActionsContainer"],
        methods: &[
            "__init__",
            "_add_action",
            "_remove_action",
            "add_argument_group",
        ],
        doc: None,
    },
    ClassSpec {
        qualname: "_MutuallyExclusiveGroup",
        bases: &["_ArgumentGroup"],
        methods: &[
            "__init__",
            "_add_action",
            "_remove_action",
            "add_mutually_exclusive_group",
        ],
        doc: None,
    },
    ClassSpec {
        qualname: "ArgumentParser",
        bases: &["_AttributeHolder", "_ActionsContainer"],
        methods: &[
            "__init__",
            "_get_kwargs",
            "add_subparsers",
            "_add_action",
            "_get_optional_actions",
            "_get_positional_actions",
            "parse_args",
            "parse_known_args",
            "_parse_known_args2",
            "_parse_known_args",
            "_read_args_from_files",
            "convert_arg_line_to_args",
            "_match_argument",
            "_match_arguments_partial",
            "_parse_optional",
            "_get_option_tuples",
            "_get_nargs_pattern",
            "parse_intermixed_args",
            "parse_known_intermixed_args",
            "_get_values",
            "_get_value",
            "_check_value",
            "format_usage",
            "format_help",
            "_get_formatter",
            "_get_validation_formatter",
            "print_usage",
            "print_help",
            "_print_message",
            "exit",
            "error",
            "_warning",
        ],
        doc: Some(
            "Command-line parser that turns argument strings into a Namespace and prints usage and help.",
        ),
    },
];

/// Module-level functions implemented natively.
const MODULE_FUNCTIONS: &[&str] = &["_get_action_name", "_copy_items", "_identity", "_prog_name"];

fn argparse_builtin(qualname: &str, method: &str) -> Value {
    Value::Builtin(format!("{ARGPARSE_PREFIX}{qualname}.{method}"))
}

/// Builds the `argparse` module value.
pub(crate) fn argparse_module_value() -> Value {
    let mut classes: Vec<(&'static str, Value)> = Vec::new();
    for spec in CLASS_SPECS {
        let bases = spec
            .bases
            .iter()
            .map(|base| match *base {
                "object" | "Exception" => builtin_type_value(base),
                name => classes
                    .iter()
                    .find(|(qualname, _)| *qualname == name)
                    .map(|(_, class)| class.clone())
                    .expect("argparse base classes are declared first"),
            })
            .collect::<Vec<_>>();
        let name = spec.qualname.rsplit('.').next().unwrap_or(spec.qualname);
        let scope = new_scope();
        {
            let mut values = scope.borrow_mut();
            let mut order = Vec::new();
            let mut insert = |key: &str, value: Value| {
                order.push(Value::String(key.to_string()));
                values.insert(key.to_string(), value);
            };
            insert("__module__", Value::String("argparse".to_string()));
            insert("__qualname__", Value::String(spec.qualname.to_string()));
            insert(
                "__doc__",
                spec.doc
                    .map(|doc| Value::String(doc.to_string()))
                    .unwrap_or(Value::None),
            );
            for method in spec.methods {
                insert(method, argparse_builtin(spec.qualname, method));
            }
            if spec.qualname == "Namespace" {
                // Defining __eq__ without __hash__ makes instances unhashable.
                insert("__hash__", Value::None);
            }
            values.insert(
                CLASS_QUALNAME_ATTR.to_string(),
                Value::String(spec.qualname.to_string()),
            );
            if spec.qualname == "ArgumentError" {
                values.insert(
                    ARGPARSE_ARGUMENT_ERROR_MARKER.to_string(),
                    Value::Bool(true),
                );
            }
            values.insert(CLASS_ATTR_ORDER_ATTR.to_string(), tuple_value(order));
        }
        let class = Value::Class {
            name: name.to_string(),
            type_params: Vec::new(),
            metaclass: None,
            bases,
            attrs: scope,
        };
        classes.push((spec.qualname, class));
    }

    // Nested classes are also reachable as attributes of their owner.
    for (nested, owner, attr) in [
        ("HelpFormatter._Section", "HelpFormatter", "_Section"),
        (
            "_SubParsersAction._ChoicesPseudoAction",
            "_SubParsersAction",
            "_ChoicesPseudoAction",
        ),
    ] {
        let nested = classes
            .iter()
            .find(|(qualname, _)| *qualname == nested)
            .map(|(_, class)| class.clone())
            .expect("nested class exists");
        if let Some((_, Value::Class { attrs, .. })) =
            classes.iter().find(|(qualname, _)| *qualname == owner)
        {
            store_ordered_scope_item(attrs, attr.to_string(), nested);
        }
    }

    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        let mut insert = |key: &str, value: Value| {
            values.insert(key.to_string(), value);
        };
        insert("__name__", Value::String("argparse".to_string()));
        insert("__doc__", Value::String(MODULE_DOC.to_string()));
        insert("__package__", Value::String(String::new()));
        insert("__version__", Value::String("1.1".to_string()));
        insert(
            "__all__",
            list_value(
                MODULE_ALL
                    .iter()
                    .map(|name| Value::String((*name).to_string()))
                    .collect(),
            ),
        );
        for (name, value) in [
            ("SUPPRESS", SUPPRESS),
            ("OPTIONAL", OPTIONAL),
            ("ZERO_OR_MORE", ZERO_OR_MORE),
            ("ONE_OR_MORE", ONE_OR_MORE),
            ("PARSER", PARSER),
            ("REMAINDER", REMAINDER),
            ("_UNRECOGNIZED_ARGS_ATTR", UNRECOGNIZED_ARGS_ATTR),
        ] {
            insert(name, Value::String(value.to_string()));
        }
        for function in MODULE_FUNCTIONS {
            insert(
                function,
                Value::Builtin(format!("{ARGPARSE_PREFIX}{function}")),
            );
        }
        for (qualname, class) in &classes {
            if !qualname.contains('.') {
                insert(qualname, class.clone());
            }
        }
        insert(
            "\0_IndentedSubactions",
            formatter::indented_subactions_class(),
        );
        insert("\0_theme", formatter::no_color_theme());
    }
    Value::Module {
        name: "argparse".to_string(),
        attrs: scope,
    }
}

/// Whether a builtin name belongs to this module.
pub(crate) fn is_argparse_builtin(name: &str) -> bool {
    name.starts_with(ARGPARSE_PREFIX)
}

/// Native argument specification of one method or function.
pub(super) struct Sig {
    pub qualname: &'static str,
    pub params: &'static [&'static str],
    pub required: usize,
    pub kwonly: &'static [&'static str],
    pub varargs: bool,
    pub varkw: bool,
}

/// Arguments bound to a [`Sig`]: positional-or-keyword slots followed by
/// keyword-only slots, plus any extra `*args` / `**kwargs`.
pub(super) struct Bound {
    pub values: Vec<Option<Value>>,
    pub extra_args: Vec<Value>,
    pub extra_kwargs: Vec<(String, Value)>,
}

impl Bound {
    pub fn get(&self, index: usize) -> Option<Value> {
        self.values.get(index).cloned().flatten()
    }

    pub fn arg(&self, index: usize) -> Value {
        self.get(index).unwrap_or(Value::None)
    }

    pub fn or(&self, index: usize, default: Value) -> Value {
        self.get(index).unwrap_or(default)
    }
}

fn plural_suffix(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn quoted_name_list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// Binds native call arguments the way CPython binds a Python function's
/// parameters, with CPython's error messages.
pub(super) fn bind_args(
    sig: &Sig,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Bound, String> {
    let qualname = sig.qualname;
    let total = sig.params.len();
    let mut values: Vec<Option<Value>> = vec![None; total + sig.kwonly.len()];
    let mut extra_args = Vec::new();
    let given_positional = args.len();
    for (index, value) in args.into_iter().enumerate() {
        if index < total {
            values[index] = Some(value);
        } else if sig.varargs {
            extra_args.push(value);
        } else {
            let given_keyword_only = keywords
                .iter()
                .filter(|(name, _)| sig.kwonly.contains(&name.as_str()))
                .count();
            let takes = if sig.required == total {
                format!("{total} positional argument{}", plural_suffix(total))
            } else {
                format!("from {} to {total} positional arguments", sig.required)
            };
            let was = if given_positional == 1 { "was" } else { "were" };
            if given_keyword_only > 0 {
                return Err(format!(
                    "TypeError: {qualname}() takes {takes} but {given_positional} positional argument{} (and {given_keyword_only} keyword-only argument{}) were given",
                    plural_suffix(given_positional),
                    plural_suffix(given_keyword_only)
                ));
            }
            return Err(format!(
                "TypeError: {qualname}() takes {takes} but {given_positional} {was} given"
            ));
        }
    }
    let mut extra_kwargs = Vec::new();
    for (keyword, value) in keywords {
        if let Some(index) = sig.params.iter().position(|name| *name == keyword) {
            if values[index].is_some() {
                return Err(format!(
                    "TypeError: {qualname}() got multiple values for argument '{keyword}'"
                ));
            }
            values[index] = Some(value);
        } else if let Some(index) = sig.kwonly.iter().position(|name| *name == keyword) {
            values[total + index] = Some(value);
        } else if sig.varkw {
            extra_kwargs.push((keyword, value));
        } else {
            let candidates = sig
                .params
                .iter()
                .chain(sig.kwonly.iter())
                .copied()
                .collect::<Vec<_>>();
            let suggestion = suggest_keyword(&keyword, &candidates)
                .map(|name| format!(". Did you mean '{name}'?"))
                .unwrap_or_default();
            return Err(format!(
                "TypeError: {qualname}() got an unexpected keyword argument '{keyword}'{suggestion}"
            ));
        }
    }
    let missing = sig.params[..sig.required]
        .iter()
        .zip(values.iter())
        .filter(|(_, value)| value.is_none())
        .map(|(name, _)| format!("'{name}'"))
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        return Err(format!(
            "TypeError: {qualname}() missing {} required positional argument{}: {}",
            missing.len(),
            plural_suffix(missing.len()),
            quoted_name_list(&missing)
        ));
    }
    Ok(Bound {
        values,
        extra_args,
        extra_kwargs,
    })
}

/// Keyword-name suggestion: the
/// closest candidate by a case-aware Levenshtein distance, when no more than
/// a third of the characters differ.
pub(super) fn suggest_keyword(name: &str, candidates: &[&str]) -> Option<String> {
    const MOVE_COST: usize = 2;
    let mut best: Option<(&str, usize)> = None;
    for candidate in candidates {
        if *candidate == name {
            continue;
        }
        let mut max_distance = (name.len() + candidate.len() + 3) * MOVE_COST / 6;
        if let Some((_, distance)) = best {
            max_distance = max_distance.min(distance.saturating_sub(1));
        }
        let distance = suggestion_distance(name.as_bytes(), candidate.as_bytes(), max_distance);
        if distance > max_distance {
            continue;
        }
        if best.is_none_or(|(_, best_distance)| distance < best_distance) {
            best = Some((candidate, distance));
        }
    }
    best.map(|(candidate, _)| candidate.to_string())
}

fn suggestion_distance(a: &[u8], b: &[u8], max_cost: usize) -> usize {
    const MOVE_COST: usize = 2;
    const CASE_COST: usize = 1;
    const MAX_STRING_SIZE: usize = 40;
    let substitution_cost = |x: u8, y: u8| -> usize {
        if x & 31 != y & 31 {
            return MOVE_COST;
        }
        if x == y {
            return 0;
        }
        if x.to_ascii_lowercase() == y.to_ascii_lowercase() {
            CASE_COST
        } else {
            MOVE_COST
        }
    };
    let mut a = a;
    let mut b = b;
    while !a.is_empty() && !b.is_empty() && a[0] == b[0] {
        a = &a[1..];
        b = &b[1..];
    }
    while !a.is_empty() && !b.is_empty() && a[a.len() - 1] == b[b.len() - 1] {
        a = &a[..a.len() - 1];
        b = &b[..b.len() - 1];
    }
    if a.is_empty() || b.is_empty() {
        return (a.len() + b.len()) * MOVE_COST;
    }
    if a.len() > MAX_STRING_SIZE || b.len() > MAX_STRING_SIZE {
        return max_cost + 1;
    }
    if b.len() < a.len() {
        std::mem::swap(&mut a, &mut b);
    }
    if (b.len() - a.len()) * MOVE_COST > max_cost {
        return max_cost + 1;
    }
    let mut buffer = (1..=a.len()).map(|i| i * MOVE_COST).collect::<Vec<_>>();
    let mut result = 0;
    for (b_index, code) in b.iter().enumerate() {
        let mut distance = b_index * MOVE_COST;
        result = distance;
        let mut minimum = usize::MAX;
        for (index, item) in a.iter().enumerate() {
            let substitute = distance + substitution_cost(*code, *item);
            distance = buffer[index];
            let insert_delete = result.min(distance) + MOVE_COST;
            result = insert_delete.min(substitute);
            buffer[index] = result;
            minimum = minimum.min(result);
        }
        if minimum > max_cost {
            return max_cost + 1;
        }
    }
    result
}

/// Interpretation of an action's `nargs` value.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Nargs {
    Single,
    Optional,
    ZeroOrMore,
    OneOrMore,
    Remainder,
    Parser,
    Suppress,
    Count(i64),
    Other,
}

pub(super) fn nargs_kind(value: &Value) -> Nargs {
    match value {
        Value::None => Nargs::Single,
        Value::String(text) | Value::IdentityString { value: text, .. } => match text.as_str() {
            OPTIONAL => Nargs::Optional,
            ZERO_OR_MORE => Nargs::ZeroOrMore,
            ONE_OR_MORE => Nargs::OneOrMore,
            REMAINDER => Nargs::Remainder,
            PARSER => Nargs::Parser,
            SUPPRESS => Nargs::Suppress,
            _ => Nargs::Other,
        },
        Value::Number(count) => Nargs::Count(*count),
        Value::Bool(flag) => Nargs::Count(i64::from(*flag)),
        _ => Nargs::Other,
    }
}

pub(super) fn is_suppress(value: &Value) -> bool {
    matches!(value, Value::String(text) | Value::IdentityString { value: text, .. } if text == SUPPRESS)
}

pub(super) fn string_of(value: &Value) -> Option<&str> {
    match value {
        Value::String(text) | Value::IdentityString { value: text, .. } => Some(text),
        _ => None,
    }
}

pub(super) fn str_value(text: impl Into<String>) -> Value {
    Value::String(text.into())
}

/// Object identity for the values argparse keys by identity (actions,
/// groups, parsers).
pub(super) fn same_object(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Instance { fields: left, .. }, Value::Instance { fields: right, .. }) => {
            Rc::ptr_eq(left, right)
        }
        (Value::Class { attrs: left, .. }, Value::Class { attrs: right, .. }) => {
            Rc::ptr_eq(left, right)
        }
        (Value::Builtin(left), Value::Builtin(right)) => left == right,
        _ => is_identical(left, right),
    }
}

/// Whether `exception` is an instance of the builtin exception class `name`.
pub(super) fn exception_is(exception: &MiniException, name: &str) -> bool {
    super::exception_is_builtin_instance(exception, name)
}

/// Whether `class` (a class value) is `target` or derives from it.
pub(super) fn class_derives_from(class: &Value, target: &Value) -> bool {
    if same_object(class, target) {
        return true;
    }
    match class {
        Value::Class { bases, .. } => bases.iter().any(|base| class_derives_from(base, target)),
        _ => false,
    }
}

/// `os.path.basename` for POSIX paths.
pub(super) fn path_basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(index) => &path[index + 1..],
        None => path,
    }
}

impl Vm {
    /// Entry point for every `argparse.*` builtin. Exception handlers of the
    /// calling frame are isolated so an exception raised anywhere in the
    /// native code propagates out of it instead of jumping to a handler.
    pub(super) fn call_argparse_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let name = name.to_string();
        match self.call_with_exception_capture(|vm| vm.argparse_dispatch(&name, args, keywords))? {
            Ok(value) => Ok(value),
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }

    pub(super) fn argparse_dispatch(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let rest = name.strip_prefix(ARGPARSE_PREFIX).unwrap_or(name);
        let (class, method) = match rest.rsplit_once('.') {
            Some((class, method)) => (class, method),
            None => ("", rest),
        };
        match class {
            "" => self.argparse_module_function(method, args, keywords),
            "_IndentedSubactions" => match method {
                "__iter__" => self.ap_bind_self("generator.__iter__", args, keywords),
                _ => {
                    let receiver = self.ap_bind_self("generator.__next__", args, keywords)?;
                    self.ap_indented_subactions_next(&receiver)
                }
            },
            "_AttributeHolder" | "Namespace" => {
                self.argparse_holder_method(class, method, args, keywords)
            }
            "FileType" => self.argparse_file_type_method(method, args, keywords),
            "HelpFormatter"
            | "HelpFormatter._Section"
            | "RawDescriptionHelpFormatter"
            | "RawTextHelpFormatter"
            | "ArgumentDefaultsHelpFormatter"
            | "MetavarTypeHelpFormatter" => {
                self.argparse_formatter_method(class, method, args, keywords)
            }
            "_ActionsContainer" | "_ArgumentGroup" | "_MutuallyExclusiveGroup" => {
                self.argparse_container_method(class, method, args, keywords)
            }
            "ArgumentParser" => self.argparse_parser_method(method, args, keywords),
            _ => self.argparse_action_method(class, method, args, keywords),
        }
    }

    // ----------------------------------------------------------------
    // Small object-protocol helpers. Every one goes through the VM so
    // user-defined classes behave as they would in CPython.
    // ----------------------------------------------------------------

    pub(super) fn ap_get(&mut self, object: &Value, name: &str) -> Result<Value, String> {
        self.load_attribute_value(object.clone(), name)
    }

    /// `getattr(object, name, None)` distinguishing a missing attribute.
    pub(super) fn ap_get_opt(
        &mut self,
        object: &Value,
        name: &str,
    ) -> Result<Option<Value>, String> {
        match self.load_attribute_catching(object.clone(), name)? {
            Ok(value) => Ok(Some(value)),
            Err(exception) if exception_is(&exception, "AttributeError") => Ok(None),
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }

    pub(super) fn ap_set(
        &mut self,
        object: &Value,
        name: &str,
        value: Value,
    ) -> Result<(), String> {
        self.store_attribute_value(object.clone(), name, value)
    }

    pub(super) fn ap_del(&mut self, object: &Value, name: &str) -> Result<(), String> {
        self.delete_attribute_value(object.clone(), name)
    }

    pub(super) fn ap_hasattr(&mut self, object: &Value, name: &str) -> Result<bool, String> {
        Ok(self.ap_get_opt(object, name)?.is_some())
    }

    pub(super) fn ap_method(
        &mut self,
        object: &Value,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let method = self.ap_get(object, name)?;
        self.ap_invoke(method, args, Vec::new())
    }

    pub(super) fn ap_method_kw(
        &mut self,
        object: &Value,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let method = self.ap_get(object, name)?;
        self.ap_invoke(method, args, keywords)
    }

    /// Calls `callee`. A method that resolved to one of this module's own
    /// builtins is dispatched directly: the outcome is the same as a full VM
    /// call, but parsers nested through subcommands do not stack the VM's
    /// generic call frames for every internal step.
    pub(super) fn ap_invoke(
        &mut self,
        callee: Value,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match callee {
            Value::BoundMethod {
                function, receiver, ..
            } if matches!(function.as_ref(), Value::Builtin(name) if is_argparse_builtin(name)) => {
                let Value::Builtin(name) = *function else {
                    unreachable!("guard matched an argparse builtin");
                };
                args.insert(0, *receiver);
                self.argparse_dispatch(&name, args, keywords)
            }
            Value::Builtin(name) if is_argparse_builtin(&name) => {
                self.argparse_dispatch(&name, args, keywords)
            }
            callee => self.call_value_with_keywords(callee, args, keywords),
        }
    }

    /// `ap_invoke` that hands back a raised exception for inspection.
    pub(super) fn ap_invoke_catching(
        &mut self,
        callee: Value,
        args: Vec<Value>,
    ) -> Result<Result<Value, MiniException>, String> {
        self.call_with_exception_capture(|vm| vm.ap_invoke(callee, args, Vec::new()))
    }

    /// `callee(*args)` for an object: actions defined here run their native
    /// `__call__` directly (found on the type, as Python looks it up).
    pub(super) fn ap_call_object(
        &mut self,
        callee: Value,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        if let Value::Instance {
            class_attrs,
            class_bases,
            ..
        } = &callee
            && let Some(Value::Builtin(name)) =
                find_class_attr(class_attrs, class_bases, "__call__")
            && is_argparse_builtin(&name)
        {
            let mut full = Vec::with_capacity(args.len() + 1);
            full.push(callee.clone());
            full.extend(args);
            return self.argparse_dispatch(&name, full, Vec::new());
        }
        self.call_value(callee, args)
    }

    pub(super) fn ap_str(&mut self, value: &Value) -> Result<String, String> {
        self.str_value(value)
    }

    pub(super) fn ap_repr(&mut self, value: &Value) -> Result<String, String> {
        self.repr_value(value)
    }

    pub(super) fn ap_truth(&mut self, value: &Value) -> Result<bool, String> {
        self.truth_value(value.clone())
    }

    pub(super) fn ap_eq(&mut self, left: &Value, right: &Value) -> Result<bool, String> {
        self.equal_values(left.clone(), right.clone())
    }

    pub(super) fn ap_contains(&mut self, container: &Value, item: &Value) -> Result<bool, String> {
        self.contains_value(item.clone(), container.clone())
    }

    pub(super) fn ap_iter(&mut self, value: &Value) -> Result<Vec<Value>, String> {
        self.collect_iterable_values(value.clone())
    }

    pub(super) fn ap_len(&mut self, value: &Value) -> Result<usize, String> {
        self.len_value(value.clone())
    }

    pub(super) fn ap_isinstance(&mut self, object: &Value, class: &Value) -> Result<bool, String> {
        let result = self.call_value(
            Value::Builtin("isinstance".to_string()),
            vec![object.clone(), class.clone()],
        )?;
        self.truth_value(result)
    }

    pub(super) fn ap_callable(&mut self, value: &Value) -> Result<bool, String> {
        let result =
            self.call_value(Value::Builtin("callable".to_string()), vec![value.clone()])?;
        self.truth_value(result)
    }

    /// `format % args` with Python's string formatting.
    pub(super) fn ap_percent(&mut self, format: &str, args: Value) -> Result<String, String> {
        percent_format_string(Some(self), format, args)
    }

    /// `str` conversion for values that must be strings in practice
    /// (option strings, prefix characters, dests).
    pub(super) fn ap_text(&mut self, value: &Value) -> Result<String, String> {
        match string_of(value) {
            Some(text) => Ok(text.to_string()),
            None => self.ap_str(value),
        }
    }

    /// An attribute name for `getattr`/`setattr`, rejecting non-strings with
    /// the builtins' message.
    pub(super) fn ap_attr_name(&mut self, value: &Value) -> Result<String, String> {
        match string_of(value) {
            Some(name) => Ok(name.to_string()),
            None => self.ap_error(
                "TypeError",
                format!("attribute name must be string, not '{}'", type_name(value)),
            ),
        }
    }

    /// The argparse module object (as CPython functions reach their module
    /// globals).
    pub(super) fn ap_module(&mut self) -> Result<Value, String> {
        if let Some(module) = lookup_string_key(&self.module_cache, "argparse")
            && matches!(module, Value::Module { .. })
        {
            return Ok(module);
        }
        self.load_stdlib_module("argparse")
    }

    pub(super) fn ap_class(&mut self, name: &str) -> Result<Value, String> {
        let module = self.ap_module()?;
        self.ap_get(&module, name)
    }

    /// The `sys` module, imported as argparse itself imports it.
    pub(super) fn ap_sys(&mut self) -> Result<Value, String> {
        if let Some(module) = lookup_string_key(&self.module_cache, "sys")
            && matches!(module, Value::Module { .. })
        {
            return Ok(module);
        }
        self.load_stdlib_module("sys")
    }

    pub(super) fn ap_sys_attr(&mut self, name: &str) -> Result<Value, String> {
        let sys = self.ap_sys()?;
        self.ap_get(&sys, name)
    }

    /// The builtin `open`, resolved like a global name: the sandbox provides
    /// no `open`, so this raises `NameError` exactly as `open(...)` does in
    /// sandboxed code, and no host file is ever reached.
    pub(super) fn ap_builtin_open(&mut self) -> Result<Value, String> {
        let builtins = self.load_stdlib_module("builtins")?;
        match self.ap_get_opt(&builtins, "open")? {
            Some(open) => Ok(open),
            // The failed `builtins.open` lookup above is already in the
            // denial log.
            None => self.ap_error("NameError", "name 'open' is not defined"),
        }
    }

    pub(super) fn ap_list_items(&mut self, value: &Value) -> Result<Vec<Value>, String> {
        match value {
            Value::List(items) => Ok(items.borrow().clone()),
            Value::Tuple(items) => Ok(items.as_ref().clone()),
            other => self.ap_iter(other),
        }
    }

    /// Raises `ValueError`/`TypeError` style errors with a message.
    pub(super) fn ap_error<T>(
        &mut self,
        kind: &str,
        message: impl Into<String>,
    ) -> Result<T, String> {
        Err(format!("{kind}: {}", message.into()))
    }

    /// Builds an `ArgumentError(argument, message)` and raises it.
    pub(super) fn ap_raise_argument_error<T>(
        &mut self,
        argument: Value,
        message: impl Into<String>,
    ) -> Result<T, String> {
        let class = self.ap_class("ArgumentError")?;
        let error = self.call_value(class, vec![argument, Value::String(message.into())])?;
        let exception = exception_from_value(error)?;
        self.raise_exception(exception, true)
    }

    /// Whether a caught exception is an instance of the module class `name`.
    pub(super) fn ap_exception_matches_class(
        &mut self,
        exception: &MiniException,
        name: &str,
    ) -> Result<bool, String> {
        let class = self.ap_class(name)?;
        match &exception.type_object {
            Some(type_object) => Ok(class_derives_from(type_object, &class)),
            None => Ok(false),
        }
    }

    /// `_get_action_name(argument)`.
    pub(super) fn ap_action_name(&mut self, argument: &Value) -> Result<Value, String> {
        if matches!(argument, Value::None) {
            return Ok(Value::None);
        }
        let option_strings = self.ap_get(argument, "option_strings")?;
        if self.ap_truth(&option_strings)? {
            let items = self.ap_list_items(&option_strings)?;
            let mut parts = Vec::new();
            for item in items {
                parts.push(self.ap_text(&item)?);
            }
            return Ok(str_value(parts.join("/")));
        }
        let metavar = self.ap_get(argument, "metavar")?;
        if !matches!(metavar, Value::None) && !is_suppress(&metavar) {
            if !matches!(metavar, Value::Tuple(_)) {
                return Ok(metavar);
            }
            let nargs = self.ap_get(argument, "nargs")?;
            let kind = nargs_kind(&nargs);
            let length = self.ap_len(&metavar)?;
            if (kind == Nargs::ZeroOrMore && length == 2) || kind == Nargs::OneOrMore {
                return Ok(str_value(self.ap_percent("%s[, %s]", metavar)?));
            }
            let items = self.ap_list_items(&metavar)?;
            let mut parts = Vec::new();
            for item in items {
                match string_of(&item) {
                    Some(text) => parts.push(text.to_string()),
                    None => {
                        return self.ap_error(
                            "TypeError",
                            format!(
                                "sequence item {}: expected str instance, {} found",
                                parts.len(),
                                type_name(&item)
                            ),
                        );
                    }
                }
            }
            return Ok(str_value(parts.join(", ")));
        }
        let dest = self.ap_get(argument, "dest")?;
        if !matches!(dest, Value::None) && !is_suppress(&dest) {
            return Ok(dest);
        }
        let choices = self.ap_get(argument, "choices")?;
        if self.ap_truth(&choices)? {
            let items = self.ap_iter(&choices)?;
            let mut parts = Vec::new();
            for item in items {
                parts.push(self.ap_str(&item)?);
            }
            return Ok(str_value(format!("{{{}}}", parts.join(","))));
        }
        Ok(Value::None)
    }

    fn argparse_module_function(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match name {
            "_get_action_name" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "_get_action_name",
                        params: &["argument"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                self.ap_action_name(&bound.arg(0))
            }
            "_copy_items" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "_copy_items",
                        params: &["items"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                self.ap_copy_items(&bound.arg(0))
            }
            "_identity" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "_identity",
                        params: &["value"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                Ok(bound.arg(0))
            }
            "_metavar_format" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "HelpFormatter._metavar_formatter.<locals>.format",
                        params: &["result", "tuple_size"],
                        required: 2,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                self.ap_metavar_format(bound.arg(0), bound.arg(1))
            }
            "_prog_name" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "_prog_name",
                        params: &["prog"],
                        required: 0,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                self.ap_prog_name(bound.arg(0))
            }
            _ => Err(format!("unknown builtin: argparse.{name}")),
        }
    }

    /// `_copy_items(items)`.
    pub(super) fn ap_copy_items(&mut self, items: &Value) -> Result<Value, String> {
        match items {
            Value::None => Ok(list_value(Vec::new())),
            Value::List(items) => Ok(list_value(items.borrow().clone())),
            other => {
                let copy = self.load_stdlib_module("copy")?;
                let copy = self.ap_get(&copy, "copy")?;
                self.call_value(copy, vec![other.clone()])
            }
        }
    }

    /// `_prog_name(prog)`: a script's prog is the basename of `sys.argv[0]`.
    pub(super) fn ap_prog_name(&mut self, prog: Value) -> Result<Value, String> {
        if !matches!(prog, Value::None) {
            return Ok(prog);
        }
        let argv = self.ap_sys_attr("argv")?;
        let arg0 = self.load_subscript_value(argv, Value::Number(0))?;
        match string_of(&arg0) {
            Some(text) => Ok(str_value(path_basename(text))),
            None => self.ap_error(
                "TypeError",
                format!(
                    "expected str, bytes or os.PathLike object, not {}",
                    type_name(&arg0)
                ),
            ),
        }
    }

    // ----------------------------------------------------------------
    // _AttributeHolder and Namespace
    // ----------------------------------------------------------------

    fn argparse_holder_method(
        &mut self,
        class: &str,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match (class, method) {
            ("_AttributeHolder", "__repr__") => {
                let bound = bind_args(
                    &Sig {
                        qualname: "_AttributeHolder.__repr__",
                        params: &["self"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                self.ap_holder_repr(&bound.arg(0))
            }
            ("_AttributeHolder", "_get_kwargs") => {
                let bound = bind_args(
                    &Sig {
                        qualname: "_AttributeHolder._get_kwargs",
                        params: &["self"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                let items = self.ap_instance_dict_items(&bound.arg(0))?;
                Ok(list_value(
                    items
                        .into_iter()
                        .map(|(key, value)| tuple_value(vec![key, value]))
                        .collect(),
                ))
            }
            ("_AttributeHolder", "_get_args") => {
                bind_args(
                    &Sig {
                        qualname: "_AttributeHolder._get_args",
                        params: &["self"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                Ok(list_value(Vec::new()))
            }
            ("Namespace", "__init__") => {
                let bound = bind_args(
                    &Sig {
                        qualname: "Namespace.__init__",
                        params: &["self"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: true,
                    },
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                for (name, value) in bound.extra_kwargs {
                    self.ap_set(&receiver, &name, value)?;
                }
                Ok(Value::None)
            }
            ("Namespace", "__eq__") => {
                let bound = bind_args(
                    &Sig {
                        qualname: "Namespace.__eq__",
                        params: &["self", "other"],
                        required: 2,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let other = bound.arg(1);
                let namespace = self.ap_class("Namespace")?;
                if !self.ap_isinstance(&other, &namespace)? {
                    return Ok(Value::NotImplemented);
                }
                let left = self.call_value(Value::Builtin("vars".to_string()), vec![receiver])?;
                let right = self.call_value(Value::Builtin("vars".to_string()), vec![other])?;
                Ok(Value::Bool(self.ap_eq(&left, &right)?))
            }
            ("Namespace", "__contains__") => {
                let bound = bind_args(
                    &Sig {
                        qualname: "Namespace.__contains__",
                        params: &["self", "key"],
                        required: 2,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let dict = self.ap_get(&receiver, "__dict__")?;
                Ok(Value::Bool(self.ap_contains(&dict, &bound.arg(1))?))
            }
            _ => Err(format!("unknown builtin: argparse.{class}.{method}")),
        }
    }

    /// The `(name, value)` items of an instance `__dict__` in order.
    pub(super) fn ap_instance_dict_items(
        &mut self,
        object: &Value,
    ) -> Result<Vec<(Value, Value)>, String> {
        if let Value::Instance { fields, .. } = object {
            return Ok(scope_dict_entries(fields));
        }
        let dict = self.ap_get(object, "__dict__")?;
        let items = self.ap_method(&dict, "items", Vec::new())?;
        let mut entries = Vec::new();
        for item in self.ap_iter(&items)? {
            let pair = self.ap_list_items(&item)?;
            if let [key, value] = pair.as_slice() {
                entries.push((key.clone(), value.clone()));
            }
        }
        Ok(entries)
    }

    /// `_AttributeHolder.__repr__`.
    fn ap_holder_repr(&mut self, receiver: &Value) -> Result<Value, String> {
        let class = self.ap_get(receiver, "__class__")?;
        let type_name = self.ap_get(&class, "__name__")?;
        let type_name = self.ap_text(&type_name)?;
        let mut arg_strings = Vec::new();
        let mut star_args = Vec::new();
        let positional = self.ap_method(receiver, "_get_args", Vec::new())?;
        for arg in self.ap_iter(&positional)? {
            arg_strings.push(self.ap_repr(&arg)?);
        }
        let keyword_items = self.ap_method(receiver, "_get_kwargs", Vec::new())?;
        for item in self.ap_iter(&keyword_items)? {
            let pair = self.ap_list_items(&item)?;
            let [name, value] = pair.as_slice() else {
                return self.ap_error("ValueError", "too many values to unpack (expected 2)");
            };
            let is_identifier = self.ap_method(name, "isidentifier", Vec::new())?;
            if self.ap_truth(&is_identifier)? {
                let name = self.ap_text(name)?;
                let value = self.ap_repr(value)?;
                arg_strings.push(format!("{name}={value}"));
            } else {
                star_args.push((name.clone(), value.clone()));
            }
        }
        if !star_args.is_empty() {
            let mut dict: Vec<(Value, Value)> = Vec::new();
            for (key, value) in star_args {
                if let Some(entry) = dict.iter_mut().find(|(existing, _)| {
                    matches!((existing, &key), (Value::String(a), Value::String(b)) if a == b)
                }) {
                    entry.1 = value;
                } else {
                    dict.push((key, value));
                }
            }
            let rendered = self.ap_repr(&dict_value(dict))?;
            arg_strings.push(format!("**{rendered}"));
        }
        Ok(str_value(format!(
            "{type_name}({})",
            arg_strings.join(", ")
        )))
    }

    // ----------------------------------------------------------------
    // FileType
    // ----------------------------------------------------------------

    fn argparse_file_type_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__init__" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "FileType.__init__",
                        params: &["self", "mode", "bufsize", "encoding", "errors"],
                        required: 1,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                // CPython emits a PendingDeprecationWarning, which the default
                // warning filters ignore.
                self.ap_set(&receiver, "_mode", bound.or(1, str_value("r")))?;
                self.ap_set(&receiver, "_bufsize", bound.or(2, Value::Number(-1)))?;
                self.ap_set(&receiver, "_encoding", bound.arg(3))?;
                self.ap_set(&receiver, "_errors", bound.arg(4))?;
                Ok(Value::None)
            }
            "__call__" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "FileType.__call__",
                        params: &["self", "string"],
                        required: 2,
                        kwonly: &[],
                        varargs: false,
                        varkw: false,
                    },
                    args,
                    keywords,
                )?;
                self.ap_file_type_call(&bound.arg(0), bound.arg(1))
            }
            "__repr__" => {
                let bound = bind_args(
                    &Sig {
                        qualname: "FileType.__repr__",
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
                let mut parts = Vec::new();
                for name in ["_mode", "_bufsize"] {
                    let value = self.ap_get(&receiver, name)?;
                    let is_default = matches!(value, Value::Number(-1));
                    if !is_default {
                        parts.push(self.ap_repr(&value)?);
                    }
                }
                for (keyword, name) in [("encoding", "_encoding"), ("errors", "_errors")] {
                    let value = self.ap_get(&receiver, name)?;
                    if !matches!(value, Value::None) {
                        parts.push(format!("{keyword}={}", self.ap_repr(&value)?));
                    }
                }
                let class = self.ap_get(&receiver, "__class__")?;
                let name = self.ap_get(&class, "__name__")?;
                let name = self.ap_text(&name)?;
                Ok(str_value(format!("{name}({})", parts.join(", "))))
            }
            _ => Err(format!("unknown builtin: argparse.FileType.{method}")),
        }
    }

    fn ap_file_type_call(&mut self, receiver: &Value, string: Value) -> Result<Value, String> {
        let mode = self.ap_get(receiver, "_mode")?;
        if matches!(string_of(&string), Some("-")) {
            let mode_text = self.ap_text(&mode)?;
            let binary = mode_text.contains('b');
            let stream = if mode_text.contains('r') {
                Some("stdin")
            } else if mode_text.chars().any(|ch| matches!(ch, 'w' | 'a' | 'x')) {
                Some("stdout")
            } else {
                None
            };
            let Some(stream) = stream else {
                let rendered = self.ap_repr(&mode)?;
                return self.ap_error("ValueError", format!("argument \"-\" with mode {rendered}"));
            };
            let stream = self.ap_sys_attr(stream)?;
            return if binary {
                self.ap_get(&stream, "buffer")
            } else {
                Ok(stream)
            };
        }
        let bufsize = self.ap_get(receiver, "_bufsize")?;
        let encoding = self.ap_get(receiver, "_encoding")?;
        let errors = self.ap_get(receiver, "_errors")?;
        // Opening goes through the sandbox's own `open`, which never reaches
        // host files.
        let open = self.ap_builtin_open()?;
        match self
            .call_value_catching(open, vec![string.clone(), mode, bufsize, encoding, errors])?
        {
            Ok(file) => Ok(file),
            Err(exception) if exception_is(&exception, "OSError") => {
                let filename = self.ap_str(&string)?;
                let error = self.ap_str(&value_from_exception(&exception))?;
                let class = self.ap_class("ArgumentTypeError")?;
                let error = self.call_value(
                    class,
                    vec![str_value(format!("can't open '{filename}': {error}"))],
                )?;
                let exception = exception_from_value(error)?;
                self.raise_exception(exception, true)
            }
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }
}

/// Native construction of `ArgumentError(argument, message)` instances (and
/// of user subclasses that do not define their own state).
impl Vm {
    pub(super) fn call_argparse_argument_error(
        &mut self,
        type_name: String,
        type_hierarchy: Vec<String>,
        type_object: Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let mut bound_args = args.clone();
        let mut argument = None;
        let mut message = None;
        for (keyword, value) in keywords {
            match keyword.as_str() {
                "argument" if bound_args.is_empty() && argument.is_none() => {
                    argument = Some(value);
                }
                "message" if bound_args.len() < 2 && message.is_none() => message = Some(value),
                "argument" | "message" => {
                    return Err(format!(
                        "TypeError: ArgumentError.__init__() got multiple values for argument '{keyword}'"
                    ));
                }
                _ => {
                    return Err(format!(
                        "TypeError: ArgumentError.__init__() got an unexpected keyword argument '{keyword}'"
                    ));
                }
            }
        }
        if bound_args.len() > 2 {
            return Err(format!(
                "TypeError: ArgumentError.__init__() takes 3 positional arguments but {} were given",
                bound_args.len() + 1
            ));
        }
        let mut drain = bound_args.drain(..);
        let argument = drain.next().or(argument);
        let message = drain.next().or(message);
        let missing = [
            ("argument", argument.is_none()),
            ("message", message.is_none()),
        ]
        .into_iter()
        .filter(|(_, missing)| *missing)
        .map(|(name, _)| format!("'{name}'"))
        .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "TypeError: ArgumentError.__init__() missing {} required positional argument{}: {}",
                missing.len(),
                plural_suffix(missing.len()),
                quoted_name_list(&missing)
            ));
        }
        let argument = argument.unwrap_or(Value::None);
        let message = message.unwrap_or(Value::None);
        let argument_name =
            match self.call_with_exception_capture(|vm| vm.ap_action_name(&argument))? {
                Ok(name) => name,
                Err(exception) => return Err(self.propagate_captured_exception(exception)),
            };
        let display = argument_error_display(self, &argument_name, &message)?;
        Ok(Value::Exception {
            type_name,
            type_hierarchy,
            type_object: Some(Box::new(type_object)),
            message: Some(display),
            args,
            attrs: vec![
                ("argument_name".to_string(), argument_name),
                ("message".to_string(), message),
            ],
            exceptions: None,
            cause: None,
            context: None,
            suppress_context: false,
            identity: Rc::new(crate::value::ExceptionIdentity::default()),
        })
    }
}

/// `ArgumentError.__str__`.
fn argument_error_display(
    vm: &mut Vm,
    argument_name: &Value,
    message: &Value,
) -> Result<String, String> {
    let message = vm.str_value(message)?;
    if matches!(argument_name, Value::None) {
        return Ok(message);
    }
    let argument_name = vm.str_value(argument_name)?;
    Ok(format!("argument {argument_name}: {message}"))
}
