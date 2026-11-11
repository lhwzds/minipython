// SPDX-License-Identifier: MIT

//! Plumbing shared by stdlib types implemented natively as ordinary classes
//! whose methods are Rust builtins (`io.StringIO` in `stringio.rs`, `csv` in
//! `csv.rs`).
//!
//! Instances are `Value::Instance`s, so subclassing, `isinstance`, `type()`,
//! instance dictionaries, and attribute lookup work through the VM's normal
//! class machinery. Method builtins are named `<prefix><Class>.<method>` and
//! bind to instances like Python functions; module functions
//! (`<prefix><function>`, no class part) do not bind.

use super::*;

/// Builtin-name prefixes owned by the native classes.
const PREFIXES: &[&str] = &[
    super::stringio::STRINGIO_PREFIX,
    super::csv::CSV_PREFIX,
    super::csv::CSV_C_PREFIX,
];

/// Whether a builtin belongs to a native class or module defined here.
pub(crate) fn is_native_builtin(name: &str) -> bool {
    PREFIXES.iter().any(|prefix| name.starts_with(prefix))
}

/// Whether a native builtin is a method that binds to instances (it names a
/// class) rather than a module function. `__new__` is a static method.
pub(crate) fn binds_like_function(name: &str) -> bool {
    if name.ends_with(".__new__") {
        return false;
    }
    if name.starts_with(super::stringio::STRINGIO_PREFIX) {
        return true;
    }
    [super::csv::CSV_PREFIX, super::csv::CSV_C_PREFIX]
        .iter()
        .any(|prefix| {
            name.strip_prefix(prefix)
                .is_some_and(|rest| rest.contains('.'))
        })
}

impl Vm {
    /// Entry point for every native class builtin. Native errors and callback
    /// exceptions retain their objects until the calling frame dispatches.
    pub(super) fn call_native_builtin(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let name = name.to_string();
        match self.call_with_exception_capture(|vm| vm.native_dispatch(&name, args, keywords))? {
            Ok(value) => Ok(value),
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }

    pub(super) fn native_dispatch(
        &mut self,
        name: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if let Some(method) = name.strip_prefix(super::stringio::STRINGIO_PREFIX) {
            return self.stringio_method(method, args, keywords);
        }
        self.csv_dispatch(name, args, keywords)
    }

    /// Raises an instance of `class` built from `message`.
    pub(super) fn native_raise<T>(&mut self, class: Value, message: String) -> Result<T, String> {
        let error = self.call_value(class, vec![Value::String(message)])?;
        let exception = exception_from_value(error)?;
        self.raise_exception(exception, true)
    }

    /// `x.__index__()` as a signed machine-size integer.
    pub(super) fn native_ssize(&mut self, value: Value) -> Result<i64, String> {
        match self.index_integer_value(value)? {
            Value::Number(number) => Ok(number),
            _ => Err("OverflowError: Python int too large to convert to C ssize_t".to_string()),
        }
    }

    /// `x.__index__()` as a C `int`.
    pub(super) fn native_c_int(&mut self, value: Value) -> Result<i64, String> {
        match self.index_integer_value(value)? {
            Value::Number(number) if i32::try_from(number).is_ok() => Ok(number),
            _ => Err("OverflowError: Python int too large to convert to C int".to_string()),
        }
    }

    /// An optional size argument: `None` or an integer that fits a signed
    /// machine-size integer.
    pub(super) fn native_optional_ssize(
        &mut self,
        value: Option<Value>,
    ) -> Result<Option<i64>, String> {
        match value {
            None | Some(Value::None) => Ok(None),
            Some(value) => {
                let is_index =
                    matches!(value, Value::Number(_) | Value::BigInt(_) | Value::Bool(_))
                        || int_subclass_integer(&value).is_some()
                        || instance_special_method(&value, "__index__").is_some();
                if !is_index {
                    return Err(format!(
                        "TypeError: argument should be integer or None, not '{}'",
                        type_name(&value)
                    ));
                }
                Ok(Some(self.native_ssize(value)?))
            }
        }
    }
}

/// A class whose methods are native builtins.
pub(super) struct NativeClass<'a> {
    pub name: &'a str,
    pub module: &'a str,
    pub doc: Option<&'a str>,
    pub bases: Vec<Value>,
    /// Class attributes in definition order.
    pub attrs: Vec<(String, Value)>,
    /// Hidden attributes (names starting with `\0`).
    pub hidden: Vec<(String, Value)>,
}

impl NativeClass<'_> {
    pub fn build(self) -> Value {
        let scope = new_scope();
        {
            let mut values = scope.borrow_mut();
            let mut order = Vec::new();
            let mut insert = |key: &str, value: Value| {
                order.push(Value::String(key.to_string()));
                values.insert(key.to_string(), value);
            };
            insert("__module__", Value::String(self.module.to_string()));
            insert("__qualname__", Value::String(self.name.to_string()));
            insert(
                "__doc__",
                self.doc
                    .map(|doc| Value::String(doc.to_string()))
                    .unwrap_or(Value::None),
            );
            for (key, value) in self.attrs {
                insert(&key, value);
            }
            for (key, value) in self.hidden {
                values.insert(key, value);
            }
            values.insert(
                CLASS_QUALNAME_ATTR.to_string(),
                Value::String(self.name.to_string()),
            );
            values.insert(CLASS_ATTR_ORDER_ATTR.to_string(), tuple_value(order));
        }
        Value::Class {
            name: self.name.to_string(),
            type_params: Vec::new(),
            metaclass: None,
            bases: self.bases,
            attrs: scope,
        }
    }
}

/// A read-only (or read-write) attribute backed by native getter/setter
/// builtins.
pub(super) fn native_property(name: &str, getter: String, setter: Option<String>) -> Value {
    Value::Property {
        fget: Some(Rc::new(Value::Builtin(getter))),
        fset: setter.map(|setter| Rc::new(Value::Builtin(setter))),
        fdel: None,
        doc: Rc::new(RefCell::new(None)),
        doc_from_getter: false,
        name: Rc::new(RefCell::new(Some(Value::String(name.to_string())))),
        identity: Rc::new(()),
    }
}

/// A new instance of a native class, created without running `__init__`.
pub(super) fn native_instance(
    class: &Value,
    fields: Vec<(String, Value)>,
) -> Result<Value, String> {
    let Value::Class {
        name, attrs, bases, ..
    } = class
    else {
        return Err("TypeError: native class expected".to_string());
    };
    let scope = new_scope();
    {
        let mut values = scope.borrow_mut();
        for (key, value) in fields {
            values.insert(key, value);
        }
    }
    Ok(Value::Instance {
        class_name: name.clone(),
        fields: scope,
        class_attrs: attrs.clone(),
        class_bases: bases.clone(),
    })
}

/// A hidden field of an instance.
pub(super) fn hidden_field(object: &Value, name: &str) -> Option<Value> {
    match object {
        Value::Instance { fields, .. } => fields.borrow().get(name).cloned(),
        _ => None,
    }
}

/// Sets a field of an instance directly (no `__setattr__`).
pub(super) fn set_hidden_field(object: &Value, name: &str, value: Value) {
    if let Value::Instance { fields, .. } = object {
        fields.borrow_mut().insert(name.to_string(), value);
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// `METH_NOARGS`: `"<qualname>() takes no arguments (N given)"`.
pub(super) fn c_no_args(
    qualname: &str,
    args: &[Value],
    keywords: &[(String, Value)],
) -> Result<(), String> {
    if !keywords.is_empty() {
        return Err(format!(
            "TypeError: {qualname}() takes no keyword arguments"
        ));
    }
    if !args.is_empty() {
        return Err(format!(
            "TypeError: {qualname}() takes no arguments ({} given)",
            args.len()
        ));
    }
    Ok(())
}

/// `METH_O`: `"<qualname>() takes exactly one argument (N given)"`.
pub(super) fn c_one_arg(
    qualname: &str,
    mut args: Vec<Value>,
    keywords: &[(String, Value)],
) -> Result<Value, String> {
    if !keywords.is_empty() {
        return Err(format!(
            "TypeError: {qualname}() takes no keyword arguments"
        ));
    }
    if args.len() != 1 {
        return Err(format!(
            "TypeError: {qualname}() takes exactly one argument ({} given)",
            args.len()
        ));
    }
    Ok(args.pop().expect("one argument"))
}

/// Positional-only arguments with no keywords:
/// `"<name> expected at most N arguments, got M"`.
pub(super) fn c_unpack(
    qualname: &str,
    name: &str,
    args: Vec<Value>,
    keywords: &[(String, Value)],
    min: usize,
    max: usize,
) -> Result<Vec<Option<Value>>, String> {
    if !keywords.is_empty() {
        return Err(format!(
            "TypeError: {qualname}() takes no keyword arguments"
        ));
    }
    c_unpack_positional(name, args, min, max)
}

/// Bounds check on the number of positional arguments.
pub(super) fn c_unpack_positional(
    name: &str,
    args: Vec<Value>,
    min: usize,
    max: usize,
) -> Result<Vec<Option<Value>>, String> {
    if args.len() < min {
        return Err(format!(
            "TypeError: {name} expected at least {min} argument{}, got {}",
            plural(min),
            args.len()
        ));
    }
    if args.len() > max {
        return Err(format!(
            "TypeError: {name} expected at most {max} argument{}, got {}",
            plural(max),
            args.len()
        ));
    }
    let mut values: Vec<Option<Value>> = args.into_iter().map(Some).collect();
    values.resize(max, None);
    Ok(values)
}

/// A slot wrapper (`__iter__`, `__next__`) called with arguments.
pub(super) fn c_slot_no_args(args: &[Value], keywords: &[(String, Value)]) -> Result<(), String> {
    if !keywords.is_empty() {
        return Err("TypeError: wrapper takes no keyword arguments".to_string());
    }
    if !args.is_empty() {
        return Err(format!(
            "TypeError: expected 0 arguments, got {}",
            args.len()
        ));
    }
    Ok(())
}

/// Argument Clinic positional-or-keyword parameters:
/// `"<name>() takes at most N arguments (M given)"`,
/// `"<name>() missing required argument 'x' (pos 1)"`.
pub(super) fn c_clinic(
    name: &str,
    params: &[&str],
    required: usize,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<Vec<Option<Value>>, String> {
    let total = params.len();
    let given = args.len() + keywords.len();
    if args.len() > total || (given > total && keywords.is_empty()) {
        return Err(format!(
            "TypeError: {name}() takes at most {total} argument{} ({given} given)",
            plural(total)
        ));
    }
    let mut values: Vec<Option<Value>> = vec![None; total];
    let positional = args.len();
    for (index, value) in args.into_iter().enumerate() {
        values[index] = Some(value);
    }
    for (keyword, value) in keywords {
        let Some(index) = params.iter().position(|param| *param == keyword) else {
            return Err(format!(
                "TypeError: {name}() got an unexpected keyword argument '{keyword}'"
            ));
        };
        if index < positional {
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
