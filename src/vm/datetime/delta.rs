// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `datetime.timedelta`.

use super::*;

const DELTA_PARAMS: &[&str] = &[
    "days",
    "seconds",
    "microseconds",
    "milliseconds",
    "minutes",
    "hours",
    "weeks",
];

/// Microseconds per unit of each constructor parameter, in CPython's
/// accumulation order (microseconds first, weeks last).
const ACCUMULATION_ORDER: &[(usize, i64)] = &[
    (2, 1),
    (3, 1_000),
    (1, 1_000_000),
    (4, 60_000_000),
    (5, 3_600_000_000),
    (0, 86_400_000_000),
    (6, 604_800_000_000),
];

/// `float.as_integer_ratio()` with CPython's errors.
pub(super) fn float_ratio(value: f64) -> Result<(BigInt, BigInt), String> {
    if value.is_nan() {
        return Err("ValueError: cannot convert NaN to integer ratio".to_string());
    }
    if value.is_infinite() {
        return Err("OverflowError: cannot convert Infinity to integer ratio".to_string());
    }
    if value == 0.0 {
        return Ok((BigInt::from(0), BigInt::from(1)));
    }
    let bits = value.to_bits();
    let negative = bits >> 63 == 1;
    let exponent = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & ((1u64 << 52) - 1);
    let (mut mantissa, mut exponent) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), exponent - 1075)
    };
    while mantissa & 1 == 0 && exponent < 0 {
        mantissa >>= 1;
        exponent += 1;
    }
    let mut numerator = BigInt::from(mantissa);
    let mut denominator = BigInt::from(1);
    if exponent >= 0 {
        numerator <<= exponent as usize;
    } else {
        denominator <<= (-exponent) as usize;
    }
    if negative {
        numerator = -numerator;
    }
    Ok((numerator, denominator))
}

/// `a / b` as the correctly rounded float (Python's `int / int`).
pub(super) fn int_true_divide(numerator: i128, denominator: i128) -> f64 {
    if numerator == 0 {
        return if denominator < 0 { -0.0 } else { 0.0 };
    }
    let negative = (numerator < 0) != (denominator < 0);
    let a = BigInt::from(numerator.unsigned_abs());
    let b = BigInt::from(denominator.unsigned_abs());
    // Scale so the integer quotient has at least 55 significant bits.
    let shift = 55 + b.bits() as i64 - a.bits() as i64;
    let (scaled_a, scaled_b) = if shift >= 0 {
        (a << shift as usize, b)
    } else {
        (a, b << (-shift) as usize)
    };
    let quotient = &scaled_a / &scaled_b;
    let sticky = (&scaled_a % &scaled_b).sign() != num_bigint::Sign::NoSign;
    // Keep 53 bits and round half to even using the dropped bits.
    let extra = quotient.bits() as i64 - 53;
    let mut mantissa = &quotient >> extra as usize;
    let dropped = &quotient - (&mantissa << extra as usize);
    let half = BigInt::from(1) << (extra - 1) as usize;
    if dropped > half || (dropped == half && (sticky || mantissa.is_odd())) {
        mantissa += 1;
    }
    let mantissa = mantissa.to_f64().unwrap_or(f64::INFINITY);
    let value = mantissa * 2f64.powi((extra - shift) as i32);
    if negative { -value } else { value }
}

/// `a / b` rounded half to even, for `b != 0`.
pub(super) fn divide_nearest_big(a: &BigInt, b: &BigInt) -> BigInt {
    let (a, b) = if b.sign() == num_bigint::Sign::Minus {
        (-a.clone(), -b.clone())
    } else {
        (a.clone(), b.clone())
    };
    let quotient = a.div_floor(&b);
    let remainder = a.mod_floor(&b);
    let twice = &remainder * 2;
    if twice > b || (twice == b && quotient.is_odd()) {
        quotient + 1
    } else {
        quotient
    }
}

pub(super) fn big_to_i128(value: &BigInt) -> Option<i128> {
    value.to_i128()
}

/// `Delta` from a microsecond count of any size.
pub(super) fn delta_from_big(total: &BigInt) -> Result<Delta, String> {
    match big_to_i128(total) {
        Some(total) => Delta::from_us(total),
        None => Err("OverflowError: Python int too large to convert to C int".to_string()),
    }
}

/// C `round()`: halfway cases away from zero.
fn c_round(value: f64) -> f64 {
    value.round()
}

pub(super) fn delta_repr(name: &str, delta: Delta) -> String {
    let mut parts = Vec::new();
    if delta.days != 0 {
        parts.push(format!("days={}", delta.days));
    }
    if delta.seconds != 0 {
        parts.push(format!("seconds={}", delta.seconds));
    }
    if delta.us != 0 {
        parts.push(format!("microseconds={}", delta.us));
    }
    if parts.is_empty() {
        format!("{name}(0)")
    } else {
        format!("{name}({})", parts.join(", "))
    }
}

pub(super) fn delta_str(delta: Delta) -> String {
    let mut text = String::new();
    if delta.days != 0 {
        let suffix = if delta.days.abs() != 1 { "s" } else { "" };
        text.push_str(&format!("{} day{suffix}, ", delta.days));
    }
    let hours = delta.seconds / 3600;
    let minutes = delta.seconds / 60 % 60;
    let seconds = delta.seconds % 60;
    text.push_str(&format!("{hours}:{minutes:02}:{seconds:02}"));
    if delta.us != 0 {
        text.push_str(&format!(".{:06}", delta.us));
    }
    text
}

/// Hash of the integer parts that define a value's equality: callers pass a
/// kind tag and the normalized fields (UTC instants for aware values), so
/// equal values hash alike. Each part's 16 little-endian bytes go through
/// 64-bit FNV-1a, then a xor-shift-multiply finalizer spreads the bits. The
/// result is non-negative, so it is never -1.
pub(super) fn hash_parts(parts: &[i128]) -> i64 {
    const FNV_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for part in parts {
        for byte in part.to_le_bytes() {
            hash = (hash ^ u64::from(byte)).wrapping_mul(FNV_PRIME);
        }
    }
    hash ^= hash >> 33;
    hash = hash.wrapping_mul(0xff51_afd7_ed55_8ccd);
    hash ^= hash >> 33;
    (hash >> 1) as i64
}

impl Vm {
    pub(super) fn dt_delta_method(
        &mut self,
        method: &str,
        mut args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        if method == "__new__" {
            let class = self.dt_class_arg(&mut args, Kind::Delta, "__new__")?;
            return self.dt_delta_new(&class, args, keywords);
        }
        let receiver = self.dt_receiver(&mut args, Kind::Delta, method, &[Kind::Delta])?;
        let State::Delta(delta) = self.dt_state(&receiver)? else {
            return self.dt_error("TypeError", "timedelta is not initialized");
        };
        match method {
            "days" => Ok(num(delta.days)),
            "seconds" => Ok(num(delta.seconds)),
            "microseconds" => Ok(num(delta.us)),
            "__repr__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(Value::String(delta_repr(&repr_type_name(&receiver), delta)))
            }
            "__str__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(Value::String(delta_str(delta)))
            }
            "__hash__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(num(hash_parts(&[
                    i128::from(delta.days),
                    i128::from(delta.seconds),
                    i128::from(delta.us),
                ])))
            }
            "__bool__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                Ok(Value::Bool(delta != Delta::ZERO))
            }
            "__neg__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                self.dt_delta_from_us(&receiver, -delta.total_us())
            }
            "__pos__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                self.dt_new_delta(&receiver, delta)
            }
            "__abs__" => {
                self.dt_slot_args(method, 0, args, &keywords)?;
                self.dt_delta_from_us(&receiver, delta.total_us().abs())
            }
            "total_seconds" => {
                self.dt_no_args("timedelta.total_seconds", &args, &keywords)?;
                Ok(float_value(int_true_divide(delta.total_us(), 1_000_000)))
            }
            "__reduce__" => {
                self.dt_no_args("timedelta.__reduce__", &args, &keywords)?;
                Ok(tuple_value(vec![
                    class_of(&receiver),
                    tuple_value(vec![num(delta.days), num(delta.seconds), num(delta.us)]),
                ]))
            }
            "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                match state_of(&other) {
                    Some(State::Delta(right)) if kind_of(&other) == Some(Kind::Delta) => Ok(
                        Value::Bool(compare_op(method, delta.total_us().cmp(&right.total_us()))),
                    ),
                    _ => self.dt_compare_unsupported(method, &receiver, &other),
                }
            }
            _ => {
                let [other] =
                    <[Value; 1]>::try_from(self.dt_slot_args(method, 1, args, &keywords)?)
                        .expect("one argument");
                self.dt_delta_binary(method, &receiver, delta, &other)
            }
        }
    }

    fn dt_delta_new(
        &mut self,
        class: &Value,
        args: Vec<Value>,
        keywords: Vec<(String, Value)>,
    ) -> Result<Value, String> {
        let spec = ArgSpec {
            fname: Some("__new__"),
            params: DELTA_PARAMS,
            required: 0,
            positional: DELTA_PARAMS.len(),
        };
        let values = self.dt_parse_args(&spec, args, keywords, &mut |_, _, value| Ok(value))?;
        let mut total = BigInt::from(0);
        let mut leftover = 0.0_f64;
        for (index, factor) in ACCUMULATION_ORDER {
            let Some(value) = &values[*index] else {
                continue;
            };
            self.dt_accumulate(
                DELTA_PARAMS[*index],
                value,
                *factor,
                &mut total,
                &mut leftover,
            )?;
        }
        if leftover != 0.0 {
            let mut whole = c_round(leftover);
            if (whole - leftover).abs() == 0.5 {
                // Halfway: round the whole sum half to even.
                let odd = if total.is_odd() { 1.0 } else { 0.0 };
                whole = 2.0 * c_round((leftover + odd) * 0.5) - odd;
            }
            total += BigInt::from(whole as i64);
        }
        let delta = delta_from_big(&total)?;
        Ok(new_instance(class, &State::Delta(delta)))
    }

    fn dt_accumulate(
        &mut self,
        tag: &str,
        value: &Value,
        factor: i64,
        total: &mut BigInt,
        leftover: &mut f64,
    ) -> Result<(), String> {
        let float = match value {
            Value::Float(number) => Some(**number),
            other => float_subclass_float(other).map(|number| *number),
        };
        if let Some(number) = float {
            if number.is_nan() {
                return self.dt_error("ValueError", "cannot convert float NaN to integer");
            }
            if number.is_infinite() {
                return self.dt_error("OverflowError", "cannot convert float infinity to integer");
            }
            let int_part = number.trunc();
            let frac_part = number - int_part;
            *total += float_to_big(int_part) * BigInt::from(factor);
            if frac_part == 0.0 {
                return Ok(());
            }
            let scaled = factor as f64 * frac_part;
            let scaled_int = scaled.trunc();
            *total += float_to_big(scaled_int);
            *leftover += scaled - scaled_int;
            return Ok(());
        }
        let integer = match value {
            Value::Bool(flag) => Some(BigInt::from(i64::from(*flag))),
            Value::Number(number) => Some(BigInt::from(*number)),
            Value::BigInt(number) => Some(number.clone()),
            other => match int_subclass_integer(other) {
                Some(Value::Number(number)) => Some(BigInt::from(number)),
                Some(Value::BigInt(number)) => Some(number),
                Some(Value::Bool(flag)) => Some(BigInt::from(i64::from(flag))),
                _ => None,
            },
        };
        match integer {
            Some(integer) => {
                *total += integer * BigInt::from(factor);
                Ok(())
            }
            None => self.dt_error(
                "TypeError",
                format!(
                    "unsupported type for timedelta {tag} component: {}",
                    tp_name(value)
                ),
            ),
        }
    }

    /// `__add__`, `__mul__`, ... of a timedelta receiver.
    fn dt_delta_binary(
        &mut self,
        method: &str,
        receiver: &Value,
        delta: Delta,
        other: &Value,
    ) -> Result<Value, String> {
        let other_delta = match (kind_of(other), state_of(other)) {
            (Some(Kind::Delta), Some(State::Delta(other))) => Some(other),
            _ => None,
        };
        let us = delta.total_us();
        match (method, other_delta) {
            ("__add__" | "__radd__", Some(right)) => {
                self.dt_delta_from_us(receiver, us + right.total_us())
            }
            ("__sub__", Some(right)) => self.dt_delta_from_us(receiver, us - right.total_us()),
            ("__rsub__", Some(right)) => self.dt_delta_from_us(receiver, right.total_us() - us),
            ("__truediv__", Some(right)) => {
                if right.total_us() == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                Ok(float_value(int_true_divide(us, right.total_us())))
            }
            ("__rtruediv__", Some(right)) => {
                if us == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                Ok(float_value(int_true_divide(right.total_us(), us)))
            }
            ("__floordiv__", Some(right)) => {
                let divisor = right.total_us();
                if divisor == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                Ok(Self::dt_int(floor_div(us, divisor)))
            }
            ("__rfloordiv__", Some(right)) => {
                if us == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                Ok(Self::dt_int(floor_div(right.total_us(), us)))
            }
            ("__mod__", Some(right)) => {
                let divisor = right.total_us();
                if divisor == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                self.dt_delta_from_us(receiver, floor_mod(us, divisor))
            }
            ("__rmod__", Some(right)) => {
                if us == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                self.dt_delta_from_us(receiver, floor_mod(right.total_us(), us))
            }
            ("__divmod__", Some(right)) => {
                let divisor = right.total_us();
                if divisor == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                let quotient = Self::dt_int(floor_div(us, divisor));
                let remainder = self.dt_delta_from_us(receiver, floor_mod(us, divisor))?;
                Ok(tuple_value(vec![quotient, remainder]))
            }
            ("__rdivmod__", Some(right)) => {
                if us == 0 {
                    return self.dt_error("ZeroDivisionError", "division by zero");
                }
                let quotient = Self::dt_int(floor_div(right.total_us(), us));
                let remainder = self.dt_delta_from_us(receiver, floor_mod(right.total_us(), us))?;
                Ok(tuple_value(vec![quotient, remainder]))
            }
            ("__mul__" | "__rmul__", None) => {
                if let Some(factor) = int_operand(other) {
                    let total = BigInt::from(us) * factor;
                    let delta = delta_from_big(&total)?;
                    return self.dt_new_delta(receiver, delta);
                }
                if let Some((numerator, denominator)) = self.dt_float_ratio(other)? {
                    if denominator.sign() == num_bigint::Sign::NoSign {
                        return self.dt_error("ZeroDivisionError", "division by zero");
                    }
                    let total = divide_nearest_big(&(BigInt::from(us) * numerator), &denominator);
                    let delta = delta_from_big(&total)?;
                    return self.dt_new_delta(receiver, delta);
                }
                self.dt_delta_unsupported(method, receiver, other)
            }
            ("__truediv__", None) => {
                if let Some(divisor) = int_operand(other) {
                    if divisor.sign() == num_bigint::Sign::NoSign {
                        return self.dt_error("ZeroDivisionError", "division by zero");
                    }
                    let total = divide_nearest_big(&BigInt::from(us), &divisor);
                    let delta = delta_from_big(&total)?;
                    return self.dt_new_delta(receiver, delta);
                }
                if let Some((numerator, denominator)) = self.dt_float_ratio(other)? {
                    if numerator.sign() == num_bigint::Sign::NoSign {
                        return self.dt_error("ZeroDivisionError", "division by zero");
                    }
                    let total = divide_nearest_big(&(BigInt::from(us) * denominator), &numerator);
                    let delta = delta_from_big(&total)?;
                    return self.dt_new_delta(receiver, delta);
                }
                self.dt_delta_unsupported(method, receiver, other)
            }
            ("__floordiv__", None) => {
                if let Some(divisor) = int_operand(other) {
                    if divisor.sign() == num_bigint::Sign::NoSign {
                        return self.dt_error("ZeroDivisionError", "division by zero");
                    }
                    let total = BigInt::from(us).div_floor(&divisor);
                    let delta = delta_from_big(&total)?;
                    return self.dt_new_delta(receiver, delta);
                }
                self.dt_delta_unsupported(method, receiver, other)
            }
            _ => self.dt_delta_unsupported(method, receiver, other),
        }
    }

    /// `float.as_integer_ratio()` of a float operand; subclasses are asked
    /// through the method, with CPython's checks of the result.
    fn dt_float_ratio(&mut self, value: &Value) -> Result<Option<(BigInt, BigInt)>, String> {
        match value {
            Value::Float(number) => float_ratio(**number).map(Some),
            Value::Instance { .. } if float_subclass_float(value).is_some() => {
                let method = self.load_attribute_value(value.clone(), "as_integer_ratio")?;
                let result = self.call_value(method, Vec::new())?;
                let Value::Tuple(items) = &result else {
                    return self.dt_error(
                        "TypeError",
                        format!(
                            "unexpected return type from as_integer_ratio(): expected tuple, not '{}'",
                            tp_name(&result)
                        ),
                    );
                };
                if items.len() != 2 {
                    return self.dt_error("ValueError", "as_integer_ratio() must return a 2-tuple");
                }
                match (int_operand(&items[0]), int_operand(&items[1])) {
                    (Some(numerator), Some(denominator)) => Ok(Some((numerator, denominator))),
                    _ => self.dt_error("TypeError", "non-integer arguments in division"),
                }
            }
            _ => Ok(None),
        }
    }

    fn dt_delta_unsupported(
        &mut self,
        method: &str,
        receiver: &Value,
        other: &Value,
    ) -> Result<Value, String> {
        self.dt_binary_unsupported(method, receiver, other)
    }
}

pub(super) fn floor_div(a: i128, b: i128) -> i128 {
    let quotient = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        quotient - 1
    } else {
        quotient
    }
}

pub(super) fn floor_mod(a: i128, b: i128) -> i128 {
    a - floor_div(a, b) * b
}

fn float_to_big(value: f64) -> BigInt {
    num_traits::FromPrimitive::from_f64(value).unwrap_or_default()
}

/// An `int` operand: ints, bools, and int subclasses.
pub(super) fn int_operand(value: &Value) -> Option<BigInt> {
    match value {
        Value::Bool(flag) => Some(BigInt::from(i64::from(*flag))),
        Value::Number(number) => Some(BigInt::from(*number)),
        Value::BigInt(number) => Some(number.clone()),
        other => match int_subclass_integer(other)? {
            Value::Number(number) => Some(BigInt::from(number)),
            Value::BigInt(number) => Some(number),
            Value::Bool(flag) => Some(BigInt::from(i64::from(flag))),
            _ => None,
        },
    }
}

/// A `float` operand: floats and float subclasses.
pub(super) fn float_operand(value: &Value) -> Option<f64> {
    match value {
        Value::Float(number) => Some(**number),
        other => float_subclass_float(other).map(|number| *number),
    }
}

/// The result of a rich comparison for an ordering.
pub(super) fn compare_op(method: &str, ordering: std::cmp::Ordering) -> bool {
    use std::cmp::Ordering::*;
    match method {
        "__eq__" => ordering == Equal,
        "__ne__" => ordering != Equal,
        "__lt__" => ordering == Less,
        "__le__" => ordering != Greater,
        "__gt__" => ordering == Greater,
        _ => ordering != Less,
    }
}

impl Vm {
    /// An operand type a binary slot cannot handle: `NotImplemented`, so
    /// the other operand (or the runtime's `TypeError`) decides.
    pub(super) fn dt_binary_unsupported(
        &mut self,
        _method: &str,
        _receiver: &Value,
        _other: &Value,
    ) -> Result<Value, String> {
        Ok(Value::NotImplemented)
    }

    /// A comparison with an unsupported type: `NotImplemented`.
    pub(super) fn dt_compare_unsupported(
        &mut self,
        _method: &str,
        _receiver: &Value,
        _other: &Value,
    ) -> Result<Value, String> {
        Ok(Value::NotImplemented)
    }
}
