// SPDX-License-Identifier: MIT

//! Character classes and case folding used by the matcher.
//!
//! Unicode classes follow CPython's `str` predicates per character: `\d` is
//! `str.isdecimal()` (general category Nd), `\w` is `str.isalnum()` (any
//! letter or number category) or `_`, and `\s` is `str.isspace()`. Case
//! folding uses the first character of the full lower/upper mapping, as
//! CPython's `re` does.

use std::collections::HashMap;
use std::sync::OnceLock;

use unicode_general_category::{GeneralCategory as G, get_general_category};

pub(super) fn ascii_lower(ch: u32) -> u32 {
    if (b'A' as u32..=b'Z' as u32).contains(&ch) {
        ch + 32
    } else {
        ch
    }
}

pub(super) fn ascii_upper(ch: u32) -> u32 {
    if (b'a' as u32..=b'z' as u32).contains(&ch) {
        ch - 32
    } else {
        ch
    }
}

pub(super) fn ascii_is_cased(ch: u32) -> bool {
    ch < 128 && (ch as u8).is_ascii_alphabetic()
}

/// Case mappings follow the Unicode version of the general-category table
/// (the one CPython 3.14 uses): characters it does not assign, or mappings
/// to such characters, come from the standard library's newer tables and
/// are ignored.
fn assigned(c: char) -> bool {
    get_general_category(c) != G::Unassigned
}

/// First character of the full lowercase mapping.
pub(super) fn lower(ch: u32) -> u32 {
    if ch < 128 {
        return ascii_lower(ch);
    }
    match char::from_u32(ch) {
        Some(c) if assigned(c) => match c.to_lowercase().next() {
            Some(mapped) if assigned(mapped) => u32::from(mapped),
            _ => ch,
        },
        _ => ch,
    }
}

/// First character of the full uppercase mapping.
pub(super) fn upper(ch: u32) -> u32 {
    if ch < 128 {
        return ascii_upper(ch);
    }
    match char::from_u32(ch) {
        Some(c) if assigned(c) => match c.to_uppercase().next() {
            Some(mapped) if assigned(mapped) => u32::from(mapped),
            _ => ch,
        },
        _ => ch,
    }
}

pub(super) fn unicode_is_cased(ch: u32) -> bool {
    ch != lower(ch) || ch != upper(ch)
}

/// Lowercase characters that share their uppercase form with other
/// lowercase characters (for example `s` and `ſ`, `σ` and `ς`), so that a
/// case-insensitive literal also matches the other forms.
pub(super) fn extra_cases(lowered: u32) -> &'static [u32] {
    static TABLE: OnceLock<HashMap<u32, Vec<u32>>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut groups: HashMap<String, Vec<u32>> = HashMap::new();
        for point in 0..=0x10ffffu32 {
            let Some(ch) = char::from_u32(point) else {
                continue;
            };
            // A character that is its own lowercase but has a different
            // uppercase has the Lowercase property (titlecase letters have
            // a different lowercase), so the cheap property check skips the
            // rest of the code space.
            if !ch.is_lowercase() || lower(point) != point || !assigned(ch) {
                continue;
            }
            let upper: String = ch.to_uppercase().collect();
            if upper.chars().eq(std::iter::once(ch)) {
                continue;
            }
            groups.entry(upper).or_default().push(point);
        }
        let mut table = HashMap::new();
        for mut group in groups.into_values() {
            if group.len() < 2 {
                continue;
            }
            group.sort_unstable();
            for &member in &group {
                let others = group
                    .iter()
                    .copied()
                    .filter(|&other| other != member)
                    .collect();
                table.insert(member, others);
            }
        }
        table
    });
    table.get(&lowered).map_or(&[], Vec::as_slice)
}

fn category(ch: u32) -> Option<G> {
    char::from_u32(ch).map(get_general_category)
}

pub(super) fn uni_is_digit(ch: u32) -> bool {
    if ch < 128 {
        return (b'0' as u32..=b'9' as u32).contains(&ch);
    }
    matches!(category(ch), Some(G::DecimalNumber))
}

pub(super) fn uni_is_space(ch: u32) -> bool {
    match ch {
        0x09..=0x0d | 0x1c..=0x20 | 0x85 | 0x2028 | 0x2029 => true,
        _ if ch < 128 => false,
        _ => matches!(category(ch), Some(G::SpaceSeparator)),
    }
}

pub(super) fn uni_is_word(ch: u32) -> bool {
    if ch < 128 {
        return ascii_is_word(ch);
    }
    matches!(
        category(ch),
        Some(
            G::UppercaseLetter
                | G::LowercaseLetter
                | G::TitlecaseLetter
                | G::ModifierLetter
                | G::OtherLetter
                | G::DecimalNumber
                | G::LetterNumber
                | G::OtherNumber
        )
    )
}

pub(super) fn ascii_is_digit(ch: u32) -> bool {
    (b'0' as u32..=b'9' as u32).contains(&ch)
}

pub(super) fn ascii_is_space(ch: u32) -> bool {
    matches!(ch, 0x09..=0x0d | 0x20)
}

pub(super) fn ascii_is_word(ch: u32) -> bool {
    ch < 128 && ((ch as u8).is_ascii_alphanumeric() || ch == b'_' as u32)
}
