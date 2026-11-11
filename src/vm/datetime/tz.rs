// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `datetime.tzinfo`, `datetime.timezone`, and calls into tzinfo methods.

use super::*;

/// `UTC±HH:MM[:SS[.ffffff]]` for a timezone without a name.
pub(super) fn timezone_default_name(offset: Delta) -> String {
    if offset == Delta::ZERO {
        return "UTC".to_string();
    }
    let total = offset.total_us();
    let sign = if total < 0 { '-' } else { '+' };
    let total = total.abs();
    let us = total % US_PER_SECOND;
    let seconds = total / US_PER_SECOND;
    let mut text = format!("UTC{sign}{:02}:{:02}", seconds / 3600, seconds / 60 % 60);
    if seconds % 60 != 0 || us != 0 {
        text.push_str(&format!(":{:02}", seconds % 60));
    }
    if us != 0 {
        text.push_str(&format!(".{us:06}"));
    }
    text
}

/// Whether `offset` is strictly between -24h and 24h.
pub(super) fn offset_in_range(offset: Delta) -> bool {
    offset.total_us().abs() < US_PER_DAY
}

impl Vm {
    /// `tzinfo.utcoffset(arg)` or `tzinfo.dst(arg)` with CPython's result
    /// checks. `Ok(None)` for a naive value or a `None` result.
    pub(super) fn dt_call_offset(
        &mut self,
        tz: &Value,
        which: &str,
        arg: &Value,
    ) -> Result<Option<Delta>, String> {
        if matches!(tz, Value::None) {
            return Ok(None);
        }
        if instance_is_native(tz)
            && let Some(State::TimeZone(offset, _)) = state_of(tz)
        {
            return Ok(if which == "utcoffset" {
                Some(offset)
            } else {
                None
            });
        }
        let method = self.load_attribute_value(tz.clone(), which)?;
        let result = self.call_value(method, vec![arg.clone()])?;
        if matches!(result, Value::None) {
            return Ok(None);
        }
        match (kind_of(&result), state_of(&result)) {
            (Some(Kind::Delta), Some(State::Delta(offset))) => {
                if !offset_in_range(offset) {
                    return self.dt_error(
                        "ValueError",
                        format!(
                            "offset must be a timedelta strictly between -timedelta(hours=24) and timedelta(hours=24), not {}",
                            delta::delta_repr(&repr_type_name(&result), offset)
                        ),
                    );
                }
                Ok(Some(offset))
            }
            _ => self.dt_error(
                "TypeError",
                format!(
                    "tzinfo.{which}() must return None or timedelta, not '{}'",
                    tp_name(&result)
                ),
            ),
        }
    }

    /// `tzinfo.tzname(arg)` with CPython's result check.
    pub(super) fn dt_call_tzname(&mut self, tz: &Value, arg: &Value) -> Result<Value, String> {
        if matches!(tz, Value::None) {
            return Ok(Value::None);
        }
        if instance_is_native(tz)
            && let Some(State::TimeZone(offset, name)) = state_of(tz)
        {
            return Ok(Value::String(
                name.unwrap_or_else(|| timezone_default_name(offset)),
            ));
        }
        let method = self.load_attribute_value(tz.clone(), "tzname")?;
        let result = self.call_value(method, vec![arg.clone()])?;
        match &result {
            Value::None | Value::String(_) | Value::IdentityString { .. } => Ok(result),
            other if str_subclass_string(other).is_some() => Ok(result),
            other => self.dt_error(
                "TypeError",
                format!(
                    "tzinfo.tzname() must return None or a string, not '{}'",
                    tp_name(other)
                ),
            ),
        }
    }

    pub(super) fn dt_tz_method(
        &mut self,
        owner: Kind,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if owner == Kind::TimeZone && method == "__new__" {
            let class = self.dt_class_arg(&mut args, Kind::TimeZone, "__new__")?;
            return self.dt_timezone_new(&class, args, keywords);
        }
        if method == "__new__" {
            // `tzinfo(*args, **kwargs)` accepts and ignores its arguments;
            // subclasses initialize themselves in `__init__`.
            let class = self.dt_class_arg(&mut args, Kind::TzInfo, "__new__")?;
            let Value::Class {
                name, attrs, bases, ..
            } = &class
            else {
                return self.dt_error("TypeError", "tzinfo.__new__(X): X is not a type object");
            };
            return Ok(Value::Instance {
                class_name: name.clone(),
                fields: new_scope(),
                class_attrs: attrs.clone(),
                class_bases: bases.clone(),
            });
        }
        let accepted: &[Kind] = if owner == Kind::TimeZone {
            &[Kind::TimeZone]
        } else {
            &[Kind::TzInfo, Kind::TimeZone]
        };
        let receiver = self.dt_receiver(&mut args, owner, method, accepted)?;
        if owner == Kind::TzInfo {
            return self.dt_tzinfo_base_method(&receiver, method, args, keywords);
        }
        let State::TimeZone(offset, name) = self.dt_state(&receiver)? else {
            return self.dt_error("TypeError", "timezone is not initialized");
        };
        let qualname = format!("timezone.{method}");
        match method {
            "utcoffset" | "dst" | "tzname" => {
                let arg = self.dt_one_arg(&qualname, args, &keywords)?;
                if !matches!(arg, Value::None) && kind_of(&arg) != Some(Kind::DateTime) {
                    return self.dt_error(
                        "TypeError",
                        format!(
                            "{method}(dt) argument must be a datetime instance or None, not {}",
                            tp_name(&arg)
                        ),
                    );
                }
                match method {
                    "utcoffset" => self.dt_new_delta(&receiver, offset),
                    "dst" => Ok(Value::None),
                    _ => Ok(Value::String(
                        name.unwrap_or_else(|| timezone_default_name(offset)),
                    )),
                }
            }
            "fromutc" => {
                let dt = self.dt_one_arg(&qualname, args, &keywords)?;
                let Some(State::DateTime(ymd, hms)) = (kind_of(&dt) == Some(Kind::DateTime))
                    .then(|| state_of(&dt))
                    .flatten()
                else {
                    return self.dt_error("TypeError", "fromutc: argument must be a datetime");
                };
                if !same_object(&hms.tz, &receiver) {
                    return self.dt_error("ValueError", "fromutc: dt.tzinfo is not self");
                }
                self.dt_datetime_add_us(&dt, ymd, hms, offset.total_us())
            }
            "__getinitargs__" => {
                self.dt_no_args("timezone.__getinitargs__", &args, &keywords)?;
                let offset_value = self.dt_new_delta(&receiver, offset)?;
                Ok(match name {
                    Some(name) => tuple_value(vec![offset_value, Value::String(name)]),
                    None => tuple_value(vec![offset_value]),
                })
            }
            "__repr__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(Value::String(timezone_repr(
                    &receiver,
                    offset,
                    name.as_deref(),
                )))
            }
            "__str__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(Value::String(
                    name.unwrap_or_else(|| timezone_default_name(offset)),
                ))
            }
            "__hash__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(num(delta::hash_parts(&[
                    i128::from(offset.days),
                    i128::from(offset.seconds),
                    i128::from(offset.us),
                ])))
            }
            "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                match (method, state_of(&other)) {
                    ("__eq__" | "__ne__", Some(State::TimeZone(right, _)))
                        if kind_of(&other) == Some(Kind::TimeZone) =>
                    {
                        Ok(Value::Bool((offset == right) == (method == "__eq__")))
                    }
                    _ => self.dt_compare_unsupported(method, &receiver, &other),
                }
            }
            _ => self.dt_error(
                "AttributeError",
                format!("'datetime.timezone' object has no attribute '{method}'"),
            ),
        }
    }

    fn dt_tzinfo_base_method(
        &mut self,
        receiver: &Value,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let qualname = format!("tzinfo.{method}");
        match method {
            "tzname" | "utcoffset" | "dst" => {
                self.dt_one_arg(&qualname, args, &keywords)?;
                self.dt_error(
                    "NotImplementedError",
                    format!("a tzinfo subclass must implement {method}()"),
                )
            }
            "fromutc" => {
                let dt = self.dt_one_arg(&qualname, args, &keywords)?;
                self.dt_tzinfo_fromutc(receiver, &dt)
            }
            "__reduce__" => {
                self.dt_no_args("tzinfo.__reduce__", &args, &keywords)?;
                let init_args = match self.dt_get_optional(receiver, "__getinitargs__")? {
                    Some(getinitargs) => self.call_value(getinitargs, Vec::new())?,
                    None => tuple_value(Vec::new()),
                };
                let state = match self.dt_get_optional(receiver, "__getstate__")? {
                    // The runtime's own `object.__getstate__` does not see
                    // instance attributes; compute CPython's default state.
                    Some(Value::BoundMethod { function, .. })
                        if matches!(function.as_ref(), Value::Builtin(_)) =>
                    {
                        instance_state_dict(receiver)
                    }
                    Some(getstate) => self.call_value(getstate, Vec::new())?,
                    None => instance_state_dict(receiver),
                };
                Ok(tuple_value(vec![class_of(receiver), init_args, state]))
            }
            _ => self.dt_error(
                "AttributeError",
                format!("'datetime.tzinfo' object has no attribute '{method}'"),
            ),
        }
    }

    /// `getattr(object, name, None)` that tells a missing attribute apart.
    pub(super) fn dt_get_optional(
        &mut self,
        object: &Value,
        name: &str,
    ) -> Result<Option<Value>, String> {
        match self.load_attribute_catching(object.clone(), name)? {
            Ok(value) => Ok(Some(value)),
            Err(exception)
                if exception.type_name == "AttributeError"
                    || exception
                        .type_hierarchy
                        .iter()
                        .any(|name| name == "AttributeError") =>
            {
                Ok(None)
            }
            Err(exception) => Err(self.propagate_captured_exception(exception)),
        }
    }

    /// The generic `tzinfo.fromutc` algorithm.
    pub(super) fn dt_tzinfo_fromutc(
        &mut self,
        receiver: &Value,
        dt: &Value,
    ) -> Result<Value, String> {
        let Some(State::DateTime(ymd, hms)) = (kind_of(dt) == Some(Kind::DateTime))
            .then(|| state_of(dt))
            .flatten()
        else {
            return self.dt_error("TypeError", "fromutc: argument must be a datetime");
        };
        if !same_object(&hms.tz, receiver) {
            return self.dt_error("ValueError", "fromutc: dt.tzinfo is not self");
        }
        let Some(offset) = self.dt_call_offset(&hms.tz, "utcoffset", dt)? else {
            return self.dt_error(
                "ValueError",
                "fromutc: non-None utcoffset() result required",
            );
        };
        let Some(dst) = self.dt_call_offset(&hms.tz, "dst", dt)? else {
            return self.dt_error("ValueError", "fromutc: non-None dst() result required");
        };
        let delta = Delta::from_us(offset.total_us() - dst.total_us())?;
        let result = self.dt_datetime_add_us(dt, ymd, hms.clone(), delta.total_us())?;
        let Some(dst) = self.dt_call_offset(&hms.tz, "dst", &result)? else {
            return self.dt_error(
                "ValueError",
                "fromutc: tz.dst() gave inconsistent results; cannot convert",
            );
        };
        if dst == Delta::ZERO {
            return Ok(result);
        }
        let Some(State::DateTime(ymd, hms)) = state_of(&result) else {
            return Ok(result);
        };
        self.dt_datetime_add_us(&result, ymd, hms, dst.total_us())
    }

    fn dt_timezone_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let spec = ArgSpec {
            fname: Some("timezone"),
            params: &["offset", "name"],
            required: 1,
            positional: 2,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |_, index, value| {
            if index == 0 && kind_of(&value) != Some(Kind::Delta) {
                return Err(format!(
                    "TypeError: timezone() argument 1 must be datetime.timedelta, not {}",
                    arg_type_name(&value)
                ));
            }
            if index == 1
                && !matches!(value, Value::String(_) | Value::IdentityString { .. })
                && str_subclass_string(&value).is_none()
            {
                return Err(format!(
                    "TypeError: timezone() argument 2 must be str, not {}",
                    arg_type_name(&value)
                ));
            }
            Ok(value)
        })?;
        let offset_value = values[0].clone().expect("offset is required");
        let Some(State::Delta(offset)) = state_of(&offset_value) else {
            return self.dt_error(
                "TypeError",
                "timezone() argument 1 must be datetime.timedelta",
            );
        };
        let name = match &values[1] {
            Some(Value::String(text)) | Some(Value::IdentityString { value: text, .. }) => {
                Some(text.clone())
            }
            Some(other) => str_subclass_string(other),
            None => None,
        };
        if !offset_in_range(offset) {
            return self.dt_error(
                "ValueError",
                format!(
                    "offset must be a timedelta strictly between -timedelta(hours=24) and timedelta(hours=24), not {}",
                    delta::delta_repr(&repr_type_name(&offset_value), offset)
                ),
            );
        }
        if offset == Delta::ZERO && name.is_none() {
            let registry = self.dt_registry(class)?;
            return Ok(registry.utc);
        }
        Ok(new_instance(class, &State::TimeZone(offset, name)))
    }
}

pub(super) fn timezone_repr(receiver: &Value, offset: Delta, name: Option<&str>) -> String {
    if offset == Delta::ZERO
        && name.is_none()
        && registry_of(receiver).is_some_and(|registry| same_object(&registry.utc, receiver))
    {
        return "datetime.timezone.utc".to_string();
    }
    let offset_repr = delta::delta_repr("datetime.timedelta", offset);
    match name {
        Some(name) => format!(
            "{}({offset_repr}, {})",
            tp_name(receiver),
            repr_string(name)
        ),
        None => format!("{}({offset_repr})", tp_name(receiver)),
    }
}

/// `object.__getstate__()` for an instance: its `__dict__` (or `None` when
/// empty), paired with a dict of slot values when the class has slots.
fn instance_state_dict(value: &Value) -> Value {
    let Value::Instance {
        fields,
        class_attrs,
        class_bases,
        ..
    } = value
    else {
        return Value::None;
    };
    let mut slot_names = Vec::new();
    let mut classes = vec![Value::Class {
        name: String::new(),
        type_params: Vec::new(),
        metaclass: None,
        bases: class_bases.clone(),
        attrs: class_attrs.clone(),
    }];
    classes.extend(mro_for_bases(class_bases).unwrap_or_default());
    for class in &classes {
        let Value::Class { attrs, .. } = class else {
            continue;
        };
        let names = match attrs.borrow().get("__slots__") {
            Some(Value::String(name)) => vec![name.clone()],
            Some(Value::Tuple(items)) => items
                .iter()
                .filter_map(|item| match item {
                    Value::String(name) => Some(name.clone()),
                    _ => None,
                })
                .collect(),
            Some(Value::List(items)) => items
                .borrow()
                .iter()
                .filter_map(|item| match item {
                    Value::String(name) => Some(name.clone()),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
        slot_names.extend(
            names
                .into_iter()
                .filter(|name| name != "__dict__" && name != "__weakref__"),
        );
    }
    let fields = fields.borrow();
    // The first position of each name in the attribute order.
    let mut positions = HashMap::new();
    if let Some(Value::Tuple(order)) = fields.get(CLASS_ATTR_ORDER_ATTR) {
        for (position, item) in order.iter().enumerate() {
            if let Value::String(text) = item {
                positions.entry(text.as_str()).or_insert(position);
            }
        }
    }
    let mut names = fields
        .keys()
        .filter(|name| !name.starts_with('\0'))
        .cloned()
        .collect::<Vec<_>>();
    names.sort_by_key(|name| positions.get(name.as_str()).copied().unwrap_or(usize::MAX));
    let mut dict = Vec::new();
    let mut slots = Vec::new();
    for name in names {
        let item = fields.get(&name).cloned().unwrap_or(Value::None);
        if slot_names.contains(&name) {
            slots.push((Value::String(name), item));
        } else {
            dict.push((Value::String(name), item));
        }
    }
    let as_dict = |entries: Vec<(Value, Value)>| {
        Value::Dict(Rc::new(RefCell::new(DictStorage::new(entries))))
    };
    let dict_state = if dict.is_empty() {
        Value::None
    } else {
        as_dict(dict)
    };
    if slots.is_empty() {
        dict_state
    } else {
        tuple_value(vec![dict_state, as_dict(slots)])
    }
}
