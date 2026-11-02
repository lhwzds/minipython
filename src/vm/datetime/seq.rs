// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `datetime.IsoCalendarDate` and `time.struct_time`: tuple subclasses
//! with named fields.

use super::*;

const STRUCT_TIME_FIELDS: [&str; 11] = [
    "tm_year",
    "tm_mon",
    "tm_mday",
    "tm_hour",
    "tm_min",
    "tm_sec",
    "tm_wday",
    "tm_yday",
    "tm_isdst",
    "tm_zone",
    "tm_gmtoff",
];
const ISOCAL_FIELDS: [&str; 3] = ["year", "week", "weekday"];
const TM_ZONE: &str = "\0tm_zone";
const TM_GMTOFF: &str = "\0tm_gmtoff";

fn tuple_instance(class: &Value, items: Vec<Value>) -> Value {
    let Value::Class {
        name, attrs, bases, ..
    } = class
    else {
        unreachable!("sequence instances are created from classes");
    };
    let fields = new_scope();
    fields
        .borrow_mut()
        .insert(TUPLE_SUBCLASS_STORAGE_FIELD.to_string(), tuple_value(items));
    Value::Instance {
        class_name: name.clone(),
        fields,
        class_attrs: attrs.clone(),
        class_bases: bases.clone(),
    }
}

fn tuple_items(value: &Value) -> Option<Rc<Vec<Value>>> {
    let Value::Instance { fields, .. } = value else {
        return None;
    };
    match fields.borrow().get(TUPLE_SUBCLASS_STORAGE_FIELD) {
        Some(Value::Tuple(items)) => Some(items.clone()),
        _ => None,
    }
}

pub(super) fn new_isocal(class: &Value, year: i64, week: i64, weekday: i64) -> Value {
    tuple_instance(class, vec![num(year), num(week), num(weekday)])
}

/// A `time.struct_time` for a date and time with `tm_isdst = dst`.
pub(super) fn new_struct_time(class: &Value, ymd: Ymd, hms: &Hms, dst: i64) -> Value {
    let yday = ymd.ordinal() - ymd_to_ordinal(ymd.year, 1, 1) + 1;
    tuple_instance(
        class,
        vec![
            num(ymd.year),
            num(ymd.month),
            num(ymd.day),
            num(hms.hour),
            num(hms.minute),
            num(hms.second),
            num(weekday(ymd.year, ymd.month, ymd.day)),
            num(yday),
            num(dst),
        ],
    )
}

fn hidden_field(value: &Value, name: &str) -> Value {
    match value {
        Value::Instance { fields, .. } => fields.borrow().get(name).cloned().unwrap_or(Value::None),
        _ => Value::None,
    }
}

fn field_repr(value: &Value) -> String {
    match value {
        Value::String(text) => repr_string(text),
        other => other.to_string(),
    }
}

/// `repr` of a sequence value without calling into Python.
pub(super) fn native_repr(kind: Kind, value: &Value) -> Option<String> {
    let items = tuple_items(value)?;
    let names: &[&str] = if kind == Kind::IsoCal {
        &ISOCAL_FIELDS
    } else {
        &STRUCT_TIME_FIELDS[..9]
    };
    let body = names
        .iter()
        .zip(items.iter())
        .map(|(name, item)| format!("{name}={}", field_repr(item)))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("{}({body})", kind.tp_name()))
}

impl Vm {
    pub(super) fn dt_seq_method(
        &mut self,
        kind: Kind,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if method == "__new__" {
            let class = self.dt_class_arg(&mut args, kind, "__new__")?;
            return if kind == Kind::IsoCal {
                self.dt_isocal_new(&class, args, keywords)
            } else {
                self.dt_struct_time_new(&class, args, keywords)
            };
        }
        let receiver = self.dt_receiver(&mut args, kind, method, &[kind])?;
        let Some(items) = tuple_items(&receiver) else {
            return self.dt_error("TypeError", "sequence is not initialized");
        };
        let fields: &[&str] = if kind == Kind::IsoCal {
            &ISOCAL_FIELDS
        } else {
            &STRUCT_TIME_FIELDS
        };
        if let Some(index) = fields.iter().position(|field| *field == method) {
            return Ok(match method {
                "tm_zone" => hidden_field(&receiver, TM_ZONE),
                "tm_gmtoff" => hidden_field(&receiver, TM_GMTOFF),
                _ => items.get(index).cloned().unwrap_or(Value::None),
            });
        }
        match method {
            "__repr__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let names: &[&str] = if kind == Kind::IsoCal {
                    &ISOCAL_FIELDS
                } else {
                    &STRUCT_TIME_FIELDS[..9]
                };
                let mut parts = Vec::new();
                for (name, item) in names.iter().zip(items.iter()) {
                    parts.push(format!("{name}={}", self.repr_value(item)?));
                }
                Ok(Value::String(format!(
                    "{}({})",
                    kind.tp_name(),
                    parts.join(", ")
                )))
            }
            "__reduce__" => {
                self.dt_no_args(
                    &format!("{}.__reduce__", kind.class_name()),
                    &args,
                    &keywords,
                )?;
                let plain = tuple_value(items.as_ref().clone());
                if kind == Kind::IsoCal {
                    return Ok(tuple_value(vec![
                        builtin_type_value("tuple"),
                        tuple_value(vec![plain]),
                    ]));
                }
                let extras = Value::Dict(Rc::new(RefCell::new(DictStorage::new(vec![
                    (
                        Value::String("tm_zone".to_string()),
                        hidden_field(&receiver, TM_ZONE),
                    ),
                    (
                        Value::String("tm_gmtoff".to_string()),
                        hidden_field(&receiver, TM_GMTOFF),
                    ),
                ]))));
                Ok(tuple_value(vec![
                    class_of(&receiver),
                    tuple_value(vec![plain, extras]),
                ]))
            }
            _ => self.dt_error(
                "AttributeError",
                format!("'{}' object has no attribute '{method}'", kind.tp_name()),
            ),
        }
    }

    fn dt_isocal_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let spec = ArgSpec {
            fname: Some("IsoCalendarDate"),
            params: &ISOCAL_FIELDS,
            required: 3,
            positional: 3,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, _, value| {
            vm.dt_c_int(&value).map(num)
        })?;
        Ok(tuple_instance(
            class,
            values
                .into_iter()
                .map(|value| value.unwrap_or(Value::None))
                .collect(),
        ))
    }

    fn dt_struct_time_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let spec = ArgSpec {
            fname: Some("struct_time"),
            params: &["sequence", "dict"],
            required: 1,
            positional: 2,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |_, _, value| Ok(value))?;
        let sequence = values[0].clone().expect("sequence is required");
        let items = self.collect_iterable_values(sequence)?;
        if items.len() < 9 {
            return self.dt_error(
                "TypeError",
                format!(
                    "time.struct_time() takes an at least 9-sequence ({}-sequence given)",
                    items.len()
                ),
            );
        }
        if items.len() > 11 {
            return self.dt_error(
                "TypeError",
                format!(
                    "time.struct_time() takes an at most 11-sequence ({}-sequence given)",
                    items.len()
                ),
            );
        }
        let instance = tuple_instance(class, items[..9].to_vec());
        if let Value::Instance { fields, .. } = &instance {
            let mut fields = fields.borrow_mut();
            fields.insert(
                TM_ZONE.to_string(),
                items.get(9).cloned().unwrap_or(Value::None),
            );
            fields.insert(
                TM_GMTOFF.to_string(),
                items.get(10).cloned().unwrap_or(Value::None),
            );
        }
        Ok(instance)
    }
}
