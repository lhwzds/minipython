// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Lib/textwrap.py (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! The parts of `textwrap` argparse uses: `wrap(text, width)` and
//! `fill(text, width, initial_indent=..., subsequent_indent=...)` with the
//! default options (tab expansion, whitespace replacement, hyphen-aware
//! chunking, long-word breaking, whitespace dropping).

use super::super::*;

/// The whitespace characters textwrap recognizes (US-ASCII only).
fn is_wrap_whitespace(ch: char) -> bool {
    matches!(ch, '\t' | '\n' | '\x0b' | '\x0c' | '\r' | ' ')
}

/// `\w` for `str` patterns: alphanumerics in Python's sense, plus `_`.
fn is_word_char(ch: char) -> bool {
    if ch == '_' {
        return true;
    }
    if ch.is_ascii() {
        return ch.is_ascii_alphanumeric();
    }
    matches!(
        get_general_category(ch),
        GeneralCategory::UppercaseLetter
            | GeneralCategory::LowercaseLetter
            | GeneralCategory::TitlecaseLetter
            | GeneralCategory::ModifierLetter
            | GeneralCategory::OtherLetter
            | GeneralCategory::DecimalNumber
            | GeneralCategory::LetterNumber
            | GeneralCategory::OtherNumber
    )
}

/// `[^\d\W]`: word characters that are not decimal digits.
fn is_letter(ch: char) -> bool {
    is_word_char(ch) && !is_decimal_char(ch)
}

/// `[\w!"\'&.,?]`.
fn is_word_punct(ch: char) -> bool {
    is_word_char(ch) || matches!(ch, '!' | '"' | '\'' | '&' | '.' | ',' | '?')
}

/// `str.expandtabs(8)`.
fn expand_tabs(text: &str) -> String {
    let mut result = String::new();
    let mut column = 0usize;
    for ch in text.chars() {
        match ch {
            '\t' => {
                let spaces = 8 - column % 8;
                result.extend(std::iter::repeat_n(' ', spaces));
                column += spaces;
            }
            '\n' | '\r' => {
                result.push(ch);
                column = 0;
            }
            other => {
                result.push(other);
                column += 1;
            }
        }
    }
    result
}

/// `TextWrapper._munge_whitespace` + `_split` with `break_on_hyphens=True`
/// (the `wordsep_re` chunking).
fn split_chunks(text: &str) -> Vec<String> {
    let text = expand_tabs(text)
        .chars()
        .map(|ch| if is_wrap_whitespace(ch) { ' ' } else { ch })
        .collect::<Vec<_>>();
    let length = text.len();
    let mut chunks = Vec::new();
    let mut start = 0;
    // `-{2,}` followed by a word character, starting at `index`.
    let em_dash_at = |index: usize| -> bool {
        let mut end = index;
        while end < length && text[end] == '-' {
            end += 1;
        }
        end - index >= 2 && end < length && is_word_char(text[end])
    };
    while start < length {
        if is_wrap_whitespace(text[start]) {
            let mut end = start;
            while end < length && is_wrap_whitespace(text[end]) {
                end += 1;
            }
            chunks.push(text[start..end].iter().collect());
            start = end;
            continue;
        }
        // Em-dash between words: (?<=wp) -{2,} (?=\w)
        if start > 0 && is_word_punct(text[start - 1]) && em_dash_at(start) {
            let mut end = start;
            while end < length && text[end] == '-' {
                end += 1;
            }
            chunks.push(text[start..end].iter().collect());
            start = end;
            continue;
        }
        // Word, possibly hyphenated: \S+? followed by one of the endings.
        let mut end = start + 1;
        let mut chunk_end = None;
        while end <= length {
            if end > start + 1 && is_wrap_whitespace(text[end - 1]) {
                break;
            }
            // Hyphenated word: -(?:(?<=lt{2}-)|(?<=lt-lt-))(?=lt-?lt)
            if end < length && text[end] == '-' {
                let behind_two_letters =
                    end >= 2 && is_letter(text[end - 2]) && is_letter(text[end - 1]);
                let behind_letter_hyphen_letter = end >= 3
                    && is_letter(text[end - 3])
                    && text[end - 2] == '-'
                    && is_letter(text[end - 1]);
                let ahead = end + 1 < length
                    && is_letter(text[end + 1])
                    && ((end + 2 < length
                        && text[end + 2] == '-'
                        && end + 3 < length
                        && is_letter(text[end + 3]))
                        || (end + 2 < length && is_letter(text[end + 2])));
                if (behind_two_letters || behind_letter_hyphen_letter) && ahead {
                    chunk_end = Some(end + 1);
                    break;
                }
            }
            // End of word: (?=ws|\Z)
            if end == length || is_wrap_whitespace(text[end]) {
                chunk_end = Some(end);
                break;
            }
            // Em-dash: (?<=wp)(?=-{2,}\w)
            if is_word_punct(text[end - 1]) && em_dash_at(end) {
                chunk_end = Some(end);
                break;
            }
            end += 1;
        }
        let end = chunk_end.unwrap_or(length);
        chunks.push(text[start..end].iter().collect());
        start = end;
    }
    chunks
}

fn char_len(text: &str) -> usize {
    text.chars().count()
}

/// `TextWrapper._wrap_chunks` with the default options.
fn wrap_chunks(
    mut chunks: Vec<String>,
    width: usize,
    initial_indent: &str,
    subsequent_indent: &str,
) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    chunks.reverse();
    while !chunks.is_empty() {
        let mut cur_line: Vec<String> = Vec::new();
        let mut cur_len = 0usize;
        let indent = if lines.is_empty() {
            initial_indent
        } else {
            subsequent_indent
        };
        let line_width = width as i64 - char_len(indent) as i64;
        if chunks
            .last()
            .is_some_and(|chunk| chunk.trim_matches(is_python_strip_char).is_empty())
            && !lines.is_empty()
        {
            chunks.pop();
        }
        while let Some(chunk) = chunks.last() {
            let length = char_len(chunk) as i64;
            if cur_len as i64 + length <= line_width {
                cur_len += length as usize;
                cur_line.push(chunks.pop().expect("chunk exists"));
            } else {
                break;
            }
        }
        if chunks
            .last()
            .is_some_and(|chunk| char_len(chunk) as i64 > line_width)
        {
            handle_long_word(&mut chunks, &mut cur_line, cur_len, line_width);
            cur_len = cur_line.iter().map(|chunk| char_len(chunk)).sum();
        }
        if cur_line
            .last()
            .is_some_and(|chunk| chunk.trim_matches(is_python_strip_char).is_empty())
        {
            let last = cur_line.pop().expect("line has a chunk");
            cur_len -= char_len(&last);
        }
        let _ = cur_len;
        if !cur_line.is_empty() {
            lines.push(format!("{indent}{}", cur_line.concat()));
        }
    }
    lines
}

/// `TextWrapper._handle_long_word` with `break_long_words=True` and
/// `break_on_hyphens=True`.
fn handle_long_word(
    reversed_chunks: &mut [String],
    cur_line: &mut Vec<String>,
    cur_len: usize,
    width: i64,
) {
    let space_left = if width < 1 { 1 } else { width - cur_len as i64 };
    if space_left > 0 {
        let space_left = space_left as usize;
        let chunk = reversed_chunks.last().expect("a long chunk exists").clone();
        let characters = chunk.chars().collect::<Vec<_>>();
        let mut end = space_left;
        if characters.len() > space_left {
            let limit = space_left.min(characters.len());
            if let Some(hyphen) = characters[..limit].iter().rposition(|ch| *ch == '-')
                && hyphen > 0
                && characters[..hyphen].iter().any(|ch| *ch != '-')
            {
                end = hyphen + 1;
            }
        }
        let end = end.min(characters.len());
        cur_line.push(characters[..end].iter().collect());
        let last = reversed_chunks.len() - 1;
        reversed_chunks[last] = characters[end..].iter().collect();
    }
}

/// Characters `str.strip()` removes (Unicode whitespace).
pub(super) fn is_python_strip_char(ch: char) -> bool {
    matches!(
        ch,
        '\t' | '\n' | '\x0b' | '\x0c' | '\r' | '\x1c' | '\x1d' | '\x1e' | '\x1f' | ' ' | '\u{85}'
    ) || (ch as u32 > 0x7f && ch.is_whitespace())
}

/// `textwrap.wrap(text, width)`.
pub(super) fn wrap(text: &str, width: i64) -> Result<Vec<String>, String> {
    if width <= 0 {
        return Err(format!("ValueError: invalid width {width} (must be > 0)"));
    }
    Ok(wrap_chunks(split_chunks(text), width as usize, "", ""))
}

/// `textwrap.fill(text, width, initial_indent=indent, subsequent_indent=indent)`.
pub(super) fn fill(text: &str, width: i64, indent: &str) -> Result<String, String> {
    if width <= 0 {
        return Err(format!("ValueError: invalid width {width} (must be > 0)"));
    }
    Ok(wrap_chunks(split_chunks(text), width as usize, indent, indent).join("\n"))
}

/// `re.compile(r'\s+', re.ASCII).sub(' ', text).strip()`.
pub(super) fn collapse_whitespace(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut in_space = false;
    for ch in text.chars() {
        if matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c') {
            if !in_space {
                result.push(' ');
                in_space = true;
            }
        } else {
            result.push(ch);
            in_space = false;
        }
    }
    result.trim_matches(is_python_strip_char).to_string()
}
