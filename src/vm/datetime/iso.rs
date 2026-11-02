// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/_pydatetime.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! `fromisoformat` parsing for `date`, `time`, and `datetime`.
//!
//! Accepted forms follow CPython 3.11+: calendar (`YYYY-MM-DD`,
//! `YYYYMMDD`) and week (`YYYY-Www[-D]`, `YYYYWww[D]`) dates; times
//! `HH[:MM[:SS[.f+]]]` or `HH[MM[SS[.f+]]]` (`,` also separates fractions,
//! extra fraction digits are truncated); offsets `Z` or `±HH[:MM[:SS[.f+]]]`
//! / `±HH[MM[SS[.f+]]]`; any single character between date and time;
//! `24:00` as midnight of the next day.

use super::*;

pub(super) struct ParsedTime {
    pub hms: Hms,
    pub offset: Option<Delta>,
    /// The time was `24:00[:00[.0]]`.
    pub hour_24: bool,
}

pub(super) struct ParsedDateTime {
    pub date: Ymd,
    pub time: ParsedTime,
}

fn invalid(text: &str) -> String {
    format!(
        "ValueError: Invalid isoformat string: {}",
        repr_string(text)
    )
}

fn digits(chars: &[char], start: usize, count: usize) -> Option<i64> {
    let slice = chars.get(start..start + count)?;
    slice.iter().try_fold(0i64, |total, ch| {
        ch.is_ascii_digit()
            .then(|| total * 10 + i64::from(*ch as u8 - b'0'))
    })
}

/// A date in one of the accepted forms at the start of `chars`; like
/// CPython, characters after a complete date are not examined.
fn parse_date(chars: &[char]) -> Option<DateFields> {
    let year = digits(chars, 0, 4)?;
    let mut position = 4;
    let extended = chars.get(position) == Some(&'-');
    if extended {
        position += 1;
    }
    if chars.get(position) == Some(&'W') {
        position += 1;
        let week = digits(chars, position, 2)?;
        position += 2;
        let mut day = 1;
        if position < chars.len() {
            if extended {
                if chars.get(position) != Some(&'-') {
                    return None;
                }
                position += 1;
            }
            day = digits(chars, position, 1)?;
        }
        return Some(DateFields::Week { year, week, day });
    }
    let month = digits(chars, position, 2)?;
    position += 2;
    if extended {
        if chars.get(position) != Some(&'-') {
            return None;
        }
        position += 1;
    }
    let day = digits(chars, position, 2)?;
    Some(DateFields::Calendar { year, month, day })
}

enum DateFields {
    Calendar { year: i64, month: i64, day: i64 },
    Week { year: i64, week: i64, day: i64 },
}

/// Resolves parsed date fields; week dates that do not exist are invalid
/// strings, calendar dates report the constructor's errors.
fn resolve_date(fields: DateFields, text: &str) -> Result<Ymd, String> {
    match fields {
        DateFields::Calendar { year, month, day } => {
            check_date(year, month, day)?;
            Ok(Ymd { year, month, day })
        }
        DateFields::Week { year, week, day } => {
            let (year, month, day) = iso_to_ymd(year, week, day).map_err(|_| invalid(text))?;
            check_date(year, month, day)?;
            Ok(Ymd { year, month, day })
        }
    }
}

/// `date.fromisoformat`.
pub(super) fn parse_date_only(text: &str) -> Result<Ymd, String> {
    let chars = text.chars().collect::<Vec<_>>();
    if !matches!(chars.len(), 7 | 8 | 10) {
        return Err(invalid(text));
    }
    let fields = parse_date(&chars).ok_or_else(|| invalid(text))?;
    resolve_date(fields, text)
}

/// Where the date ends in a `datetime` string: the separator is the
/// character after it.
fn date_length(chars: &[char]) -> Option<usize> {
    let len = chars.len();
    if len <= 7 {
        return Some(len);
    }
    let digit_at = |index: usize| chars.get(index).is_some_and(char::is_ascii_digit);
    if chars[4] == '-' {
        if chars[5] != 'W' {
            return Some(10);
        }
        if chars.get(8) == Some(&'-') {
            if len == 9 {
                return None;
            }
            if len > 10 && digit_at(10) {
                return Some(8);
            }
            return Some(10);
        }
        return Some(8);
    }
    if chars[4] == 'W' {
        let mut end = 7;
        while end < len && digit_at(end) {
            end += 1;
        }
        if end < 9 {
            return Some(end);
        }
        return Some(if end % 2 == 0 { 7 } else { 8 });
    }
    Some(8)
}

/// `datetime.fromisoformat`.
pub(super) fn parse_datetime(text: &str) -> Result<ParsedDateTime, String> {
    let chars = text.chars().collect::<Vec<_>>();
    if chars.len() < 7 {
        return Err(invalid(text));
    }
    let date_end = date_length(&chars).ok_or_else(|| invalid(text))?;
    let date_chars = &chars[..date_end.min(chars.len())];
    let fields = parse_date(date_chars).ok_or_else(|| invalid(text))?;
    let time_chars = if date_end < chars.len() {
        &chars[date_end + 1..]
    } else {
        &[][..]
    };
    let time = if date_end < chars.len() {
        parse_time_and_offset(time_chars).ok_or_else(|| invalid(text))?
    } else {
        ParsedTime {
            hms: Hms::midnight(),
            offset: None,
            hour_24: false,
        }
    };
    let date = resolve_date(fields, text)?;
    let time = finish_hour_24(time)?;
    Ok(ParsedDateTime { date, time })
}

/// `time.fromisoformat`.
pub(super) fn parse_time_only(text: &str) -> Result<ParsedTime, String> {
    let chars = text.chars().collect::<Vec<_>>();
    let body = match chars.first() {
        Some('T') => &chars[1..],
        _ => &chars[..],
    };
    let time = parse_time_and_offset(body).ok_or_else(|| invalid(text))?;
    let mut time = finish_hour_24(time)?;
    if time.hour_24 {
        time.hms.hour = 0;
        time.hour_24 = false;
    }
    Ok(time)
}

/// Validates `24:00`: everything after the hour must be zero.
fn finish_hour_24(mut time: ParsedTime) -> Result<ParsedTime, String> {
    if time.hms.hour == 24 {
        if time.hms.minute != 0 || time.hms.second != 0 || time.hms.us != 0 {
            return Err(
                "ValueError: minute, second, and microsecond must be 0 when hour is 24".to_string(),
            );
        }
        time.hour_24 = true;
    }
    Ok(time)
}

/// Components `HH[:MM[:SS[.f]]]` of a time or an offset.
struct Components {
    values: [i64; 3],
    us: i64,
    /// One unparsed character followed the last component (or a fraction
    /// separator had no digits).
    trailing: bool,
}

fn parse_components(chars: &[char]) -> Option<Components> {
    let len = chars.len();
    let mut values = [0i64; 3];
    let mut position = 0;
    let mut extended = false;
    for index in 0..3 {
        values[index] = digits(chars, position, 2)?;
        position += 2;
        if position >= len {
            return Some(Components {
                values,
                us: 0,
                trailing: false,
            });
        }
        let separator = chars[position];
        position += 1;
        if index == 0 {
            extended = separator == ':';
        }
        if position >= len {
            // The last character was consumed as a separator.
            return Some(Components {
                values,
                us: 0,
                trailing: true,
            });
        }
        if extended && separator == ':' && index < 2 {
            continue;
        }
        if separator == '.' || separator == ',' {
            if index < 2 {
                return None;
            }
            break;
        }
        if !extended {
            // Basic format: the character starts the next component.
            position -= 1;
            continue;
        }
        return None;
    }
    // Fraction digits: up to six are significant, more are skipped.
    let significant = (len - position).min(6);
    let mut us = 0;
    for offset in 0..significant {
        let ch = chars[position + offset];
        if !ch.is_ascii_digit() {
            return None;
        }
        us = us * 10 + i64::from(ch as u8 - b'0');
    }
    for _ in significant..6 {
        us *= 10;
    }
    position += significant;
    while position < len && chars[position].is_ascii_digit() {
        position += 1;
    }
    Some(Components {
        values,
        us,
        trailing: position < len,
    })
}

fn parse_time_and_offset(chars: &[char]) -> Option<ParsedTime> {
    if chars.is_empty() {
        return None;
    }
    let marker = chars
        .iter()
        .position(|ch| matches!(ch, 'Z' | '+' | '-'))
        .unwrap_or(chars.len());
    let time = parse_components(&chars[..marker])?;
    let offset = if marker == chars.len() {
        if time.trailing {
            return None;
        }
        None
    } else {
        Some(parse_offset(&chars[marker..])?)
    };
    let [hour, minute, second] = time.values;
    Some(ParsedTime {
        hms: Hms {
            hour,
            minute,
            second,
            us: time.us,
            tz: Value::None,
            fold: 0,
        },
        offset,
        hour_24: false,
    })
}

fn parse_offset(chars: &[char]) -> Option<Delta> {
    match chars.first()? {
        'Z' => (chars.len() == 1).then_some(Delta::ZERO),
        sign @ ('+' | '-') => {
            let components = parse_components(&chars[1..])?;
            if components.trailing {
                return None;
            }
            let [hours, minutes, seconds] = components.values;
            let whole = hours * 3600 + minutes * 60 + seconds;
            if whole == 0 {
                // CPython ignores the fraction of a zero offset.
                return Some(Delta::ZERO);
            }
            let total = i128::from(whole) * US_PER_SECOND + i128::from(components.us);
            let total = if *sign == '-' { -total } else { total };
            Delta::from_us(total).ok()
        }
        _ => None,
    }
}
