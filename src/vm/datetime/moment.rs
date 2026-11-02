// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `datetime.datetime`, timestamps, and local-time conversions.

use super::*;

const DATETIME_PARAMS: &[&str] = &[
    "year",
    "month",
    "day",
    "hour",
    "minute",
    "second",
    "microsecond",
    "tzinfo",
    "fold",
];

/// Seconds from 0001-01-01 to 1970-01-01.
const EPOCH_SECONDS: i64 = (date::EPOCH_ORDINAL - 1) * 86_400;

fn unix_seconds(ymd: Ymd, hms: &Hms) -> i64 {
    (ymd.ordinal() - date::EPOCH_ORDINAL) * 86_400 + hms.hour * 3600 + hms.minute * 60 + hms.second
}

pub(super) fn datetime_state_bytes(ymd: Ymd, hms: &Hms, with_fold: bool) -> [u8; 10] {
    let date = date::date_state_bytes(ymd);
    let time = timeofday::time_state_bytes(hms, false);
    let month = date[2] | if with_fold && hms.fold != 0 { 0x80 } else { 0 };
    [
        date[0], date[1], month, date[3], time[0], time[1], time[2], time[3], time[4], time[5],
    ]
}

/// The date and time of a Unix time, or C `gmtime`'s overflow error.
fn unix_to_fields(seconds: i64, us: i64) -> Result<(Ymd, Hms), String> {
    let days = seconds.div_euclid(86_400);
    let rest = seconds.rem_euclid(86_400);
    let ymd = Ymd::from_ordinal(days + date::EPOCH_ORDINAL);
    if i32::try_from(ymd.year - 1900).is_err() {
        return Err("OSError: [Errno 75] Value too large for defined data type".to_string());
    }
    Ok((
        ymd,
        Hms {
            hour: rest / 3600,
            minute: rest / 60 % 60,
            second: rest % 60,
            us,
            tz: Value::None,
            fold: 0,
        },
    ))
}

/// Rounds to the nearest integer, ties to even.
fn round_half_even(value: f64) -> f64 {
    let rounded = value.round();
    if (value - rounded).abs() == 0.5 {
        2.0 * (value / 2.0).round()
    } else {
        rounded
    }
}

impl Vm {
    pub(super) fn dt_datetime_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "__new__" => {
                let class = self.dt_class_arg(&mut args, Kind::DateTime, "__new__")?;
                return self.dt_datetime_new(&class, args, keywords);
            }
            "now" | "utcnow" | "fromtimestamp" | "utcfromtimestamp" | "strptime" | "combine"
            | "fromisoformat" => {
                let class = self.dt_class_arg(&mut args, Kind::DateTime, method)?;
                return self.dt_datetime_classmethod(&class, method, args, keywords);
            }
            _ => {}
        }
        let receiver = self.dt_receiver(&mut args, Kind::DateTime, method, &[Kind::DateTime])?;
        let State::DateTime(ymd, hms) = self.dt_state(&receiver)? else {
            return self.dt_error("TypeError", "datetime is not initialized");
        };
        let noargs = |name: &str| format!("datetime.{name}");
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
                    format::repr_state(&receiver, &State::DateTime(ymd, hms), |_| tz_repr.clone())
                        .expect("datetime repr"),
                ))
            }
            "__str__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let isoformat = self.load_attribute_value(receiver.clone(), "isoformat")?;
                self.call_value(isoformat, vec![Value::String(" ".to_string())])
            }
            "isoformat" => {
                let spec = ArgSpec {
                    fname: Some("isoformat"),
                    params: &["sep", "timespec"],
                    required: 0,
                    positional: 2,
                };
                let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, index, value| {
                    if index == 0 {
                        let text = match &value {
                            Value::String(text) | Value::IdentityString { value: text, .. } => {
                                Some(text.clone())
                            }
                            other => str_subclass_string(other),
                        };
                        return match text {
                            Some(text) if text.chars().count() == 1 => Ok(Value::String(text)),
                            Some(text) => Err(format!(
                                "TypeError: isoformat() argument 1 must be a unicode character, not a string of length {}",
                                text.chars().count()
                            )),
                            None => Err(format!(
                                "TypeError: isoformat() argument 1 must be a unicode character, not {}",
                                arg_type_name(&value)
                            )),
                        };
                    }
                    vm.dt_str_arg(&value, "isoformat", 2).map(Value::String)
                })?;
                let sep = match &values[0] {
                    Some(Value::String(text)) => text.clone(),
                    _ => "T".to_string(),
                };
                let timespec = match &values[1] {
                    Some(Value::String(text)) => text.clone(),
                    _ => "auto".to_string(),
                };
                let time = format::time_isoformat(&hms, &timespec)?;
                let mut text = format!("{}{sep}{time}", format::date_isoformat(ymd));
                if let Some(offset) = self.dt_call_offset(&hms.tz, "utcoffset", &receiver)? {
                    text.push_str(&format::format_offset(offset, ":"));
                }
                Ok(Value::String(text))
            }
            "utcoffset" | "dst" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                match self.dt_call_offset(&hms.tz, method, &receiver)? {
                    Some(offset) => self.dt_new_delta(&receiver, offset),
                    None => Ok(Value::None),
                }
            }
            "tzname" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                self.dt_call_tzname(&hms.tz, &receiver)
            }
            "date" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                let registry = self.dt_registry(&receiver)?;
                Ok(new_instance(&registry.date, &State::Date(ymd)))
            }
            "time" | "timetz" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                let registry = self.dt_registry(&receiver)?;
                let mut time = hms.clone();
                if method == "time" {
                    time.tz = Value::None;
                }
                Ok(new_instance(&registry.time, &State::Time(time)))
            }
            "ctime" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(Value::String(format::ctime(ymd, &hms)))
            }
            "timetuple" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                let dst = match self.dt_call_offset(&hms.tz, "dst", &receiver)? {
                    None => -1,
                    Some(dst) => i64::from(dst != Delta::ZERO),
                };
                let registry = self.dt_registry(&receiver)?;
                Ok(seq::new_struct_time(&registry.struct_time, ymd, &hms, dst))
            }
            "utctimetuple" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                let registry = self.dt_registry(&receiver)?;
                let (ymd, hms) = match self.dt_call_offset(&hms.tz, "utcoffset", &receiver)? {
                    None => (ymd, hms),
                    Some(offset) => {
                        let shifted =
                            self.dt_datetime_add_us(&receiver, ymd, hms, -offset.total_us())?;
                        match state_of(&shifted) {
                            Some(State::DateTime(ymd, hms)) => (ymd, hms),
                            _ => return self.dt_error("TypeError", "bad datetime"),
                        }
                    }
                };
                Ok(seq::new_struct_time(&registry.struct_time, ymd, &hms, 0))
            }
            "timestamp" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                if hms.is_naive_tz() {
                    let seconds = self.dt_local_to_unix(ymd, &hms, hms.fold)?;
                    return Ok(float_value(seconds as f64 + hms.us as f64 / 1e6));
                }
                let Some(offset) = self.dt_call_offset(&hms.tz, "utcoffset", &receiver)? else {
                    return self.dt_error(
                        "TypeError",
                        "can't subtract offset-naive and offset-aware datetimes",
                    );
                };
                let total = (i128::from(ymd.ordinal() - date::EPOCH_ORDINAL)) * US_PER_DAY
                    + hms.day_us()
                    - offset.total_us();
                Ok(float_value(delta::int_true_divide(total, 1_000_000)))
            }
            "replace" | "__replace__" => {
                let spec = ArgSpec {
                    fname: Some("replace"),
                    params: DATETIME_PARAMS,
                    required: 0,
                    positional: 8,
                };
                let values =
                    self.dt_parse_args(&spec, args, keywords, &mut |vm, index, value| {
                        if index == 7 {
                            Ok(value)
                        } else {
                            vm.dt_c_int(&value).map(num)
                        }
                    })?;
                let pick = |index: usize, current: i64| match &values[index] {
                    Some(Value::Number(value)) => *value,
                    _ => current,
                };
                let replaced_date = Ymd {
                    year: pick(0, ymd.year),
                    month: pick(1, ymd.month),
                    day: pick(2, ymd.day),
                };
                let replaced = Hms {
                    hour: pick(3, hms.hour),
                    minute: pick(4, hms.minute),
                    second: pick(5, hms.second),
                    us: pick(6, hms.us),
                    tz: values[7].clone().unwrap_or_else(|| hms.tz.clone()),
                    fold: pick(8, hms.fold),
                };
                check_date(replaced_date.year, replaced_date.month, replaced_date.day)?;
                check_time(
                    replaced.hour,
                    replaced.minute,
                    replaced.second,
                    replaced.us,
                    replaced.fold,
                )?;
                self.dt_check_tzinfo(&replaced.tz)?;
                self.dt_make_datetime(&class_of(&receiver), replaced_date, replaced)
            }
            "astimezone" => {
                let spec = ArgSpec {
                    fname: Some("astimezone"),
                    params: &["tz"],
                    required: 0,
                    positional: 1,
                };
                let values =
                    self.dt_parse_args(&spec, args, keywords, &mut |_, _, value| Ok(value))?;
                let target = values[0].clone().unwrap_or(Value::None);
                self.dt_check_tzinfo(&target)?;
                self.dt_astimezone(&receiver, ymd, hms, target)
            }
            "__hash__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                let probe = if hms.fold != 0 {
                    let mut unfolded = hms.clone();
                    unfolded.fold = 0;
                    new_instance(&class_of(&receiver), &State::DateTime(ymd, unfolded))
                } else {
                    receiver.clone()
                };
                let total = i128::from(ymd.ordinal()) * US_PER_DAY + hms.day_us();
                let hash = match self.dt_call_offset(&hms.tz, "utcoffset", &probe)? {
                    None => delta::hash_parts(&[4, total]),
                    Some(offset) => delta::hash_parts(&[5, total - offset.total_us()]),
                };
                Ok(num(hash))
            }
            "__reduce_ex__" => {
                self.dt_exact_args("__reduce_ex__", 1, &args, &keywords)?;
                let protocol = self.dt_c_int(&args[0])?;
                Ok(self.dt_datetime_reduce(&receiver, ymd, &hms, protocol > 3))
            }
            "__reduce__" => {
                self.dt_no_args(&noargs(method), &args, &keywords)?;
                Ok(self.dt_datetime_reduce(&receiver, ymd, &hms, false))
            }
            "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                self.dt_datetime_compare(method, &receiver, ymd, hms, &other)
            }
            "__add__" | "__radd__" | "__sub__" | "__rsub__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                self.dt_datetime_binary(method, &receiver, ymd, hms, &other)
            }
            _ => self.dt_error(
                "AttributeError",
                format!("'datetime.datetime' object has no attribute '{method}'"),
            ),
        }
    }

    fn dt_datetime_reduce(
        &mut self,
        receiver: &Value,
        ymd: Ymd,
        hms: &Hms,
        with_fold: bool,
    ) -> Value {
        let state = bytes_value(datetime_state_bytes(ymd, hms, with_fold).to_vec());
        let args = if hms.is_naive_tz() {
            tuple_value(vec![state])
        } else {
            tuple_value(vec![state, hms.tz.clone()])
        };
        tuple_value(vec![class_of(receiver), args])
    }

    fn dt_datetime_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        // Pickle support: datetime(b'...', tzinfo).
        if (1..=2).contains(&args.len())
            && keywords.is_empty()
            && let Some(state) = date::pickle_state_bytes(&args[0], 10)
            && (1..=12).contains(&(state[2] & 0x7f))
        {
            let tz = args.get(1).cloned().unwrap_or(Value::None);
            if !matches!(tz, Value::None) && !kind_of(&tz).is_some_and(Kind::is_tzinfo) {
                return self.dt_error("TypeError", "bad tzinfo state arg");
            }
            let ymd = Ymd {
                year: i64::from(state[0]) * 256 + i64::from(state[1]),
                month: i64::from(state[2] & 0x7f),
                day: i64::from(state[3]),
            };
            let hms = Hms {
                hour: i64::from(state[4]),
                minute: i64::from(state[5]),
                second: i64::from(state[6]),
                us: (i64::from(state[7]) << 16) | (i64::from(state[8]) << 8) | i64::from(state[9]),
                tz,
                fold: i64::from(state[2] >> 7),
            };
            return Ok(new_instance(class, &State::DateTime(ymd, hms)));
        }
        let spec = ArgSpec {
            fname: None,
            params: DATETIME_PARAMS,
            required: 3,
            positional: 8,
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |vm, index, value| {
            if index == 7 {
                Ok(value)
            } else {
                vm.dt_c_int(&value).map(num)
            }
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
        let hms = Hms {
            hour: int(3),
            minute: int(4),
            second: int(5),
            us: int(6),
            tz: values[7].clone().unwrap_or(Value::None),
            fold: int(8),
        };
        check_date(ymd.year, ymd.month, ymd.day)?;
        check_time(hms.hour, hms.minute, hms.second, hms.us, hms.fold)?;
        self.dt_check_tzinfo(&hms.tz)?;
        Ok(new_instance(class, &State::DateTime(ymd, hms)))
    }

    /// `cls(year, month, day)` for subclasses, midnight for `datetime`.
    pub(super) fn dt_datetime_from_ymd_only(
        &mut self,
        class: &Value,
        ymd: Ymd,
    ) -> Result<Value, String> {
        if class_is_native(class) {
            check_date(ymd.year, ymd.month, ymd.day)?;
            return Ok(new_instance(class, &State::DateTime(ymd, Hms::midnight())));
        }
        self.call_value(
            class.clone(),
            vec![num(ymd.year), num(ymd.month), num(ymd.day)],
        )
    }

    /// `dt + timedelta(microseconds=us)` preserving the datetime's class;
    /// the result has `fold=0`.
    pub(super) fn dt_datetime_add_us(
        &mut self,
        dt: &Value,
        ymd: Ymd,
        hms: Hms,
        us: i128,
    ) -> Result<Value, String> {
        let total = i128::from(ymd.ordinal()) * US_PER_DAY + hms.day_us() + us;
        let (ordinal, day_us) = split_day_us(total);
        if !(1..=i128::from(MAX_ORDINAL)).contains(&ordinal) {
            return self.dt_error("OverflowError", "date value out of range");
        }
        let mut shifted = hms.with_day_us(day_us);
        shifted.fold = 0;
        self.dt_make_datetime(&class_of(dt), Ymd::from_ordinal(ordinal as i64), shifted)
    }

    fn dt_datetime_binary(
        &mut self,
        method: &str,
        receiver: &Value,
        ymd: Ymd,
        hms: Hms,
        other: &Value,
    ) -> Result<Value, String> {
        match (method, kind_of(other), state_of(other)) {
            ("__add__" | "__radd__", Some(Kind::Delta), Some(State::Delta(delta))) => {
                self.dt_datetime_add_us(receiver, ymd, hms, delta.total_us())
            }
            ("__sub__", Some(Kind::Delta), Some(State::Delta(delta))) => {
                self.dt_datetime_add_us(receiver, ymd, hms, -delta.total_us())
            }
            ("__sub__", Some(Kind::DateTime), Some(State::DateTime(right_ymd, right_hms))) => {
                self.dt_datetime_difference(receiver, ymd, &hms, other, right_ymd, &right_hms)
            }
            ("__rsub__", Some(Kind::DateTime), Some(State::DateTime(left_ymd, left_hms))) => {
                self.dt_datetime_difference(other, left_ymd, &left_hms, receiver, ymd, &hms)
            }
            _ => self.dt_binary_unsupported(method, receiver, other),
        }
    }

    fn dt_datetime_difference(
        &mut self,
        left: &Value,
        left_ymd: Ymd,
        left_hms: &Hms,
        right: &Value,
        right_ymd: Ymd,
        right_hms: &Hms,
    ) -> Result<Value, String> {
        let mut offset_difference = 0;
        if !same_object(&left_hms.tz, &right_hms.tz) {
            let left_offset = self.dt_call_offset(&left_hms.tz, "utcoffset", left)?;
            let right_offset = self.dt_call_offset(&right_hms.tz, "utcoffset", right)?;
            match (left_offset, right_offset) {
                (None, None) => {}
                (Some(a), Some(b)) => offset_difference = a.total_us() - b.total_us(),
                _ => {
                    return self.dt_error(
                        "TypeError",
                        "can't subtract offset-naive and offset-aware datetimes",
                    );
                }
            }
        }
        let total = (i128::from(left_ymd.ordinal()) - i128::from(right_ymd.ordinal())) * US_PER_DAY
            + left_hms.day_us()
            - right_hms.day_us()
            - offset_difference;
        self.dt_delta_from_us(left, total)
    }

    fn dt_datetime_compare(
        &mut self,
        method: &str,
        receiver: &Value,
        ymd: Ymd,
        hms: Hms,
        other: &Value,
    ) -> Result<Value, String> {
        let Some(State::DateTime(right_ymd, right_hms)) = (kind_of(other) == Some(Kind::DateTime))
            .then(|| state_of(other))
            .flatten()
        else {
            return self.dt_compare_unsupported(method, receiver, other);
        };
        let left_total = i128::from(ymd.ordinal()) * US_PER_DAY + hms.day_us();
        let right_total = i128::from(right_ymd.ordinal()) * US_PER_DAY + right_hms.day_us();
        if same_object(&hms.tz, &right_hms.tz) {
            return Ok(Value::Bool(delta::compare_op(
                method,
                left_total.cmp(&right_total),
            )));
        }
        let left_offset = self.dt_call_offset(&hms.tz, "utcoffset", receiver)?;
        let right_offset = self.dt_call_offset(&right_hms.tz, "utcoffset", other)?;
        let equality = matches!(method, "__eq__" | "__ne__");
        let ordering = match (left_offset, right_offset) {
            (None, None) => left_total.cmp(&right_total),
            (Some(a), Some(b)) => (left_total - a.total_us()).cmp(&(right_total - b.total_us())),
            _ => {
                if equality {
                    return Ok(Value::Bool(method == "__ne__"));
                }
                return self.dt_error(
                    "TypeError",
                    "can't compare offset-naive and offset-aware datetimes",
                );
            }
        };
        if equality
            && ordering == std::cmp::Ordering::Equal
            && left_offset.is_some()
            && (self.dt_fold_changes_offset(receiver, ymd, &hms, left_offset)?
                || self.dt_fold_changes_offset(other, right_ymd, &right_hms, right_offset)?)
        {
            // PEP 495: an inter-zone comparison involving an ambiguous or
            // missing time is never equal.
            return Ok(Value::Bool(method == "__ne__"));
        }
        Ok(Value::Bool(delta::compare_op(method, ordering)))
    }

    /// Whether flipping `fold` changes the UTC offset (PEP 495).
    fn dt_fold_changes_offset(
        &mut self,
        value: &Value,
        ymd: Ymd,
        hms: &Hms,
        offset: Option<Delta>,
    ) -> Result<bool, String> {
        if instance_is_native(&hms.tz) || matches!(hms.tz, Value::None) {
            return Ok(false);
        }
        let mut flipped = hms.clone();
        flipped.fold = 1 - hms.fold;
        let flipped_value = new_instance(&class_of(value), &State::DateTime(ymd, flipped));
        let flipped_offset = self.dt_call_offset(&hms.tz, "utcoffset", &flipped_value)?;
        Ok(flipped_offset != offset)
    }

    fn dt_astimezone(
        &mut self,
        receiver: &Value,
        ymd: Ymd,
        hms: Hms,
        target: Value,
    ) -> Result<Value, String> {
        let mut source_tz = if hms.is_naive_tz() {
            self.dt_local_timezone_from_local(receiver, ymd, &hms)?
        } else {
            hms.tz.clone()
        };
        if same_object(&source_tz, &target) {
            return Ok(receiver.clone());
        }
        let mut offset = self.dt_call_offset(&source_tz, "utcoffset", receiver)?;
        if offset.is_none() {
            source_tz = self.dt_local_timezone_from_local(receiver, ymd, &hms)?;
            if same_object(&source_tz, &target) {
                return Ok(receiver.clone());
            }
            offset = self.dt_call_offset(&source_tz, "utcoffset", receiver)?;
        }
        let offset = offset.unwrap_or(Delta::ZERO);
        let utc = self.dt_datetime_add_us(receiver, ymd, hms, -offset.total_us())?;
        let Some(State::DateTime(utc_ymd, mut utc_hms)) = state_of(&utc) else {
            return self.dt_error("TypeError", "astimezone() produced a non-datetime");
        };
        let target = if matches!(target, Value::None) {
            let seconds = unix_seconds(utc_ymd, &utc_hms);
            self.dt_local_timezone(receiver, seconds)?
        } else {
            target
        };
        // Attach the target zone to the UTC value and let fromutc() finish.
        utc_hms.tz = target.clone();
        set_state(&utc, &State::DateTime(utc_ymd, utc_hms));
        let fromutc = self.load_attribute_value(target, "fromutc")?;
        self.call_value(fromutc, vec![utc])
    }

    /// The fixed-offset local zone in effect at a Unix time.
    fn dt_local_timezone(&mut self, anchor: &Value, seconds: i64) -> Result<Value, String> {
        let offset = self.dt_local_offset(seconds);
        self.dt_make_timezone(
            anchor,
            Delta::from_us(i128::from(offset) * US_PER_SECOND)?,
            Some(host_clock::LOCAL_TIMEZONE_NAME.to_string()),
        )
    }

    /// The local zone for a naive datetime read as local time (both folds
    /// are resolved, as CPython does).
    fn dt_local_timezone_from_local(
        &mut self,
        anchor: &Value,
        ymd: Ymd,
        hms: &Hms,
    ) -> Result<Value, String> {
        let seconds = self.dt_local_to_unix(ymd, hms, hms.fold)?;
        let seconds2 = self.dt_local_to_unix(ymd, hms, 1 - hms.fold)?;
        let seconds = if seconds2 != seconds {
            seconds2
        } else {
            seconds
        };
        self.dt_local_timezone(anchor, seconds)
    }

    /// Checks that local time at a Unix time has a representable year.
    fn dt_local_probe(&mut self, seconds: i64) -> Result<(), String> {
        let local = seconds + self.dt_local_offset(seconds);
        let (ymd, _) = unix_to_fields(local, 0)?;
        if !(MINYEAR..=MAXYEAR).contains(&ymd.year) {
            return self.dt_error(
                "ValueError",
                format!("year must be in {MINYEAR}..{MAXYEAR}, not {}", ymd.year),
            );
        }
        Ok(())
    }

    /// The Unix time of a naive local datetime, probing the local offset a
    /// day either side to find gaps and folds.
    fn dt_local_to_unix(&mut self, ymd: Ymd, hms: &Hms, fold: i64) -> Result<i64, String> {
        let t = unix_seconds(ymd, hms);
        self.dt_local_probe(t)?;
        let u1 = t - self.dt_local_offset(t);
        self.dt_local_probe(u1)?;
        let u2 = if fold != 0 { u1 + 86_400 } else { u1 - 86_400 };
        self.dt_local_probe(u2)?;
        let _ = EPOCH_SECONDS;
        Ok(u1)
    }

    /// Splits a timestamp into whole seconds and microseconds, rounding
    /// the microseconds half to even.
    fn dt_timestamp_timeval(&mut self, value: &Value) -> Result<(i64, i64), String> {
        if let Some(number) = delta::float_operand(value) {
            if number.is_nan() {
                return self.dt_error("ValueError", "Invalid value NaN (not a number)");
            }
            let int_part = number.trunc();
            let mut fraction = round_half_even((number - int_part) * 1e6);
            let mut int_part = int_part;
            if fraction >= 1e6 {
                fraction -= 1e6;
                int_part += 1.0;
            } else if fraction < 0.0 {
                fraction += 1e6;
                int_part -= 1.0;
            }
            if !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&int_part) {
                return self.dt_error(
                    "OverflowError",
                    "timestamp out of range for platform time_t",
                );
            }
            return Ok((int_part as i64, fraction as i64));
        }
        match delta::int_operand(value) {
            Some(integer) => match integer.to_i64() {
                Some(seconds) => Ok((seconds, 0)),
                None => self.dt_error(
                    "OverflowError",
                    "timestamp out of range for platform time_t",
                ),
            },
            None => self.dt_error(
                "TypeError",
                format!("argument must be int or float, not {}", tp_name(value)),
            ),
        }
    }

    /// A timestamp as whole seconds, rounded down (`date.fromtimestamp`).
    pub(super) fn dt_timestamp_seconds_floor(&mut self, value: &Value) -> Result<i64, String> {
        if let Some(number) = delta::float_operand(value) {
            if number.is_nan() {
                return self.dt_error("ValueError", "Invalid value NaN (not a number)");
            }
            let floored = number.floor();
            if !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&floored) {
                return self.dt_error(
                    "OverflowError",
                    "timestamp out of range for platform time_t",
                );
            }
            return Ok(floored as i64);
        }
        match self.dt_index_big(value)? {
            Some(integer) => match integer.to_i64() {
                Some(seconds) => Ok(seconds),
                None => self.dt_error(
                    "OverflowError",
                    "timestamp out of range for platform time_t",
                ),
            },
            None => self.dt_error(
                "TypeError",
                format!(
                    "'{}' object cannot be interpreted as an integer",
                    tp_name(value)
                ),
            ),
        }
    }

    /// `datetime_from_timet_and_us`: a datetime from a Unix time in local
    /// time (`utc == false`) or UTC, attaching `tz` before `fromutc`.
    fn dt_datetime_from_unix(
        &mut self,
        class: &Value,
        seconds: i64,
        us: i64,
        utc: bool,
        tz: Value,
    ) -> Result<Value, String> {
        let local = if utc {
            seconds
        } else {
            seconds + self.dt_local_offset(seconds)
        };
        let (ymd, mut hms) = unix_to_fields(local, us)?;
        if !utc {
            // CPython probes a day earlier to detect a fold.
            self.dt_local_probe(seconds - 86_400)?;
        }
        hms.tz = tz.clone();
        check_date(ymd.year, ymd.month, ymd.day)?;
        let result = self.dt_make_datetime(class, ymd, hms)?;
        if matches!(tz, Value::None) {
            return Ok(result);
        }
        let fromutc = self.load_attribute_value(tz, "fromutc")?;
        self.call_value(fromutc, vec![result])
    }

    fn dt_datetime_classmethod(
        &mut self,
        class: &Value,
        method: &str,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        match method {
            "now" => {
                let spec = ArgSpec {
                    fname: Some("now"),
                    params: &["tz"],
                    required: 0,
                    positional: 1,
                };
                let values =
                    self.dt_parse_args(&spec, args, keywords, &mut |_, _, value| Ok(value))?;
                let tz = values[0].clone().unwrap_or(Value::None);
                self.dt_check_tzinfo(&tz)?;
                let micros = self.dt_now_micros();
                let utc = !matches!(tz, Value::None);
                self.dt_datetime_from_unix(
                    class,
                    micros.div_euclid(1_000_000),
                    micros.rem_euclid(1_000_000),
                    utc,
                    tz,
                )
            }
            "utcnow" => {
                self.dt_deprecation(
                    "datetime.datetime.utcnow() is deprecated and scheduled for removal in a future version. Use timezone-aware objects to represent datetimes in UTC: datetime.datetime.now(datetime.UTC).",
                )?;
                self.dt_no_args("datetime.utcnow", &args, &keywords)?;
                let micros = self.dt_now_micros();
                self.dt_datetime_from_unix(
                    class,
                    micros.div_euclid(1_000_000),
                    micros.rem_euclid(1_000_000),
                    true,
                    Value::None,
                )
            }
            "fromtimestamp" => {
                let spec = ArgSpec {
                    fname: Some("fromtimestamp"),
                    params: &["timestamp", "tz"],
                    required: 1,
                    positional: 2,
                };
                let values =
                    self.dt_parse_args(&spec, args, keywords, &mut |_, _, value| Ok(value))?;
                let timestamp = values[0].clone().expect("timestamp is required");
                let tz = values[1].clone().unwrap_or(Value::None);
                self.dt_check_tzinfo(&tz)?;
                let (seconds, us) = self.dt_timestamp_timeval(&timestamp)?;
                let utc = !matches!(tz, Value::None);
                self.dt_datetime_from_unix(class, seconds, us, utc, tz)
            }
            "utcfromtimestamp" => {
                self.dt_deprecation(
                    "datetime.datetime.utcfromtimestamp() is deprecated and scheduled for removal in a future version. Use timezone-aware objects to represent datetimes in UTC: datetime.datetime.fromtimestamp(timestamp, datetime.UTC).",
                )?;
                self.dt_exact_args("utcfromtimestamp", 1, &args, &keywords)?;
                let (seconds, us) = self.dt_timestamp_timeval(&args[0])?;
                self.dt_datetime_from_unix(class, seconds, us, true, Value::None)
            }
            "combine" => {
                let spec = ArgSpec {
                    fname: Some("combine"),
                    params: &["date", "time", "tzinfo"],
                    required: 2,
                    positional: 3,
                };
                let values = self.dt_parse_args(
                    &spec,
                    args,
                    keywords,
                    &mut |_, index, value| match index {
                        0 if !kind_of(&value).is_some_and(Kind::is_date) => Err(format!(
                            "TypeError: combine() argument 1 must be datetime.date, not {}",
                            arg_type_name(&value)
                        )),
                        1 if kind_of(&value) != Some(Kind::Time) => Err(format!(
                            "TypeError: combine() argument 2 must be datetime.time, not {}",
                            arg_type_name(&value)
                        )),
                        _ => Ok(value),
                    },
                )?;
                let date_value = values[0].clone().expect("date is required");
                let time_value = values[1].clone().expect("time is required");
                let Some(ymd) = state_of(&date_value).as_ref().and_then(date::date_part) else {
                    return self
                        .dt_error("TypeError", "combine() argument 1 must be datetime.date");
                };
                let Some(State::Time(mut hms)) = state_of(&time_value) else {
                    return self
                        .dt_error("TypeError", "combine() argument 2 must be datetime.time");
                };
                if let Some(tz) = values[2].clone() {
                    self.dt_check_tzinfo(&tz)?;
                    hms.tz = tz;
                }
                self.dt_make_datetime(class, ymd, hms)
            }
            "fromisoformat" => {
                let text = self.dt_one_arg("datetime.fromisoformat", args, &keywords)?;
                let text = self.dt_fromisoformat_text(&text)?;
                self.dt_datetime_fromisoformat(class, &text)
            }
            "strptime" => {
                self.dt_exact_args("strptime", 2, &args, &keywords)?;
                let text = self.dt_str_arg(&args[0], "strptime", 1)?;
                let format = self.dt_str_arg(&args[1], "strptime", 2)?;
                self.dt_strptime(class, Kind::DateTime, &text, &format)
            }
            _ => self.dt_error(
                "AttributeError",
                format!("type object 'datetime.datetime' has no attribute '{method}'"),
            ),
        }
    }

    pub(super) fn dt_datetime_fromisoformat(
        &mut self,
        class: &Value,
        text: &str,
    ) -> Result<Value, String> {
        let parsed = iso::parse_datetime(text)?;
        let mut ymd = parsed.date;
        let mut hms = parsed.time.hms;
        check_date(ymd.year, ymd.month, ymd.day)?;
        if parsed.time.hour_24 {
            hms.hour = 0;
            let next = ymd.ordinal() + 1;
            ymd = Ymd::from_ordinal(next);
            check_date(ymd.year, ymd.month, ymd.day)?;
        }
        check_time(hms.hour, hms.minute, hms.second, hms.us, 0)?;
        hms.tz = self.dt_iso_timezone(class, parsed.time.offset)?;
        self.dt_make_datetime(class, ymd, hms)
    }

    /// `datetime.today()` and `datetime.now()` for the local zone.
    pub(super) fn dt_datetime_today(&mut self, class: &Value) -> Result<Value, String> {
        let micros = self.dt_now_micros();
        self.dt_datetime_from_unix(
            class,
            micros.div_euclid(1_000_000),
            micros.rem_euclid(1_000_000),
            false,
            Value::None,
        )
    }
}
