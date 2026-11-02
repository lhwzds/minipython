// SPDX-License-Identifier: MIT

//! The per-character `str` predicates (`isdigit`, `isalpha`, ...) on
//! the Unicode 16.0 database that CPython 3.14 uses.
//!
//! General categories come from `unicode-general-category` (Unicode 16.0).
//! Properties read from the standard library or `unicode-ident`, which may
//! follow a newer Unicode version, are restricted to characters assigned in
//! Unicode 16.0. Numeric types are not derivable from the general category,
//! so the characters that differ are listed below.

use unicode_general_category::{GeneralCategory as G, get_general_category};

/// Unicode 16.0 characters with a digit value (`Numeric_Type=Digit`) that
/// are not decimal digits (general category `Nd`).
const DIGIT_NOT_DECIMAL: &[(u32, u32)] = &[
    (0xb2, 0xb3),
    (0xb9, 0xb9),
    (0x1369, 0x1371),
    (0x19da, 0x19da),
    (0x2070, 0x2070),
    (0x2074, 0x2079),
    (0x2080, 0x2089),
    (0x2460, 0x2468),
    (0x2474, 0x247c),
    (0x2488, 0x2490),
    (0x24ea, 0x24ea),
    (0x24f5, 0x24fd),
    (0x24ff, 0x24ff),
    (0x2776, 0x277e),
    (0x2780, 0x2788),
    (0x278a, 0x2792),
    (0x10a40, 0x10a43),
    (0x10e60, 0x10e68),
    (0x11052, 0x1105a),
    (0x1f100, 0x1f10a),
];

/// Unicode 16.0 characters with a numeric value that are not in a number
/// category (`Nd`, `Nl`, `No`): CJK ideographs with Unihan numeric values.
const NUMERIC_NOT_NUMBER: &[(u32, u32)] = &[
    (0x3405, 0x3405),
    (0x3483, 0x3483),
    (0x382a, 0x382a),
    (0x3b4d, 0x3b4d),
    (0x4e00, 0x4e00),
    (0x4e03, 0x4e03),
    (0x4e07, 0x4e07),
    (0x4e09, 0x4e09),
    (0x4e24, 0x4e24),
    (0x4e5d, 0x4e5d),
    (0x4e8c, 0x4e8c),
    (0x4e94, 0x4e94),
    (0x4e96, 0x4e96),
    (0x4eac, 0x4eac),
    (0x4ebf, 0x4ec0),
    (0x4edf, 0x4edf),
    (0x4ee8, 0x4ee8),
    (0x4f0d, 0x4f0d),
    (0x4f70, 0x4f70),
    (0x4fe9, 0x4fe9),
    (0x5006, 0x5006),
    (0x5104, 0x5104),
    (0x5146, 0x5146),
    (0x5169, 0x5169),
    (0x516b, 0x516b),
    (0x516d, 0x516d),
    (0x5341, 0x5341),
    (0x5343, 0x5345),
    (0x534c, 0x534c),
    (0x53c1, 0x53c4),
    (0x56db, 0x56db),
    (0x58f1, 0x58f1),
    (0x58f9, 0x58f9),
    (0x5e7a, 0x5e7a),
    (0x5efe, 0x5eff),
    (0x5f0c, 0x5f0e),
    (0x5f10, 0x5f10),
    (0x62d0, 0x62d0),
    (0x62fe, 0x62fe),
    (0x634c, 0x634c),
    (0x67d2, 0x67d2),
    (0x6d1e, 0x6d1e),
    (0x6f06, 0x6f06),
    (0x7396, 0x7396),
    (0x767e, 0x767e),
    (0x7695, 0x7695),
    (0x79ed, 0x79ed),
    (0x8086, 0x8086),
    (0x842c, 0x842c),
    (0x8cae, 0x8cae),
    (0x8cb3, 0x8cb3),
    (0x8d30, 0x8d30),
    (0x920e, 0x920e),
    (0x94a9, 0x94a9),
    (0x9621, 0x9621),
    (0x9646, 0x9646),
    (0x964c, 0x964c),
    (0x9678, 0x9678),
    (0x96f6, 0x96f6),
    (0xf96b, 0xf96b),
    (0xf973, 0xf973),
    (0xf978, 0xf978),
    (0xf9b2, 0xf9b2),
    (0xf9d1, 0xf9d1),
    (0xf9d3, 0xf9d3),
    (0xf9fd, 0xf9fd),
    (0x20001, 0x20001),
    (0x20064, 0x20064),
    (0x200e2, 0x200e2),
    (0x20121, 0x20121),
    (0x2092a, 0x2092a),
    (0x20983, 0x20983),
    (0x2098c, 0x2098c),
    (0x2099c, 0x2099c),
    (0x20aea, 0x20aea),
    (0x20afd, 0x20afd),
    (0x20b19, 0x20b19),
    (0x22390, 0x22390),
    (0x22998, 0x22998),
    (0x23b1b, 0x23b1b),
    (0x2626d, 0x2626d),
    (0x2f890, 0x2f890),
];

fn in_ranges(ranges: &[(u32, u32)], ch: char) -> bool {
    let point = u32::from(ch);
    ranges
        .binary_search_by(|&(start, end)| {
            if end < point {
                std::cmp::Ordering::Less
            } else if start > point {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

fn assigned(ch: char) -> bool {
    get_general_category(ch) != G::Unassigned
}

/// `str.isspace()`: bidirectional class `WS`, `B`, or `S`, or category `Zs`.
pub(crate) fn is_space(ch: char) -> bool {
    match ch {
        '\t'..='\r' | '\u{1c}'..='\u{20}' | '\u{85}' | '\u{2028}' | '\u{2029}' => true,
        _ if ch.is_ascii() => false,
        _ => get_general_category(ch) == G::SpaceSeparator,
    }
}

/// `str.isalpha()`: a letter category (`Lu`, `Ll`, `Lt`, `Lm`, `Lo`).
pub(crate) fn is_alpha(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphabetic();
    }
    matches!(
        get_general_category(ch),
        G::UppercaseLetter
            | G::LowercaseLetter
            | G::TitlecaseLetter
            | G::ModifierLetter
            | G::OtherLetter
    )
}

/// `str.isdecimal()`: a decimal digit (category `Nd`).
pub(crate) fn is_decimal(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_digit();
    }
    get_general_category(ch) == G::DecimalNumber
}

/// `str.isdigit()`: a decimal digit or another character with a digit value.
pub(crate) fn is_digit(ch: char) -> bool {
    is_decimal(ch) || (!ch.is_ascii() && in_ranges(DIGIT_NOT_DECIMAL, ch))
}

/// `str.isnumeric()`: any character with a numeric value.
pub(crate) fn is_numeric(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_digit();
    }
    matches!(
        get_general_category(ch),
        G::DecimalNumber | G::LetterNumber | G::OtherNumber
    ) || in_ranges(NUMERIC_NOT_NUMBER, ch)
}

/// `str.isalnum()`.
pub(crate) fn is_alnum(ch: char) -> bool {
    is_alpha(ch) || is_numeric(ch)
}

/// The `Lowercase` property.
pub(crate) fn is_lower(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_lowercase();
    }
    match get_general_category(ch) {
        G::LowercaseLetter => true,
        G::Unassigned => false,
        _ => ch.is_lowercase(),
    }
}

/// The `Uppercase` property.
pub(crate) fn is_upper(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_uppercase();
    }
    match get_general_category(ch) {
        G::UppercaseLetter => true,
        G::Unassigned => false,
        _ => ch.is_uppercase(),
    }
}

/// A titlecase letter (category `Lt`).
pub(crate) fn is_title(ch: char) -> bool {
    !ch.is_ascii() && get_general_category(ch) == G::TitlecaseLetter
}

/// `XID_Start`, for the first character of an identifier.
pub(crate) fn is_xid_start(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphabetic();
    }
    unicode_ident::is_xid_start(ch) && assigned(ch)
}

/// `XID_Continue`, for the other characters of an identifier.
pub(crate) fn is_xid_continue(ch: char) -> bool {
    if ch.is_ascii() {
        return ch.is_ascii_alphanumeric() || ch == '_';
    }
    unicode_ident::is_xid_continue(ch) && assigned(ch)
}
