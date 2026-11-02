// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `repr`, `isoformat`, `ctime`, and `strftime` for the datetime types.
//!
//! `strftime` follows the C locale. The supported directives are the
//! C99/POSIX ones that the macOS and glibc C libraries render identically,
//! plus CPython's own `%f`, `%z`, `%:z`, and `%Z`; see `strftime_directive`.

use super::*;

pub(super) const DAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];
pub(super) const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// The `repr` of a datetime-family value; `tz_repr` renders a tzinfo.
pub(super) fn repr_state(
    value: &Value,
    state: &State,
    tz_repr: impl Fn(&Value) -> String,
) -> Option<String> {
    let name = repr_type_name(value);
    Some(match state {
        State::Delta(delta) => delta::delta_repr(&name, *delta),
        State::Date(ymd) => format!("{name}({}, {}, {})", ymd.year, ymd.month, ymd.day),
        State::Time(hms) => {
            let mut text = if hms.us != 0 {
                format!(
                    "{name}({}, {}, {}, {}",
                    hms.hour, hms.minute, hms.second, hms.us
                )
            } else if hms.second != 0 {
                format!("{name}({}, {}, {}", hms.hour, hms.minute, hms.second)
            } else {
                format!("{name}({}, {}", hms.hour, hms.minute)
            };
            if !hms.is_naive_tz() {
                text.push_str(&format!(", tzinfo={}", tz_repr(&hms.tz)));
            }
            if hms.fold != 0 {
                text.push_str(", fold=1");
            }
            text.push(')');
            text
        }
        State::DateTime(ymd, hms) => {
            let mut text = format!(
                "{name}({}, {}, {}, {}, {}",
                ymd.year, ymd.month, ymd.day, hms.hour, hms.minute
            );
            if hms.us != 0 {
                text.push_str(&format!(", {}, {}", hms.second, hms.us));
            } else if hms.second != 0 {
                text.push_str(&format!(", {}", hms.second));
            }
            if hms.fold != 0 {
                text.push_str(", fold=1");
            }
            if !hms.is_naive_tz() {
                text.push_str(&format!(", tzinfo={}", tz_repr(&hms.tz)));
            }
            text.push(')');
            text
        }
        State::TimeZone(offset, name) => tz::timezone_repr(value, *offset, name.as_deref()),
    })
}

pub(super) fn date_isoformat(ymd: Ymd) -> String {
    format!("{:04}-{:02}-{:02}", ymd.year, ymd.month, ymd.day)
}

/// The time part of `isoformat` for a `timespec`.
pub(super) fn time_isoformat(hms: &Hms, timespec: &str) -> Result<String, String> {
    let timespec = if timespec == "auto" {
        if hms.us != 0 {
            "microseconds"
        } else {
            "seconds"
        }
    } else {
        timespec
    };
    Ok(match timespec {
        "hours" => format!("{:02}", hms.hour),
        "minutes" => format!("{:02}:{:02}", hms.hour, hms.minute),
        "seconds" => format!("{:02}:{:02}:{:02}", hms.hour, hms.minute, hms.second),
        "milliseconds" => format!(
            "{:02}:{:02}:{:02}.{:03}",
            hms.hour,
            hms.minute,
            hms.second,
            hms.us / 1000
        ),
        "microseconds" => format!(
            "{:02}:{:02}:{:02}.{:06}",
            hms.hour, hms.minute, hms.second, hms.us
        ),
        _ => return Err("ValueError: Unknown timespec value".to_string()),
    })
}

/// `+HH:MM[:SS[.ffffff]]` (or without colons).
pub(super) fn format_offset(offset: Delta, separator: &str) -> String {
    let total = offset.total_us();
    let sign = if total < 0 { '-' } else { '+' };
    let total = total.abs();
    let us = total % US_PER_SECOND;
    let seconds = total / US_PER_SECOND;
    let mut text = format!(
        "{sign}{:02}{separator}{:02}",
        seconds / 3600,
        seconds / 60 % 60
    );
    if seconds % 60 != 0 || us != 0 {
        text.push_str(&format!("{separator}{:02}", seconds % 60));
    }
    if us != 0 {
        text.push_str(&format!(".{us:06}"));
    }
    text
}

/// `ctime()`: `Wed Jan  1 00:00:00 2020`.
pub(super) fn ctime(ymd: Ymd, hms: &Hms) -> String {
    let weekday = weekday(ymd.year, ymd.month, ymd.day) as usize;
    format!(
        "{} {} {:2} {:02}:{:02}:{:02} {:04}",
        &DAY_NAMES[weekday][..3],
        &MONTH_NAMES[(ymd.month - 1) as usize][..3],
        ymd.day,
        hms.hour,
        hms.minute,
        hms.second,
        ymd.year
    )
}

/// The broken-down time `strftime` formats.
pub(super) struct TimeParts {
    pub year: i64,
    pub month: i64,
    pub day: i64,
    pub hour: i64,
    pub minute: i64,
    pub second: i64,
    pub us: i64,
    /// Monday is 0.
    pub weekday: i64,
    /// 1-based day of the year.
    pub yday: i64,
}

impl TimeParts {
    /// Parts from a `time.struct_time`-like 9-tuple, with the C library's
    /// range checks (as `time.strftime` applies them).
    pub fn from_tuple(items: &[i64], us: i64) -> Result<TimeParts, String> {
        let error = |message: &str| Err(format!("ValueError: {message}"));
        let [year, month, day, hour, minute, second, wday, yday, _] = *items else {
            return Err(format!(
                "TypeError: function takes exactly 9 arguments ({} given)",
                items.len()
            ));
        };
        let month = match month {
            0 => 1,
            1..=12 => month,
            _ => return error("month out of range"),
        };
        let day = match day {
            0 => 1,
            1..=31 => day,
            _ => return error("day of month out of range"),
        };
        if !(0..=23).contains(&hour) {
            return error("hour out of range");
        }
        if !(0..=59).contains(&minute) {
            return error("minute out of range");
        }
        if !(0..=61).contains(&second) {
            return error("seconds out of range");
        }
        // C's `(wday + 1) % 7` keeps the sign of a negative day.
        let sunday_based = (wday + 1) % 7;
        if sunday_based < 0 {
            return error("day of week out of range");
        }
        let yday = match yday {
            0 => 1,
            1..=366 => yday,
            _ => return error("day of year out of range"),
        };
        Ok(TimeParts {
            year,
            month,
            day,
            hour,
            minute,
            second,
            us,
            weekday: (sunday_based + 6) % 7,
            yday,
        })
    }

    pub fn new(ymd: Ymd, hms: &Hms) -> TimeParts {
        TimeParts {
            year: ymd.year,
            month: ymd.month,
            day: ymd.day,
            hour: hms.hour,
            minute: hms.minute,
            second: hms.second,
            us: hms.us,
            weekday: weekday(ymd.year, ymd.month, ymd.day),
            yday: ymd.ordinal() - ymd_to_ordinal(ymd.year, 1, 1) + 1,
        }
    }
}

/// A value CPython substitutes before calling the C `strftime`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum TzField {
    /// `%z`: `+HHMM[SS[.ffffff]]`.
    Offset,
    /// `%:z`: `+HH:MM[:SS[.ffffff]]`.
    ColonOffset,
    /// `%Z`: the zone name.
    Name,
}

#[derive(Clone, Copy, PartialEq)]
enum Pad {
    Default,
    None,
    Space,
    Zero,
}

fn padded(value: i64, width: usize, default_char: char, pad: Pad) -> String {
    match (pad, default_char) {
        (Pad::None, _) => value.to_string(),
        (Pad::Space, _) | (Pad::Default, ' ') => format!("{value:>width$}"),
        _ => {
            if value < 0 {
                format!("-{:0>width$}", -value, width = width.saturating_sub(1))
            } else {
                format!("{value:0>width$}")
            }
        }
    }
}

/// C-locale `strftime` with CPython's `%f`/`%z`/`%:z`/`%Z` handling. The
/// tzinfo replacements are computed on first use, as CPython does.
pub(super) fn strftime(
    format: &str,
    parts: &TimeParts,
    tz: &mut dyn FnMut(TzField) -> Result<String, String>,
) -> Result<String, String> {
    let chars = format.chars().collect::<Vec<_>>();
    let mut cache: Vec<(TzField, String)> = Vec::new();
    let mut replacement = |field: TzField| -> Result<String, String> {
        if let Some((_, text)) = cache.iter().find(|(cached, _)| *cached == field) {
            return Ok(text.clone());
        }
        let text = tz(field)?;
        cache.push((field, text.clone()));
        Ok(text)
    };
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch != '%' {
            out.push(ch);
            index += 1;
            continue;
        }
        let Some(&next) = chars.get(index + 1) else {
            out.push('%');
            break;
        };
        // CPython replaces these before the C library sees the format.
        match next {
            'f' => {
                out.push_str(&format!("{:06}", parts.us));
                index += 2;
                continue;
            }
            'z' => {
                out.push_str(&replacement(TzField::Offset)?);
                index += 2;
                continue;
            }
            'Z' => {
                out.push_str(&replacement(TzField::Name)?);
                index += 2;
                continue;
            }
            ':' if chars.get(index + 2) == Some(&'z') => {
                out.push_str(&replacement(TzField::ColonOffset)?);
                index += 3;
                continue;
            }
            _ => {}
        }
        // Optional padding flag, then optional E/O modifier.
        let mut cursor = index + 1;
        let pad = match chars[cursor] {
            '-' => Pad::None,
            '_' => Pad::Space,
            '0' => Pad::Zero,
            _ => Pad::Default,
        };
        if pad != Pad::Default {
            cursor += 1;
        }
        let mut modifier = None;
        if let Some(&candidate @ ('E' | 'O')) = chars.get(cursor) {
            modifier = Some(candidate);
            cursor += 1;
        }
        let Some(&directive) = chars.get(cursor) else {
            // A trailing incomplete directive is copied as written.
            out.extend(&chars[index..]);
            break;
        };
        match strftime_directive(directive, modifier, pad, parts) {
            Some(text) => out.push_str(&text),
            None => out.extend(&chars[index..=cursor]),
        }
        index = cursor + 1;
    }
    Ok(out)
}

fn modifier_allowed(directive: char, modifier: Option<char>) -> bool {
    match modifier {
        None => true,
        Some('E') => matches!(directive, 'c' | 'C' | 'x' | 'X' | 'y' | 'Y'),
        Some(_) => matches!(
            directive,
            'd' | 'e' | 'H' | 'I' | 'm' | 'M' | 'S' | 'u' | 'U' | 'V' | 'w' | 'W' | 'y'
        ),
    }
}

/// One C-locale directive, or `None` for a directive outside the portable
/// set (copied through unchanged).
fn strftime_directive(
    directive: char,
    modifier: Option<char>,
    pad: Pad,
    parts: &TimeParts,
) -> Option<String> {
    if !modifier_allowed(directive, modifier) {
        return None;
    }
    let hour12 = match parts.hour % 12 {
        0 => 12,
        hour => hour,
    };
    let (iso_year, iso_week, _) = iso_calendar(parts.year, parts.month, parts.day);
    let sunday_weekday = (parts.weekday + 1) % 7;
    let text = match directive {
        'a' => DAY_NAMES[parts.weekday as usize][..3].to_string(),
        'A' => DAY_NAMES[parts.weekday as usize].to_string(),
        'b' | 'h' => MONTH_NAMES[(parts.month - 1) as usize][..3].to_string(),
        'B' => MONTH_NAMES[(parts.month - 1) as usize].to_string(),
        'c' => format!(
            "{} {} {:2} {:02}:{:02}:{:02} {:04}",
            &DAY_NAMES[parts.weekday as usize][..3],
            &MONTH_NAMES[(parts.month - 1) as usize][..3],
            parts.day,
            parts.hour,
            parts.minute,
            parts.second,
            parts.year
        ),
        'C' => format!("{:02}", parts.year.div_euclid(100)),
        'd' => padded(parts.day, 2, '0', pad),
        'D' | 'x' => format!(
            "{:02}/{:02}/{:02}",
            parts.month,
            parts.day,
            parts.year.rem_euclid(100)
        ),
        'e' => padded(parts.day, 2, ' ', pad),
        'F' => format!("{:04}-{:02}-{:02}", parts.year, parts.month, parts.day),
        'g' => format!("{:02}", iso_year.rem_euclid(100)),
        'G' => format!("{iso_year:04}"),
        'H' => padded(parts.hour, 2, '0', pad),
        'I' => padded(hour12, 2, '0', pad),
        'j' => padded(parts.yday, 3, '0', pad),
        'k' => padded(parts.hour, 2, ' ', pad),
        'l' => padded(hour12, 2, ' ', pad),
        'm' => padded(parts.month, 2, '0', pad),
        'M' => padded(parts.minute, 2, '0', pad),
        'n' => "\n".to_string(),
        'p' => if parts.hour < 12 { "AM" } else { "PM" }.to_string(),
        'r' => format!(
            "{:02}:{:02}:{:02} {}",
            hour12,
            parts.minute,
            parts.second,
            if parts.hour < 12 { "AM" } else { "PM" }
        ),
        'R' => format!("{:02}:{:02}", parts.hour, parts.minute),
        's' => {
            let days =
                ymd_to_ordinal(parts.year, parts.month, parts.day) - ymd_to_ordinal(1970, 1, 1);
            (days * 86_400 + parts.hour * 3600 + parts.minute * 60 + parts.second).to_string()
        }
        'S' => padded(parts.second, 2, '0', pad),
        't' => "\t".to_string(),
        'T' | 'X' => format!("{:02}:{:02}:{:02}", parts.hour, parts.minute, parts.second),
        'u' => (parts.weekday + 1).to_string(),
        'U' => padded((parts.yday + 6 - sunday_weekday) / 7, 2, '0', pad),
        'V' => padded(iso_week, 2, '0', pad),
        'w' => sunday_weekday.to_string(),
        'W' => padded((parts.yday + 6 - parts.weekday) / 7, 2, '0', pad),
        'y' => format!("{:02}", parts.year.rem_euclid(100)),
        'Y' => format!("{:04}", parts.year),
        '%' => "%".to_string(),
        _ => return None,
    };
    Some(text)
}

impl Vm {
    /// One `strftime` tzinfo replacement for an object whose tzinfo is `tz`
    /// (`arg` is what `utcoffset`/`tzname` receive).
    pub(super) fn dt_tz_replacement(
        &mut self,
        tz: &Value,
        arg: &Value,
        field: TzField,
        parts: &TimeParts,
    ) -> Result<String, String> {
        if matches!(tz, Value::None) {
            return Ok(String::new());
        }
        match field {
            TzField::Offset | TzField::ColonOffset => {
                let separator = if field == TzField::Offset { "" } else { ":" };
                Ok(self
                    .dt_call_offset(tz, "utcoffset", arg)?
                    .map(|offset| format_offset(offset, separator))
                    .unwrap_or_default())
            }
            TzField::Name => match self.dt_call_tzname(tz, arg)? {
                Value::String(name) | Value::IdentityString { value: name, .. } => Ok(name),
                Value::None => Ok(String::new()),
                other => {
                    // CPython escapes the name with its own `replace` method
                    // and hands the result to the C `strftime`.
                    let replace = self.load_attribute_value(other, "replace")?;
                    let escaped = self.call_value(
                        replace,
                        vec![
                            Value::String("%".to_string()),
                            Value::String("%%".to_string()),
                        ],
                    )?;
                    let escaped = match &escaped {
                        Value::String(text) | Value::IdentityString { value: text, .. } => {
                            text.clone()
                        }
                        other => match str_subclass_string(other) {
                            Some(text) => text,
                            None => {
                                return self.dt_error(
                                    "TypeError",
                                    "tzname.replace() did not return a string",
                                );
                            }
                        },
                    };
                    strftime(&escaped, parts, &mut |_| Ok(String::new()))
                }
            },
        }
    }

    /// `strftime` for a date part, a time part, and a tzinfo.
    pub(super) fn dt_strftime(
        &mut self,
        format: &str,
        parts: &TimeParts,
        tz: &Value,
        arg: &Value,
    ) -> Result<String, String> {
        strftime(format, parts, &mut |field| {
            self.dt_tz_replacement(tz, arg, field, parts)
        })
    }
}
