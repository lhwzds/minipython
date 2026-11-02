// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `datetime.time`.

use super::*;

const TIME_PARAMS: &[&str] = &["hour", "minute", "second", "microsecond", "tzinfo", "fold"];

/// Pickle state bytes of a time of day (fold sets the high bit of the hour
/// for protocols above 3).
pub(super) fn time_state_bytes(hms: &Hms, with_fold: bool) -> [u8; 6] {
    let hour = hms.hour as u8 | if with_fold && hms.fold != 0 { 0x80 } else { 0 };
    [
        hour,
        hms.minute as u8,
        hms.second as u8,
        (hms.us >> 16) as u8,
        (hms.us >> 8 & 0xff) as u8,
        (hms.us & 0xff) as u8,
    ]
}

/// How CPython orders aware times with different offsets: whole seconds
/// shifted by the offset's days and seconds, then the time's own
/// microseconds (an offset's microseconds are ignored).
pub(super) fn time_compare_key(hms: &Hms, offset: Delta) -> (i128, i128) {
    let seconds = i128::from(hms.hour * 3600 + hms.minute * 60 + hms.second)
        - i128::from(offset.days) * SECONDS_PER_DAY
        - i128::from(offset.seconds);
    (seconds, i128::from(hms.us))
}

impl Vm {
    pub(super) fn dt_time_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__new__" => {
                let class = self.dt_class_arg(&mut args, Kind::Time, "__new__")?;
                return self.dt_time_new(&class, args, keywords);
            }
            "fromisoformat" => {
                let class = self.dt_class_arg(&mut args, Kind::Time, method)?;
                let text = self.dt_one_arg("time.fromisoformat", args, &keywords)?;
                let text = self.dt_fromisoformat_text(&text)?;
                let parsed = iso::parse_time_only(&text)?;
                let tz = self.dt_iso_timezone(&class, parsed.offset)?;
                let hms = Hms { tz, ..parsed.hms };
                return self.dt_time_from_parts(&class, hms);
            }
            "strptime" => {
                let class = self.dt_class_arg(&mut args, Kind::Time, method)?;
                self.dt_exact_args("strptime", 2, &args, &keywords)?;
                let text = self.dt_str_arg(&args[0], "strptime", 1)?;
                let format = self.dt_str_arg(&args[1], "strptime", 2)?;
                return self.dt_strptime(&class, Kind::Time, &text, &format);
            }
            _ => {}
        }
        let receiver = self.dt_receiver(&mut args, Kind::Time, method, &[Kind::Time])?;
        let State::Time(hms) = self.dt_state(&receiver)? else {
            return self.dt_error("TypeError", "time is not initialized");
        };
        let noargs = |name: &str| format!("time.{name}");
        match method {
            "hour" => Ok(num(hms.hour)),
            "minute" => Ok(num(hms.minute)),
            "second" => Ok(num(hms.second)),
            "microsecond" => Ok(num(hms.us)),
            "tzinfo" => Ok(hms.tz.clone()),
            "fold" => Ok(num(hms.fold)),
            "__repr__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let tz_repr = if hms.is_naive_tz() {
                    String::new()
                } else {
                    self.repr_value(&hms.tz)?
                };
                Ok(Value::String(
                    format::repr_state(&receiver, &State::Time(hms), |_| tz_repr.clone())
                        .expect("time repr"),
                ))
            }
            "__str__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let isoformat = self.load_attribute_value(receiver.clone(), "isoformat")?;
                self.call_value(isoformat, Vec::new())
            }
            "isoformat" => {
                let spec = ArgSpec {
                    fname: Some("isoformat"),
                    params: &["timespec"],
                    required: 0,
                    positional: 1,
                };
                let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, _, value| {
                    vm.dt_str_arg(&value, "isoformat", 1).map(Value::String)
                })?;
                let timespec = match &values[0] {
                    Some(Value::String(text)) => text.clone(),
                    _ => "auto".to_string(),
                };
                let mut text = format::time_isoformat(&hms, &timespec)?;
                if let Some(offset) = self.dt_call_offset(&hms.tz, "utcoffset", &Value::None)? {
                    text.push_str(&format::format_offset(offset, ":"));
                }
                Ok(Value::String(text))
            }
            "strftime" => {
                let format = self.dt_strftime_arg(args, keywords)?;
                let ymd = Ymd {
                    year: 1900,
                    month: 1,
                    day: 1,
                };
                let parts = format::TimeParts::new(ymd, &hms);
                let text = self.dt_strftime(&format, &parts, &hms.tz, &Value::None)?;
                Ok(Value::String(text))
            }
            "__format__" => {
                let spec = self.dt_format_spec_arg(args, &keywords)?;
                self.dt_format_via_strftime(&receiver, spec)
            }
            "utcoffset" | "dst" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                match self.dt_call_offset(&hms.tz, method, &Value::None)? {
                    Some(offset) => self.dt_new_delta(&receiver, offset),
                    None => Ok(Value::None),
                }
            }
            "tzname" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                self.dt_call_tzname(&hms.tz, &Value::None)
            }
            "replace" | "__replace__" => {
                let spec = ArgSpec {
                    fname: Some("replace"),
                    params: TIME_PARAMS,
                    required: 0,
                    positional: 5,
                };
                let values =
                    self.dt_parse_args(&spec, args, keywords, &mut |vm, index, value| {
                        if index == 4 {
                            Ok(value)
                        } else {
                            vm.dt_c_int(&value).map(num)
                        }
                    })?;
                let pick = |index: usize, current: i64| match &values[index] {
                    Some(Value::Number(value)) => *value,
                    _ => current,
                };
                let replaced = Hms {
                    hour: pick(0, hms.hour),
                    minute: pick(1, hms.minute),
                    second: pick(2, hms.second),
                    us: pick(3, hms.us),
                    tz: values[4].clone().unwrap_or_else(|| hms.tz.clone()),
                    fold: pick(5, hms.fold),
                };
                check_time(
                    replaced.hour,
                    replaced.minute,
                    replaced.second,
                    replaced.us,
                    replaced.fold,
                )?;
                self.dt_check_tzinfo(&replaced.tz)?;
                self.dt_make_time(&class_of(&receiver), replaced)
            }
            "__hash__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let probe = if hms.fold != 0 {
                    let mut unfolded = hms.clone();
                    unfolded.fold = 0;
                    new_instance(&class_of(&receiver), &State::Time(unfolded))
                } else {
                    receiver.clone()
                };
                let _ = probe;
                let offset = self.dt_call_offset(&hms.tz, "utcoffset", &Value::None)?;
                let hash = match offset {
                    None => delta::hash_parts(&[2, hms.day_us()]),
                    // CPython hashes the full offset but compares without its
                    // microseconds (see `time_compare_key`).
                    Some(offset) => delta::hash_parts(&[3, hms.day_us() - offset.total_us()]),
                };
                Ok(num(hash))
            }
            "__reduce_ex__" => {
                self.dt_exact_args("__reduce_ex__", 1, &args, &keywords)?;
                let protocol = self.dt_c_int(&args[0])?;
                Ok(self.dt_time_reduce(&receiver, &hms, protocol > 3))
            }
            "__reduce__" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(self.dt_time_reduce(&receiver, &hms, false))
            }
            "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                let Some(State::Time(right)) = (kind_of(&other) == Some(Kind::Time))
                    .then(|| state_of(&other))
                    .flatten()
                else {
                    return self.dt_compare_unsupported(method, &receiver, &other);
                };
                if same_object(&hms.tz, &right.tz) {
                    return Ok(Value::Bool(delta::compare_op(
                        method,
                        hms.day_us().cmp(&right.day_us()),
                    )));
                }
                let left_offset = self.dt_call_offset(&hms.tz, "utcoffset", &Value::None)?;
                let right_offset = self.dt_call_offset(&right.tz, "utcoffset", &Value::None)?;
                let ordering = match (left_offset, right_offset) {
                    (None, None) => hms.day_us().cmp(&right.day_us()),
                    (Some(a), Some(b)) if a == b => hms.day_us().cmp(&right.day_us()),
                    (Some(a), Some(b)) => {
                        time_compare_key(&hms, a).cmp(&time_compare_key(&right, b))
                    }
                    _ => {
                        return match method {
                            "__eq__" => Ok(Value::Bool(false)),
                            "__ne__" => Ok(Value::Bool(true)),
                            _ => self.dt_error(
                                "TypeError",
                                "can't compare offset-naive and offset-aware times",
                            ),
                        };
                    }
                };
                Ok(Value::Bool(delta::compare_op(method, ordering)))
            }
            _ => self.dt_error(
                "AttributeError",
                format!("'datetime.time' object has no attribute '{method}'"),
            ),
        }
    }

    fn dt_time_reduce(&mut self, receiver: &Value, hms: &Hms, with_fold: bool) -> Value {
        let state = bytes_value(time_state_bytes(hms, with_fold).to_vec());
        let args = if hms.is_naive_tz() {
            tuple_value(vec![state])
        } else {
            tuple_value(vec![state, hms.tz.clone()])
        };
        tuple_value(vec![class_of(receiver), args])
    }

    fn dt_time_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        // Pickle support: time(b'...', tzinfo).
        if (1..=2).contains(&args.len())
            && keywords.is_empty()
            && let Some(state) = date::pickle_state_bytes(&args[0], 6)
            && (state[0] & 0x7f) < 24
        {
            let tz = args.get(1).cloned().unwrap_or(Value::None);
            if !matches!(tz, Value::None) && !kind_of(&tz).is_some_and(Kind::is_tzinfo) {
                return self.dt_error("TypeError", "bad tzinfo state arg");
            }
            let hms = Hms {
                hour: i64::from(state[0] & 0x7f),
                minute: i64::from(state[1]),
                second: i64::from(state[2]),
                us: (i64::from(state[3]) << 16) | (i64::from(state[4]) << 8) | i64::from(state[5]),
                tz,
                fold: i64::from(state[0] >> 7),
            };
            return Ok(new_instance(class, &State::Time(hms)));
        }
        let spec = ArgSpec {
            fname: None,
            params: TIME_PARAMS,
            required: 0,
            positional: 5,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, index, value| {
            if index == 4 {
                Ok(value)
            } else {
                vm.dt_c_int(&value).map(num)
            }
        })?;
        let int = |index: usize| match &values[index] {
            Some(Value::Number(value)) => *value,
            _ => 0,
        };
        let hms = Hms {
            hour: int(0),
            minute: int(1),
            second: int(2),
            us: int(3),
            tz: values[4].clone().unwrap_or(Value::None),
            fold: int(5),
        };
        check_time(hms.hour, hms.minute, hms.second, hms.us, hms.fold)?;
        self.dt_check_tzinfo(&hms.tz)?;
        Ok(new_instance(class, &State::Time(hms)))
    }

    /// A time from parsed parts: native, or `cls(h, m, s, us, tzinfo)`.
    pub(super) fn dt_time_from_parts(&mut self, class: &Value, hms: Hms) -> Result<Value, String> {
        check_time(hms.hour, hms.minute, hms.second, hms.us, hms.fold)?;
        self.dt_make_time(class, hms)
    }

    /// The tzinfo for a parsed ISO offset: `None`, `timezone.utc`, or a
    /// fixed `timezone`.
    pub(super) fn dt_iso_timezone(
        &mut self,
        anchor: &Value,
        offset: Option<Delta>,
    ) -> Result<Value, String> {
        match offset {
            None => Ok(Value::None),
            Some(offset) => {
                if !tz::offset_in_range(offset) {
                    return self.dt_error(
                        "ValueError",
                        format!(
                            "offset must be a timedelta strictly between -timedelta(hours=24) and timedelta(hours=24), not {}",
                            delta::delta_repr("datetime.timedelta", offset)
                        ),
                    );
                }
                self.dt_make_timezone(anchor, offset, None)
            }
        }
    }
}
