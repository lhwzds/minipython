// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/csv.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `csv.DictReader` and `csv.DictWriter`.
//!
//! Both are Python-level classes in CPython, so every collaborator is
//! reached through attribute lookup on `self` (`self.reader`,
//! `self.fieldnames`, `self.restkey`, `self.writer`, `self._dict_to_list`,
//! ...) and their state lives in ordinary instance attributes.

use super::*;

fn bind(
    qualname: &'static str,
    params: &'static [&'static str],
    required: usize,
    varargs: bool,
    args: Vec<Value>,
    keywords: Vec<(String, Value)>,
) -> Result<super::super::argparse::Bound, String> {
    bind_args(
        &Sig {
            qualname,
            params,
            required,
            kwonly: &[],
            varargs,
            varkw: varargs,
        },
        args,
        keywords,
    )
}

impl Vm {
    fn csv_get(&mut self, object: &Value, name: &str) -> Result<Value, String> {
        self.load_attribute_value(object.clone(), name)
    }

    fn csv_set(&mut self, object: &Value, name: &str, value: Value) -> Result<(), String> {
        self.store_attribute_value(object.clone(), name, value)
    }

    fn csv_call_method(
        &mut self,
        object: &Value,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, String> {
        let method = self.csv_get(object, name)?;
        self.call_value(method, args)
    }

    /// `next(iterator)`, returning `None` at the end.
    fn csv_next(&mut self, iterator: &Value) -> Result<Option<Value>, String> {
        match self.call_with_exception_capture(|vm| {
            vm.call_value(Value::Builtin("next".to_string()), vec![iterator.clone()])
        })? {
            Ok(value) => Ok(Some(value)),
            Err(exception) if exception.type_name == "StopIteration" => Ok(None),
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }

    /// A `fieldnames` argument that is its own iterator (a generator, for
    /// example) is read into a list once; other values are kept as given.
    fn csv_materialize_fieldnames(&mut self, fieldnames: Value) -> Result<Value, String> {
        if matches!(fieldnames, Value::None) {
            return Ok(fieldnames);
        }
        let iterator = self.get_iter_propagating(fieldnames.clone())?;
        if identity_bits(&iterator) == identity_bits(&fieldnames) {
            return self.call_value(Value::Builtin("list".to_string()), vec![fieldnames]);
        }
        Ok(fieldnames)
    }

    /// `reader(f, dialect, *args, **kwds)` / `writer(...)`.
    fn csv_make(
        &mut self,
        function: &str,
        target: Value,
        dialect: Value,
        extra: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let mut args = vec![target, dialect];
        args.extend(extra);
        let module = self.csv_module_for_globals()?;
        let callee = self.load_attribute_value(module, function)?;
        self.call_value_with_keywords(callee, args, keywords)
    }

    /// The `csv` module, where the classes look up module-level names.
    pub(super) fn csv_module_for_globals(&mut self) -> Result<Value, String> {
        if let Some(module @ Value::Module { .. }) = lookup_string_key(&self.module_cache, "csv") {
            return Ok(module);
        }
        self.load_stdlib_module("csv")
    }

    pub(super) fn csv_dict_reader_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__init__" => {
                let bound = bind(
                    "DictReader.__init__",
                    &["self", "f", "fieldnames", "restkey", "restval", "dialect"],
                    2,
                    true,
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let fieldnames = self.csv_materialize_fieldnames(bound.arg(2))?;
                let dialect = bound.or(5, Value::String("excel".to_string()));
                self.csv_set(&receiver, "_fieldnames", fieldnames)?;
                self.csv_set(&receiver, "restkey", bound.arg(3))?;
                self.csv_set(&receiver, "restval", bound.arg(4))?;
                let reader = self.csv_make(
                    "reader",
                    bound.arg(1),
                    dialect.clone(),
                    bound.extra_args,
                    bound.extra_kwargs,
                )?;
                self.csv_set(&receiver, "reader", reader)?;
                self.csv_set(&receiver, "dialect", dialect)?;
                self.csv_set(&receiver, "line_num", Value::Number(0))?;
                Ok(Value::None)
            }
            "__iter__" => {
                let bound = bind("DictReader.__iter__", &["self"], 1, false, args, keywords)?;
                Ok(bound.arg(0))
            }
            "fieldnames" => {
                let bound = bind("DictReader.fieldnames", &["self"], 1, false, args, keywords)?;
                let receiver = bound.arg(0);
                self.csv_dict_reader_fieldnames(&receiver)
            }
            "fieldnames_setter" => {
                let bound = bind(
                    "DictReader.fieldnames",
                    &["self", "value"],
                    2,
                    false,
                    args,
                    keywords,
                )?;
                self.csv_set(&bound.arg(0), "_fieldnames", bound.arg(1))?;
                Ok(Value::None)
            }
            "__next__" => {
                let bound = bind("DictReader.__next__", &["self"], 1, false, args, keywords)?;
                let receiver = bound.arg(0);
                self.csv_dict_reader_next(&receiver)
            }
            _ => Err(format!("unknown builtin: csv.DictReader.{method}")),
        }
    }

    fn csv_dict_reader_fieldnames(&mut self, receiver: &Value) -> Result<Value, String> {
        let current = self.csv_get(receiver, "_fieldnames")?;
        if matches!(current, Value::None) {
            let reader = self.csv_get(receiver, "reader")?;
            if let Some(first) = self.csv_next(&reader)? {
                self.csv_set(receiver, "_fieldnames", first)?;
            }
        }
        let reader = self.csv_get(receiver, "reader")?;
        let line_num = self.csv_get(&reader, "line_num")?;
        self.csv_set(receiver, "line_num", line_num)?;
        self.csv_get(receiver, "_fieldnames")
    }

    fn csv_dict_reader_next(&mut self, receiver: &Value) -> Result<Value, String> {
        let line_num = self.csv_get(receiver, "line_num")?;
        if self.equal_values(line_num, Value::Number(0))? {
            // Used only for its side effect.
            self.csv_get(receiver, "fieldnames")?;
        }
        let reader = self.csv_get(receiver, "reader")?;
        let mut row = match self.csv_next(&reader)? {
            Some(row) => row,
            None => return Err("StopIteration".to_string()),
        };
        let line_num = self.csv_get(&reader, "line_num")?;
        self.csv_set(receiver, "line_num", line_num)?;
        // Unlike the basic reader, blank rows are skipped.
        loop {
            let empty = list_value(Vec::new());
            if !self.equal_values(row.clone(), empty)? {
                break;
            }
            let reader = self.csv_get(receiver, "reader")?;
            row = match self.csv_next(&reader)? {
                Some(row) => row,
                None => return Err("StopIteration".to_string()),
            };
        }
        let fieldnames = self.csv_get(receiver, "fieldnames")?;
        let pairs = self.call_value(
            Value::Builtin("zip".to_string()),
            vec![fieldnames, row.clone()],
        )?;
        let record = self.call_value(builtin_type_value("dict"), vec![pairs])?;
        let fieldnames = self.csv_get(receiver, "fieldnames")?;
        let field_count = self.len_value(fieldnames)?;
        let row_count = self.len_value(row.clone())?;
        if field_count < row_count {
            let restkey = self.csv_get(receiver, "restkey")?;
            let index = self.call_value(
                builtin_type_value("slice"),
                vec![Value::Number(field_count as i64), Value::None],
            )?;
            let rest = self.load_subscript_value(row, index)?;
            self.csv_call_method(&record, "__setitem__", vec![restkey, rest])?;
        } else if field_count > row_count {
            let fieldnames = self.csv_get(receiver, "fieldnames")?;
            let index = self.call_value(
                builtin_type_value("slice"),
                vec![Value::Number(row_count as i64), Value::None],
            )?;
            let missing = self.load_subscript_value(fieldnames, index)?;
            for key in self.collect_iterable_values_propagating(missing)? {
                let restval = self.csv_get(receiver, "restval")?;
                self.csv_call_method(&record, "__setitem__", vec![key, restval])?;
            }
        }
        Ok(record)
    }

    pub(super) fn csv_dict_writer_method(
        &mut self,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__init__" => {
                let bound = bind(
                    "DictWriter.__init__",
                    &[
                        "self",
                        "f",
                        "fieldnames",
                        "restval",
                        "extrasaction",
                        "dialect",
                    ],
                    3,
                    true,
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let fieldnames = self.csv_materialize_fieldnames(bound.arg(2))?;
                self.csv_set(&receiver, "fieldnames", fieldnames)?;
                self.csv_set(
                    &receiver,
                    "restval",
                    bound.or(3, Value::String(String::new())),
                )?;
                let action = bound.or(4, Value::String("raise".to_string()));
                let action = self.csv_call_method(&action, "lower", Vec::new())?;
                let valid =
                    matches!(&action, Value::String(text) if text == "raise" || text == "ignore");
                if !valid {
                    let shown = self.str_value(&action)?;
                    return Err(format!(
                        "ValueError: extrasaction ({shown}) must be 'raise' or 'ignore'"
                    ));
                }
                self.csv_set(&receiver, "extrasaction", action)?;
                let writer = self.csv_make(
                    "writer",
                    bound.arg(1),
                    bound.or(5, Value::String("excel".to_string())),
                    bound.extra_args,
                    bound.extra_kwargs,
                )?;
                self.csv_set(&receiver, "writer", writer)?;
                Ok(Value::None)
            }
            "writeheader" => {
                let bound = bind(
                    "DictWriter.writeheader",
                    &["self"],
                    1,
                    false,
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let fieldnames = self.csv_get(&receiver, "fieldnames")?;
                let pairs = self.call_value(
                    Value::Builtin("zip".to_string()),
                    vec![fieldnames.clone(), fieldnames],
                )?;
                let header = self.call_value(builtin_type_value("dict"), vec![pairs])?;
                self.csv_call_method(&receiver, "writerow", vec![header])
            }
            "_dict_to_list" => {
                let bound = bind(
                    "DictWriter._dict_to_list",
                    &["self", "rowdict"],
                    2,
                    false,
                    args,
                    keywords,
                )?;
                self.csv_dict_to_list(&bound.arg(0), bound.arg(1))
            }
            "writerow" => {
                let bound = bind(
                    "DictWriter.writerow",
                    &["self", "rowdict"],
                    2,
                    false,
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let row = self.csv_call_method(&receiver, "_dict_to_list", vec![bound.arg(1)])?;
                let writer = self.csv_get(&receiver, "writer")?;
                self.csv_call_method(&writer, "writerow", vec![row])
            }
            "writerows" => {
                let bound = bind(
                    "DictWriter.writerows",
                    &["self", "rowdicts"],
                    2,
                    false,
                    args,
                    keywords,
                )?;
                let receiver = bound.arg(0);
                let convert = self.csv_get(&receiver, "_dict_to_list")?;
                let rows = self.call_value(
                    Value::Builtin("map".to_string()),
                    vec![convert, bound.arg(1)],
                )?;
                let writer = self.csv_get(&receiver, "writer")?;
                self.csv_call_method(&writer, "writerows", vec![rows])
            }
            _ => Err(format!("unknown builtin: csv.DictWriter.{method}")),
        }
    }

    fn csv_dict_to_list(&mut self, receiver: &Value, rowdict: Value) -> Result<Value, String> {
        let action = self.csv_get(receiver, "extrasaction")?;
        if self.equal_values(action, Value::String("raise".to_string()))? {
            let keys = self.csv_call_method(&rowdict, "keys", Vec::new())?;
            let fieldnames = self.csv_get(receiver, "fieldnames")?;
            let wrong = self.subtract_values(keys, fieldnames)?;
            if self.truth_value(wrong.clone())? {
                let mut shown = Vec::new();
                for key in self.collect_iterable_values_propagating(wrong)? {
                    shown.push(self.repr_value(&key)?);
                }
                return Err(format!(
                    "ValueError: dict contains fields not in fieldnames: {}",
                    shown.join(", ")
                ));
            }
        }
        // Each field is rowdict's value for the key, or restval when absent.
        let fieldnames = self.csv_get(receiver, "fieldnames")?;
        let get = self.csv_get(&rowdict, "get")?;
        let mut values = Vec::new();
        for key in self.collect_iterable_values_propagating(fieldnames)? {
            let restval = self.csv_get(receiver, "restval")?;
            values.push(self.call_value(get.clone(), vec![key, restval])?);
        }
        self.consume_allocation_budget(values.len() * std::mem::size_of::<Value>())?;
        self.get_iter_propagating(list_value(values))
    }
}
