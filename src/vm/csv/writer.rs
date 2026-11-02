// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Modules/_csv.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `_csv.writer`: record formatting.
//!
//! Each row is joined field by field: the quoting mode decides which
//! fields start out quoted, special characters (the delimiter, the quote and
//! escape characters, line breaks, and any character of the line
//! terminator) either force quoting or are escaped, and the finished record
//! is passed to the target's `write` method, whose result `writerow`
//! returns.

use super::super::native_class::{c_one_arg, c_unpack_positional, native_instance};
use super::*;

const WRITE_FIELD: &str = "\0csv_write";
const DIALECT_FIELD: &str = "\0csv_dialect";

const NEED_ESCAPE: &str = "need to escape, but no escapechar set";
const EMPTY_SPACE_FIELD: &str =
    "empty field must be quoted if delimiter is a space and skipinitialspace is true";

/// A record being joined.
struct Record {
    text: String,
    fields: usize,
}

impl Vm {
    pub(super) fn csv_new_writer(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let [target, dialect] = c_unpack_positional("writer", args, 1, 2)?
            .try_into()
            .expect("two slots");
        let target = target.expect("required");
        let write = match self.load_attribute_catching(target, "write")? {
            Ok(write) => Some(write),
            Err(exception) if exception.type_name == "AttributeError" => None,
            Err(exception) => return Err(self.propagate_captured_exception(exception)),
        };
        let callable = match &write {
            Some(write) => {
                let result =
                    self.call_value(Value::Builtin("callable".to_string()), vec![write.clone()])?;
                self.truth_value(result)?
            }
            None => false,
        };
        let Some(write) = write.filter(|_| callable) else {
            return Err("TypeError: argument 1 must have a \"write\" method".to_string());
        };
        let dialect = self.csv_call_dialect(dialect, keywords)?;
        let class = self.csv_state(STATE_WRITER_TYPE)?;
        native_instance(
            &class,
            vec![
                (WRITE_FIELD.to_string(), write),
                (DIALECT_FIELD.to_string(), dialect),
            ],
        )
    }

    pub(super) fn csv_writer_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if method == "__new__" {
            return Err(cannot_create("writer", args.first()));
        }
        if args.is_empty() {
            return Err(format!(
                "TypeError: descriptor '{method}' of '_csv.writer' object needs an argument"
            ));
        }
        let receiver = args.remove(0);
        match method {
            "writerow" => {
                let row = c_one_arg("writer.writerow", args, &keywords)?;
                self.csv_writerow(&receiver, row)
            }
            "writerows" => {
                let rows = c_one_arg("writer.writerows", args, &keywords)?;
                self.for_each_iterable_value(rows, |vm, row| {
                    vm.csv_writerow(&receiver, row)?;
                    Ok(())
                })?;
                Ok(Value::None)
            }
            "dialect" => Ok(field(&receiver, DIALECT_FIELD).unwrap_or(Value::None)),
            "__setattr__" | "__delattr__" => {
                let name = args.first().and_then(string_of).unwrap_or_default();
                if name == "dialect" {
                    return Err("AttributeError: readonly attribute".to_string());
                }
                Err(no_new_attribute(
                    "_csv.writer",
                    &name,
                    method == "__delattr__",
                ))
            }
            "__reduce_ex__" => Err(csv_pickle_error("writer", &receiver)),
            "__getattr__" => Err(missing_attribute("_csv.writer", &args)),
            _ => Err(format!("unknown builtin: _csv.writer.{method}")),
        }
    }

    fn csv_writerow(&mut self, receiver: &Value, row: Value) -> Result<Value, String> {
        let dialect = field(receiver, DIALECT_FIELD).unwrap_or(Value::None);
        let Some(config) = dialect_config(&dialect) else {
            return Err("SystemError: csv writer has no dialect".to_string());
        };
        let write = field(receiver, WRITE_FIELD).unwrap_or(Value::None);
        let mut iterator = match self.get_iter_capturing(row.clone())? {
            Ok(iterator) => iterator,
            Err(exception) if exception.type_name == "TypeError" => {
                return self.csv_error(format!("iterable expected, not {}", type_name(&row)));
            }
            Err(exception) => return Err(self.propagate_captured_exception(exception)),
        };
        let mut record = Record {
            text: String::new(),
            fields: 0,
        };
        let mut null_field = false;
        loop {
            let value = match self.advance_owned_iterator_capturing(&mut iterator)? {
                Ok(IteratorAdvance::Yield(value)) => value,
                Ok(IteratorAdvance::Complete(_)) | Ok(IteratorAdvance::Raised) => break,
                Err(exception) => return Err(self.propagate_captured_exception(exception)),
            };
            let is_none = matches!(value, Value::None);
            let quoted = match config.quoting {
                QUOTE_NONNUMERIC => !self.csv_is_number(&value),
                QUOTE_ALL => true,
                QUOTE_STRINGS => string_of(&value).is_some(),
                QUOTE_NOTNULL => !is_none,
                _ => false,
            };
            null_field = is_none;
            let text = if is_none {
                None
            } else if let Some(text) = string_of(&value) {
                Some(text)
            } else {
                Some(self.str_value(&value)?)
            };
            if let Err(message) = join_append(&mut record, &config, text.as_deref(), quoted) {
                return self.csv_error(message);
            }
        }
        if record.fields > 0 && record.text.is_empty() {
            if config.quoting == QUOTE_NONE
                || (null_field
                    && (config.quoting == QUOTE_STRINGS || config.quoting == QUOTE_NOTNULL))
            {
                return self.csv_error("single empty field record must be quoted");
            }
            record.fields -= 1;
            if let Err(message) = join_append(&mut record, &config, None, true) {
                return self.csv_error(message);
            }
        }
        record.text.push_str(&config.lineterminator);
        self.consume_allocation_budget(record.text.len())?;
        self.call_value(write, vec![Value::String(record.text)])
    }

    /// `PyNumber_Check`: numbers and objects with `__index__`, `__int__`,
    /// or `__float__`.
    fn csv_is_number(&mut self, value: &Value) -> bool {
        match value {
            Value::Number(_)
            | Value::BigInt(_)
            | Value::Bool(_)
            | Value::Float(_)
            | Value::Complex { .. } => true,
            Value::String(_) | Value::IdentityString { .. } | Value::None => false,
            other => {
                int_subclass_integer(other).is_some()
                    || float_subclass_value(other).is_some()
                    || ["__index__", "__int__", "__float__", "__complex__"]
                        .iter()
                        .any(|name| instance_special_method(other, name).is_some())
                    || matches!(
                        type_name(other),
                        "decimal.Decimal" | "Decimal" | "Fraction" | "fractions.Fraction"
                    )
            }
        }
    }
}

/// Appends one field to the record, quoting or escaping as the dialect
/// requires. Errors are `_csv.Error` messages.
fn join_append(
    record: &mut Record,
    config: &Config,
    field: Option<&str>,
    mut quoted: bool,
) -> Result<(), String> {
    let text = field.unwrap_or("");
    if text.is_empty() && config.delimiter == ' ' && config.skipinitialspace {
        if config.quoting == QUOTE_NONE
            || (field.is_none()
                && (config.quoting == QUOTE_STRINGS || config.quoting == QUOTE_NOTNULL))
        {
            return Err(EMPTY_SPACE_FIELD.to_string());
        }
        quoted = true;
    }
    let quotechar = config.quotechar;
    let escapechar = config.escapechar;
    let mut body = String::with_capacity(text.len());
    for c in text.chars() {
        let special = c == config.delimiter
            || Some(c) == escapechar
            || Some(c) == quotechar
            || c == '\n'
            || c == '\r'
            || config.lineterminator.contains(c);
        if special {
            let mut want_escape = false;
            if config.quoting == QUOTE_NONE {
                want_escape = true;
            } else {
                if Some(c) == quotechar {
                    if config.doublequote {
                        body.push(c);
                    } else {
                        want_escape = true;
                    }
                } else if Some(c) == escapechar {
                    want_escape = true;
                }
                if !want_escape {
                    quoted = true;
                }
            }
            if want_escape {
                let Some(escape) = escapechar else {
                    return Err(NEED_ESCAPE.to_string());
                };
                body.push(escape);
            }
        }
        body.push(c);
    }
    if record.fields > 0 {
        record.text.push(config.delimiter);
    }
    let quote = quotechar.filter(|_| quoted);
    if let Some(quote) = quote {
        record.text.push(quote);
    }
    record.text.push_str(&body);
    if let Some(quote) = quote {
        record.text.push(quote);
    }
    record.fields += 1;
    Ok(())
}
