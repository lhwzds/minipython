// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `datetime.date` (and the methods `datetime.datetime` inherits from it).

use super::*;

const DATE_PARAMS: &[&str] = &["year", "month", "day"];

/// The date part of a date or datetime state.
pub(super) fn date_part(state: &State) -> Option<Ymd> {
    match state {
        State::Date(ymd) | State::DateTime(ymd, _) => Some(*ymd),
        _ => None,
    }
}

impl Vm {
    pub(super) fn dt_date_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__new__" => {
                let class = self.dt_class_arg(&mut args, Kind::Date, "__new__")?;
                return self.dt_date_new(&class, args, keywords);
            }
            "today" | "fromtimestamp" | "fromordinal" | "fromisoformat" | "fromisocalendar"
            | "strptime" => {
                let class = self.dt_class_arg(&mut args, Kind::Date, method)?;
                return self.dt_date_classmethod(&class, method, args, keywords);
            }
            _ => {}
        }
        let receiver =
            self.dt_receiver(&mut args, Kind::Date, method, &[Kind::Date, Kind::DateTime])?;
        let state = self.dt_state(&receiver)?;
        let Some(ymd) = date_part(&state) else {
            return self.dt_error("TypeError", "date is not initialized");
        };
        let noargs = |name: &str| format!("date.{name}");
        match method {
            "year" => Ok(num(ymd.year)),
            "month" => Ok(num(ymd.month)),
            "day" => Ok(num(ymd.day)),
            "__repr__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(Value::String(format!(
                    "{}({}, {}, {})",
                    repr_type_name(&receiver),
                    ymd.year,
                    ymd.month,
                    ymd.day
                )))
            }
            "__str__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let isoformat = self.load_attribute_value(receiver.clone(), "isoformat")?;
                self.call_value(isoformat, Vec::new())
            }
            "isoformat" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(Value::String(format::date_isoformat(ymd)))
            }
            "__hash__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(num(delta::hash_parts(&[
                    1,
                    i128::from(ymd.year),
                    i128::from(ymd.month),
                    i128::from(ymd.day),
                ])))
            }
            "toordinal" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(num(ymd.ordinal()))
            }
            "weekday" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(num(weekday(ymd.year, ymd.month, ymd.day)))
            }
            "isoweekday" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(num(weekday(ymd.year, ymd.month, ymd.day) + 1))
            }
            "isocalendar" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                let (year, week, day) = iso_calendar(ymd.year, ymd.month, ymd.day);
                let registry = self.dt_registry(&receiver)?;
                Ok(seq::new_isocal(&registry.isocal, year, week, day))
            }
            "ctime" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(Value::String(format::ctime(ymd, &Hms::midnight())))
            }
            "timetuple" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                let registry = self.dt_registry(&receiver)?;
                Ok(seq::new_struct_time(
                    &registry.struct_time,
                    ymd,
                    &Hms::midnight(),
                    -1,
                ))
            }
            "strftime" => {
                let format = self.dt_strftime_arg(args, keywords)?;
                let hms = match &state {
                    State::DateTime(_, hms) => hms.clone(),
                    _ => Hms::midnight(),
                };
                // CPython formats the fields of `self.timetuple()`.
                let timetuple = self.load_attribute_value(receiver.clone(), "timetuple")?;
                let timetuple = self.call_value(timetuple, Vec::new())?;
                let items = self.dt_time_tuple_items(&timetuple)?;
                let parts = format::TimeParts::from_tuple(&items, hms.us)?;
                let tz = hms.tz.clone();
                let text = self.dt_strftime(&format, &parts, &tz, &receiver)?;
                Ok(Value::String(text))
            }
            "__format__" => {
                let spec = self.dt_format_spec_arg(args, &keywords)?;
                self.dt_format_via_strftime(&receiver, spec)
            }
            "replace" | "__replace__" => {
                self.dt_date_replace(&receiver, ymd, method, args, keywords)
            }
            "__reduce__" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(tuple_value(vec![
                    class_of(&receiver),
                    tuple_value(vec![bytes_value(date_state_bytes(ymd).to_vec())]),
                ]))
            }
            "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                // Only plain dates compare with dates; a datetime is not a date
                // for comparisons.
                if kind_of(&receiver) == Some(Kind::Date)
                    && kind_of(&other) == Some(Kind::Date)
                    && let Some(State::Date(right)) = state_of(&other)
                {
                    return Ok(Value::Bool(delta::compare_op(
                        method,
                        ymd.ordinal().cmp(&right.ordinal()),
                    )));
                }
                self.dt_compare_unsupported(method, &receiver, &other)
            }
            "__add__" | "__radd__" | "__sub__" | "__rsub__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                self.dt_date_binary(method, &receiver, ymd, &other)
            }
            _ => self.dt_error(
                "AttributeError",
                format!("'datetime.date' object has no attribute '{method}'"),
            ),
        }
    }

    /// The integers of a `time.struct_time` or a tuple.
    pub(super) fn dt_time_tuple_items(&mut self, value: &Value) -> Result<Vec<i64>, String> {
        let items = match value {
            Value::Tuple(items) => items.as_ref().clone(),
            Value::Instance { fields, .. } => match fields
                .borrow()
                .get(TUPLE_SUBCLASS_STORAGE_FIELD)
            {
                Some(Value::Tuple(items)) => items.as_ref().clone(),
                _ => return self.dt_error("TypeError", "Tuple or struct_time argument required"),
            },
            _ => return self.dt_error("TypeError", "Tuple or struct_time argument required"),
        };
        let mut ints = Vec::with_capacity(items.len());
        for item in &items {
            ints.push(self.dt_c_int(item)?);
        }
        Ok(ints)
    }

    pub(super) fn dt_strftime_arg(
        &mut self,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<String, String> {
        let spec = ArgSpec {
            fname: Some("strftime"),
            params: &["format"],
            required: 1,
            positional: 1,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, _, value| {
            vm.dt_str_arg(&value, "strftime", 1).map(Value::String)
        })?;
        match &values[0] {
            Some(Value::String(text)) => Ok(text.clone()),
            _ => unreachable!("format is required and converted"),
        }
    }

    pub(super) fn dt_format_spec_arg(
        &mut self,
        args: Vec<Value>,
        keywords: &[(String, Value)],
    ) -> Result<String, String> {
        self.dt_exact_args("__format__", 1, &args, keywords)?;
        self.dt_str_arg(&args[0], "__format__", 1)
    }

    /// `__format__`: `str(self)` for an empty spec, else `self.strftime(spec)`.
    pub(super) fn dt_format_via_strftime(
        &mut self,
        receiver: &Value,
        spec: String,
    ) -> Result<Value, String> {
        if spec.is_empty() {
            return Ok(Value::String(self.str_value(receiver)?));
        }
        let strftime = self.load_attribute_value(receiver.clone(), "strftime")?;
        self.call_value(strftime, vec![Value::String(spec)])
    }

    fn dt_date_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        // Pickle support: date(b'\x07\xe4\x01\x02').
        if args.len() == 1
            && keywords.is_empty()
            && let Some(state) = pickle_state_bytes(&args[0], 4)
            && (1..=12).contains(&(state[2] & 0x7f))
        {
            let ymd = Ymd {
                year: i64::from(state[0]) * 256 + i64::from(state[1]),
                month: i64::from(state[2]),
                day: i64::from(state[3]),
            };
            return Ok(new_instance(class, &State::Date(ymd)));
        }
        let spec = ArgSpec {
            fname: None,
            params: DATE_PARAMS,
            required: 3,
            positional: 3,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, _, value| {
            vm.dt_c_int(&value).map(num)
        })?;
        let int = |index: usize| match &values[index] {
            Some(Value::Number(value)) => *value,
            _ => 0,
        };
        let ymd = Ymd {
            year: int(0),
            month: int(1),
            day: int(2),
        };
        check_date(ymd.year, ymd.month, ymd.day)?;
        Ok(new_instance(class, &State::Date(ymd)))
    }

    fn dt_date_replace(
        &mut self,
        receiver: &Value,
        ymd: Ymd,
        _method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let spec = ArgSpec {
            fname: Some("replace"),
            params: DATE_PARAMS,
            required: 0,
            positional: 3,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, _, value| {
            vm.dt_c_int(&value).map(num)
        })?;
        let pick = |index: usize, current: i64| match &values[index] {
            Some(Value::Number(value)) => *value,
            _ => current,
        };
        let replaced = Ymd {
            year: pick(0, ymd.year),
            month: pick(1, ymd.month),
            day: pick(2, ymd.day),
        };
        let class = class_of(receiver);
        if kind_of(receiver) == Some(Kind::DateTime) {
            // `date.replace` on a datetime builds `type(self)(y, m, d)`.
            check_date(replaced.year, replaced.month, replaced.day)?;
            return self.dt_datetime_from_ymd_only(&class, replaced);
        }
        check_date(replaced.year, replaced.month, replaced.day)?;
        self.dt_make_date(&class, replaced)
    }

    fn dt_date_binary(
        &mut self,
        method: &str,
        receiver: &Value,
        ymd: Ymd,
        other: &Value,
    ) -> Result<Value, String> {
        let other_kind = kind_of(other);
        let other_state = state_of(other);
        match (method, other_kind, other_state) {
            ("__add__" | "__radd__", Some(Kind::Delta), Some(State::Delta(delta))) => {
                self.dt_date_add_days(receiver, ymd, delta.days)
            }
            ("__sub__", Some(Kind::Delta), Some(State::Delta(delta))) => {
                self.dt_date_add_days(receiver, ymd, -delta.days)
            }
            ("__sub__", Some(Kind::Date), Some(State::Date(right)))
                if kind_of(receiver) == Some(Kind::Date) =>
            {
                let days = ymd.ordinal() - right.ordinal();
                self.dt_delta_from_us(receiver, i128::from(days) * US_PER_DAY)
            }
            ("__rsub__", Some(Kind::Date), Some(State::Date(left)))
                if kind_of(receiver) == Some(Kind::Date) =>
            {
                let days = left.ordinal() - ymd.ordinal();
                self.dt_delta_from_us(receiver, i128::from(days) * US_PER_DAY)
            }
            _ => self.dt_binary_unsupported(method, receiver, other),
        }
    }

    fn dt_date_add_days(&mut self, receiver: &Value, ymd: Ymd, days: i64) -> Result<Value, String> {
        let ordinal = ymd.ordinal() + days;
        if !(1..=MAX_ORDINAL).contains(&ordinal) {
            return self.dt_error("OverflowError", "date value out of range");
        }
        self.dt_make_date(&class_of(receiver), Ymd::from_ordinal(ordinal))
    }

    fn dt_date_classmethod(
        &mut self,
        class: &Value,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "today" => {
                self.dt_no_args("date.today", &args, &keywords)?;
                if kind_of_class(class) == Some(Kind::DateTime) {
                    return self.dt_datetime_today(class);
                }
                let micros = self.dt_now_micros();
                let seconds = micros.div_euclid(1_000_000);
                let local = seconds + self.dt_local_offset(seconds);
                let ymd = Ymd::from_ordinal(local.div_euclid(86_400) + EPOCH_ORDINAL);
                self.dt_make_date(class, ymd)
            }
            "fromtimestamp" => {
                let timestamp = self.dt_one_arg("date.fromtimestamp", args, &keywords)?;
                let seconds = self.dt_timestamp_seconds_floor(&timestamp)?;
                let local = seconds + self.dt_local_offset(seconds);
                let ymd = Ymd::from_ordinal(local.div_euclid(86_400) + EPOCH_ORDINAL);
                check_date(ymd.year, ymd.month, ymd.day)?;
                self.dt_make_date(class, ymd)
            }
            "fromordinal" => {
                self.dt_exact_args("fromordinal", 1, &args, &keywords)?;
                let ordinal = self.dt_c_int(&args[0])?;
                if ordinal < 1 {
                    return self.dt_error("ValueError", "ordinal must be >= 1");
                }
                let ymd = Ymd::from_ordinal(ordinal);
                if kind_of_class(class) == Some(Kind::DateTime) {
                    return self.dt_datetime_from_ymd_only(class, ymd);
                }
                self.dt_make_date(class, ymd)
            }
            "fromisoformat" => {
                let text = self.dt_one_arg("date.fromisoformat", args, &keywords)?;
                let text = self.dt_fromisoformat_text(&text)?;
                if kind_of_class(class) == Some(Kind::DateTime) {
                    return self.dt_datetime_fromisoformat(class, &text);
                }
                let ymd = iso::parse_date_only(&text)?;
                self.dt_make_date(class, ymd)
            }
            "fromisocalendar" => {
                let spec = ArgSpec {
                    fname: Some("fromisocalendar"),
                    params: &["year", "week", "day"],
                    required: 3,
                    positional: 3,
                };
                let values = self
                    .dt_parse_args(&spec, args, keywords, &mut |vm, _, value| {
                        vm.dt_c_int(&value).map(num)
                    })
                    .map_err(|message| {
                        if message.starts_with("OverflowError") {
                            "ValueError: ISO calendar component out of range".to_string()
                        } else {
                            message
                        }
                    })?;
                let int = |index: usize| match &values[index] {
                    Some(Value::Number(value)) => *value,
                    _ => 0,
                };
                let (year, month, day) = iso_to_ymd(int(0), int(1), int(2))?;
                check_date(year, month, day)?;
                let ymd = Ymd { year, month, day };
                if kind_of_class(class) == Some(Kind::DateTime) {
                    return self.dt_datetime_from_ymd_only(class, ymd);
                }
                self.dt_make_date(class, ymd)
            }
            "strptime" => {
                self.dt_exact_args("strptime", 2, &args, &keywords)?;
                let text = self.dt_str_arg(&args[0], "strptime", 1)?;
                let format = self.dt_str_arg(&args[1], "strptime", 2)?;
                self.dt_strptime(class, Kind::Date, &text, &format)
            }
            _ => self.dt_error(
                "AttributeError",
                format!("type object 'datetime.date' has no attribute '{method}'"),
            ),
        }
    }

    /// The text argument of `fromisoformat`.
    pub(super) fn dt_fromisoformat_text(&mut self, value: &Value) -> Result<String, String> {
        match value {
            Value::String(text) | Value::IdentityString { value: text, .. } => Ok(text.clone()),
            other => match str_subclass_string(other) {
                Some(text) => Ok(text),
                None => self.dt_error("TypeError", "fromisoformat: argument must be str"),
            },
        }
    }
}

/// Ordinal of 1970-01-01.
pub(super) const EPOCH_ORDINAL: i64 = 719_163;

/// The bytes of a pickle state argument of the given length, when `value`
/// is `bytes` (or a latin-1 `str`).
pub(super) fn pickle_state_bytes(value: &Value, length: usize) -> Option<Vec<u8>> {
    let bytes = match value {
        Value::Bytes(bytes) => bytes.as_ref().clone(),
        Value::String(text) => {
            if text.chars().any(|ch| u32::from(ch) > 0xff) {
                return None;
            }
            text.chars().map(|ch| ch as u8).collect()
        }
        _ => return None,
    };
    (bytes.len() == length).then_some(bytes)
}

pub(super) fn date_state_bytes(ymd: Ymd) -> [u8; 4] {
    [
        (ymd.year / 256) as u8,
        (ymd.year % 256) as u8,
        ymd.month as u8,
        ymd.day as u8,
    ]
}
