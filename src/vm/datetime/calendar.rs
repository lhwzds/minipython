// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Modules/_datetimemodule.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Proleptic Gregorian calendar arithmetic shared by every `datetime` type.
//!
//! Ordinal 1 is 0001-01-01, a Monday. All functions are total over `i64`
//! (years outside `MINYEAR..=MAXYEAR` are computed, then rejected by the
//! callers with CPython's messages).

pub(super) const MINYEAR: i64 = 1;
pub(super) const MAXYEAR: i64 = 9999;
/// `date.max.toordinal()`.
pub(super) const MAX_ORDINAL: i64 = 3_652_059;

pub(super) const US_PER_SECOND: i128 = 1_000_000;
pub(super) const SECONDS_PER_DAY: i128 = 86_400;
pub(super) const US_PER_DAY: i128 = US_PER_SECOND * SECONDS_PER_DAY;
pub(super) const MAX_DELTA_DAYS: i128 = 999_999_999;

const DAYS_BEFORE_MONTH: [i64; 13] = [0, 0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];

pub(super) fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

pub(super) fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => 0,
    }
}

pub(super) fn days_before_year(year: i64) -> i64 {
    let y = year - 1;
    y * 365 + y.div_euclid(4) - y.div_euclid(100) + y.div_euclid(400)
}

pub(super) fn days_before_month(year: i64, month: i64) -> i64 {
    let index = month.clamp(1, 12) as usize;
    DAYS_BEFORE_MONTH[index] + i64::from(month > 2 && is_leap(year))
}

pub(super) fn ymd_to_ordinal(year: i64, month: i64, day: i64) -> i64 {
    days_before_year(year) + days_before_month(year, month) + day
}

/// The date of an ordinal; ordinals below 1 or above `MAX_ORDINAL` give
/// years outside the supported range.
pub(super) fn ordinal_to_ymd(ordinal: i64) -> (i64, i64, i64) {
    const DAYS_IN_400Y: i64 = 146_097;
    const DAYS_IN_100Y: i64 = 36_524;
    const DAYS_IN_4Y: i64 = 1_461;
    let n = ordinal - 1;
    let (n400, n) = (n.div_euclid(DAYS_IN_400Y), n.rem_euclid(DAYS_IN_400Y));
    let mut year = n400 * 400 + 1;
    let (n100, n) = (n / DAYS_IN_100Y, n % DAYS_IN_100Y);
    let (n4, n) = (n / DAYS_IN_4Y, n % DAYS_IN_4Y);
    let (n1, n) = (n / 365, n % 365);
    year += n100 * 100 + n4 * 4 + n1;
    if n1 == 4 || n100 == 4 {
        return (year - 1, 12, 31);
    }
    let leap = n1 == 3 && (n4 != 24 || n100 == 3);
    let mut month = (n + 50) >> 5;
    let mut preceding = DAYS_BEFORE_MONTH[month as usize] + i64::from(month > 2 && leap);
    if preceding > n {
        month -= 1;
        let length = if month == 2 && leap {
            29
        } else {
            days_in_month(2001, month)
        };
        preceding -= length;
    }
    (year, month, n - preceding + 1)
}

/// Monday is 0 and Sunday is 6.
pub(super) fn weekday(year: i64, month: i64, day: i64) -> i64 {
    (ymd_to_ordinal(year, month, day) + 6).rem_euclid(7)
}

/// Ordinal of the Monday that starts ISO week 1 of `year`.
pub(super) fn iso_week1_monday(year: i64) -> i64 {
    let first_day = ymd_to_ordinal(year, 1, 1);
    let first_weekday = (first_day + 6).rem_euclid(7);
    let monday = first_day - first_weekday;
    if first_weekday > 3 {
        monday + 7
    } else {
        monday
    }
}

/// `(iso_year, week, weekday)` with weeks and weekdays counted from 1.
pub(super) fn iso_calendar(year: i64, month: i64, day: i64) -> (i64, i64, i64) {
    let today = ymd_to_ordinal(year, month, day);
    let mut iso_year = year;
    let mut monday = iso_week1_monday(iso_year);
    let mut week = (today - monday).div_euclid(7);
    let weekday = (today - monday).rem_euclid(7);
    if week < 0 {
        iso_year -= 1;
        monday = iso_week1_monday(iso_year);
        week = (today - monday).div_euclid(7);
    } else if week >= 52 && today >= iso_week1_monday(iso_year + 1) {
        iso_year += 1;
        week = 0;
    }
    (iso_year, week + 1, weekday + 1)
}

/// `date.fromisocalendar` validation and conversion. Errors are CPython's
/// `ValueError` messages; the resulting year may still be out of range.
pub(super) fn iso_to_ymd(year: i64, week: i64, day: i64) -> Result<(i64, i64, i64), String> {
    if !(MINYEAR..=MAXYEAR).contains(&year) {
        return Err(format!(
            "ValueError: year must be in {MINYEAR}..{MAXYEAR}, not {year}"
        ));
    }
    if !(1..53).contains(&week) {
        let mut out_of_range = true;
        if week == 53 {
            // Years starting on a Thursday (or a Wednesday in leap years)
            // have 53 ISO weeks.
            let first_weekday = ymd_to_ordinal(year, 1, 1).rem_euclid(7);
            if first_weekday == 4 || (first_weekday == 3 && is_leap(year)) {
                out_of_range = false;
            }
        }
        if out_of_range {
            return Err(format!("ValueError: Invalid week: {week}"));
        }
    }
    if !(1..8).contains(&day) {
        return Err(format!(
            "ValueError: Invalid weekday: {day} (range is [1, 7])"
        ));
    }
    let ordinal = iso_week1_monday(year) + (week - 1) * 7 + (day - 1);
    Ok(ordinal_to_ymd(ordinal))
}

/// Validates a date with CPython's messages.
pub(super) fn check_date(year: i64, month: i64, day: i64) -> Result<(), String> {
    if !(MINYEAR..=MAXYEAR).contains(&year) {
        return Err(format!(
            "ValueError: year must be in {MINYEAR}..{MAXYEAR}, not {year}"
        ));
    }
    if !(1..=12).contains(&month) {
        return Err(format!("ValueError: month must be in 1..12, not {month}"));
    }
    let last = days_in_month(year, month);
    if !(1..=last).contains(&day) {
        return Err(format!(
            "ValueError: day {day} must be in range 1..{last} for month {month} in year {year}"
        ));
    }
    Ok(())
}

/// Validates a time of day with CPython's messages.
pub(super) fn check_time(
    hour: i64,
    minute: i64,
    second: i64,
    microsecond: i64,
    fold: i64,
) -> Result<(), String> {
    if !(0..=23).contains(&hour) {
        return Err(format!("ValueError: hour must be in 0..23, not {hour}"));
    }
    if !(0..=59).contains(&minute) {
        return Err(format!("ValueError: minute must be in 0..59, not {minute}"));
    }
    if !(0..=59).contains(&second) {
        return Err(format!("ValueError: second must be in 0..59, not {second}"));
    }
    if !(0..=999_999).contains(&microsecond) {
        return Err(format!(
            "ValueError: microsecond must be in 0..999999, not {microsecond}"
        ));
    }
    if fold != 0 && fold != 1 {
        return Err(format!(
            "ValueError: fold must be either 0 or 1, not {fold}"
        ));
    }
    Ok(())
}

/// Splits a total microsecond count into a date ordinal and microseconds
/// since midnight.
pub(super) fn split_day_us(total: i128) -> (i128, i128) {
    (total.div_euclid(US_PER_DAY), total.rem_euclid(US_PER_DAY))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinals_round_trip_across_the_range() {
        for ordinal in (1..=MAX_ORDINAL).step_by(97).chain([1, MAX_ORDINAL]) {
            let (y, m, d) = ordinal_to_ymd(ordinal);
            assert_eq!(ymd_to_ordinal(y, m, d), ordinal, "{y}-{m}-{d}");
        }
        assert_eq!(ordinal_to_ymd(1), (1, 1, 1));
        assert_eq!(ordinal_to_ymd(MAX_ORDINAL), (9999, 12, 31));
        assert_eq!(ordinal_to_ymd(MAX_ORDINAL + 1), (10000, 1, 1));
        assert_eq!(ordinal_to_ymd(0), (0, 12, 31));
    }

    #[test]
    fn iso_calendar_matches_known_weeks() {
        assert_eq!(iso_calendar(2020, 1, 1), (2020, 1, 3));
        assert_eq!(iso_calendar(2021, 1, 1), (2020, 53, 5));
        assert_eq!(iso_calendar(2019, 12, 30), (2020, 1, 1));
        assert_eq!(iso_to_ymd(2020, 53, 1), Ok((2020, 12, 28)));
    }
}
