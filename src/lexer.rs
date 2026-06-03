use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::OnceLock;

use encoding_rs::Encoding;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use unic_ucd_name_aliases::{NameAliasType, name_aliases_of};
use unicode_normalization::UnicodeNormalization;

const MAX_INDENT_LEVELS: usize = 100;
const MAX_BRACKET_DEPTH: usize = 200;
const DEFAULT_MAX_INT_STR_DIGITS: usize = 4300;
const MIN_MAX_INT_STR_DIGITS: usize = 640;
const TABSIZE: usize = 8;

thread_local! {
    static MAX_INT_STR_DIGITS: Cell<usize> = const { Cell::new(DEFAULT_MAX_INT_STR_DIGITS) };
    static SOURCE_LOCATION_CACHE: RefCell<Option<SourceLocationCache>> = const { RefCell::new(None) };
}

#[derive(Debug, Clone)]
struct SourceLocationCache {
    ptr: usize,
    len: usize,
    line_starts: Vec<usize>,
    byte_offsets: Vec<usize>,
    byte_line_starts: Vec<usize>,
}

impl SourceLocationCache {
    fn new(chars: &[char]) -> Self {
        let mut byte_offsets = Vec::with_capacity(chars.len() + 1);
        byte_offsets.push(0);
        for ch in chars {
            let next = byte_offsets.last().copied().unwrap_or(0) + ch.len_utf8();
            byte_offsets.push(next);
        }

        let mut line_starts = vec![0];
        let mut byte_line_starts = vec![0];
        let mut current = 0usize;
        while current < chars.len() {
            match chars[current] {
                '\n' => {
                    current += 1;
                    line_starts.push(current);
                    byte_line_starts.push(byte_offsets[current]);
                }
                '\r' => {
                    current += 1;
                    if current < chars.len() && chars[current] == '\n' {
                        current += 1;
                    }
                    line_starts.push(current);
                    byte_line_starts.push(byte_offsets[current]);
                }
                _ => current += 1,
            }
        }

        Self {
            ptr: chars.as_ptr() as usize,
            len: chars.len(),
            line_starts,
            byte_offsets,
            byte_line_starts,
        }
    }

    fn char_location(&self, index: usize) -> (usize, usize) {
        let line_index = self.line_index(index);
        let column = index.saturating_sub(self.line_starts[line_index]) + 1;
        (line_index + 1, column)
    }

    fn byte_location(&self, index: usize) -> (usize, usize) {
        let line_index = self.line_index(index);
        let column = self.byte_offsets[index].saturating_sub(self.byte_line_starts[line_index]) + 1;
        (line_index + 1, column)
    }

    fn line_index(&self, index: usize) -> usize {
        self.line_starts.partition_point(|start| *start <= index) - 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Indentation {
    column: usize,
    alt_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Encoding(String),
    Identifier(String),
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Async,
    Await,
    Def,
    Class,
    Lambda,
    Return,
    Yield,
    Raise,
    Del,
    Global,
    Nonlocal,
    Assert,
    Try,
    Except,
    With,
    As,
    Finally,
    From,
    Import,
    Break,
    Continue,
    Pass,
    And,
    Or,
    Not,
    Is,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    LeftBrace,
    RightBrace,
    Dot,
    Ellipsis,
    At,
    AtEqual,
    Comma,
    Colon,
    ColonEqual,
    Semicolon,
    Newline,
    Indent,
    Dedent,
    Plus,
    PlusEqual,
    Minus,
    MinusEqual,
    Arrow,
    Star,
    StarEqual,
    Slash,
    SlashEqual,
    DoubleSlash,
    DoubleSlashEqual,
    Percent,
    PercentEqual,
    DoubleStar,
    DoubleStarEqual,
    Pipe,
    PipeEqual,
    Caret,
    CaretEqual,
    Ampersand,
    AmpersandEqual,
    Tilde,
    LeftShift,
    LeftShiftEqual,
    RightShift,
    RightShiftEqual,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Number(i64),
    BigInt(String),
    Float(String),
    Imaginary(String),
    String(String),
    Bytes(Vec<u8>),
    FString(Vec<TokenFStringPart>),
    TString(Vec<TokenFStringPart>),
    FStringStart(String),
    FStringMiddle(String),
    FStringEnd(String),
    TStringStart(String),
    TStringMiddle(String),
    TStringEnd(String),
    TypeComment(String),
    TypeIgnore(String),
    Exclamation,
    True,
    False,
    None,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenFStringPart {
    Literal(String),
    Expression {
        source: String,
        conversion: Option<TokenFStringConversion>,
        format_spec: Option<Vec<TokenFStringPart>>,
        debug_label: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenFStringConversion {
    Str,
    Repr,
    Ascii,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexWarning {
    pub category: LexWarningCategory,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexWarningCategory {
    SyntaxWarning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceEncoding {
    pub encoding: String,
    pub consumed_lines: Vec<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpannedToken {
    pub token: Token,
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub byte_column: usize,
    pub byte_end_column: usize,
}

fn push_token(
    tokens: &mut Vec<SpannedToken>,
    chars: &[char],
    token: Token,
    start: usize,
    end: usize,
) {
    let start = start.min(chars.len());
    let end = end.min(chars.len()).max(start);
    let (line, column) = source_location(chars, start);
    let (end_line, end_column) = source_location(chars, end);
    let (_, byte_column) = source_byte_location(chars, start);
    let (_, byte_end_column) = source_byte_location(chars, end);
    tokens.push(SpannedToken {
        token,
        start,
        end,
        line,
        column,
        end_line,
        end_column,
        byte_column,
        byte_end_column,
    });
}

fn push_synthetic_token(
    tokens: &mut Vec<SpannedToken>,
    token: Token,
    index: usize,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
) {
    tokens.push(SpannedToken {
        token,
        start: index,
        end: index,
        line,
        column,
        end_line,
        end_column,
        byte_column: column,
        byte_end_column: end_column,
    });
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    lex_with_warnings(source).map(|(tokens, _warnings)| tokens)
}

pub(crate) fn lex_for_parse(source: &str) -> Result<Vec<Token>, String> {
    lex_with_warnings_for_parse(source).map(|(tokens, _warnings)| tokens)
}

pub fn lex_with_warnings(source: &str) -> Result<(Vec<Token>, Vec<LexWarning>), String> {
    lex_with_diagnostics(source).map_err(|error| error.message)
}

pub(crate) fn lex_with_warnings_for_parse(
    source: &str,
) -> Result<(Vec<Token>, Vec<LexWarning>), String> {
    let (tokens, warnings) =
        lex_with_spans_mode(source, LexMode::Parse).map_err(|error| error.message)?;
    Ok((
        tokens.into_iter().map(|spanned| spanned.token).collect(),
        warnings,
    ))
}

pub fn lex_with_diagnostics(source: &str) -> Result<(Vec<Token>, Vec<LexWarning>), LexError> {
    let (tokens, warnings) = lex_with_spans(source)?;
    Ok((
        tokens.into_iter().map(|spanned| spanned.token).collect(),
        warnings,
    ))
}

pub fn lex_with_spans(source: &str) -> Result<(Vec<SpannedToken>, Vec<LexWarning>), LexError> {
    lex_with_spans_mode(source, LexMode::Strict)
}

pub fn get_int_max_str_digits() -> usize {
    MAX_INT_STR_DIGITS.with(Cell::get)
}

pub fn set_int_max_str_digits(maxdigits: usize) -> Result<(), String> {
    if maxdigits != 0 && maxdigits < MIN_MAX_INT_STR_DIGITS {
        return Err(format!(
            "ValueError: maxdigits must be 0 or larger than {MIN_MAX_INT_STR_DIGITS}"
        ));
    }

    MAX_INT_STR_DIGITS.with(|limit| limit.set(maxdigits));
    Ok(())
}

pub(crate) fn lex_with_spans_for_parse(
    source: &str,
) -> Result<(Vec<SpannedToken>, Vec<LexWarning>), LexError> {
    lex_with_spans_mode(source, LexMode::Parse)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionLineMetadata {
    token_index: usize,
    first_line: usize,
    line_sequence: Vec<usize>,
}

pub(crate) fn function_definition_start_lines(tokens: &[SpannedToken]) -> Vec<usize> {
    function_line_metadata(tokens)
        .into_iter()
        .map(|metadata| metadata.first_line)
        .collect()
}

pub(crate) fn function_definition_line_sequences(tokens: &[SpannedToken]) -> Vec<Vec<usize>> {
    function_line_metadata(tokens)
        .into_iter()
        .map(|metadata| metadata.line_sequence)
        .collect()
}

pub(crate) fn generator_expression_line_sequences(
    tokens: &[SpannedToken],
) -> Vec<(usize, Vec<usize>)> {
    generator_expression_line_metadata(tokens)
        .into_iter()
        .map(|metadata| (metadata.first_line, metadata.line_sequence))
        .collect()
}

fn function_line_metadata(tokens: &[SpannedToken]) -> Vec<FunctionLineMetadata> {
    explicit_function_line_metadata(tokens)
}

fn explicit_function_line_metadata(tokens: &[SpannedToken]) -> Vec<FunctionLineMetadata> {
    let mut metadata = Vec::new();
    let mut at_statement_start = true;
    let mut pending_decorator_line = None;

    for (index, token) in tokens.iter().enumerate() {
        match &token.token {
            Token::Encoding(_)
            | Token::TypeComment(_)
            | Token::TypeIgnore(_)
            | Token::Indent
            | Token::Dedent
            | Token::Newline
            | Token::Semicolon => {
                at_statement_start = true;
            }
            Token::At if at_statement_start => {
                pending_decorator_line.get_or_insert(token.line);
                at_statement_start = false;
            }
            Token::Def => {
                let first_line = pending_decorator_line.take().unwrap_or(token.line);
                let line_sequence = function_body_token_range(tokens, index)
                    .map(|range| function_body_line_sequence(first_line, &tokens[range]))
                    .unwrap_or_else(|| vec![first_line]);
                metadata.push(FunctionLineMetadata {
                    token_index: index,
                    first_line,
                    line_sequence,
                });
                at_statement_start = false;
            }
            Token::Class if at_statement_start => {
                pending_decorator_line = None;
                at_statement_start = false;
            }
            Token::Eof => break,
            _ => {
                if at_statement_start {
                    pending_decorator_line = None;
                }
                at_statement_start = false;
            }
        }
    }

    metadata
}

fn generator_expression_line_metadata(tokens: &[SpannedToken]) -> Vec<FunctionLineMetadata> {
    let mut metadata = Vec::new();
    let mut paren_depth = 0usize;

    for (index, token) in tokens.iter().enumerate() {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => paren_depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            Token::For if paren_depth > 0 => {
                let Some(left_index) = nearest_unmatched_left_paren(tokens, index) else {
                    continue;
                };
                let Some(in_index) = comprehension_in_index(tokens, index) else {
                    continue;
                };
                let element_line = first_significant_line(&tokens[left_index + 1..index]);
                let target_line = first_significant_line(&tokens[index + 1..in_index]);
                let iter_line = first_significant_line(&tokens[in_index + 1..]);
                if let (Some(element_line), Some(target_line), Some(iter_line)) =
                    (element_line, target_line, iter_line)
                {
                    metadata.push(FunctionLineMetadata {
                        token_index: index,
                        first_line: element_line,
                        line_sequence: vec![
                            iter_line,
                            element_line,
                            iter_line,
                            target_line,
                            element_line,
                            iter_line,
                        ],
                    });
                }
            }
            _ => {}
        }
    }

    metadata
}

fn nearest_unmatched_left_paren(tokens: &[SpannedToken], before: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().take(before).rev() {
        match token.token {
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth += 1,
            Token::LeftParen => {
                if depth == 0 {
                    return Some(index);
                }
                depth = depth.saturating_sub(1);
            }
            Token::LeftBracket | Token::LeftBrace => {
                if depth == 0 {
                    return None;
                }
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }
    None
}

fn comprehension_in_index(tokens: &[SpannedToken], for_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(for_index + 1) {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                if depth == 0 {
                    return None;
                }
                depth = depth.saturating_sub(1);
            }
            Token::In if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn first_significant_line(tokens: &[SpannedToken]) -> Option<usize> {
    trim_line_metadata_tokens(tokens).iter().find_map(|token| {
        if is_line_metadata_ignorable(&token.token) {
            None
        } else {
            Some(token.line)
        }
    })
}

fn function_body_token_range(
    tokens: &[SpannedToken],
    def_index: usize,
) -> Option<std::ops::Range<usize>> {
    let colon = find_function_header_colon(tokens, def_index)?;
    let mut start = colon + 1;

    while matches!(
        tokens.get(start).map(|token| &token.token),
        Some(Token::TypeComment(_))
    ) {
        start += 1;
    }

    if matches!(
        tokens.get(start).map(|token| &token.token),
        Some(Token::Newline)
    ) {
        start += 1;
        if !matches!(
            tokens.get(start).map(|token| &token.token),
            Some(Token::Indent)
        ) {
            return None;
        }
        start += 1;

        let mut depth = 1usize;
        let mut end = start;
        while let Some(token) = tokens.get(end) {
            match token.token {
                Token::Indent => depth += 1,
                Token::Dedent => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some(start..end);
                    }
                }
                Token::Eof => return Some(start..end),
                _ => {}
            }
            end += 1;
        }
        return Some(start..end);
    }

    let mut end = start;
    while let Some(token) = tokens.get(end) {
        if matches!(token.token, Token::Newline | Token::Dedent | Token::Eof) {
            break;
        }
        end += 1;
    }
    Some(start..end)
}

fn find_function_header_colon(tokens: &[SpannedToken], def_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(def_index + 1) {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1);
            }
            Token::Colon if depth == 0 => return Some(index),
            Token::Newline | Token::Eof => return None,
            _ => {}
        }
    }
    None
}

fn function_body_line_sequence(first_line: usize, tokens: &[SpannedToken]) -> Vec<usize> {
    let mut lines = vec![first_line];
    let sequence = block_line_metadata(tokens, false);
    lines.extend(sequence.normal);
    lines.extend(sequence.cold);
    lines
}

fn split_function_body_statements(tokens: &[SpannedToken]) -> Vec<&[SpannedToken]> {
    let mut statements = Vec::new();
    let mut start = 0usize;
    let mut paren_depth = 0usize;
    let mut block_depth = 0usize;

    for (index, token) in tokens.iter().enumerate() {
        match token.token {
            Token::Indent => block_depth += 1,
            Token::Dedent => {
                block_depth = block_depth.saturating_sub(1);
                if paren_depth == 0 && block_depth == 0 {
                    if next_token_continues_compound_statement(tokens, index + 1) {
                        continue;
                    }
                    if start < index {
                        statements.push(&tokens[start..=index]);
                    }
                    start = index + 1;
                }
            }
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => paren_depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            Token::Semicolon | Token::Newline if paren_depth == 0 && block_depth == 0 => {
                if matches!(token.token, Token::Newline)
                    && matches!(
                        tokens.get(index + 1).map(|token| &token.token),
                        Some(Token::Indent)
                    )
                {
                    continue;
                }
                if start < index {
                    statements.push(&tokens[start..index]);
                }
                start = index + 1;
            }
            _ => {}
        }
    }

    if start < tokens.len() {
        statements.push(&tokens[start..]);
    }

    statements
}

fn next_token_continues_compound_statement(tokens: &[SpannedToken], start: usize) -> bool {
    tokens
        .iter()
        .skip(start)
        .find(|token| !matches!(token.token, Token::Newline))
        .is_some_and(|token| {
            matches!(
                token.token,
                Token::Elif | Token::Else | Token::Except | Token::Finally
            )
        })
}

#[derive(Default)]
struct StatementLineSequence {
    normal: Vec<usize>,
    cold: Vec<usize>,
}

impl StatementLineSequence {
    fn from_normal(normal: Vec<usize>) -> Self {
        Self {
            normal,
            cold: Vec::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.normal.is_empty() && self.cold.is_empty()
    }
}

fn statement_line_metadata(tokens: &[SpannedToken]) -> StatementLineSequence {
    let tokens = trim_line_metadata_tokens(tokens);
    let Some(first) = tokens.first() else {
        return StatementLineSequence::default();
    };

    if matches!(
        first.token,
        Token::String(_) | Token::FString(_) | Token::TString(_)
    ) {
        return StatementLineSequence::default();
    }

    if matches!(
        tokens,
        [
            SpannedToken {
                token: Token::Async,
                ..
            },
            SpannedToken {
                token: Token::For,
                ..
            },
            ..
        ]
    ) {
        return loop_statement_line_metadata(tokens, 1);
    }

    if matches!(first.token, Token::For | Token::While) {
        return loop_statement_line_metadata(tokens, 0);
    }

    if matches!(first.token, Token::If) {
        return conditional_statement_line_metadata(tokens);
    }

    if matches!(first.token, Token::Try) {
        return try_statement_line_metadata(tokens);
    }

    if is_annotation_only_statement(tokens) {
        return StatementLineSequence::default();
    }

    if matches!(first.token, Token::Return) {
        let expr = trim_line_metadata_tokens(&tokens[1..]);
        let mut lines = expression_line_sequence(expr);
        lines.push(first.line);
        return StatementLineSequence::from_normal(lines);
    }

    if let Some(index) = top_level_assignment_operator_index(tokens) {
        let target = trim_line_metadata_tokens(&tokens[..index]);
        let value = trim_line_metadata_tokens(&tokens[index + 1..]);
        if matches!(tokens[index].token, Token::Equal) {
            let mut lines = expression_line_sequence(value);
            lines.extend(attribute_expression_line_sequence(target));
            return StatementLineSequence::from_normal(lines);
        }

        let target_lines = attribute_expression_line_sequence(target);
        let mut lines = target_lines.clone();
        lines.extend(expression_line_sequence(value));
        lines.push(first.line);
        if let Some(line) = target_lines.last() {
            lines.push(*line);
        }
        return StatementLineSequence::from_normal(lines);
    }

    StatementLineSequence::from_normal(expression_line_sequence(tokens))
}

fn loop_statement_line_metadata(
    tokens: &[SpannedToken],
    keyword_index: usize,
) -> StatementLineSequence {
    let Some(keyword) = tokens.get(keyword_index) else {
        return StatementLineSequence::default();
    };
    let Some(loop_colon) = top_level_colon_index(tokens) else {
        return StatementLineSequence::default();
    };
    let else_index = top_level_loop_else_index(tokens);
    let body_end = else_index.unwrap_or(tokens.len());
    let body_sequence = clause_body_line_metadata(tokens, loop_colon, body_end);

    let mut lines = vec![keyword.line];
    let mut body_lines = body_sequence.normal;
    drop_nested_compound_backedge_line(&mut body_lines);
    lines.extend(body_lines);

    let is_while = matches!(keyword.token, Token::While);
    let has_else = else_index.is_some();
    if (!is_while || !has_else) && (!is_while || loop_body_can_reach_backedge(&lines, keyword.line))
    {
        lines.push(keyword.line);
    }

    if let Some(else_index) = else_index
        && let Some(else_colon) =
            top_level_colon_index(&tokens[else_index..]).map(|index| else_index + index)
    {
        let else_body = clause_body_line_metadata(tokens, else_colon, tokens.len());
        lines.extend(else_body.normal);
        let mut cold = body_sequence.cold;
        cold.extend(else_body.cold);
        if is_while {
            drop_nested_compound_backedge_line(&mut cold);
        }
        return StatementLineSequence {
            normal: lines,
            cold,
        };
    }

    StatementLineSequence {
        normal: lines,
        cold: body_sequence.cold,
    }
}

fn top_level_loop_else_index(tokens: &[SpannedToken]) -> Option<usize> {
    let mut paren_depth = 0usize;
    let mut block_depth = 0usize;

    for (index, token) in tokens.iter().enumerate().skip(1) {
        match token.token {
            Token::Indent => block_depth += 1,
            Token::Dedent => block_depth = block_depth.saturating_sub(1),
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => paren_depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            Token::Else if paren_depth == 0 && block_depth == 0 => return Some(index),
            _ => {}
        }
    }

    None
}

fn loop_body_can_reach_backedge(lines: &[usize], loop_line: usize) -> bool {
    lines
        .last()
        .is_some_and(|line| *line != loop_line && lines.len() > 2)
}

fn drop_nested_compound_backedge_line(lines: &mut Vec<usize>) {
    if lines.len() > 1 && lines.first() == lines.last() {
        lines.pop();
    }
}

fn conditional_statement_line_metadata(tokens: &[SpannedToken]) -> StatementLineSequence {
    let Some(keyword) = tokens.first() else {
        return StatementLineSequence::default();
    };
    let mut lines = vec![keyword.line];
    let body_sequence = compound_body_line_metadata(tokens);
    lines.extend(body_sequence.normal);
    lines.push(keyword.line);
    StatementLineSequence {
        normal: lines,
        cold: body_sequence.cold,
    }
}

fn try_statement_line_metadata(tokens: &[SpannedToken]) -> StatementLineSequence {
    let Some(keyword) = tokens.first() else {
        return StatementLineSequence::default();
    };
    let Some(try_colon) = top_level_colon_index(tokens) else {
        return StatementLineSequence::default();
    };

    let clause_indices = top_level_try_clause_indices(tokens);
    let try_body_end = clause_indices.first().copied().unwrap_or(tokens.len());
    let try_body = clause_body_line_metadata(tokens, try_colon, try_body_end);

    let mut sequence = StatementLineSequence {
        normal: vec![keyword.line],
        cold: try_body.cold,
    };
    sequence.normal.extend(try_body.normal);

    for (position, clause_index) in clause_indices.iter().copied().enumerate() {
        let clause_end = clause_indices
            .get(position + 1)
            .copied()
            .unwrap_or(tokens.len());
        let Some(clause_colon) = top_level_colon_index(&tokens[clause_index..clause_end])
            .map(|index| clause_index + index)
        else {
            continue;
        };
        let clause_body = clause_body_line_metadata(tokens, clause_colon, clause_end);
        let clause_line = tokens[clause_index].line;

        match tokens[clause_index].token {
            Token::Except => {
                sequence.cold.push(clause_line);
                sequence.cold.extend(clause_body.normal);
                sequence.cold.extend(clause_body.cold);
                sequence.cold.push(clause_line);
            }
            Token::Else | Token::Finally => {
                sequence.normal.push(clause_line);
                sequence.normal.extend(clause_body.normal);
                sequence.cold.extend(clause_body.cold);
            }
            _ => {}
        }
    }

    sequence
}

fn top_level_try_clause_indices(tokens: &[SpannedToken]) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut paren_depth = 0usize;
    let mut block_depth = 0usize;

    for (index, token) in tokens.iter().enumerate().skip(1) {
        match token.token {
            Token::Indent => block_depth += 1,
            Token::Dedent => block_depth = block_depth.saturating_sub(1),
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => paren_depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                paren_depth = paren_depth.saturating_sub(1);
            }
            Token::Except | Token::Else | Token::Finally
                if paren_depth == 0 && block_depth == 0 =>
            {
                indices.push(index);
            }
            _ => {}
        }
    }

    indices
}

fn clause_body_line_metadata(
    tokens: &[SpannedToken],
    colon_index: usize,
    end_index: usize,
) -> StatementLineSequence {
    if colon_index + 1 >= end_index {
        return StatementLineSequence::default();
    }
    let body = trim_line_metadata_tokens(&tokens[colon_index + 1..end_index]);
    block_line_metadata(body, true)
}

fn compound_body_line_metadata(tokens: &[SpannedToken]) -> StatementLineSequence {
    let Some(colon) = top_level_colon_index(tokens) else {
        return StatementLineSequence::default();
    };
    let body = trim_line_metadata_tokens(&tokens[colon + 1..]);
    block_line_metadata(body, true)
}

fn block_line_metadata(tokens: &[SpannedToken], fallback_empty: bool) -> StatementLineSequence {
    let mut sequence = StatementLineSequence::default();
    for statement in split_function_body_statements(tokens) {
        let statement = trim_line_metadata_tokens(statement);
        let statement_sequence = statement_line_metadata(statement);
        if statement_sequence.is_empty() {
            if let Some(first) = statement.first() {
                if fallback_empty {
                    push_line_if_changed(&mut sequence.normal, first.line);
                }
            }
        } else {
            sequence.normal.extend(statement_sequence.normal);
            sequence.cold.extend(statement_sequence.cold);
        }
    }

    sequence
}

fn top_level_colon_index(tokens: &[SpannedToken]) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1);
            }
            Token::Colon if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}

fn trim_line_metadata_tokens(tokens: &[SpannedToken]) -> &[SpannedToken] {
    let mut start = 0usize;
    let mut end = tokens.len();

    while start < end && is_line_metadata_ignorable(&tokens[start].token) {
        start += 1;
    }
    while end > start && is_line_metadata_ignorable(&tokens[end - 1].token) {
        end -= 1;
    }

    &tokens[start..end]
}

fn is_line_metadata_ignorable(token: &Token) -> bool {
    matches!(
        token,
        Token::Encoding(_)
            | Token::TypeComment(_)
            | Token::TypeIgnore(_)
            | Token::Newline
            | Token::Indent
            | Token::Dedent
            | Token::Eof
    )
}

fn is_annotation_only_statement(tokens: &[SpannedToken]) -> bool {
    let mut depth = 0usize;
    let mut has_colon = false;
    for token in tokens {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1);
            }
            Token::Colon if depth == 0 => has_colon = true,
            Token::Equal if depth == 0 => return false,
            _ => {}
        }
    }
    has_colon
}

fn top_level_assignment_operator_index(tokens: &[SpannedToken]) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate() {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1);
            }
            Token::Equal
            | Token::PlusEqual
            | Token::MinusEqual
            | Token::StarEqual
            | Token::SlashEqual
            | Token::DoubleSlashEqual
            | Token::PercentEqual
            | Token::AtEqual
            | Token::DoubleStarEqual
            | Token::AmpersandEqual
            | Token::PipeEqual
            | Token::CaretEqual
            | Token::LeftShiftEqual
            | Token::RightShiftEqual
                if depth == 0 =>
            {
                return Some(index);
            }
            _ => {}
        }
    }
    None
}

fn expression_line_sequence(tokens: &[SpannedToken]) -> Vec<usize> {
    let tokens = trim_line_metadata_tokens(tokens);
    if tokens.is_empty() {
        return Vec::new();
    }

    let call_lines = call_expression_line_sequence(tokens);
    if !call_lines.is_empty() {
        return call_lines;
    }

    let attr_lines = attribute_expression_line_sequence(tokens);
    if !attr_lines.is_empty() {
        return attr_lines;
    }

    token_expression_lines(tokens)
}

fn call_expression_line_sequence(tokens: &[SpannedToken]) -> Vec<usize> {
    for (index, token) in tokens.iter().enumerate() {
        if !matches!(token.token, Token::LeftParen) {
            continue;
        }

        let callee = trim_line_metadata_tokens(&tokens[..index]);
        let callee_lines = attribute_expression_line_sequence(callee);
        let Some(call_line) = callee_lines.last().copied() else {
            continue;
        };
        let args_end = matching_right_paren_index(tokens, index).unwrap_or(tokens.len());
        let args = if args_end > index + 1 {
            &tokens[index + 1..args_end]
        } else {
            &[]
        };

        let mut lines = callee_lines;
        lines.extend(token_expression_lines(args));
        lines.push(call_line);
        return lines;
    }

    Vec::new()
}

fn matching_right_paren_index(tokens: &[SpannedToken], left_index: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (index, token) in tokens.iter().enumerate().skip(left_index) {
        match token.token {
            Token::LeftParen => depth += 1,
            Token::RightParen => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn attribute_expression_line_sequence(tokens: &[SpannedToken]) -> Vec<usize> {
    let tokens = trim_wrapping_parentheses(trim_line_metadata_tokens(tokens));
    let mut lines = Vec::new();
    let mut expect_attribute_name = false;

    for token in tokens {
        match &token.token {
            Token::Identifier(_) if !expect_attribute_name && lines.is_empty() => {
                lines.push(token.line);
            }
            Token::Identifier(_) if expect_attribute_name => {
                lines.push(token.line);
                expect_attribute_name = false;
            }
            Token::Dot if !lines.is_empty() => {
                expect_attribute_name = true;
            }
            Token::LeftParen | Token::RightParen => {}
            _ => return Vec::new(),
        }
    }

    if lines.len() >= 2 { lines } else { Vec::new() }
}

fn trim_wrapping_parentheses(mut tokens: &[SpannedToken]) -> &[SpannedToken] {
    loop {
        let Some(first) = tokens.first() else {
            return tokens;
        };
        if !matches!(first.token, Token::LeftParen) {
            return tokens;
        }
        let Some(last_index) = matching_right_paren_index(tokens, 0) else {
            return tokens;
        };
        if last_index + 1 != tokens.len() {
            return tokens;
        }
        tokens = &tokens[1..last_index];
    }
}

fn token_expression_lines(tokens: &[SpannedToken]) -> Vec<usize> {
    let mut lines = Vec::new();
    for token in tokens {
        if matches!(
            token.token,
            Token::Identifier(_)
                | Token::Number(_)
                | Token::BigInt(_)
                | Token::Float(_)
                | Token::Imaginary(_)
                | Token::String(_)
                | Token::Bytes(_)
                | Token::FString(_)
                | Token::TString(_)
                | Token::True
                | Token::False
                | Token::None
                | Token::Ellipsis
        ) {
            push_line_if_changed(&mut lines, token.line);
        }
    }
    lines
}

fn push_line_if_changed(lines: &mut Vec<usize>, line: usize) {
    if lines.last().copied() != Some(line) {
        lines.push(line);
    }
}

pub fn tokenize_with_spans(source: &str) -> Result<Vec<SpannedToken>, LexError> {
    let (tokens, _warnings) = lex_with_spans_mode(source, LexMode::Tokenize)?;
    Ok(tokens)
}

pub fn tokenize_cpython_with_spans(source: &str) -> Result<Vec<SpannedToken>, LexError> {
    let tokens = tokenize_with_spans(source)?;
    let chars = source.chars().collect::<Vec<_>>();
    prepare_source_location_cache(&chars);
    let mut expanded = Vec::with_capacity(tokens.len());

    for token in tokens {
        match &token.token {
            Token::FString(_) => expand_cpython_interpolated_string(
                &chars,
                &mut expanded,
                &token,
                InterpolatedStringKind::Formatted,
            )?,
            Token::TString(_) => expand_cpython_interpolated_string(
                &chars,
                &mut expanded,
                &token,
                InterpolatedStringKind::Template,
            )?,
            _ => expanded.push(token),
        }
    }

    Ok(expanded)
}

pub(crate) fn decode_source_for_parse(source: &[u8]) -> Result<String, String> {
    let lines = source_lines(source);
    let detected = detect_source_encoding(&lines)?;
    decode_source_bytes(source, &detected.encoding).map(normalize_source_newlines)
}

pub fn tokenize_bytes_with_spans(source: &[u8]) -> Result<Vec<SpannedToken>, String> {
    let lines = source_lines(source);
    let detected = detect_source_encoding(&lines)?;
    let decoded = normalize_source_newlines(decode_source_bytes(source, &detected.encoding)?);
    let mut tokens = tokenize_with_spans(&decoded)
        .map_err(|error| format!("{}: {}", error.message, error.line))?;
    tokens.insert(
        0,
        SpannedToken {
            token: Token::Encoding(encoding_token_name(&detected.encoding)),
            start: 0,
            end: 0,
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 0,
            byte_column: 0,
            byte_end_column: 0,
        },
    );
    Ok(tokens)
}

pub fn detect_source_encoding(lines: &[&[u8]]) -> Result<SourceEncoding, String> {
    let Some(first) = lines.first() else {
        return Ok(SourceEncoding {
            encoding: "utf-8".to_string(),
            consumed_lines: Vec::new(),
        });
    };

    let (first_line, has_bom) = strip_utf8_bom(first);
    reject_null_bytes(&first_line)?;

    if let Some(raw_encoding) = find_coding_cookie(&first_line) {
        let encoding = normalize_source_encoding(&raw_encoding)?;
        reject_bom_mismatch(has_bom, &encoding)?;
        validate_consumed_lines(&encoding, &[first_line.as_slice()])?;
        return Ok(SourceEncoding {
            encoding: detected_encoding(has_bom, &encoding),
            consumed_lines: consumed_first_line(first_line),
        });
    }

    let mut consumed_lines = consumed_first_line(first_line.clone());
    let mut candidate_lines = vec![first_line];

    if can_second_line_declare_encoding(candidate_lines[0].as_slice()) {
        if let Some(second) = lines.get(1) {
            let second_line = second.to_vec();
            reject_null_bytes(&second_line)?;
            if let Some(raw_encoding) = find_coding_cookie(&second_line) {
                let encoding = normalize_source_encoding(&raw_encoding)?;
                reject_bom_mismatch(has_bom, &encoding)?;
                candidate_lines.push(second_line.clone());
                if !second_line.is_empty() {
                    consumed_lines.push(second_line);
                }
                validate_consumed_lines(&encoding, &candidate_lines_as_slices(&candidate_lines))?;
                return Ok(SourceEncoding {
                    encoding: detected_encoding(has_bom, &encoding),
                    consumed_lines,
                });
            }

            candidate_lines.push(second_line.clone());
            if !second_line.is_empty() {
                consumed_lines.push(second_line);
            }
        }
    }

    validate_consumed_lines("utf-8", &candidate_lines_as_slices(&candidate_lines))?;
    Ok(SourceEncoding {
        encoding: if has_bom { "utf-8-sig" } else { "utf-8" }.to_string(),
        consumed_lines,
    })
}

fn source_lines(source: &[u8]) -> Vec<&[u8]> {
    let mut lines = Vec::new();
    let mut start = 0;
    let mut current = 0;
    while current < source.len() {
        if source[current] == b'\n' {
            lines.push(&source[start..=current]);
            current += 1;
            start = current;
        } else if source[current] == b'\r' {
            if source.get(current + 1) == Some(&b'\n') {
                lines.push(&source[start..=current + 1]);
                current += 2;
            } else {
                lines.push(&source[start..=current]);
                current += 1;
            }
            start = current;
        } else {
            current += 1;
        }
    }
    if start < source.len() {
        lines.push(&source[start..]);
    }
    lines
}

fn decode_source_bytes(source: &[u8], encoding: &str) -> Result<String, String> {
    let source = if matches!(encoding, "utf-8-sig") {
        source.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(source)
    } else {
        source
    };

    match encoding {
        "ascii" | "us-ascii" => decode_ascii(source),
        "utf-8" | "utf8" | "utf-8-sig" => std::str::from_utf8(source)
            .map(str::to_string)
            .map_err(|_| "'utf-8' codec can't decode byte".to_string()),
        "latin1" | "iso-8859-1" => Ok(source.iter().map(|byte| *byte as char).collect()),
        "iso8859-15" | "iso-8859-15" => Ok(source.iter().map(|byte| latin9_char(*byte)).collect()),
        "cp1252" | "windows-1252" => {
            decode_encoding_rs(source, encoding_rs::WINDOWS_1252, encoding)
        }
        "cp949" | "windows-949" | "euc-kr" => {
            decode_encoding_rs(source, encoding_rs::EUC_KR, encoding)
        }
        "cp932" | "ms932" | "windows-31j" | "shift-jis" => {
            decode_encoding_rs(source, encoding_rs::SHIFT_JIS, encoding)
        }
        other => match Encoding::for_label(other.as_bytes()) {
            Some(decoder) => decode_encoding_rs(source, decoder, other),
            None => Err(format!("unknown encoding: {other}")),
        },
    }
}

fn decode_encoding_rs(
    source: &[u8],
    decoder: &'static Encoding,
    encoding: &str,
) -> Result<String, String> {
    if source
        .iter()
        .any(|byte| is_python_undefined_encoding_byte(encoding, *byte))
    {
        return Err(format!("'{encoding}' codec can't decode byte"));
    }

    decoder
        .decode_without_bom_handling_and_without_replacement(source)
        .map(|decoded| decoded.into_owned())
        .ok_or_else(|| format!("'{encoding}' codec can't decode byte"))
}

fn is_python_undefined_encoding_byte(encoding: &str, byte: u8) -> bool {
    let normalized = encoding
        .chars()
        .filter(|ch| *ch != '-' && *ch != '_')
        .flat_map(char::to_lowercase)
        .collect::<String>();
    match normalized.as_str() {
        "cp1251" | "windows1251" => byte == 0x98,
        "cp1252" | "windows1252" => matches!(byte, 0x81 | 0x8d | 0x8f | 0x90 | 0x9d),
        _ => false,
    }
}

fn normalize_source_newlines(source: String) -> String {
    if !source.contains('\r') {
        return source;
    }

    let mut normalized = String::with_capacity(source.len());
    let mut chars = source.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            normalized.push('\n');
        } else {
            normalized.push(ch);
        }
    }
    normalized
}

fn decode_ascii(source: &[u8]) -> Result<String, String> {
    if source.iter().any(|byte| *byte > 0x7f) {
        return Err("'ascii' codec can't decode byte".to_string());
    }
    Ok(source.iter().map(|byte| *byte as char).collect())
}

fn latin9_char(byte: u8) -> char {
    match byte {
        0xA4 => '\u{20AC}',
        0xA6 => '\u{0160}',
        0xA8 => '\u{0161}',
        0xB4 => '\u{017D}',
        0xB8 => '\u{017E}',
        0xBC => '\u{0152}',
        0xBD => '\u{0153}',
        0xBE => '\u{0178}',
        _ => byte as char,
    }
}

fn encoding_token_name(encoding: &str) -> String {
    if encoding == "utf-8-sig" {
        "utf-8".to_string()
    } else {
        encoding.to_string()
    }
}

fn strip_utf8_bom(line: &[u8]) -> (Vec<u8>, bool) {
    const UTF8_BOM: &[u8] = b"\xEF\xBB\xBF";
    if let Some(stripped) = line.strip_prefix(UTF8_BOM) {
        (stripped.to_vec(), true)
    } else {
        (line.to_vec(), false)
    }
}

fn consumed_first_line(line: Vec<u8>) -> Vec<Vec<u8>> {
    if line.is_empty() {
        Vec::new()
    } else {
        vec![line]
    }
}

fn candidate_lines_as_slices(lines: &[Vec<u8>]) -> Vec<&[u8]> {
    lines.iter().map(Vec::as_slice).collect()
}

fn reject_null_bytes(line: &[u8]) -> Result<(), String> {
    if line.contains(&0) {
        Err("source code cannot contain null bytes".to_string())
    } else {
        Ok(())
    }
}

fn find_coding_cookie(line: &[u8]) -> Option<String> {
    let mut current = 0;
    while current < line.len() && matches!(line[current], b' ' | b'\t' | b'\x0c') {
        current += 1;
    }
    if line.get(current) != Some(&b'#') {
        return None;
    }

    let mut search = current + 1;
    while search + b"coding".len() <= line.len() {
        if line[search..].starts_with(b"coding") {
            let mut after = search + b"coding".len();
            if matches!(line.get(after), Some(b':' | b'=')) {
                after += 1;
                while after < line.len() && matches!(line[after], b' ' | b'\t') {
                    after += 1;
                }
                let name_start = after;
                while after < line.len()
                    && matches!(
                        line[after],
                        b'a'..=b'z'
                            | b'A'..=b'Z'
                            | b'0'..=b'9'
                            | b'-'
                            | b'_'
                            | b'.'
                    )
                {
                    after += 1;
                }
                if after > name_start {
                    return Some(String::from_utf8_lossy(&line[name_start..after]).into_owned());
                }
            }
        }
        search += 1;
    }

    None
}

fn normalize_source_encoding(raw: &str) -> Result<String, String> {
    let lower = raw.to_ascii_lowercase();
    let hyphenated = lower.replace('_', "-");
    let normal_probe = hyphenated.chars().take(12).collect::<String>();

    let normalized = if normal_probe == "utf-8" || normal_probe.starts_with("utf-8-") {
        "utf-8".to_string()
    } else if matches!(
        normal_probe.as_str(),
        "latin" | "latin-1" | "iso-8859-1" | "iso-latin-1"
    ) || normal_probe.starts_with("latin-1-")
        || normal_probe.starts_with("iso-8859-1-")
        || normal_probe.starts_with("iso-latin-1-")
    {
        "iso-8859-1".to_string()
    } else if is_known_source_encoding(&hyphenated)
        || Encoding::for_label(hyphenated.as_bytes()).is_some()
    {
        hyphenated
    } else {
        return Err(format!("unknown encoding: {raw}"));
    };

    Ok(normalized)
}

fn is_known_source_encoding(encoding: &str) -> bool {
    matches!(
        encoding,
        "ascii"
            | "us-ascii"
            | "utf-8"
            | "utf8"
            | "latin"
            | "latin1"
            | "iso-8859-1"
            | "iso8859-15"
            | "iso-8859-15"
            | "cp1252"
            | "windows-1252"
            | "cp949"
            | "windows-949"
            | "euc-kr"
            | "cp932"
            | "ms932"
            | "windows-31j"
            | "shift-jis"
    )
}

fn reject_bom_mismatch(has_bom: bool, encoding: &str) -> Result<(), String> {
    if has_bom && encoding != "utf-8" {
        Err(format!("encoding problem: {encoding} with BOM"))
    } else {
        Ok(())
    }
}

fn detected_encoding(has_bom: bool, encoding: &str) -> String {
    if has_bom {
        "utf-8-sig".to_string()
    } else {
        encoding.to_string()
    }
}

fn can_second_line_declare_encoding(first_line: &[u8]) -> bool {
    let mut current = 0;
    while current < first_line.len() && matches!(first_line[current], b' ' | b'\t' | b'\x0c') {
        current += 1;
    }
    matches!(first_line.get(current), None | Some(b'\n' | b'\r' | b'#'))
}

fn validate_consumed_lines(encoding: &str, lines: &[&[u8]]) -> Result<(), String> {
    match encoding {
        "ascii" => {
            if lines
                .iter()
                .any(|line| line.iter().any(|byte| *byte > 0x7f))
            {
                Err("'ascii' codec can't decode byte".to_string())
            } else {
                Ok(())
            }
        }
        "utf-8" | "utf8" => {
            for line in lines {
                std::str::from_utf8(line)
                    .map_err(|_| "'utf-8' codec can't decode byte".to_string())?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexMode {
    Strict,
    Parse,
    Tokenize,
}

impl LexMode {
    fn rejects_unclosed_brackets(self) -> bool {
        !matches!(self, LexMode::Parse)
    }

    fn rejects_unmatched_closing_brackets(self) -> bool {
        !matches!(self, LexMode::Tokenize)
    }

    fn rejects_number_suffixes(self) -> bool {
        !matches!(self, LexMode::Tokenize)
    }

    fn rejects_leading_zero_decimal_integers(self) -> bool {
        !matches!(self, LexMode::Tokenize)
    }

    fn enforces_decimal_int_digit_limit(self) -> bool {
        !matches!(self, LexMode::Tokenize)
    }

    fn emits_implicit_final_newline(self) -> bool {
        matches!(self, LexMode::Tokenize)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Bracket {
    opening: char,
    closing: char,
    allows_type_comments: bool,
}

fn lex_with_spans_mode(
    source: &str,
    mode: LexMode,
) -> Result<(Vec<SpannedToken>, Vec<LexWarning>), LexError> {
    let chars: Vec<char> = source.chars().collect();
    prepare_source_location_cache(&chars);
    let mut tokens = Vec::new();
    let mut warnings = Vec::new();
    let mut current = 0;
    let mut at_line_start = true;
    let mut indent_stack = vec![Indentation {
        column: 0,
        alt_column: 0,
    }];
    let mut bracket_stack = Vec::new();
    let mut forced_eof_location = None;
    let mut suppress_line_join_blank_newline = false;
    let mut pending_function_header = false;

    while current < chars.len() {
        if at_line_start {
            let mut indent = Indentation {
                column: 0,
                alt_column: 0,
            };

            while current < chars.len() {
                match chars[current] {
                    ' ' => {
                        indent.column += 1;
                        indent.alt_column += 1;
                        current += 1;
                    }
                    '\t' => {
                        indent.column += TABSIZE - (indent.column % TABSIZE);
                        indent.alt_column += 1;
                        current += 1;
                    }
                    '\x0c' => {
                        indent.column = 0;
                        indent.alt_column = 0;
                        current += 1;
                    }
                    _ => break,
                }
            }

            if current >= chars.len() {
                if mode.emits_implicit_final_newline() && indent.column > 0 {
                    let (line, column) = source_location(&chars, chars.len());
                    push_synthetic_token(
                        &mut tokens,
                        Token::Newline,
                        chars.len(),
                        line,
                        column,
                        line,
                        column + 1,
                    );
                    forced_eof_location = Some((line + 1, 1));
                }
                break;
            }

            if bracket_stack.is_empty() && is_explicit_line_join_start(&chars, current) {
                if previous_logical_line_ended_with_colon(&tokens) {
                    update_indentation(indent, &mut indent_stack, &mut tokens, &chars, current)
                        .map_err(|message| lex_error_at(&chars, current, message))?;
                } else {
                    validate_explicit_line_join_indentation(indent, &indent_stack)
                        .map_err(|message| lex_error_at(&chars, current, message))?;
                }
                at_line_start = false;
            } else if !bracket_stack.is_empty() {
                at_line_start = false;
            } else if chars[current] != '\n' && chars[current] != '\r' && chars[current] != '#' {
                update_indentation(indent, &mut indent_stack, &mut tokens, &chars, current)
                    .map_err(|message| lex_error_at(&chars, current, message))?;
                at_line_start = false;
            }
        }

        let token_start = current;
        let ch = chars[current];
        if suppress_line_join_blank_newline && !matches!(ch, ' ' | '\t' | '\x0c' | '\n' | '\r') {
            suppress_line_join_blank_newline = false;
        }

        match ch {
            ' ' | '\t' | '\x0c' => {
                current += 1;
            }
            '\0' => {
                return Err(lex_error_at(
                    &chars,
                    current,
                    "source code cannot contain null bytes",
                ));
            }
            '\n' => {
                current += 1;
                if bracket_stack.is_empty() && !suppress_line_join_blank_newline {
                    push_token(&mut tokens, &chars, Token::Newline, token_start, current);
                }
                suppress_line_join_blank_newline = false;
                at_line_start = true;
            }
            '\r' => {
                current += 1;
                if current < chars.len() && chars[current] == '\n' {
                    current += 1;
                }
                if bracket_stack.is_empty() && !suppress_line_join_blank_newline {
                    push_token(&mut tokens, &chars, Token::Newline, token_start, current);
                }
                suppress_line_join_blank_newline = false;
                at_line_start = true;
            }
            '(' => {
                if bracket_stack.len() >= MAX_BRACKET_DEPTH {
                    return Err(lex_error_at(&chars, current, "too many nested parentheses"));
                }
                let allows_type_comments = pending_function_header && bracket_stack.is_empty();
                if allows_type_comments {
                    pending_function_header = false;
                }
                bracket_stack.push(Bracket {
                    opening: '(',
                    closing: ')',
                    allows_type_comments,
                });
                current += 1;
                push_token(&mut tokens, &chars, Token::LeftParen, token_start, current);
            }
            ')' => {
                close_bracket(&mut bracket_stack, &chars, current, ')', mode)?;
                current += 1;
                push_token(&mut tokens, &chars, Token::RightParen, token_start, current);
            }
            '[' => {
                if bracket_stack.len() >= MAX_BRACKET_DEPTH {
                    return Err(lex_error_at(&chars, current, "too many nested parentheses"));
                }
                bracket_stack.push(Bracket {
                    opening: '[',
                    closing: ']',
                    allows_type_comments: false,
                });
                current += 1;
                push_token(
                    &mut tokens,
                    &chars,
                    Token::LeftBracket,
                    token_start,
                    current,
                );
            }
            ']' => {
                close_bracket(&mut bracket_stack, &chars, current, ']', mode)?;
                current += 1;
                push_token(
                    &mut tokens,
                    &chars,
                    Token::RightBracket,
                    token_start,
                    current,
                );
            }
            '{' => {
                if bracket_stack.len() >= MAX_BRACKET_DEPTH {
                    return Err(lex_error_at(&chars, current, "too many nested parentheses"));
                }
                bracket_stack.push(Bracket {
                    opening: '{',
                    closing: '}',
                    allows_type_comments: false,
                });
                current += 1;
                push_token(&mut tokens, &chars, Token::LeftBrace, token_start, current);
            }
            '}' => {
                close_bracket(&mut bracket_stack, &chars, current, '}', mode)?;
                current += 1;
                push_token(&mut tokens, &chars, Token::RightBrace, token_start, current);
            }
            '.' if current + 2 < chars.len()
                && chars[current + 1] == '.'
                && chars[current + 2] == '.' =>
            {
                current += 3;
                push_token(&mut tokens, &chars, Token::Ellipsis, token_start, current);
            }
            '.' if current + 1 < chars.len()
                && chars[current + 1] == '_'
                && mode.rejects_number_suffixes()
                && !can_start_attribute_selector(&tokens)
                && !can_start_relative_import_module_after_dot(&tokens) =>
            {
                return Err(number_lex_error(
                    &chars,
                    current,
                    invalid_decimal_literal_message(),
                ));
            }
            '.' if current + 1 < chars.len() && chars[current + 1].is_ascii_digit() => {
                let (token, next) = lex_number(&chars, current, &mut warnings, mode)
                    .map_err(|message| number_lex_error(&chars, current, message))?;
                if mode.rejects_number_suffixes()
                    && let Some(error) = number_suffix_error(&chars, current, next, &token)
                {
                    return Err(error);
                }
                push_token(&mut tokens, &chars, token, token_start, next);
                current = next;
            }
            '.' => {
                current += 1;
                push_token(&mut tokens, &chars, Token::Dot, token_start, current);
            }
            '@' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::AtEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::At, token_start, current);
                }
            }
            ',' => {
                current += 1;
                push_token(&mut tokens, &chars, Token::Comma, token_start, current);
            }
            ':' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::ColonEqual, token_start, current);
                } else {
                    if bracket_stack.is_empty() {
                        pending_function_header = false;
                    }
                    push_token(&mut tokens, &chars, Token::Colon, token_start, current);
                }
            }
            ';' => {
                current += 1;
                push_token(&mut tokens, &chars, Token::Semicolon, token_start, current);
            }
            '+' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::PlusEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Plus, token_start, current);
                }
            }
            '-' => {
                current += 1;
                if current < chars.len() && chars[current] == '>' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::Arrow, token_start, current);
                } else if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::MinusEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Minus, token_start, current);
                }
            }
            '*' => {
                current += 1;
                if current < chars.len() && chars[current] == '*' {
                    current += 1;
                    if current < chars.len() && chars[current] == '=' {
                        current += 1;
                        push_token(
                            &mut tokens,
                            &chars,
                            Token::DoubleStarEqual,
                            token_start,
                            current,
                        );
                    } else {
                        push_token(&mut tokens, &chars, Token::DoubleStar, token_start, current);
                    }
                } else if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::StarEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Star, token_start, current);
                }
            }
            '/' => {
                current += 1;
                if current < chars.len() && chars[current] == '/' {
                    current += 1;
                    if current < chars.len() && chars[current] == '=' {
                        current += 1;
                        push_token(
                            &mut tokens,
                            &chars,
                            Token::DoubleSlashEqual,
                            token_start,
                            current,
                        );
                    } else {
                        push_token(
                            &mut tokens,
                            &chars,
                            Token::DoubleSlash,
                            token_start,
                            current,
                        );
                    }
                } else if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::SlashEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Slash, token_start, current);
                }
            }
            '%' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(
                        &mut tokens,
                        &chars,
                        Token::PercentEqual,
                        token_start,
                        current,
                    );
                } else {
                    push_token(&mut tokens, &chars, Token::Percent, token_start, current);
                }
            }
            '|' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::PipeEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Pipe, token_start, current);
                }
            }
            '^' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::CaretEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Caret, token_start, current);
                }
            }
            '&' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(
                        &mut tokens,
                        &chars,
                        Token::AmpersandEqual,
                        token_start,
                        current,
                    );
                } else {
                    push_token(&mut tokens, &chars, Token::Ampersand, token_start, current);
                }
            }
            '~' => {
                current += 1;
                push_token(&mut tokens, &chars, Token::Tilde, token_start, current);
            }
            '\\' => {
                current += 1;
                match chars.get(current) {
                    Some('\n') => {
                        current += 1;
                        if current == chars.len() {
                            return Err(lex_error_at(
                                &chars,
                                current.saturating_sub(1),
                                "unexpected EOF while parsing",
                            ));
                        }
                        at_line_start = false;
                        suppress_line_join_blank_newline = true;
                    }
                    Some('\r') => {
                        current += 1;
                        if matches!(chars.get(current), Some('\n')) {
                            current += 1;
                        }
                        if current == chars.len() {
                            return Err(lex_error_at(
                                &chars,
                                current.saturating_sub(1),
                                "unexpected EOF while parsing",
                            ));
                        }
                        at_line_start = false;
                        suppress_line_join_blank_newline = true;
                    }
                    Some(_) => {
                        return Err(lex_error_span(
                            &chars,
                            current,
                            current + 1,
                            "unexpected character after line continuation character",
                        ));
                    }
                    None => {
                        return Err(lex_error_at(
                            &chars,
                            current.saturating_sub(1),
                            "unexpected EOF while parsing",
                        ));
                    }
                }
            }
            '=' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::EqualEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Equal, token_start, current);
                }
            }
            '!' => {
                let operator_start = current;
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::BangEqual, token_start, current);
                } else {
                    return Err(lex_error_at(
                        &chars,
                        operator_start,
                        "unexpected character: !",
                    ));
                }
            }
            '<' => {
                current += 1;
                if current < chars.len() && chars[current] == '<' {
                    current += 1;
                    if current < chars.len() && chars[current] == '=' {
                        current += 1;
                        push_token(
                            &mut tokens,
                            &chars,
                            Token::LeftShiftEqual,
                            token_start,
                            current,
                        );
                    } else {
                        push_token(&mut tokens, &chars, Token::LeftShift, token_start, current);
                    }
                } else if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(&mut tokens, &chars, Token::LessEqual, token_start, current);
                } else {
                    push_token(&mut tokens, &chars, Token::Less, token_start, current);
                }
            }
            '>' => {
                current += 1;
                if current < chars.len() && chars[current] == '>' {
                    current += 1;
                    if current < chars.len() && chars[current] == '=' {
                        current += 1;
                        push_token(
                            &mut tokens,
                            &chars,
                            Token::RightShiftEqual,
                            token_start,
                            current,
                        );
                    } else {
                        push_token(&mut tokens, &chars, Token::RightShift, token_start, current);
                    }
                } else if current < chars.len() && chars[current] == '=' {
                    current += 1;
                    push_token(
                        &mut tokens,
                        &chars,
                        Token::GreaterEqual,
                        token_start,
                        current,
                    );
                } else {
                    push_token(&mut tokens, &chars, Token::Greater, token_start, current);
                }
            }
            '#' => {
                current += 1;
                let comment_start = current;
                while current < chars.len() && chars[current] != '\n' && chars[current] != '\r' {
                    current += 1;
                }

                let can_emit_type_comment = bracket_stack.is_empty()
                    || bracket_stack
                        .last()
                        .is_some_and(|bracket| bracket.allows_type_comments);
                if can_emit_type_comment {
                    if let Some(token) = lex_type_comment(&chars[comment_start..current]) {
                        push_token(&mut tokens, &chars, token, token_start, current);
                    }
                }
                if current >= chars.len() && mode.emits_implicit_final_newline() {
                    let (line, column) = source_location(&chars, chars.len());
                    push_synthetic_token(
                        &mut tokens,
                        Token::Newline,
                        chars.len(),
                        line,
                        column,
                        line,
                        column + 1,
                    );
                    forced_eof_location = Some((line + 1, 1));
                }
            }
            '"' | '\'' => {
                let (value, next) = lex_string(&chars, current, false, &mut warnings)
                    .map_err(|message| string_lex_error(&chars, current, current, message))?;
                push_token(&mut tokens, &chars, Token::String(value), token_start, next);
                current = next;
            }
            '0'..='9' => {
                let (token, next) = lex_number(&chars, current, &mut warnings, mode)
                    .map_err(|message| number_lex_error(&chars, current, message))?;
                if mode.rejects_number_suffixes()
                    && let Some(error) = number_suffix_error(&chars, current, next, &token)
                {
                    return Err(error);
                }
                push_token(&mut tokens, &chars, token, token_start, next);
                current = next;
            }
            ch if is_identifier_start(ch) => {
                let start = current;

                while current < chars.len() && is_identifier_continue(chars[current]) {
                    current += 1;
                }

                let word: String = chars[start..current].iter().collect();
                if current < chars.len() && matches!(chars[current], '"' | '\'') {
                    if let Some(prefix) = parse_string_prefix(&word) {
                        let next = if prefix.bytes {
                            let (value, next) =
                                lex_bytes_string(&chars, current, prefix.raw, &mut warnings)
                                    .map_err(|message| {
                                        bytes_lex_error(&chars, start, current, message)
                                    })?;
                            push_token(&mut tokens, &chars, Token::Bytes(value), start, next);
                            next
                        } else if prefix.template {
                            let (parts, next) = lex_f_string(
                                &chars,
                                current,
                                prefix.raw,
                                InterpolatedStringKind::Template,
                                &mut warnings,
                            )
                            .map_err(|message| string_lex_error(&chars, start, current, message))?;
                            push_token(&mut tokens, &chars, Token::TString(parts), start, next);
                            next
                        } else if prefix.formatted {
                            let (parts, next) = lex_f_string(
                                &chars,
                                current,
                                prefix.raw,
                                InterpolatedStringKind::Formatted,
                                &mut warnings,
                            )
                            .map_err(|message| string_lex_error(&chars, start, current, message))?;
                            push_token(&mut tokens, &chars, Token::FString(parts), start, next);
                            next
                        } else {
                            let (value, next) =
                                lex_string(&chars, current, prefix.raw, &mut warnings).map_err(
                                    |message| string_lex_error(&chars, start, current, message),
                                )?;
                            push_token(&mut tokens, &chars, Token::String(value), start, next);
                            next
                        };
                        current = next;
                        continue;
                    }

                    if is_invalid_string_prefix(&word) {
                        return Err(lex_error_span(
                            &chars,
                            start,
                            current,
                            "prefixes are incompatible",
                        ));
                    }
                }

                let token = match word.as_str() {
                    "if" => Token::If,
                    "elif" => Token::Elif,
                    "else" => Token::Else,
                    "while" => Token::While,
                    "for" => Token::For,
                    "in" => Token::In,
                    "async" => Token::Async,
                    "await" => Token::Await,
                    "def" => Token::Def,
                    "class" => Token::Class,
                    "lambda" => Token::Lambda,
                    "return" => Token::Return,
                    "yield" => Token::Yield,
                    "raise" => Token::Raise,
                    "del" => Token::Del,
                    "global" => Token::Global,
                    "nonlocal" => Token::Nonlocal,
                    "assert" => Token::Assert,
                    "try" => Token::Try,
                    "except" => Token::Except,
                    "with" => Token::With,
                    "as" => Token::As,
                    "finally" => Token::Finally,
                    "from" => Token::From,
                    "import" => Token::Import,
                    "break" => Token::Break,
                    "continue" => Token::Continue,
                    "pass" => Token::Pass,
                    "and" => Token::And,
                    "or" => Token::Or,
                    "not" => Token::Not,
                    "is" => Token::Is,
                    "True" => Token::True,
                    "False" => Token::False,
                    "None" => Token::None,
                    _ => Token::Identifier(normalize_identifier(&word)),
                };
                if matches!(token, Token::Def) {
                    pending_function_header = true;
                }
                push_token(&mut tokens, &chars, token, start, current);
            }
            _ => {
                if is_invalid_non_printable(ch) {
                    return Err(lex_error_at(
                        &chars,
                        current,
                        format!("invalid non-printable character U+{:04X}", ch as u32),
                    ));
                }

                return Err(lex_error_at(
                    &chars,
                    current,
                    format!("unexpected character: {ch}"),
                ));
            }
        }
    }

    if mode.rejects_unclosed_brackets() && !bracket_stack.is_empty() {
        return Err(lex_error_span(
            &chars,
            chars.len(),
            chars.len(),
            "EOF in multi-line statement",
        ));
    }

    let synthetic_eof_location = forced_eof_location.or_else(|| {
        if mode.emits_implicit_final_newline() && needs_implicit_final_newline(&tokens) {
            let (line, column) = source_location(&chars, chars.len());
            push_synthetic_token(
                &mut tokens,
                Token::Newline,
                chars.len(),
                line,
                column,
                line,
                column + 1,
            );
            Some((line + 1, 1))
        } else {
            None
        }
    });

    while indent_stack.len() > 1 {
        indent_stack.pop();
        push_token(&mut tokens, &chars, Token::Dedent, chars.len(), chars.len());
    }

    if let Some((line, column)) = synthetic_eof_location {
        push_synthetic_token(
            &mut tokens,
            Token::Eof,
            chars.len(),
            line,
            column,
            line,
            column,
        );
    } else {
        push_token(&mut tokens, &chars, Token::Eof, chars.len(), chars.len());
    }

    Ok((tokens, warnings))
}

fn expand_cpython_interpolated_string(
    chars: &[char],
    expanded: &mut Vec<SpannedToken>,
    token: &SpannedToken,
    kind: InterpolatedStringKind,
) -> Result<(), LexError> {
    let Some(parts) = interpolated_string_source_parts(chars, token.start, token.end) else {
        return Err(lex_error_at(
            chars,
            token.start,
            format!("invalid {} source span", kind.label()),
        ));
    };

    push_token(
        expanded,
        chars,
        kind.cpython_start_token(chars_text(chars, token.start, parts.content_start)),
        token.start,
        parts.content_start,
    );
    expand_cpython_interpolated_content(chars, expanded, parts, kind)?;
    push_token(
        expanded,
        chars,
        kind.cpython_end_token(chars_text(chars, parts.content_end, token.end)),
        parts.content_end,
        token.end,
    );

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InterpolatedStringSourceParts {
    content_start: usize,
    content_end: usize,
    quote: char,
    triple: bool,
}

fn interpolated_string_source_parts(
    chars: &[char],
    start: usize,
    end: usize,
) -> Option<InterpolatedStringSourceParts> {
    let mut quote_start = start;
    while quote_start < end && !matches!(chars.get(quote_start), Some('\'' | '"')) {
        quote_start += 1;
    }
    let quote = *chars.get(quote_start)?;
    let triple =
        quote_start + 2 < end && chars[quote_start + 1] == quote && chars[quote_start + 2] == quote;
    let quote_len = if triple { 3 } else { 1 };
    let content_start = quote_start + quote_len;
    if end < content_start + quote_len {
        return None;
    }
    let content_end = end - quote_len;
    Some(InterpolatedStringSourceParts {
        content_start,
        content_end,
        quote,
        triple,
    })
}

fn expand_cpython_interpolated_content(
    chars: &[char],
    expanded: &mut Vec<SpannedToken>,
    parts: InterpolatedStringSourceParts,
    kind: InterpolatedStringKind,
) -> Result<(), LexError> {
    let mut current = parts.content_start;
    let mut literal_start = None;
    let mut literal_end = parts.content_start;
    let mut literal = String::new();

    while current < parts.content_end {
        match chars[current] {
            '{' if current + 1 < parts.content_end && chars[current + 1] == '{' => {
                if literal_start.is_none() {
                    literal_start = Some(current);
                }
                literal.push('{');
                current += 2;
                literal_end = current - 1;
                push_cpython_interpolated_middle(
                    chars,
                    expanded,
                    kind,
                    &mut literal,
                    &mut literal_start,
                    literal_end,
                );
            }
            '}' if current + 1 < parts.content_end && chars[current + 1] == '}' => {
                if literal_start.is_none() {
                    literal_start = Some(current);
                }
                literal.push('}');
                current += 2;
                literal_end = current - 1;
                push_cpython_interpolated_middle(
                    chars,
                    expanded,
                    kind,
                    &mut literal,
                    &mut literal_start,
                    literal_end,
                );
            }
            '{' => {
                push_cpython_interpolated_middle(
                    chars,
                    expanded,
                    kind,
                    &mut literal,
                    &mut literal_start,
                    literal_end,
                );
                push_token(expanded, chars, Token::LeftBrace, current, current + 1);
                let expression = scan_cpython_interpolated_expression(
                    chars,
                    current + 1,
                    parts.content_end,
                    parts.quote,
                    parts.triple,
                    kind,
                )?;
                append_cpython_interpolated_expression_tokens(
                    chars,
                    expanded,
                    expression.expression_start,
                    expression.expression_end,
                )?;
                if let Some(equal_index) = expression.debug_equal {
                    push_token(expanded, chars, Token::Equal, equal_index, equal_index + 1);
                }
                if let Some((bang_index, conversion_index)) = expression.conversion {
                    push_token(
                        expanded,
                        chars,
                        Token::Exclamation,
                        bang_index,
                        bang_index + 1,
                    );
                    push_token(
                        expanded,
                        chars,
                        Token::Identifier(chars[conversion_index].to_string()),
                        conversion_index,
                        conversion_index + 1,
                    );
                }
                if let Some((colon_index, format_start, format_end)) = expression.format_spec {
                    push_token(expanded, chars, Token::Colon, colon_index, colon_index + 1);
                    append_cpython_interpolated_format_spec_tokens(
                        chars,
                        expanded,
                        kind,
                        format_start,
                        format_end,
                        parts.quote,
                        parts.triple,
                    )?;
                }
                push_token(
                    expanded,
                    chars,
                    Token::RightBrace,
                    expression.close_brace,
                    expression.close_brace + 1,
                );
                current = expression.close_brace + 1;
            }
            '}' => {
                return Err(lex_error_at(
                    chars,
                    current,
                    kind.single_right_brace_message(),
                ));
            }
            ch => {
                if literal_start.is_none() {
                    literal_start = Some(current);
                }
                literal.push(ch);
                current += 1;
                literal_end = current;
            }
        }
    }

    push_cpython_interpolated_middle(
        chars,
        expanded,
        kind,
        &mut literal,
        &mut literal_start,
        literal_end,
    );
    Ok(())
}

fn push_cpython_interpolated_middle(
    chars: &[char],
    expanded: &mut Vec<SpannedToken>,
    kind: InterpolatedStringKind,
    literal: &mut String,
    literal_start: &mut Option<usize>,
    literal_end: usize,
) {
    if literal.is_empty() {
        return;
    }
    let start = literal_start
        .take()
        .expect("non-empty interpolated literal has a start span");
    push_token(
        expanded,
        chars,
        kind.cpython_middle_token(std::mem::take(literal)),
        start,
        literal_end,
    );
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CpythonInterpolatedExpression {
    expression_start: usize,
    expression_end: usize,
    debug_equal: Option<usize>,
    conversion: Option<(usize, usize)>,
    format_spec: Option<(usize, usize, usize)>,
    close_brace: usize,
}

fn scan_cpython_interpolated_expression(
    chars: &[char],
    start: usize,
    end: usize,
    quote: char,
    triple: bool,
    kind: InterpolatedStringKind,
) -> Result<CpythonInterpolatedExpression, LexError> {
    let mut current = start;
    let mut depth = 0usize;

    while current < end {
        match chars[current] {
            '"' | '\'' if quote_starts_interpolated_expression_string(chars, start, current) => {
                current = skip_quoted_expression_string(chars, current)
                    .map_err(|message| lex_error_at(chars, current, message))?;
            }
            _ if interpolated_string_terminates_at(chars, current, quote, triple) => {
                return Err(lex_error_at(
                    chars,
                    current,
                    kind.expecting_right_brace_message(),
                ));
            }
            '"' | '\'' => {
                current = skip_quoted_expression_string(chars, current)
                    .map_err(|message| lex_error_at(chars, current, message))?;
            }
            '#' => skip_f_string_expression_comment(chars, &mut current, quote, triple),
            '(' | '[' | '{' => {
                depth += 1;
                current += 1;
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                current += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                current += 1;
            }
            '}' if expression_source_is_empty(chars, start, current) => {
                return Err(lex_error_at(
                    chars,
                    current,
                    kind.empty_expression_before_right_brace_message(),
                ));
            }
            '}' => {
                return Ok(CpythonInterpolatedExpression {
                    expression_start: start,
                    expression_end: current,
                    debug_equal: None,
                    conversion: None,
                    format_spec: None,
                    close_brace: current,
                });
            }
            '!' if depth == 0 && current + 1 < end && chars[current + 1] == '=' => {
                current += 2;
            }
            '!' if depth == 0 && expression_source_is_empty(chars, start, current) => {
                return Err(lex_error_at(
                    chars,
                    current,
                    format!("{}: valid expression required before '!'", kind.label()),
                ));
            }
            '!' if depth == 0 => {
                let conversion_index = current + 1;
                if conversion_index >= end {
                    return Err(lex_error_at(
                        chars,
                        current,
                        kind.unterminated_message(triple),
                    ));
                }
                let mut after_conversion = conversion_index + 1;
                skip_f_string_field_padding_and_comments(
                    chars,
                    &mut after_conversion,
                    quote,
                    triple,
                );
                match chars.get(after_conversion) {
                    Some('}') => {
                        return Ok(CpythonInterpolatedExpression {
                            expression_start: start,
                            expression_end: current,
                            debug_equal: None,
                            conversion: Some((current, conversion_index)),
                            format_spec: None,
                            close_brace: after_conversion,
                        });
                    }
                    Some(':') => {
                        let (format_end, close_brace) = scan_cpython_interpolated_format_spec(
                            chars,
                            after_conversion + 1,
                            end,
                            quote,
                            triple,
                            kind,
                        )?;
                        return Ok(CpythonInterpolatedExpression {
                            expression_start: start,
                            expression_end: current,
                            debug_equal: None,
                            conversion: Some((current, conversion_index)),
                            format_spec: Some((after_conversion, after_conversion + 1, format_end)),
                            close_brace,
                        });
                    }
                    _ => {
                        return Err(lex_error_at(
                            chars,
                            current,
                            format!("{}: expecting ':' or '}}'", kind.label()),
                        ));
                    }
                }
            }
            '=' if depth == 0 && is_f_string_debug_equal(chars, start, current) => {
                let mut after_equal = current + 1;
                skip_f_string_field_padding_and_comments(chars, &mut after_equal, quote, triple);
                match chars.get(after_equal) {
                    Some('}') => {
                        return Ok(CpythonInterpolatedExpression {
                            expression_start: start,
                            expression_end: current,
                            debug_equal: Some(current),
                            conversion: None,
                            format_spec: None,
                            close_brace: after_equal,
                        });
                    }
                    Some('!') => {
                        let conversion_index = after_equal + 1;
                        let mut after_conversion = conversion_index + 1;
                        skip_f_string_field_padding_and_comments(
                            chars,
                            &mut after_conversion,
                            quote,
                            triple,
                        );
                        match chars.get(after_conversion) {
                            Some('}') => {
                                return Ok(CpythonInterpolatedExpression {
                                    expression_start: start,
                                    expression_end: current,
                                    debug_equal: Some(current),
                                    conversion: Some((after_equal, conversion_index)),
                                    format_spec: None,
                                    close_brace: after_conversion,
                                });
                            }
                            Some(':') => {
                                let (format_end, close_brace) =
                                    scan_cpython_interpolated_format_spec(
                                        chars,
                                        after_conversion + 1,
                                        end,
                                        quote,
                                        triple,
                                        kind,
                                    )?;
                                return Ok(CpythonInterpolatedExpression {
                                    expression_start: start,
                                    expression_end: current,
                                    debug_equal: Some(current),
                                    conversion: Some((after_equal, conversion_index)),
                                    format_spec: Some((
                                        after_conversion,
                                        after_conversion + 1,
                                        format_end,
                                    )),
                                    close_brace,
                                });
                            }
                            _ => {
                                return Err(lex_error_at(
                                    chars,
                                    after_equal,
                                    format!("{}: expecting ':' or '}}'", kind.label()),
                                ));
                            }
                        }
                    }
                    Some(':') => {
                        let (format_end, close_brace) = scan_cpython_interpolated_format_spec(
                            chars,
                            after_equal + 1,
                            end,
                            quote,
                            triple,
                            kind,
                        )?;
                        return Ok(CpythonInterpolatedExpression {
                            expression_start: start,
                            expression_end: current,
                            debug_equal: Some(current),
                            conversion: None,
                            format_spec: Some((after_equal, after_equal + 1, format_end)),
                            close_brace,
                        });
                    }
                    _ => {
                        return Err(lex_error_at(
                            chars,
                            current,
                            format!("{}: expecting '!', or ':', or '}}'", kind.label()),
                        ));
                    }
                }
            }
            ':' if depth == 0 && expression_source_is_empty(chars, start, current) => {
                return Err(lex_error_at(
                    chars,
                    current,
                    format!("{}: valid expression required before ':'", kind.label()),
                ));
            }
            ':' if depth == 0 => {
                let (format_end, close_brace) = scan_cpython_interpolated_format_spec(
                    chars,
                    current + 1,
                    end,
                    quote,
                    triple,
                    kind,
                )?;
                return Ok(CpythonInterpolatedExpression {
                    expression_start: start,
                    expression_end: current,
                    debug_equal: None,
                    conversion: None,
                    format_spec: Some((current, current + 1, format_end)),
                    close_brace,
                });
            }
            _ => current += 1,
        }
    }

    Err(lex_error_at(
        chars,
        start,
        kind.unterminated_message(triple),
    ))
}

fn scan_cpython_interpolated_format_spec(
    chars: &[char],
    start: usize,
    end: usize,
    quote: char,
    triple: bool,
    kind: InterpolatedStringKind,
) -> Result<(usize, usize), LexError> {
    let mut current = start;
    let mut depth = 0usize;

    while current < end {
        match chars[current] {
            _ if interpolated_string_terminates_at(chars, current, quote, triple) => {
                return Err(lex_error_at(chars, current, kind.format_spec_end_message()));
            }
            '{' => {
                depth += 1;
                current += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                current += 1;
            }
            '}' => return Ok((current, current)),
            _ => current += 1,
        }
    }

    Err(lex_error_at(chars, start, kind.format_spec_end_message()))
}

fn append_cpython_interpolated_format_spec_tokens(
    chars: &[char],
    expanded: &mut Vec<SpannedToken>,
    kind: InterpolatedStringKind,
    start: usize,
    end: usize,
    quote: char,
    triple: bool,
) -> Result<(), LexError> {
    let mut current = start;
    let mut literal_start = None;
    let mut literal_end = start;
    let mut literal = String::new();

    while current < end {
        match chars[current] {
            '{' if current + 1 < end && chars[current + 1] == '{' => {
                if literal_start.is_none() {
                    literal_start = Some(current);
                }
                literal.push('{');
                current += 2;
                literal_end = current - 1;
                push_cpython_interpolated_middle(
                    chars,
                    expanded,
                    kind,
                    &mut literal,
                    &mut literal_start,
                    literal_end,
                );
            }
            '}' if current + 1 < end && chars[current + 1] == '}' => {
                if literal_start.is_none() {
                    literal_start = Some(current);
                }
                literal.push('}');
                current += 2;
                literal_end = current - 1;
                push_cpython_interpolated_middle(
                    chars,
                    expanded,
                    kind,
                    &mut literal,
                    &mut literal_start,
                    literal_end,
                );
            }
            '{' => {
                push_cpython_interpolated_middle(
                    chars,
                    expanded,
                    kind,
                    &mut literal,
                    &mut literal_start,
                    literal_end,
                );
                push_token(expanded, chars, Token::LeftBrace, current, current + 1);
                let expression = scan_cpython_interpolated_expression(
                    chars,
                    current + 1,
                    end,
                    quote,
                    triple,
                    kind,
                )?;
                append_cpython_interpolated_expression_tokens(
                    chars,
                    expanded,
                    expression.expression_start,
                    expression.expression_end,
                )?;
                if let Some(equal_index) = expression.debug_equal {
                    push_token(expanded, chars, Token::Equal, equal_index, equal_index + 1);
                }
                if let Some((bang_index, conversion_index)) = expression.conversion {
                    push_token(
                        expanded,
                        chars,
                        Token::Exclamation,
                        bang_index,
                        bang_index + 1,
                    );
                    push_token(
                        expanded,
                        chars,
                        Token::Identifier(chars[conversion_index].to_string()),
                        conversion_index,
                        conversion_index + 1,
                    );
                }
                if let Some((colon_index, format_start, format_end)) = expression.format_spec {
                    push_token(expanded, chars, Token::Colon, colon_index, colon_index + 1);
                    append_cpython_interpolated_format_spec_tokens(
                        chars,
                        expanded,
                        kind,
                        format_start,
                        format_end,
                        quote,
                        triple,
                    )?;
                }
                push_token(
                    expanded,
                    chars,
                    Token::RightBrace,
                    expression.close_brace,
                    expression.close_brace + 1,
                );
                current = expression.close_brace + 1;
            }
            ch => {
                if literal_start.is_none() {
                    literal_start = Some(current);
                }
                literal.push(ch);
                current += 1;
                literal_end = current;
            }
        }
    }

    push_cpython_interpolated_middle(
        chars,
        expanded,
        kind,
        &mut literal,
        &mut literal_start,
        literal_end,
    );
    Ok(())
}

fn append_cpython_interpolated_expression_tokens(
    chars: &[char],
    expanded: &mut Vec<SpannedToken>,
    start: usize,
    end: usize,
) -> Result<(), LexError> {
    if start >= end {
        return Ok(());
    }

    let source = chars_text(chars, start, end);
    let source_len = source.chars().count();
    let mut tokens =
        tokenize_with_spans(&source).map_err(|error| lex_error_at(chars, start, error.message))?;
    add_missing_cpython_interpolated_expression_newlines(&source, &mut tokens);
    for token in tokens {
        if matches!(token.token, Token::Eof) {
            continue;
        }
        if matches!(token.token, Token::Indent | Token::Dedent) {
            continue;
        }
        if matches!(token.token, Token::Newline)
            && token.start == source_len
            && token.end == source_len
        {
            continue;
        }
        let absolute_token = SpannedToken {
            token: token.token,
            start: start + token.start,
            end: start + token.end,
            line: 0,
            column: 0,
            end_line: 0,
            end_column: 0,
            byte_column: 0,
            byte_end_column: 0,
        };
        match &absolute_token.token {
            Token::FString(_) => expand_cpython_interpolated_string(
                chars,
                expanded,
                &absolute_token,
                InterpolatedStringKind::Formatted,
            )?,
            Token::TString(_) => expand_cpython_interpolated_string(
                chars,
                expanded,
                &absolute_token,
                InterpolatedStringKind::Template,
            )?,
            _ => push_token(
                expanded,
                chars,
                absolute_token.token,
                absolute_token.start,
                absolute_token.end,
            ),
        }
    }

    Ok(())
}

fn add_missing_cpython_interpolated_expression_newlines(
    source: &str,
    tokens: &mut Vec<SpannedToken>,
) {
    let source_chars = source.chars().collect::<Vec<_>>();
    let mut current = 0;

    while current < source_chars.len() {
        let newline_len = match source_chars[current] {
            '\n' => 1,
            '\r' if matches!(source_chars.get(current + 1), Some('\n')) => 2,
            '\r' => 1,
            _ => {
                current += 1;
                continue;
            }
        };
        let newline_end = current + newline_len;
        let already_emitted = tokens.iter().any(|token| {
            matches!(token.token, Token::Newline)
                && token.start == current
                && token.end == newline_end
        });
        if !already_emitted {
            tokens.push(SpannedToken {
                token: Token::Newline,
                start: current,
                end: newline_end,
                line: 0,
                column: 0,
                end_line: 0,
                end_column: 0,
                byte_column: 0,
                byte_end_column: 0,
            });
        }
        current = newline_end;
    }

    tokens.sort_by_key(|token| token.start);
}

fn chars_text(chars: &[char], start: usize, end: usize) -> String {
    chars[start..end].iter().collect()
}

fn needs_implicit_final_newline(tokens: &[SpannedToken]) -> bool {
    matches!(
        tokens.last().map(|spanned| &spanned.token),
        Some(token) if !matches!(token, Token::Newline | Token::Dedent)
    )
}

fn is_explicit_line_join_start(chars: &[char], current: usize) -> bool {
    matches!(chars.get(current), Some('\\')) && matches!(chars.get(current + 1), Some('\n' | '\r'))
}

fn previous_logical_line_ended_with_colon(tokens: &[SpannedToken]) -> bool {
    tokens
        .iter()
        .rev()
        .skip_while(|token| matches!(token.token, Token::Newline))
        .next()
        .is_some_and(|token| matches!(token.token, Token::Colon))
}

fn lex_number(
    chars: &[char],
    start: usize,
    warnings: &mut Vec<LexWarning>,
    mode: LexMode,
) -> Result<(Token, usize), String> {
    let mut current = start;
    let mut is_float = false;

    if chars[current] == '0' && current + 1 < chars.len() {
        match chars[current + 1] {
            'b' | 'B' => return lex_prefixed_integer(chars, start, 2, warnings, mode),
            'o' | 'O' => return lex_prefixed_integer(chars, start, 8, warnings, mode),
            'x' | 'X' => return lex_prefixed_integer(chars, start, 16, warnings, mode),
            _ => {}
        }
    }

    if chars[current] == '.' {
        is_float = true;
        current += 1;
        scan_digit_part(chars, &mut current, mode)?;
    } else {
        scan_digit_part(chars, &mut current, mode)?;

        if current < chars.len() && chars[current] == '.' {
            if current + 1 < chars.len()
                && chars[current + 1] == '_'
                && mode.rejects_number_suffixes()
            {
                return Err(invalid_decimal_literal_message().to_string());
            }

            is_float = true;
            current += 1;
            scan_digit_part(chars, &mut current, mode)?;
        }
    }

    if starts_valid_exponent(chars, current) {
        is_float = true;
        current += 1;

        if current < chars.len() && matches!(chars[current], '+' | '-') {
            current += 1;
        }

        if !scan_digit_part(chars, &mut current, mode)? {
            return Err(invalid_decimal_literal_message().to_string());
        }
    } else if starts_invalid_exponent(chars, current) && mode.rejects_number_suffixes() {
        return Err(invalid_decimal_literal_message().to_string());
    }

    let text: String = chars[start..current].iter().collect();
    if current < chars.len() && matches!(chars[current], 'j' | 'J') {
        current += 1;
        if current < chars.len()
            && (chars[current].is_ascii_digit() || chars[current] == '_')
            && mode.rejects_number_suffixes()
        {
            let mut invalid_end = current + 1;
            while invalid_end < chars.len()
                && (chars[invalid_end].is_ascii_alphanumeric() || chars[invalid_end] == '_')
            {
                invalid_end += 1;
            }
            let text: String = chars[start..invalid_end].iter().collect();
            return Err(format!("invalid number: {text}"));
        }

        warn_invalid_number_keyword_suffix(chars, current, "imaginary", warnings);
        return Ok((Token::Imaginary(text), current));
    }

    let normalized = text.replace('_', "");
    if is_float {
        normalized
            .parse::<f64>()
            .map_err(|_| invalid_decimal_literal_message().to_string())?;
        warn_invalid_number_keyword_suffix(chars, current, "decimal", warnings);
        Ok((Token::Float(text), current))
    } else {
        if mode.rejects_leading_zero_decimal_integers()
            && normalized.len() > 1
            && normalized.starts_with('0')
            && normalized.chars().any(|ch| ch != '0')
        {
            return Err(
                "leading zeros in decimal integer literals are not permitted; use an 0o prefix for octal integers"
                    .to_string(),
            );
        }

        warn_invalid_number_keyword_suffix(chars, current, "decimal", warnings);
        Ok((
            integer_token_from_digits(&normalized, 10, &text, mode)?,
            current,
        ))
    }
}

fn lex_prefixed_integer(
    chars: &[char],
    start: usize,
    radix: u32,
    warnings: &mut Vec<LexWarning>,
    mode: LexMode,
) -> Result<(Token, usize), String> {
    let mut current = start + 2;
    let mut seen_digit = false;
    let mut previous_underscore = false;

    if current < chars.len()
        && chars[current] == '_'
        && current + 1 < chars.len()
        && is_digit_for_radix(chars[current + 1], radix)
    {
        current += 1;
        previous_underscore = true;
    }

    while current < chars.len() {
        let ch = chars[current];
        if is_digit_for_radix(ch, radix) {
            seen_digit = true;
            previous_underscore = false;
            current += 1;
            continue;
        }

        if ch == '_' {
            if current + 1 < chars.len()
                && chars[current + 1].is_ascii_digit()
                && !is_digit_for_radix(chars[current + 1], radix)
            {
                if !mode.rejects_number_suffixes() && seen_digit {
                    break;
                }
                if !mode.rejects_number_suffixes() {
                    return Ok((Token::Number(0), start + 1));
                }
                return Err(invalid_digit_for_radix_message(chars[current + 1], radix));
            }
            let invalid = !seen_digit
                || previous_underscore
                || current + 1 >= chars.len()
                || !is_digit_for_radix(chars[current + 1], radix);
            if invalid {
                if !mode.rejects_number_suffixes() && seen_digit {
                    break;
                }
                if !mode.rejects_number_suffixes() {
                    return Ok((Token::Number(0), start + 1));
                }
                return Err(invalid_prefixed_integer_message(radix).to_string());
            }
            previous_underscore = true;
            current += 1;
            continue;
        }

        if ch.is_ascii_digit() {
            if !mode.rejects_number_suffixes() && seen_digit {
                break;
            }
            if !mode.rejects_number_suffixes() {
                return Ok((Token::Number(0), start + 1));
            }
            return Err(invalid_digit_for_radix_message(ch, radix));
        }

        break;
    }

    if !seen_digit {
        if !mode.rejects_number_suffixes() {
            return Ok((Token::Number(0), start + 1));
        }
        return Err(invalid_prefixed_integer_message(radix).to_string());
    }

    let text: String = chars[start..current].iter().collect();
    let digits: String = chars[start + 2..current]
        .iter()
        .filter(|ch| **ch != '_')
        .collect();
    warn_invalid_number_keyword_suffix(chars, current, prefixed_integer_kind(radix), warnings);
    Ok((
        integer_token_from_digits(&digits, radix, &text, mode)?,
        current,
    ))
}

fn starts_valid_exponent(chars: &[char], marker: usize) -> bool {
    if marker >= chars.len() || !matches!(chars[marker], 'e' | 'E') {
        return false;
    }

    let mut digit = marker + 1;
    if digit < chars.len() && matches!(chars[digit], '+' | '-') {
        digit += 1;
    }

    digit < chars.len() && chars[digit].is_ascii_digit()
}

fn starts_invalid_exponent(chars: &[char], marker: usize) -> bool {
    if marker >= chars.len() || !matches!(chars[marker], 'e' | 'E') {
        return false;
    }

    match chars.get(marker + 1) {
        None => true,
        Some('_' | '+' | '-') => true,
        _ => false,
    }
}

fn is_digit_for_radix(ch: char, radix: u32) -> bool {
    ch.to_digit(radix).is_some()
}

fn invalid_decimal_literal_message() -> &'static str {
    "invalid decimal literal"
}

fn invalid_prefixed_integer_message(radix: u32) -> &'static str {
    match radix {
        2 => "invalid binary literal",
        8 => "invalid octal literal",
        16 => "invalid hexadecimal literal",
        _ => "invalid number",
    }
}

fn invalid_digit_for_radix_message(ch: char, radix: u32) -> String {
    match radix {
        2 => format!("invalid digit '{ch}' in binary literal"),
        8 => format!("invalid digit '{ch}' in octal literal"),
        16 => "invalid hexadecimal literal".to_string(),
        _ => format!("invalid number: {ch}"),
    }
}

fn prefixed_integer_kind(radix: u32) -> &'static str {
    match radix {
        2 => "binary",
        8 => "octal",
        16 => "hexadecimal",
        _ => "number",
    }
}

fn warn_invalid_number_keyword_suffix(
    chars: &[char],
    current: usize,
    kind: &str,
    warnings: &mut Vec<LexWarning>,
) {
    if let Some(keyword) = number_suffix_warning_keyword(chars, current) {
        warnings.push(syntax_warning_span(
            chars,
            current,
            current + keyword.len(),
            format!("invalid {kind} literal"),
        ));
    }
}

fn number_suffix_warning_keyword(chars: &[char], start: usize) -> Option<&'static str> {
    ["and", "else", "for", "if", "in", "is", "not", "or"]
        .into_iter()
        .find(|keyword| {
            chars
                .get(start..start + keyword.len())
                .is_some_and(|slice| slice.iter().copied().eq(keyword.chars()))
                && chars
                    .get(start + keyword.len())
                    .map_or(true, |ch| !is_identifier_continue(*ch))
        })
}

fn integer_token_from_digits(
    digits: &str,
    radix: u32,
    original: &str,
    mode: LexMode,
) -> Result<Token, String> {
    if radix == 10 && mode.enforces_decimal_int_digit_limit() {
        let max_digits = get_int_max_str_digits();
        if max_digits != 0 && digits.len() > max_digits {
            return Err(format!(
                "Exceeds the limit ({max_digits} digits) for integer string conversion: value has {} digits; use sys.set_int_max_str_digits() to increase the limit - Consider hexadecimal for huge integer literals to avoid decimal conversion limits.",
                digits.len()
            ));
        }
    }

    let value = BigInt::parse_bytes(digits.as_bytes(), radix)
        .ok_or_else(|| format!("invalid number: {original}"))?;
    if let Some(value) = value.to_i64() {
        Ok(Token::Number(value))
    } else {
        Ok(Token::BigInt(value.to_string()))
    }
}

fn close_bracket(
    bracket_stack: &mut Vec<Bracket>,
    chars: &[char],
    current: usize,
    actual: char,
    mode: LexMode,
) -> Result<(), LexError> {
    let Some(expected) = bracket_stack.pop() else {
        if !mode.rejects_unmatched_closing_brackets() {
            return Ok(());
        }
        return Err(lex_error_at(
            chars,
            current,
            format!("unmatched '{actual}'"),
        ));
    };

    if expected.closing != actual {
        if !mode.rejects_unmatched_closing_brackets() {
            return Ok(());
        }
        return Err(lex_error_at(
            chars,
            current,
            format!(
                "closing parenthesis '{actual}' does not match opening parenthesis '{}'",
                expected.opening
            ),
        ));
    }

    Ok(())
}

fn is_identifier_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic() || unicode_ident::is_xid_start(ch)
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric() || unicode_ident::is_xid_continue(ch)
}

fn is_invalid_non_printable(ch: char) -> bool {
    (ch.is_control() && !matches!(ch, '\t' | '\n' | '\r' | '\x0c'))
        || (ch.is_whitespace() && !matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c'))
}

fn normalize_identifier(value: &str) -> String {
    if value.is_ascii() {
        value.to_string()
    } else {
        value.nfkc().collect()
    }
}

fn can_start_attribute_selector(tokens: &[SpannedToken]) -> bool {
    matches!(
        tokens.last().map(|spanned| &spanned.token),
        Some(
            Token::Identifier(_)
                | Token::Number(_)
                | Token::BigInt(_)
                | Token::Float(_)
                | Token::Imaginary(_)
                | Token::String(_)
                | Token::Bytes(_)
                | Token::FString(_)
                | Token::TString(_)
                | Token::True
                | Token::False
                | Token::None
                | Token::RightParen
                | Token::RightBracket
                | Token::RightBrace
        )
    )
}

fn can_start_relative_import_module_after_dot(tokens: &[SpannedToken]) -> bool {
    let mut index = tokens.len();
    while index > 0 {
        index -= 1;
        match &tokens[index].token {
            Token::Dot | Token::Ellipsis => {}
            Token::From => return true,
            _ => return false,
        }
    }
    false
}

fn lex_type_comment(chars: &[char]) -> Option<Token> {
    let comment: String = chars.iter().collect();
    let body = comment.trim_start().strip_prefix("type:")?.trim_start();

    if is_type_ignore_body(body) {
        Some(Token::TypeIgnore(body.to_string()))
    } else {
        Some(Token::TypeComment(body.to_string()))
    }
}

fn is_type_ignore_body(body: &str) -> bool {
    let Some(rest) = body.strip_prefix("ignore") else {
        return false;
    };

    rest.chars()
        .next()
        .map(|ch| ch.is_ascii() && !ch.is_ascii_alphanumeric())
        .unwrap_or(true)
}

fn lex_string(
    chars: &[char],
    start: usize,
    raw: bool,
    warnings: &mut Vec<LexWarning>,
) -> Result<(String, usize), String> {
    let quote = chars[start];
    let triple = start + 2 < chars.len() && chars[start + 1] == quote && chars[start + 2] == quote;
    let mut current = start + if triple { 3 } else { 1 };
    let mut value = String::new();

    while current < chars.len() {
        if triple {
            if current + 2 < chars.len()
                && chars[current] == quote
                && chars[current + 1] == quote
                && chars[current + 2] == quote
            {
                return Ok((value, current + 3));
            }
        } else if chars[current] == quote {
            return Ok((value, current + 1));
        } else if matches!(chars[current], '\n' | '\r') {
            return Err(unterminated_string_message(chars, start, triple).to_string());
        }

        if chars[current] == '\\' {
            let escape_start = current;
            current += 1;
            if current >= chars.len() {
                return Err(unterminated_string_message(chars, start, triple).to_string());
            }

            if raw {
                if push_raw_string_line_continuation(chars, &mut current, &mut value) {
                    continue;
                }
                value.push('\\');
                value.push(chars[current]);
                current += 1;
                continue;
            }

            let escaped = chars[current];
            current += 1;
            match escaped {
                '\n' => {}
                '\r' => {
                    if current < chars.len() && chars[current] == '\n' {
                        current += 1;
                    }
                }
                '\\' => value.push('\\'),
                '\'' => value.push('\''),
                '"' => value.push('"'),
                'n' => value.push('\n'),
                'r' => value.push('\r'),
                't' => value.push('\t'),
                'a' => value.push('\x07'),
                'b' => value.push('\x08'),
                'f' => value.push('\x0c'),
                'v' => value.push('\x0b'),
                '0'..='7' => value.push(read_string_octal_escape(
                    escaped,
                    chars,
                    &mut current,
                    warnings,
                    escape_start,
                )?),
                'x' => value.push(read_escape_codepoint(
                    chars,
                    &mut current,
                    'x',
                    2,
                    quote,
                    triple,
                )?),
                'u' => value.push(read_escape_codepoint(
                    chars,
                    &mut current,
                    'u',
                    4,
                    quote,
                    triple,
                )?),
                'U' => value.push(read_escape_codepoint(
                    chars,
                    &mut current,
                    'U',
                    8,
                    quote,
                    triple,
                )?),
                'N' => value.push(read_unicode_name_escape(
                    chars,
                    &mut current,
                    quote,
                    triple,
                )?),
                other => {
                    warnings.push(syntax_warning_span(
                        chars,
                        escape_start,
                        current,
                        invalid_escape_warning(other),
                    ));
                    value.push('\\');
                    value.push(other);
                }
            }
            continue;
        }

        value.push(chars[current]);
        current += 1;
    }

    Err(unterminated_string_message(chars, start, triple).to_string())
}

fn lex_bytes_string(
    chars: &[char],
    start: usize,
    raw: bool,
    warnings: &mut Vec<LexWarning>,
) -> Result<(Vec<u8>, usize), String> {
    let quote = chars[start];
    let triple = start + 2 < chars.len() && chars[start + 1] == quote && chars[start + 2] == quote;
    let mut current = start + if triple { 3 } else { 1 };
    let mut value = Vec::new();

    while current < chars.len() {
        if triple {
            if current + 2 < chars.len()
                && chars[current] == quote
                && chars[current + 1] == quote
                && chars[current + 2] == quote
            {
                return Ok((value, current + 3));
            }
        } else if chars[current] == quote {
            return Ok((value, current + 1));
        } else if matches!(chars[current], '\n' | '\r') {
            return Err(unterminated_string_message(chars, start, triple).to_string());
        }

        if chars[current] == '\\' {
            let escape_start = current;
            current += 1;
            if current >= chars.len() {
                return Err(unterminated_string_message(chars, start, triple).to_string());
            }

            if raw {
                if push_raw_bytes_line_continuation(chars, &mut current, &mut value) {
                    continue;
                }
                value.push(b'\\');
                push_ascii_byte(chars[current], &mut value)?;
                current += 1;
                continue;
            }

            let escaped = chars[current];
            current += 1;
            match escaped {
                '\n' => {}
                '\r' => {
                    if current < chars.len() && chars[current] == '\n' {
                        current += 1;
                    }
                }
                '\\' => value.push(b'\\'),
                '\'' => value.push(b'\''),
                '"' => value.push(b'"'),
                'n' => value.push(b'\n'),
                'r' => value.push(b'\r'),
                't' => value.push(b'\t'),
                'a' => value.push(0x07),
                'b' => value.push(0x08),
                'f' => value.push(0x0c),
                'v' => value.push(0x0b),
                'x' => value.push(read_byte_hex_escape(chars, &mut current, quote, triple)?),
                '0'..='7' => value.push(read_byte_octal_escape(
                    escaped,
                    chars,
                    &mut current,
                    warnings,
                    escape_start,
                )?),
                other => {
                    warnings.push(syntax_warning_span(
                        chars,
                        escape_start,
                        current,
                        invalid_escape_warning(other),
                    ));
                    value.push(b'\\');
                    push_ascii_byte(other, &mut value)?;
                }
            }
            continue;
        }

        push_ascii_byte(chars[current], &mut value)?;
        current += 1;
    }

    Err(unterminated_string_message(chars, start, triple).to_string())
}

fn unterminated_string_message(chars: &[char], start: usize, triple: bool) -> &'static str {
    if triple {
        "unterminated triple-quoted string literal"
    } else if ends_with_escaped_quote(chars, start) {
        "unterminated string literal; perhaps you escaped the end quote"
    } else {
        "unterminated string literal"
    }
}

fn ends_with_escaped_quote(chars: &[char], quote_start: usize) -> bool {
    let Some(quote) = chars.get(quote_start).copied() else {
        return false;
    };

    matches!(
        chars.get(chars.len().saturating_sub(2)..),
        Some([backslash, end_quote]) if *backslash == '\\' && *end_quote == quote
    )
}

fn push_ascii_byte(ch: char, value: &mut Vec<u8>) -> Result<(), String> {
    if ch.is_ascii() {
        value.push(ch as u8);
        Ok(())
    } else {
        Err("bytes can only contain ASCII literal characters".to_string())
    }
}

fn push_raw_string_line_continuation(
    chars: &[char],
    current: &mut usize,
    value: &mut String,
) -> bool {
    match chars.get(*current) {
        Some('\n') => {
            value.push('\\');
            value.push('\n');
            *current += 1;
            true
        }
        Some('\r') => {
            value.push('\\');
            value.push('\n');
            *current += 1;
            if matches!(chars.get(*current), Some('\n')) {
                *current += 1;
            }
            true
        }
        _ => false,
    }
}

fn push_raw_bytes_line_continuation(
    chars: &[char],
    current: &mut usize,
    value: &mut Vec<u8>,
) -> bool {
    match chars.get(*current) {
        Some('\n') => {
            value.push(b'\\');
            value.push(b'\n');
            *current += 1;
            true
        }
        Some('\r') => {
            value.push(b'\\');
            value.push(b'\n');
            *current += 1;
            if matches!(chars.get(*current), Some('\n')) {
                *current += 1;
            }
            true
        }
        _ => false,
    }
}

fn read_byte_hex_escape(
    chars: &[char],
    current: &mut usize,
    quote: char,
    triple: bool,
) -> Result<u8, String> {
    if *current + 2 > chars.len() {
        return Err("invalid bytes escape: \\x".to_string());
    }

    if string_terminates_before_escape_digits(chars, *current, 2, quote, triple) {
        return Err("invalid bytes escape: \\x".to_string());
    }

    let text: String = chars[*current..*current + 2].iter().collect();
    if !text.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("invalid bytes escape: \\x{text}"));
    }

    *current += 2;
    u8::from_str_radix(&text, 16).map_err(|_| format!("invalid bytes escape: \\x{text}"))
}

fn read_byte_octal_escape(
    first: char,
    chars: &[char],
    current: &mut usize,
    warnings: &mut Vec<LexWarning>,
    escape_start: usize,
) -> Result<u8, String> {
    let mut text = String::from(first);
    while text.len() < 3 && *current < chars.len() && matches!(chars[*current], '0'..='7') {
        text.push(chars[*current]);
        *current += 1;
    }

    let value =
        u16::from_str_radix(&text, 8).map_err(|_| format!("invalid bytes escape: \\{text}"))?;
    if value > 0xff {
        warnings.push(syntax_warning_span(
            chars,
            escape_start,
            *current,
            invalid_octal_escape_warning(&text),
        ));
    }
    Ok((value & 0xff) as u8)
}

fn read_string_octal_escape(
    first: char,
    chars: &[char],
    current: &mut usize,
    warnings: &mut Vec<LexWarning>,
    escape_start: usize,
) -> Result<char, String> {
    let mut text = String::from(first);
    while text.len() < 3 && *current < chars.len() && matches!(chars[*current], '0'..='7') {
        text.push(chars[*current]);
        *current += 1;
    }

    let value =
        u32::from_str_radix(&text, 8).map_err(|_| format!("invalid string escape: \\{text}"))?;
    if value > 0xff {
        warnings.push(syntax_warning_span(
            chars,
            escape_start,
            *current,
            invalid_octal_escape_warning(&text),
        ));
    }
    char::from_u32(value).ok_or_else(|| format!("invalid string escape: \\{text}"))
}

fn invalid_escape_warning(ch: char) -> String {
    format!(
        "\"\\{ch}\" is an invalid escape sequence. Such sequences will not work in the future. Did you mean \"\\\\{ch}\"? A raw string is also an option."
    )
}

fn invalid_octal_escape_warning(text: &str) -> String {
    format!(
        "\"\\{text}\" is an invalid octal escape sequence. Such sequences will not work in the future. Did you mean \"\\\\{text}\"? A raw string is also an option."
    )
}

fn syntax_warning_span(chars: &[char], start: usize, end: usize, message: String) -> LexWarning {
    let (line, column) = source_location(chars, start);
    let (end_line, end_column) = source_location(chars, end);
    LexWarning {
        category: LexWarningCategory::SyntaxWarning,
        message,
        line,
        column,
        end_line,
        end_column,
    }
}

fn lex_error_at(chars: &[char], index: usize, message: impl Into<String>) -> LexError {
    lex_error_span(chars, index, index.saturating_add(1), message)
}

fn lex_error_span(
    chars: &[char],
    start: usize,
    end: usize,
    message: impl Into<String>,
) -> LexError {
    let start = start.min(chars.len());
    let end = end.min(chars.len()).max(start);
    let (line, column) = source_location(chars, start);
    let (end_line, end_column) = source_location(chars, end);
    LexError {
        message: message.into(),
        line,
        column,
        end_line,
        end_column,
    }
}

fn string_lex_error(
    chars: &[char],
    literal_start: usize,
    quote_start: usize,
    message: String,
) -> LexError {
    if is_unterminated_string_error(&message) {
        let triple = quote_start + 2 < chars.len()
            && chars[quote_start + 1] == chars[quote_start]
            && chars[quote_start + 2] == chars[quote_start];
        let content_start = quote_start + if triple { 3 } else { 1 };
        let end = if triple {
            chars.len()
        } else {
            chars[content_start..]
                .iter()
                .position(|ch| matches!(ch, '\n' | '\r'))
                .map(|position| content_start + position)
                .unwrap_or(chars.len())
        };
        return lex_error_span(chars, literal_start, end.max(literal_start + 1), message);
    }

    lex_error_at(chars, literal_start, message)
}

fn bytes_lex_error(
    chars: &[char],
    literal_start: usize,
    quote_start: usize,
    message: String,
) -> LexError {
    if message == "bytes can only contain ASCII literal characters" {
        let literal_end = bytes_literal_end(chars, quote_start).unwrap_or(chars.len());
        return lex_error_span(chars, literal_start, literal_end, message);
    }

    string_lex_error(chars, literal_start, quote_start, message)
}

fn bytes_literal_end(chars: &[char], quote_start: usize) -> Option<usize> {
    let quote = *chars.get(quote_start)?;
    let triple = quote_start + 2 < chars.len()
        && chars[quote_start + 1] == quote
        && chars[quote_start + 2] == quote;
    let mut current = quote_start + if triple { 3 } else { 1 };

    while current < chars.len() {
        if triple {
            if current + 2 < chars.len()
                && chars[current] == quote
                && chars[current + 1] == quote
                && chars[current + 2] == quote
            {
                return Some(current + 3);
            }
        } else if chars[current] == quote {
            return Some(current + 1);
        } else if matches!(chars[current], '\n' | '\r') {
            return Some(current);
        }

        if chars[current] == '\\' && current + 1 < chars.len() {
            current += 2;
        } else {
            current += 1;
        }
    }

    None
}

fn number_lex_error(chars: &[char], start: usize, message: impl Into<String>) -> LexError {
    let end = number_literal_error_end(chars, start);
    lex_error_span(chars, start, end.max(start + 1), message)
}

fn number_literal_error_end(chars: &[char], start: usize) -> usize {
    let mut current = start;

    while current < chars.len() {
        let ch = chars[current];
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
            current += 1;
            continue;
        }

        if matches!(ch, '+' | '-')
            && current > start
            && matches!(chars.get(current - 1), Some('e' | 'E'))
        {
            current += 1;
            continue;
        }

        break;
    }

    current
}

fn number_suffix_error(
    chars: &[char],
    start: usize,
    current: usize,
    token: &Token,
) -> Option<LexError> {
    let ch = *chars.get(current)?;

    if ch == '\u{2044}' {
        return Some(lex_error_span(
            chars,
            current,
            current + 1,
            "invalid character '⁄' (U+2044)",
        ));
    }

    if !is_identifier_continue(ch) {
        return None;
    }

    if number_suffix_warning_keyword(chars, current).is_some() {
        return None;
    }

    let mut end = current + 1;
    while end < chars.len() && is_identifier_continue(chars[end]) {
        end += 1;
    }

    Some(lex_error_span(
        chars,
        start,
        end,
        number_suffix_error_message(chars, start, token),
    ))
}

fn number_suffix_error_message(chars: &[char], start: usize, token: &Token) -> &'static str {
    if start + 1 < chars.len() && chars[start] == '0' {
        match chars[start + 1] {
            'b' | 'B' => return invalid_prefixed_integer_message(2),
            'o' | 'O' => return invalid_prefixed_integer_message(8),
            'x' | 'X' => return invalid_prefixed_integer_message(16),
            _ => {}
        }
    }

    if matches!(token, Token::Imaginary(_)) {
        "invalid imaginary literal"
    } else {
        invalid_decimal_literal_message()
    }
}

fn is_unterminated_string_error(message: &str) -> bool {
    message.starts_with("unterminated string literal")
        || matches!(
            message,
            "unterminated triple-quoted string literal"
                | "unterminated f-string literal"
                | "unterminated triple-quoted f-string literal"
                | "unterminated t-string literal"
                | "unterminated triple-quoted t-string literal"
        )
}

fn source_location(chars: &[char], index: usize) -> (usize, usize) {
    with_source_location_cache(chars, |cache| cache.char_location(index.min(chars.len())))
}

fn source_byte_location(chars: &[char], index: usize) -> (usize, usize) {
    with_source_location_cache(chars, |cache| cache.byte_location(index.min(chars.len())))
}

fn prepare_source_location_cache(chars: &[char]) {
    SOURCE_LOCATION_CACHE.with(|cache| {
        *cache.borrow_mut() = Some(SourceLocationCache::new(chars));
    });
}

fn with_source_location_cache<R>(
    chars: &[char],
    callback: impl FnOnce(&SourceLocationCache) -> R,
) -> R {
    SOURCE_LOCATION_CACHE.with(|cache| {
        let ptr = chars.as_ptr() as usize;
        let len = chars.len();
        let needs_rebuild = cache
            .borrow()
            .as_ref()
            .is_none_or(|cached| cached.ptr != ptr || cached.len != len);
        if needs_rebuild {
            *cache.borrow_mut() = Some(SourceLocationCache::new(chars));
        }

        let borrowed = cache.borrow();
        callback(borrowed.as_ref().expect("source location cache is set"))
    })
}

fn read_unicode_name_escape(
    chars: &[char],
    current: &mut usize,
    quote: char,
    triple: bool,
) -> Result<char, String> {
    if *current >= chars.len() || chars[*current] != '{' {
        return Err("malformed Unicode name escape: \\N".to_string());
    }

    *current += 1;
    let name_start = *current;
    while *current < chars.len() {
        if chars[*current] == '}' {
            let name: String = chars[name_start..*current].iter().collect();
            *current += 1;
            return lookup_unicode_name(&name)
                .ok_or_else(|| format!("unknown Unicode character name: {name}"));
        }

        if interpolated_string_terminates_at(chars, *current, quote, triple) {
            return Err("malformed Unicode name escape: \\N".to_string());
        }

        *current += 1;
    }

    Err("malformed Unicode name escape: \\N".to_string())
}

fn lookup_unicode_name(name: &str) -> Option<char> {
    if let Some(character) = unicode_names2::character(name) {
        if unicode_standard_name_matches(character, name) {
            return Some(character);
        }
    }

    lookup_unicode_alias(name)
}

fn unicode_standard_name_matches(character: char, name: &str) -> bool {
    let Some(standard_name) = unicode_names2::name(character) else {
        return false;
    };
    let standard_name = standard_name.to_string();

    standard_name.eq_ignore_ascii_case(name)
}

fn lookup_unicode_alias(name: &str) -> Option<char> {
    unicode_name_aliases()
        .get(&name.to_ascii_uppercase())
        .copied()
}

fn unicode_name_aliases() -> &'static HashMap<String, char> {
    static ALIASES: OnceLock<HashMap<String, char>> = OnceLock::new();
    ALIASES.get_or_init(build_unicode_name_aliases)
}

fn build_unicode_name_aliases() -> HashMap<String, char> {
    let mut aliases = HashMap::new();

    for codepoint in 0..=char::MAX as u32 {
        let Some(character) = char::from_u32(codepoint) else {
            continue;
        };
        let Some(alias_types) = NameAliasType::of(character) else {
            continue;
        };

        for alias_type in alias_types {
            let Some(names) = name_aliases_of(character, *alias_type) else {
                continue;
            };

            for name in names {
                aliases.insert(name.to_ascii_uppercase(), character);
            }
        }
    }

    aliases
}

fn lex_f_string(
    chars: &[char],
    start: usize,
    raw: bool,
    kind: InterpolatedStringKind,
    warnings: &mut Vec<LexWarning>,
) -> Result<(Vec<TokenFStringPart>, usize), String> {
    let quote = chars[start];
    let triple = start + 2 < chars.len() && chars[start + 1] == quote && chars[start + 2] == quote;
    let mut current = start + if triple { 3 } else { 1 };
    let mut literal = String::new();
    let mut parts = Vec::new();

    while current < chars.len() {
        if triple {
            if current + 2 < chars.len()
                && chars[current] == quote
                && chars[current + 1] == quote
                && chars[current + 2] == quote
            {
                push_f_string_literal(&mut parts, &mut literal);
                return Ok((parts, current + 3));
            }
        } else if chars[current] == quote {
            push_f_string_literal(&mut parts, &mut literal);
            return Ok((parts, current + 1));
        } else if matches!(chars[current], '\n' | '\r') {
            return Err(kind.unterminated_message(triple).to_string());
        }

        match chars[current] {
            '{' if current + 1 < chars.len() && chars[current + 1] == '{' => {
                literal.push('{');
                current += 2;
            }
            '}' if current + 1 < chars.len() && chars[current + 1] == '}' => {
                literal.push('}');
                current += 2;
            }
            '{' => {
                push_f_string_literal(&mut parts, &mut literal);
                let (source, conversion, format_spec, debug_label, next) =
                    read_f_string_expression(
                        chars,
                        current + 1,
                        kind,
                        quote,
                        triple,
                        raw,
                        warnings,
                    )?;
                parts.push(TokenFStringPart::Expression {
                    source,
                    conversion,
                    format_spec,
                    debug_label,
                });
                current = next;
            }
            '}' => return Err(kind.single_right_brace_message().to_string()),
            '\\' => {
                current += 1;
                if current >= chars.len() {
                    return Err(kind.unterminated_message(triple).to_string());
                }

                if raw {
                    if push_raw_string_line_continuation(chars, &mut current, &mut literal) {
                        continue;
                    }
                    literal.push('\\');
                    match chars[current] {
                        ch if ch == quote || ch == '\\' => {
                            literal.push(ch);
                            current += 1;
                        }
                        _ => {}
                    }
                    continue;
                }

                let escape_start = current - 1;
                let escaped_index = current;
                let escaped = chars[current];
                current += 1;
                match escaped {
                    '\n' => {}
                    '\r' => {
                        if current < chars.len() && chars[current] == '\n' {
                            current += 1;
                        }
                    }
                    '\\' => literal.push('\\'),
                    '\'' => literal.push('\''),
                    '"' => literal.push('"'),
                    'n' => literal.push('\n'),
                    'r' => literal.push('\r'),
                    't' => literal.push('\t'),
                    'a' => literal.push('\x07'),
                    'b' => literal.push('\x08'),
                    'f' => literal.push('\x0c'),
                    'v' => literal.push('\x0b'),
                    '0'..='7' => literal.push(read_string_octal_escape(
                        escaped,
                        chars,
                        &mut current,
                        warnings,
                        escape_start,
                    )?),
                    'x' => literal.push(read_escape_codepoint(
                        chars,
                        &mut current,
                        'x',
                        2,
                        quote,
                        triple,
                    )?),
                    'u' => literal.push(read_escape_codepoint(
                        chars,
                        &mut current,
                        'u',
                        4,
                        quote,
                        triple,
                    )?),
                    'U' => literal.push(read_escape_codepoint(
                        chars,
                        &mut current,
                        'U',
                        8,
                        quote,
                        triple,
                    )?),
                    'N' => literal.push(read_unicode_name_escape(
                        chars,
                        &mut current,
                        quote,
                        triple,
                    )?),
                    '{' | '}' => {
                        warnings.push(syntax_warning_span(
                            chars,
                            escape_start,
                            escaped_index + 1,
                            invalid_escape_warning(escaped),
                        ));
                        literal.push('\\');
                        current = escaped_index;
                    }
                    other => {
                        warnings.push(syntax_warning_span(
                            chars,
                            escape_start,
                            current,
                            invalid_escape_warning(other),
                        ));
                        literal.push('\\');
                        literal.push(other);
                    }
                }
            }
            ch => {
                literal.push(ch);
                current += 1;
            }
        }
    }

    Err(kind.unterminated_message(triple).to_string())
}

fn push_f_string_literal(parts: &mut Vec<TokenFStringPart>, literal: &mut String) {
    if !literal.is_empty() {
        parts.push(TokenFStringPart::Literal(std::mem::take(literal)));
    }
}

fn read_f_string_expression(
    chars: &[char],
    start: usize,
    kind: InterpolatedStringKind,
    quote: char,
    triple: bool,
    raw: bool,
    warnings: &mut Vec<LexWarning>,
) -> Result<
    (
        String,
        Option<TokenFStringConversion>,
        Option<Vec<TokenFStringPart>>,
        Option<String>,
        usize,
    ),
    String,
> {
    let mut current = start;
    let mut depth = 0usize;

    while current < chars.len() {
        match chars[current] {
            '"' | '\'' if quote_starts_interpolated_expression_string(chars, start, current) => {
                current = skip_quoted_expression_string(chars, current)?;
            }
            _ if interpolated_string_terminates_at(chars, current, quote, triple) => {
                return Err(kind.expecting_right_brace_message().to_string());
            }
            '"' | '\'' => {
                current = skip_quoted_expression_string(chars, current)?;
            }
            '#' => skip_f_string_expression_comment(chars, &mut current, quote, triple),
            '(' | '[' | '{' => {
                depth += 1;
                current += 1;
            }
            ')' | ']' => {
                depth = depth.saturating_sub(1);
                current += 1;
            }
            '}' if depth > 0 => {
                depth -= 1;
                current += 1;
            }
            '}' if expression_source_is_empty(chars, start, current) => {
                return Err(kind
                    .empty_expression_before_right_brace_message()
                    .to_string());
            }
            '}' => {
                let source = chars[start..current].iter().collect::<String>();
                return Ok((source, None, None, None, current + 1));
            }
            '!' if depth == 0 && current + 1 < chars.len() && chars[current + 1] == '=' => {
                current += 2;
            }
            '!' if depth == 0 && expression_source_is_empty(chars, start, current) => {
                return Err(format!(
                    "{}: valid expression required before '!'",
                    kind.label()
                ));
            }
            '!' if depth == 0 => {
                let source = chars[start..current].iter().collect::<String>();
                let (conversion, next) =
                    read_f_string_conversion(chars, current, kind, quote, triple)?;
                current = next;
                skip_f_string_conversion_padding(chars, &mut current);
                if current < chars.len()
                    && interpolated_string_terminates_at(chars, current, quote, triple)
                {
                    return Err(kind.expecting_right_brace_message().to_string());
                }
                match chars.get(current) {
                    Some('}') => return Ok((source, Some(conversion), None, None, current + 1)),
                    Some(':') => {
                        let (format_spec, next) = read_f_string_format_spec(
                            chars,
                            current + 1,
                            kind,
                            quote,
                            triple,
                            raw,
                            warnings,
                        )?;
                        return Ok((source, Some(conversion), Some(format_spec), None, next));
                    }
                    _ => return Err(format!("{}: expecting ':' or '}}'", kind.label())),
                }
            }
            '=' if depth == 0 && expression_source_is_empty(chars, start, current) => {
                return Err(format!(
                    "{}: valid expression required before '='",
                    kind.label()
                ));
            }
            '=' if depth == 0 && is_f_string_debug_equal(chars, start, current) => {
                if expression_source_is_empty(chars, start, current) {
                    return Err(format!(
                        "{}: valid expression required before '='",
                        kind.label()
                    ));
                }

                let debug_equal = current;
                let source = chars[start..debug_equal].iter().collect::<String>();
                current += 1;
                skip_f_string_field_padding_and_comments(chars, &mut current, quote, triple);
                let debug_label = f_string_debug_label(chars, start, current, quote, triple)?;
                match chars.get(current) {
                    Some('}') => {
                        return Ok((
                            source,
                            Some(TokenFStringConversion::Repr),
                            None,
                            Some(debug_label),
                            current + 1,
                        ));
                    }
                    Some('!') => {
                        let (conversion, next) =
                            read_f_string_conversion(chars, current, kind, quote, triple)?;
                        current = next;
                        skip_f_string_conversion_padding(chars, &mut current);
                        if current < chars.len()
                            && interpolated_string_terminates_at(chars, current, quote, triple)
                        {
                            return Err(kind.expecting_right_brace_message().to_string());
                        }
                        match chars.get(current) {
                            Some('}') => {
                                return Ok((
                                    source,
                                    Some(conversion),
                                    None,
                                    Some(debug_label),
                                    current + 1,
                                ));
                            }
                            Some(':') => {
                                let (format_spec, next) = read_f_string_format_spec(
                                    chars,
                                    current + 1,
                                    kind,
                                    quote,
                                    triple,
                                    raw,
                                    warnings,
                                )?;
                                return Ok((
                                    source,
                                    Some(conversion),
                                    Some(format_spec),
                                    Some(debug_label),
                                    next,
                                ));
                            }
                            _ => return Err(format!("{}: expecting ':' or '}}'", kind.label())),
                        }
                    }
                    Some(':') => {
                        let (format_spec, next) = read_f_string_format_spec(
                            chars,
                            current + 1,
                            kind,
                            quote,
                            triple,
                            raw,
                            warnings,
                        )?;
                        return Ok((source, None, Some(format_spec), Some(debug_label), next));
                    }
                    _ => {
                        return Err(format!("{}: expecting '!', or ':', or '}}'", kind.label()));
                    }
                }
            }
            ':' if depth == 0 && expression_source_is_empty(chars, start, current) => {
                return Err(format!(
                    "{}: valid expression required before ':'",
                    kind.label()
                ));
            }
            ':' if depth == 0 => {
                let source = chars[start..current].iter().collect::<String>();
                let (format_spec, next) = read_f_string_format_spec(
                    chars,
                    current + 1,
                    kind,
                    quote,
                    triple,
                    raw,
                    warnings,
                )?;
                return Ok((source, None, Some(format_spec), None, next));
            }
            _ => current += 1,
        }
    }

    Err(kind.unterminated_message(triple).to_string())
}

fn read_f_string_format_spec(
    chars: &[char],
    start: usize,
    kind: InterpolatedStringKind,
    quote: char,
    triple: bool,
    raw: bool,
    warnings: &mut Vec<LexWarning>,
) -> Result<(Vec<TokenFStringPart>, usize), String> {
    let mut current = start;
    let mut literal = String::new();
    let mut parts = Vec::new();

    while current < chars.len() {
        match chars[current] {
            '{' if current + 1 < chars.len() && chars[current + 1] == '{' => {
                literal.push('{');
                current += 2;
            }
            '}' => {
                push_f_string_literal(&mut parts, &mut literal);
                return Ok((parts, current + 1));
            }
            '\n' | '\r' if !triple => return Err(kind.format_spec_newline_message().to_string()),
            '\n' | '\r' => {
                literal.push(chars[current]);
                current += 1;
            }
            '\\' => {
                current += 1;
                if current >= chars.len() {
                    return Err(kind.format_spec_end_message().to_string());
                }

                if raw {
                    if push_raw_string_line_continuation(chars, &mut current, &mut literal) {
                        continue;
                    }
                    literal.push('\\');
                    match chars[current] {
                        ch if ch == quote || ch == '\\' => {
                            literal.push(ch);
                            current += 1;
                        }
                        _ => {}
                    }
                    continue;
                }

                let escape_start = current - 1;
                let escaped_index = current;
                let escaped = chars[current];
                current += 1;
                match escaped {
                    '\n' => {}
                    '\r' => {
                        if current < chars.len() && chars[current] == '\n' {
                            current += 1;
                        }
                    }
                    '\\' => literal.push('\\'),
                    '\'' => literal.push('\''),
                    '"' => literal.push('"'),
                    'n' => literal.push('\n'),
                    'r' => literal.push('\r'),
                    't' => literal.push('\t'),
                    'a' => literal.push('\x07'),
                    'b' => literal.push('\x08'),
                    'f' => literal.push('\x0c'),
                    'v' => literal.push('\x0b'),
                    '0'..='7' => literal.push(read_string_octal_escape(
                        escaped,
                        chars,
                        &mut current,
                        warnings,
                        escape_start,
                    )?),
                    'x' => literal.push(read_escape_codepoint(
                        chars,
                        &mut current,
                        'x',
                        2,
                        quote,
                        triple,
                    )?),
                    'u' => literal.push(read_escape_codepoint(
                        chars,
                        &mut current,
                        'u',
                        4,
                        quote,
                        triple,
                    )?),
                    'U' => literal.push(read_escape_codepoint(
                        chars,
                        &mut current,
                        'U',
                        8,
                        quote,
                        triple,
                    )?),
                    'N' => literal.push(read_unicode_name_escape(
                        chars,
                        &mut current,
                        quote,
                        triple,
                    )?),
                    '{' | '}' => {
                        warnings.push(syntax_warning_span(
                            chars,
                            escape_start,
                            escaped_index + 1,
                            invalid_escape_warning(escaped),
                        ));
                        literal.push('\\');
                        current = escaped_index;
                    }
                    other => {
                        warnings.push(syntax_warning_span(
                            chars,
                            escape_start,
                            current,
                            invalid_escape_warning(other),
                        ));
                        literal.push('\\');
                        literal.push(other);
                    }
                }
            }
            '{' => {
                push_f_string_literal(&mut parts, &mut literal);
                let (source, conversion, format_spec, debug_label, next) =
                    read_f_string_expression(
                        chars,
                        current + 1,
                        kind,
                        quote,
                        triple,
                        raw,
                        warnings,
                    )?;
                parts.push(TokenFStringPart::Expression {
                    source,
                    conversion,
                    format_spec,
                    debug_label,
                });
                current = next;
            }
            ch => {
                literal.push(ch);
                current += 1;
            }
        }
    }

    Err(kind.format_spec_end_message().to_string())
}

fn read_f_string_conversion(
    chars: &[char],
    bang: usize,
    kind: InterpolatedStringKind,
    quote: char,
    triple: bool,
) -> Result<(TokenFStringConversion, usize), String> {
    let conversion_index = bang + 1;
    if conversion_index >= chars.len() {
        return Err(kind.unterminated_message(triple).to_string());
    }
    if interpolated_string_terminates_at(chars, conversion_index, quote, triple) {
        return Err(kind.expecting_right_brace_message().to_string());
    }

    let conversion_char = chars[conversion_index];
    if matches!(conversion_char, ':' | '}') {
        return Err(format!("{}: missing conversion character", kind.label()));
    }
    if is_f_string_conversion_padding(conversion_char) {
        return Err(format!(
            "{}: conversion type must come right after the exclamation mark",
            kind.label()
        ));
    }

    let next = conversion_index + 1;
    if chars
        .get(next)
        .is_some_and(|ch| is_f_string_conversion_name_char(*ch))
    {
        let text = collect_f_string_conversion_name(chars, conversion_index);
        return Err(format!(
            "{}: invalid conversion character: {text}",
            kind.label()
        ));
    }

    let conversion = match conversion_char {
        's' => TokenFStringConversion::Str,
        'r' => TokenFStringConversion::Repr,
        'a' => TokenFStringConversion::Ascii,
        ch => {
            return Err(format!(
                "{}: invalid conversion character: {ch}",
                kind.label()
            ));
        }
    };

    Ok((conversion, next))
}

fn skip_f_string_conversion_padding(chars: &[char], current: &mut usize) {
    while *current < chars.len() && is_f_string_conversion_padding(chars[*current]) {
        *current += 1;
    }
}

fn skip_f_string_field_padding_and_comments(
    chars: &[char],
    current: &mut usize,
    quote: char,
    triple: bool,
) {
    loop {
        while *current < chars.len() && is_interpolated_expression_whitespace(chars[*current]) {
            *current += 1;
        }
        if *current < chars.len() && chars[*current] == '#' {
            skip_f_string_expression_comment(chars, current, quote, triple);
            continue;
        }
        break;
    }
}

fn skip_f_string_expression_comment(
    chars: &[char],
    current: &mut usize,
    quote: char,
    triple: bool,
) {
    while *current < chars.len()
        && !matches!(chars[*current], '\n' | '\r')
        && !interpolated_string_terminates_at(chars, *current, quote, triple)
    {
        *current += 1;
    }
}

fn f_string_debug_label(
    chars: &[char],
    start: usize,
    end: usize,
    quote: char,
    triple: bool,
) -> Result<String, String> {
    let mut current = start;
    let mut label = String::new();

    while current < end {
        match chars[current] {
            '"' | '\'' => {
                let next = skip_quoted_expression_string(chars, current)?;
                label.extend(chars[current..next].iter());
                current = next;
            }
            '#' => {
                skip_f_string_expression_comment(chars, &mut current, quote, triple);
            }
            ch => {
                label.push(ch);
                current += 1;
            }
        }
    }

    Ok(label)
}

fn is_f_string_conversion_padding(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\x0c')
}

fn is_f_string_conversion_name_char(ch: char) -> bool {
    ch == '_' || ch.is_alphanumeric()
}

fn collect_f_string_conversion_name(chars: &[char], start: usize) -> String {
    let mut current = start;
    while current < chars.len() && is_f_string_conversion_name_char(chars[current]) {
        current += 1;
    }

    chars[start..current].iter().collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InterpolatedStringKind {
    Formatted,
    Template,
}

impl InterpolatedStringKind {
    fn label(self) -> &'static str {
        match self {
            InterpolatedStringKind::Formatted => "f-string",
            InterpolatedStringKind::Template => "t-string",
        }
    }

    fn cpython_start_token(self, text: String) -> Token {
        match self {
            InterpolatedStringKind::Formatted => Token::FStringStart(text),
            InterpolatedStringKind::Template => Token::TStringStart(text),
        }
    }

    fn cpython_middle_token(self, text: String) -> Token {
        match self {
            InterpolatedStringKind::Formatted => Token::FStringMiddle(text),
            InterpolatedStringKind::Template => Token::TStringMiddle(text),
        }
    }

    fn cpython_end_token(self, text: String) -> Token {
        match self {
            InterpolatedStringKind::Formatted => Token::FStringEnd(text),
            InterpolatedStringKind::Template => Token::TStringEnd(text),
        }
    }

    fn unterminated_message(self, triple: bool) -> &'static str {
        match (self, triple) {
            (InterpolatedStringKind::Formatted, true) => {
                "unterminated triple-quoted f-string literal"
            }
            (InterpolatedStringKind::Formatted, false) => "unterminated f-string literal",
            (InterpolatedStringKind::Template, true) => {
                "unterminated triple-quoted t-string literal"
            }
            (InterpolatedStringKind::Template, false) => "unterminated t-string literal",
        }
    }

    fn single_right_brace_message(self) -> &'static str {
        match self {
            InterpolatedStringKind::Formatted => "single '}' is not allowed in f-string literal",
            InterpolatedStringKind::Template => "t-string: single '}' is not allowed",
        }
    }

    fn empty_expression_before_right_brace_message(self) -> &'static str {
        match self {
            InterpolatedStringKind::Formatted => "f-string: valid expression required before '}'",
            InterpolatedStringKind::Template => "t-string: valid expression required before '}'",
        }
    }

    fn format_spec_end_message(self) -> &'static str {
        match self {
            InterpolatedStringKind::Formatted => "f-string: expecting '}', or format specs",
            InterpolatedStringKind::Template => "t-string: expecting '}', or format specs",
        }
    }

    fn format_spec_newline_message(self) -> &'static str {
        match self {
            InterpolatedStringKind::Formatted => {
                "f-string: newlines are not allowed in format specifiers"
            }
            InterpolatedStringKind::Template => {
                "t-string: newlines are not allowed in format specifiers"
            }
        }
    }

    fn expecting_right_brace_message(self) -> &'static str {
        match self {
            InterpolatedStringKind::Formatted => "f-string: expecting '}'",
            InterpolatedStringKind::Template => "t-string: expecting '}'",
        }
    }
}

fn is_f_string_debug_equal(chars: &[char], start: usize, current: usize) -> bool {
    if chars.get(current + 1) == Some(&'=') {
        return false;
    }

    let previous = chars[start..current]
        .iter()
        .rev()
        .find(|ch| !is_interpolated_expression_whitespace(**ch));

    !matches!(previous, None | Some('=' | '!' | '<' | '>' | ':'))
}

fn expression_source_is_empty(chars: &[char], start: usize, end: usize) -> bool {
    chars[start..end]
        .iter()
        .all(|ch| is_interpolated_expression_whitespace(*ch))
}

fn is_interpolated_expression_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\x0c')
}

fn skip_quoted_expression_string(chars: &[char], start: usize) -> Result<usize, String> {
    let quote = chars[start];
    let triple = start + 2 < chars.len() && chars[start + 1] == quote && chars[start + 2] == quote;
    let mut current = start + if triple { 3 } else { 1 };

    while current < chars.len() {
        if triple {
            if current + 2 < chars.len()
                && chars[current] == quote
                && chars[current + 1] == quote
                && chars[current + 2] == quote
            {
                return Ok(current + 3);
            }
        } else if chars[current] == quote {
            return Ok(current + 1);
        }

        if chars[current] == '\\' {
            current += 2;
        } else {
            current += 1;
        }
    }

    Err("unterminated string in f-string expression".to_string())
}

fn quote_starts_interpolated_expression_string(
    chars: &[char],
    expression_start: usize,
    quote_index: usize,
) -> bool {
    if !matches!(chars.get(quote_index), Some('\'' | '"')) {
        return false;
    }

    let mut prefix_start = quote_index;
    while prefix_start > expression_start && chars[prefix_start - 1].is_ascii_alphabetic() {
        prefix_start -= 1;
    }
    let prefix = chars[prefix_start..quote_index].iter().collect::<String>();
    let has_valid_prefix = prefix_start < quote_index && parse_string_prefix(&prefix).is_some();
    if prefix_start < quote_index && !has_valid_prefix {
        return false;
    }
    if skip_quoted_expression_string(chars, quote_index).is_err() {
        return false;
    }

    chars[expression_start..prefix_start]
        .iter()
        .rev()
        .find(|ch| !is_interpolated_expression_whitespace(**ch))
        .is_none_or(|ch| {
            if prefix_start == quote_index && chars[quote_index - 1].is_whitespace() {
                return true;
            }
            matches!(
                *ch,
                '(' | '['
                    | '{'
                    | ','
                    | ':'
                    | '='
                    | '+'
                    | '-'
                    | '*'
                    | '/'
                    | '%'
                    | '@'
                    | '~'
                    | '<'
                    | '>'
                    | '!'
                    | '&'
                    | '|'
                    | '^'
            )
        })
}

fn read_escape_codepoint(
    chars: &[char],
    current: &mut usize,
    prefix: char,
    digits: usize,
    quote: char,
    triple: bool,
) -> Result<char, String> {
    if *current + digits > chars.len() {
        return Err(format!("truncated string escape: \\{prefix}"));
    }

    if string_terminates_before_escape_digits(chars, *current, digits, quote, triple) {
        return Err(format!("truncated string escape: \\{prefix}"));
    }

    let text: String = chars[*current..*current + digits].iter().collect();
    if !text.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(format!("invalid string escape: \\{prefix}{text}"));
    }

    *current += digits;
    let value = u32::from_str_radix(&text, 16)
        .map_err(|_| format!("invalid string escape: \\{prefix}{text}"))?;
    char::from_u32(value).ok_or_else(|| format!("invalid string escape: \\{prefix}{text}"))
}

fn string_terminates_before_escape_digits(
    chars: &[char],
    start: usize,
    digits: usize,
    quote: char,
    triple: bool,
) -> bool {
    for offset in 0..digits {
        let index = start + offset;
        if triple {
            if index + 2 < chars.len()
                && chars[index] == quote
                && chars[index + 1] == quote
                && chars[index + 2] == quote
            {
                return true;
            }
        } else if index < chars.len() && chars[index] == quote {
            return true;
        }
    }

    false
}

fn interpolated_string_terminates_at(
    chars: &[char],
    current: usize,
    quote: char,
    triple: bool,
) -> bool {
    if triple {
        return current + 2 < chars.len()
            && chars[current] == quote
            && chars[current + 1] == quote
            && chars[current + 2] == quote;
    }

    current < chars.len() && chars[current] == quote
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StringPrefix {
    raw: bool,
    formatted: bool,
    template: bool,
    bytes: bool,
}

fn parse_string_prefix(word: &str) -> Option<StringPrefix> {
    match word {
        "r" | "R" => Some(StringPrefix {
            raw: true,
            formatted: false,
            template: false,
            bytes: false,
        }),
        "u" | "U" => Some(StringPrefix {
            raw: false,
            formatted: false,
            template: false,
            bytes: false,
        }),
        "f" | "F" => Some(StringPrefix {
            raw: false,
            formatted: true,
            template: false,
            bytes: false,
        }),
        "fr" | "fR" | "Fr" | "FR" | "rf" | "rF" | "Rf" | "RF" => Some(StringPrefix {
            raw: true,
            formatted: true,
            template: false,
            bytes: false,
        }),
        "t" | "T" => Some(StringPrefix {
            raw: false,
            formatted: false,
            template: true,
            bytes: false,
        }),
        "tr" | "tR" | "Tr" | "TR" | "rt" | "rT" | "Rt" | "RT" => Some(StringPrefix {
            raw: true,
            formatted: false,
            template: true,
            bytes: false,
        }),
        "b" | "B" => Some(StringPrefix {
            raw: false,
            formatted: false,
            template: false,
            bytes: true,
        }),
        "br" | "bR" | "Br" | "BR" | "rb" | "rB" | "Rb" | "RB" => Some(StringPrefix {
            raw: true,
            formatted: false,
            template: false,
            bytes: true,
        }),
        _ => None,
    }
}

fn is_invalid_string_prefix(word: &str) -> bool {
    !word.is_empty()
        && word.chars().all(|ch| {
            matches!(
                ch,
                'r' | 'R' | 'u' | 'U' | 'f' | 'F' | 't' | 'T' | 'b' | 'B'
            )
        })
}

fn scan_digit_part(chars: &[char], current: &mut usize, mode: LexMode) -> Result<bool, String> {
    let start = *current;

    while *current < chars.len() {
        if chars[*current].is_ascii_digit() {
            *current += 1;
            continue;
        }

        if chars[*current] == '_' {
            let invalid = *current == start
                || *current + 1 >= chars.len()
                || !chars[*current + 1].is_ascii_digit()
                || !chars[*current - 1].is_ascii_digit();
            if invalid {
                if !mode.rejects_number_suffixes() {
                    break;
                }
                return Err(invalid_decimal_literal_message().to_string());
            }
            *current += 1;
            continue;
        }

        break;
    }

    Ok(*current > start)
}

fn update_indentation(
    indent: Indentation,
    indent_stack: &mut Vec<Indentation>,
    tokens: &mut Vec<SpannedToken>,
    chars: &[char],
    current: usize,
) -> Result<(), String> {
    let current_indent = *indent_stack
        .last()
        .expect("indent stack always contains the base indent");

    if indent.column > current_indent.column {
        if indent.alt_column <= current_indent.alt_column {
            return Err("inconsistent use of tabs and spaces in indentation".to_string());
        }
        if indent_stack.len() >= MAX_INDENT_LEVELS {
            return Err("too many levels of indentation".to_string());
        }
        indent_stack.push(indent);
        push_token(tokens, chars, Token::Indent, current, current);
        return Ok(());
    }

    while indent.column
        < indent_stack
            .last()
            .expect("indent stack always contains the base indent")
            .column
    {
        indent_stack.pop();
        push_token(tokens, chars, Token::Dedent, current, current);
    }

    let matched_indent = *indent_stack
        .last()
        .expect("indent stack always contains the base indent");

    if indent.column != matched_indent.column {
        return Err("unindent does not match any outer indentation level".to_string());
    }

    if indent.alt_column != matched_indent.alt_column {
        return Err("inconsistent use of tabs and spaces in indentation".to_string());
    }

    Ok(())
}

fn validate_explicit_line_join_indentation(
    indent: Indentation,
    indent_stack: &[Indentation],
) -> Result<(), String> {
    let current_indent = *indent_stack
        .last()
        .expect("indent stack always contains the base indent");

    if indent.column > current_indent.column {
        return Ok(());
    }

    let Some(matched_indent) = indent_stack
        .iter()
        .rev()
        .copied()
        .find(|level| level.column == indent.column)
    else {
        return Err("unindent does not match any outer indentation level".to_string());
    };

    if indent.alt_column != matched_indent.alt_column {
        return Err("inconsistent use of tabs and spaces in indentation".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_BRACKET_DEPTH, MAX_INDENT_LEVELS, Token, TokenFStringConversion, TokenFStringPart,
        function_definition_line_sequences, generator_expression_line_sequences, lex,
        lex_with_spans_for_parse,
    };

    #[test]
    fn lexes_print_number() {
        assert_eq!(
            lex("print(123)"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(123),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_unicode_identifiers() {
        assert_eq!(
            lex("tenπ = 31.4\n变量_π = tenπ\nK = ｘ"),
            Ok(vec![
                Token::Identifier("tenπ".to_string()),
                Token::Equal,
                Token::Float("31.4".to_string()),
                Token::Newline,
                Token::Identifier("变量_π".to_string()),
                Token::Equal,
                Token::Identifier("tenπ".to_string()),
                Token::Newline,
                Token::Identifier("K".to_string()),
                Token::Equal,
                Token::Identifier("x".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn tracks_async_for_function_line_sequence() {
        let (tokens, _warnings) = lex_with_spans_for_parse(
            "async def test(aseq):\n    async for i in aseq:\n        body",
        )
        .unwrap();
        assert_eq!(
            function_definition_line_sequences(&tokens),
            vec![vec![1, 2, 3, 2]]
        );
    }

    #[test]
    fn tracks_if_function_line_sequence() {
        let (tokens, _warnings) =
            lex_with_spans_for_parse("def if1(x):\n    x()\n    if TRUE:\n        pass").unwrap();
        assert_eq!(
            function_definition_line_sequences(&tokens),
            vec![vec![1, 2, 3, 4, 3]]
        );
    }

    #[test]
    fn tracks_loop_with_nested_if_function_line_sequence() {
        let (tokens, _warnings) =
            lex_with_spans_for_parse("def f():\n    for i in x:\n        if y:\n            pass")
                .unwrap();
        assert_eq!(
            function_definition_line_sequences(&tokens),
            vec![vec![1, 2, 3, 4, 2]]
        );
    }

    #[test]
    fn tracks_try_except_loop_function_line_sequence() {
        let (tokens, _warnings) = lex_with_spans_for_parse(
            "def f():\n    for x in it:\n        try:\n            if C1:\n                yield 2\n        except OSError:\n            pass",
        )
        .unwrap();
        assert_eq!(
            function_definition_line_sequences(&tokens),
            vec![vec![1, 2, 3, 4, 5, 4, 2, 6, 7, 6]]
        );
    }

    #[test]
    fn tracks_while_else_try_break_function_line_sequence() {
        let (tokens, _warnings) = lex_with_spans_for_parse(
            "def f():\n    while name:\n        try:\n            break\n        except:\n            pass\n    else:\n        1 if 1 else 1",
        )
        .unwrap();
        assert_eq!(
            function_definition_line_sequences(&tokens),
            vec![vec![1, 2, 3, 4, 8, 5, 6]]
        );
    }

    #[test]
    fn tracks_generator_expression_line_sequence() {
        let (tokens, _warnings) = lex_with_spans_for_parse(
            "def return_genexp():\n    return (1\n            for\n            x\n            in\n            y)",
        )
        .unwrap();
        assert_eq!(
            generator_expression_line_sequences(&tokens),
            vec![(2, vec![6, 2, 6, 4, 2, 6])]
        );
    }

    #[test]
    fn lexes_float_literals() {
        assert_eq!(
            lex(
                "1.5 .5 1. 1e3 1.5e-2 3.14 314. 0.314 000.314 .314 3e14 3E14 3e-14 3e+14 3.e14 .3e14 3.1e4"
            ),
            Ok(vec![
                Token::Float("1.5".to_string()),
                Token::Float(".5".to_string()),
                Token::Float("1.".to_string()),
                Token::Float("1e3".to_string()),
                Token::Float("1.5e-2".to_string()),
                Token::Float("3.14".to_string()),
                Token::Float("314.".to_string()),
                Token::Float("0.314".to_string()),
                Token::Float("000.314".to_string()),
                Token::Float(".314".to_string()),
                Token::Float("3e14".to_string()),
                Token::Float("3E14".to_string()),
                Token::Float("3e-14".to_string()),
                Token::Float("3e+14".to_string()),
                Token::Float("3.e14".to_string()),
                Token::Float(".3e14".to_string()),
                Token::Float("3.1e4".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_number_keyword_boundaries() {
        assert_eq!(
            lex("1else 1jand 0xfor 0x1ffor"),
            Ok(vec![
                Token::Number(1),
                Token::Else,
                Token::Imaginary("1".to_string()),
                Token::And,
                Token::Number(15),
                Token::Or,
                Token::Number(511),
                Token::Or,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_imaginary_literals() {
        assert_eq!(
            lex("1j 1J 1.5j .5j 1.j 1e3j 1_00_00.5j"),
            Ok(vec![
                Token::Imaginary("1".to_string()),
                Token::Imaginary("1".to_string()),
                Token::Imaginary("1.5".to_string()),
                Token::Imaginary(".5".to_string()),
                Token::Imaginary("1.".to_string()),
                Token::Imaginary("1e3".to_string()),
                Token::Imaginary("1_00_00.5".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_number_separators() {
        assert_eq!(
            lex("1_000 1_000.5 1.5_0 1e1_0"),
            Ok(vec![
                Token::Number(1000),
                Token::Float("1_000.5".to_string()),
                Token::Float("1.5_0".to_string()),
                Token::Float("1e1_0".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_large_integer_literals() {
        assert_eq!(
            lex("9223372036854775808 0xffffffffffffffff 1_000_000_000_000_000_000_000"),
            Ok(vec![
                Token::BigInt("9223372036854775808".to_string()),
                Token::BigInt("18446744073709551615".to_string()),
                Token::BigInt("1000000000000000000000".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn rejects_invalid_number_separators() {
        assert_eq!(lex("1_"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1__0"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1e_1"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1.2_"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1e2_"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1e+"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1_.4"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1._4"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("._5"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1.0e+_1"), Err("invalid decimal literal".to_string()));
        assert_eq!(lex("1.4_e1"), Err("invalid decimal literal".to_string()));
    }

    #[test]
    fn lexes_prefixed_integer_literals() {
        assert_eq!(
            lex("0b1001 0B10 0o377 0O7 0xff 0XFF 0b_1010 0o_7 0x_f"),
            Ok(vec![
                Token::Number(9),
                Token::Number(2),
                Token::Number(255),
                Token::Number(7),
                Token::Number(255),
                Token::Number(255),
                Token::Number(10),
                Token::Number(7),
                Token::Number(15),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn rejects_invalid_prefixed_integer_literals() {
        assert_eq!(lex("0b"), Err("invalid binary literal".to_string()));
        assert_eq!(
            lex("0b12"),
            Err("invalid digit '2' in binary literal".to_string())
        );
        assert_eq!(
            lex("0o18"),
            Err("invalid digit '8' in octal literal".to_string())
        );
        assert_eq!(lex("0x"), Err("invalid hexadecimal literal".to_string()));
        assert_eq!(lex("0x_"), Err("invalid hexadecimal literal".to_string()));
    }

    #[test]
    fn rejects_nonzero_leading_decimal_zeroes() {
        assert_eq!(
            lex("012"),
            Err("leading zeros in decimal integer literals are not permitted; use an 0o prefix for octal integers".to_string())
        );
        assert_eq!(
            lex("0_7"),
            Err("leading zeros in decimal integer literals are not permitted; use an 0o prefix for octal integers".to_string())
        );
        assert_eq!(lex("0_0_0"), Ok(vec![Token::Number(0), Token::Eof]));
    }

    #[test]
    fn lexes_attribute_dot_after_parenthesized_number() {
        assert_eq!(
            lex("(1).value"),
            Ok(vec![
                Token::LeftParen,
                Token::Number(1),
                Token::RightParen,
                Token::Dot,
                Token::Identifier("value".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_attribute_dot_after_bytes_literal() {
        assert_eq!(
            lex("b'ab'.__iter__"),
            Ok(vec![
                Token::Bytes(vec![b'a', b'b']),
                Token::Dot,
                Token::Identifier("__iter__".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_underscore_relative_import_module_after_dot() {
        assert_eq!(
            lex("from ._threading_handler import install_threading_hook"),
            Ok(vec![
                Token::From,
                Token::Dot,
                Token::Identifier("_threading_handler".to_string()),
                Token::Import,
                Token::Identifier("install_threading_hook".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn skips_whitespace() {
        assert_eq!(
            lex("print( 123 )"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(123),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_plus() {
        assert_eq!(
            lex("print(1 + 2)"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(1),
                Token::Plus,
                Token::Number(2),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_arithmetic_operators() {
        assert_eq!(
            lex("1 - 2 * 3 @ 4 / 5 // 6 % 7 ** 8"),
            Ok(vec![
                Token::Number(1),
                Token::Minus,
                Token::Number(2),
                Token::Star,
                Token::Number(3),
                Token::At,
                Token::Number(4),
                Token::Slash,
                Token::Number(5),
                Token::DoubleSlash,
                Token::Number(6),
                Token::Percent,
                Token::Number(7),
                Token::DoubleStar,
                Token::Number(8),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_ellipsis() {
        assert_eq!(lex("..."), Ok(vec![Token::Ellipsis, Token::Eof]));
    }

    #[test]
    fn lexes_augmented_assignment_operators() {
        assert_eq!(
            lex("x += 1\nx -= 2\nx *= 3\nx @= 4\nx /= 5\nx //= 6\nx %= 7\nx **= 8"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::PlusEqual,
                Token::Number(1),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::MinusEqual,
                Token::Number(2),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::StarEqual,
                Token::Number(3),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::AtEqual,
                Token::Number(4),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::SlashEqual,
                Token::Number(5),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::DoubleSlashEqual,
                Token::Number(6),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::PercentEqual,
                Token::Number(7),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::DoubleStarEqual,
                Token::Number(8),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_bitwise_and_shift_operators() {
        assert_eq!(
            lex("~1 | 2 ^ 3 & 4 << 5 >> 6"),
            Ok(vec![
                Token::Tilde,
                Token::Number(1),
                Token::Pipe,
                Token::Number(2),
                Token::Caret,
                Token::Number(3),
                Token::Ampersand,
                Token::Number(4),
                Token::LeftShift,
                Token::Number(5),
                Token::RightShift,
                Token::Number(6),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_bitwise_augmented_assignment_operators() {
        assert_eq!(
            lex("x |= 1\nx ^= 2\nx &= 3\nx <<= 4\nx >>= 5"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::PipeEqual,
                Token::Number(1),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::CaretEqual,
                Token::Number(2),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::AmpersandEqual,
                Token::Number(3),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::LeftShiftEqual,
                Token::Number(4),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::RightShiftEqual,
                Token::Number(5),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_matrix_multiply_and_ellipsis_tokens() {
        assert_eq!(
            lex("a @ b\nx @= y\n..."),
            Ok(vec![
                Token::Identifier("a".to_string()),
                Token::At,
                Token::Identifier("b".to_string()),
                Token::Newline,
                Token::Identifier("x".to_string()),
                Token::AtEqual,
                Token::Identifier("y".to_string()),
                Token::Newline,
                Token::Ellipsis,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_equal() {
        assert_eq!(
            lex("x = 1"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(1),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_equal_equal() {
        assert_eq!(
            lex("x == 1"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::EqualEqual,
                Token::Number(1),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_colon_equal() {
        assert_eq!(
            lex("x := 1"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::ColonEqual,
                Token::Number(1),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_comparison_operators() {
        assert_eq!(
            lex("1 != 2 < 3 > 0 <= 4 >= 4"),
            Ok(vec![
                Token::Number(1),
                Token::BangEqual,
                Token::Number(2),
                Token::Less,
                Token::Number(3),
                Token::Greater,
                Token::Number(0),
                Token::LessEqual,
                Token::Number(4),
                Token::GreaterEqual,
                Token::Number(4),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_boolean_literals() {
        assert_eq!(
            lex("True False"),
            Ok(vec![Token::True, Token::False, Token::Eof])
        );
    }

    #[test]
    fn lexes_none_literal() {
        assert_eq!(lex("None"), Ok(vec![Token::None, Token::Eof]));
    }

    #[test]
    fn lexes_boolean_operators() {
        assert_eq!(
            lex("True and not False or True"),
            Ok(vec![
                Token::True,
                Token::And,
                Token::Not,
                Token::False,
                Token::Or,
                Token::True,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_identity_operator() {
        assert_eq!(
            lex("x is not None"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::Is,
                Token::Not,
                Token::None,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_string() {
        assert_eq!(
            lex("print(\"hello\")"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("hello".to_string()),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_single_quoted_strings() {
        assert_eq!(
            lex("print('hello')"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("hello".to_string()),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_string_escapes() {
        assert_eq!(
            lex("'line\\nbreak' \"tab\\tquote\\\"\" '\\x41' '\\u0042' '\\U00000043'"),
            Ok(vec![
                Token::String("line\nbreak".to_string()),
                Token::String("tab\tquote\"".to_string()),
                Token::String("A".to_string()),
                Token::String("B".to_string()),
                Token::String("C".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_string_line_continuations() {
        assert_eq!(
            lex("'a\\\nb' \"c\\\r\nd\" '''e\\\nf'''"),
            Ok(vec![
                Token::String("ab".to_string()),
                Token::String("cd".to_string()),
                Token::String("ef".to_string()),
                Token::Eof,
            ])
        );
        assert_eq!(
            lex("r'a\\\nb' R\"c\\\r\nd\""),
            Ok(vec![
                Token::String("a\\\nb".to_string()),
                Token::String("c\\\nd".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_string_octal_escapes() {
        assert_eq!(
            lex("'\\1' '\\01' '\\001' '\\377' '\\400' '\\777'"),
            Ok(vec![
                Token::String("\u{1}".to_string()),
                Token::String("\u{1}".to_string()),
                Token::String("\u{1}".to_string()),
                Token::String("\u{ff}".to_string()),
                Token::String("\u{100}".to_string()),
                Token::String("\u{1ff}".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_unicode_name_escapes() {
        assert_eq!(
            lex(
                r#"'\N{LATIN CAPITAL LETTER A}' '\N{latin small letter a with diaeresis}' f'\N{GREEK CAPITAL LETTER DELTA}{1}' t'\N{AMPERSAND}'"#
            ),
            Ok(vec![
                Token::String("A".to_string()),
                Token::String("ä".to_string()),
                Token::FString(vec![
                    TokenFStringPart::Literal("Δ".to_string()),
                    TokenFStringPart::Expression {
                        source: "1".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                ]),
                Token::TString(vec![TokenFStringPart::Literal("&".to_string())]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_unicode_name_alias_escapes() {
        assert_eq!(
            lex(
                r#"'\N{LF}' '\N{line feed}' '\N{NUL}' '\N{bom}' f'\N{NEW LINE}{1}' t'\N{BYTE ORDER MARK}'"#
            ),
            Ok(vec![
                Token::String("\n".to_string()),
                Token::String("\n".to_string()),
                Token::String("\0".to_string()),
                Token::String("\u{feff}".to_string()),
                Token::FString(vec![
                    TokenFStringPart::Literal("\n".to_string()),
                    TokenFStringPart::Expression {
                        source: "1".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                ]),
                Token::TString(vec![TokenFStringPart::Literal("\u{feff}".to_string())]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_raw_and_triple_quoted_strings() {
        assert_eq!(
            lex("r\"\\n\" '''line\nbreak'''"),
            Ok(vec![
                Token::String("\\n".to_string()),
                Token::String("line\nbreak".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_bytes_literals() {
        assert_eq!(
            lex("b'abc' + B\"abc\" br'\\n' RB\"\\x41\" b'\\x41\\n\\377'"),
            Ok(vec![
                Token::Bytes(b"abc".to_vec()),
                Token::Plus,
                Token::Bytes(b"abc".to_vec()),
                Token::Bytes(b"\\n".to_vec()),
                Token::Bytes(b"\\x41".to_vec()),
                Token::Bytes(vec![b'A', b'\n', 0xff]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_bytes_line_continuations() {
        assert_eq!(
            lex("b'a\\\nb' br'c\\\r\nd'"),
            Ok(vec![
                Token::Bytes(b"ab".to_vec()),
                Token::Bytes(b"c\\\nd".to_vec()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_cpython_string_prefix_matrix() {
        for prefix in ["", "r", "R", "u", "U"] {
            assert_eq!(
                lex(&format!("{prefix}'x'")),
                Ok(vec![Token::String("x".to_string()), Token::Eof]),
                "prefix {prefix:?}"
            );
        }

        for prefix in ["f", "F", "fr", "fR", "Fr", "FR", "rf", "rF", "Rf", "RF"] {
            assert_eq!(
                lex(&format!("{prefix}'x'")),
                Ok(vec![
                    Token::FString(vec![TokenFStringPart::Literal("x".to_string())]),
                    Token::Eof,
                ]),
                "prefix {prefix:?}"
            );
        }

        for prefix in ["t", "T", "tr", "tR", "Tr", "TR", "rt", "rT", "Rt", "RT"] {
            assert_eq!(
                lex(&format!("{prefix}'x'")),
                Ok(vec![
                    Token::TString(vec![TokenFStringPart::Literal("x".to_string())]),
                    Token::Eof,
                ]),
                "prefix {prefix:?}"
            );
        }

        for prefix in ["b", "B", "br", "bR", "Br", "BR", "rb", "rB", "Rb", "RB"] {
            assert_eq!(
                lex(&format!("{prefix}'x'")),
                Ok(vec![Token::Bytes(b"x".to_vec()), Token::Eof]),
                "prefix {prefix:?}"
            );
        }
    }

    #[test]
    fn rejects_non_ascii_bytes_literals() {
        assert_eq!(
            lex("b'café'"),
            Err("bytes can only contain ASCII literal characters".to_string())
        );
    }

    #[test]
    fn rejects_cpython_unterminated_string_forms() {
        assert_eq!(
            lex("'blech"),
            Err("unterminated string literal".to_string())
        );
        assert_eq!(
            lex("\"blech"),
            Err("unterminated string literal".to_string())
        );
        assert_eq!(
            lex("\"blech\\\""),
            Err("unterminated string literal; perhaps you escaped the end quote".to_string())
        );
        assert_eq!(
            lex("r\"blech\\\""),
            Err("unterminated string literal; perhaps you escaped the end quote".to_string())
        );
        assert_eq!(
            lex("'''blech"),
            Err("unterminated triple-quoted string literal".to_string())
        );
        assert_eq!(
            lex("\"\"\"blech"),
            Err("unterminated triple-quoted string literal".to_string())
        );
        assert_eq!(
            lex("b'''blech"),
            Err("unterminated triple-quoted string literal".to_string())
        );
    }

    #[test]
    fn rejects_cpython_invalid_string_escape_forms() {
        assert_eq!(
            lex(r#"'\x'"#),
            Err("truncated string escape: \\x".to_string())
        );
        assert_eq!(
            lex(r#"'\x0g'"#),
            Err("invalid string escape: \\x0g".to_string())
        );
        assert_eq!(
            lex(r#"'\u123'"#),
            Err("truncated string escape: \\u".to_string())
        );
        assert_eq!(
            lex(r#"'\u12x4'"#),
            Err("invalid string escape: \\u12x4".to_string())
        );
        assert_eq!(
            lex(r#"'\U00110000'"#),
            Err("invalid string escape: \\U00110000".to_string())
        );
        assert_eq!(
            lex(r#"b'\x'"#),
            Err("invalid bytes escape: \\x".to_string())
        );
        assert_eq!(
            lex(r#"b'\x0g'"#),
            Err("invalid bytes escape: \\x0g".to_string())
        );
        assert_eq!(
            lex(r#"'\N'"#),
            Err("malformed Unicode name escape: \\N".to_string())
        );
        assert_eq!(
            lex(r#"'\N{'"#),
            Err("malformed Unicode name escape: \\N".to_string())
        );
        assert_eq!(
            lex(r#"'\N{bad}'"#),
            Err("unknown Unicode character name: bad".to_string())
        );
        assert_eq!(
            lex(r#"'\N{LATIN_CAPITAL_LETTER_A}'"#),
            Err("unknown Unicode character name: LATIN_CAPITAL_LETTER_A".to_string())
        );
        assert_eq!(
            lex(r#"'\N{NEW_LINE}'"#),
            Err("unknown Unicode character name: NEW_LINE".to_string())
        );
        assert_eq!(
            lex(r#"'\N{LINE    FEED}'"#),
            Err("unknown Unicode character name: LINE    FEED".to_string())
        );
        assert_eq!(
            lex(r#"'\N{linefeed}'"#),
            Err("unknown Unicode character name: linefeed".to_string())
        );
    }

    #[test]
    fn lexes_f_string_parts() {
        assert_eq!(
            lex("f\"hello {name!r} {{ok}} {3!=4!s  }\""),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Literal("hello ".to_string()),
                    TokenFStringPart::Expression {
                        source: "name".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: None,
                    },
                    TokenFStringPart::Literal(" {ok} ".to_string()),
                    TokenFStringPart::Expression {
                        source: "3!=4".to_string(),
                        conversion: Some(TokenFStringConversion::Str),
                        format_spec: None,
                        debug_label: None,
                    },
                ]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_f_string_escaped_brace_literals() {
        assert_eq!(
            lex(r#"f"\x7b1+1}}" f"\u007b1+1""#),
            Ok(vec![
                Token::FString(vec![TokenFStringPart::Literal("{1+1}".to_string())]),
                Token::FString(vec![TokenFStringPart::Literal("{1+1".to_string())]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_f_string_backslash_before_doubled_braces() {
        assert_eq!(
            lex(r#"f"\{{{1+1}" rf"\{{{1+1}""#),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Literal("\\{".to_string()),
                    TokenFStringPart::Expression {
                        source: "1+1".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                ]),
                Token::FString(vec![
                    TokenFStringPart::Literal("\\{".to_string()),
                    TokenFStringPart::Expression {
                        source: "1+1".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                ]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_f_string_line_continuations() {
        assert_eq!(
            lex("f'a\\\n{1}b' rf'c\\\r\n{2}d'"),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Literal("a".to_string()),
                    TokenFStringPart::Expression {
                        source: "1".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                    TokenFStringPart::Literal("b".to_string()),
                ]),
                Token::FString(vec![
                    TokenFStringPart::Literal("c\\\n".to_string()),
                    TokenFStringPart::Expression {
                        source: "2".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                    TokenFStringPart::Literal("d".to_string()),
                ]),
                Token::Eof,
            ])
        );
        assert_eq!(
            lex("rf'{1:a\\\nb}' rt'{2:c\\\nd}'"),
            Ok(vec![
                Token::FString(vec![TokenFStringPart::Expression {
                    source: "1".to_string(),
                    conversion: None,
                    format_spec: Some(vec![TokenFStringPart::Literal("a\\\nb".to_string())]),
                    debug_label: None,
                }]),
                Token::TString(vec![TokenFStringPart::Expression {
                    source: "2".to_string(),
                    conversion: None,
                    format_spec: Some(vec![TokenFStringPart::Literal("c\\\nd".to_string())]),
                    debug_label: None,
                }]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_raw_f_string_literals_and_empty_format_specs() {
        assert_eq!(
            lex("rf\"{name}\\n\" f\"{name:}\" f\"{name!a:}\""),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Expression {
                        source: "name".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                    TokenFStringPart::Literal("\\n".to_string()),
                ]),
                Token::FString(vec![TokenFStringPart::Expression {
                    source: "name".to_string(),
                    conversion: None,
                    format_spec: Some(Vec::new()),
                    debug_label: None,
                }]),
                Token::FString(vec![TokenFStringPart::Expression {
                    source: "name".to_string(),
                    conversion: Some(TokenFStringConversion::Ascii),
                    format_spec: Some(Vec::new()),
                    debug_label: None,
                }]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_prefixed_same_quote_string_inside_f_string_expression() {
        assert_eq!(
            lex(r#"f'{f' {decorator}\n' if decorator else ''} def'"#),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Expression {
                        source: r#"f' {decorator}\n' if decorator else ''"#.to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                    TokenFStringPart::Literal(" def".to_string()),
                ]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_f_string_format_specs() {
        assert_eq!(
            lex("f\"{value:10.2f} {value:{width}}\""),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Expression {
                        source: "value".to_string(),
                        conversion: None,
                        format_spec: Some(vec![TokenFStringPart::Literal("10.2f".to_string())]),
                        debug_label: None,
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "value".to_string(),
                        conversion: None,
                        format_spec: Some(vec![TokenFStringPart::Expression {
                            source: "width".to_string(),
                            conversion: None,
                            format_spec: None,
                            debug_label: None,
                        }]),
                        debug_label: None,
                    },
                ]),
                Token::Eof,
            ])
        );
        assert_eq!(
            lex("f\"{value:}}}\""),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Expression {
                        source: "value".to_string(),
                        conversion: None,
                        format_spec: Some(Vec::new()),
                        debug_label: None,
                    },
                    TokenFStringPart::Literal("}".to_string()),
                ]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_raw_and_non_raw_f_string_format_spec_escapes() {
        assert_eq!(
            lex(r#"f"{value:\x41}" rf"{value:\x41}""#),
            Ok(vec![
                Token::FString(vec![TokenFStringPart::Expression {
                    source: "value".to_string(),
                    conversion: None,
                    format_spec: Some(vec![TokenFStringPart::Literal("A".to_string())]),
                    debug_label: None,
                }]),
                Token::FString(vec![TokenFStringPart::Expression {
                    source: "value".to_string(),
                    conversion: None,
                    format_spec: Some(vec![TokenFStringPart::Literal("\\x41".to_string())]),
                    debug_label: None,
                }]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_f_string_debug_expressions() {
        assert_eq!(
            lex("f\"{x=} {x =} {x = } {x = !s} {x=!s} {x=:} {x = :} {3*x+15=}\""),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Expression {
                        source: "x".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: Some("x=".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "x ".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: Some("x =".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "x ".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: Some("x = ".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "x ".to_string(),
                        conversion: Some(TokenFStringConversion::Str),
                        format_spec: None,
                        debug_label: Some("x = ".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "x".to_string(),
                        conversion: Some(TokenFStringConversion::Str),
                        format_spec: None,
                        debug_label: Some("x=".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "x".to_string(),
                        conversion: None,
                        format_spec: Some(Vec::new()),
                        debug_label: Some("x=".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "x ".to_string(),
                        conversion: None,
                        format_spec: Some(Vec::new()),
                        debug_label: Some("x = ".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "3*x+15".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: Some("3*x+15=".to_string()),
                    },
                ]),
                Token::Eof,
            ])
        );
        assert_eq!(
            lex("f\"{1==2=} {1 != 2 == 3 != 4=}\""),
            Ok(vec![
                Token::FString(vec![
                    TokenFStringPart::Expression {
                        source: "1==2".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: Some("1==2=".to_string()),
                    },
                    TokenFStringPart::Literal(" ".to_string()),
                    TokenFStringPart::Expression {
                        source: "1 != 2 == 3 != 4".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: None,
                        debug_label: Some("1 != 2 == 3 != 4=".to_string()),
                    },
                ]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_t_string_parts() {
        assert_eq!(
            lex("t'hello {name!r:.2f}' rt'{path}\\Documents' t'{value = }'"),
            Ok(vec![
                Token::TString(vec![
                    TokenFStringPart::Literal("hello ".to_string()),
                    TokenFStringPart::Expression {
                        source: "name".to_string(),
                        conversion: Some(TokenFStringConversion::Repr),
                        format_spec: Some(vec![TokenFStringPart::Literal(".2f".to_string())]),
                        debug_label: None,
                    },
                ]),
                Token::TString(vec![
                    TokenFStringPart::Expression {
                        source: "path".to_string(),
                        conversion: None,
                        format_spec: None,
                        debug_label: None,
                    },
                    TokenFStringPart::Literal("\\Documents".to_string()),
                ]),
                Token::TString(vec![TokenFStringPart::Expression {
                    source: "value ".to_string(),
                    conversion: Some(TokenFStringConversion::Repr),
                    format_spec: None,
                    debug_label: Some("value = ".to_string()),
                }]),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn rejects_invalid_f_string_forms() {
        assert_eq!(
            lex("f\"}\""),
            Err("single '}' is not allowed in f-string literal".to_string())
        );
        assert_eq!(
            lex("f\"{}\""),
            Err("f-string: valid expression required before '}'".to_string())
        );
        assert_eq!(
            lex("f\"{name!x}\""),
            Err("f-string: invalid conversion character: x".to_string())
        );
        assert_eq!(
            lex("f'{3! s}'"),
            Err("f-string: conversion type must come right after the exclamation mark".to_string())
        );
        assert_eq!(
            lex("f'{3!ss}'"),
            Err("f-string: invalid conversion character: ss".to_string())
        );
        assert_eq!(
            lex("f'{3!ss:}'"),
            Err("f-string: invalid conversion character: ss".to_string())
        );
        assert_eq!(
            lex("f'{1:d\n}'"),
            Err("f-string: newlines are not allowed in format specifiers".to_string())
        );
        assert_eq!(lex("f'{1#}'"), Err("f-string: expecting '}'".to_string()));
        assert_eq!(
            lex("t'{1:d\n}'"),
            Err("t-string: newlines are not allowed in format specifiers".to_string())
        );
        assert_eq!(
            lex("f'''{1:d\n}'''"),
            Ok(vec![
                Token::FString(vec![TokenFStringPart::Expression {
                    source: "1".to_string(),
                    conversion: None,
                    format_spec: Some(vec![TokenFStringPart::Literal("d\n".to_string())]),
                    debug_label: None,
                }]),
                Token::Eof,
            ])
        );
        assert_eq!(lex("f'{3'"), Err("f-string: expecting '}'".to_string()));
        assert_eq!(lex("f'{3!'"), Err("f-string: expecting '}'".to_string()));
        assert_eq!(lex("f'{3!s'"), Err("f-string: expecting '}'".to_string()));
        assert_eq!(lex("f'x{'"), Err("f-string: expecting '}'".to_string()));
        assert_eq!(lex("t'{'"), Err("t-string: expecting '}'".to_string()));
        assert_eq!(lex("t'{a'"), Err("t-string: expecting '}'".to_string()));
    }

    #[test]
    fn rejects_cpython_unterminated_interpolated_string_forms() {
        assert_eq!(lex("f'"), Err("unterminated f-string literal".to_string()));
        assert_eq!(
            lex("f'''"),
            Err("unterminated triple-quoted f-string literal".to_string())
        );
        assert_eq!(
            lex("f\"\"\""),
            Err("unterminated triple-quoted f-string literal".to_string())
        );
        assert_eq!(lex("t'"), Err("unterminated t-string literal".to_string()));
        assert_eq!(
            lex("t'''"),
            Err("unterminated triple-quoted t-string literal".to_string())
        );
        assert_eq!(
            lex("t''''"),
            Err("unterminated triple-quoted t-string literal".to_string())
        );
    }

    #[test]
    fn rejects_unterminated_string() {
        assert_eq!(
            lex("\"hello"),
            Err("unterminated string literal".to_string())
        );
    }

    #[test]
    fn lexes_comma() {
        assert_eq!(
            lex("print(1, 2)"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(1),
                Token::Comma,
                Token::Number(2),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_semicolon() {
        assert_eq!(
            lex("x = 1; print(x)"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(1),
                Token::Semicolon,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Identifier("x".to_string()),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_list_brackets() {
        assert_eq!(
            lex("[1, 2]"),
            Ok(vec![
                Token::LeftBracket,
                Token::Number(1),
                Token::Comma,
                Token::Number(2),
                Token::RightBracket,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_dict_braces() {
        assert_eq!(
            lex("{\"a\": 1}"),
            Ok(vec![
                Token::LeftBrace,
                Token::String("a".to_string()),
                Token::Colon,
                Token::Number(1),
                Token::RightBrace,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn ignores_newlines_inside_brackets() {
        assert_eq!(
            lex("print(\n    1,\n    2\n)\nprint(3)"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(1),
                Token::Comma,
                Token::Number(2),
                Token::RightParen,
                Token::Newline,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(3),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_newline() {
        assert_eq!(
            lex("print(1)\nprint(2)"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(1),
                Token::RightParen,
                Token::Newline,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(2),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_if_block_indentation() {
        assert_eq!(
            lex("if True:\n    print(\"yes\")"),
            Ok(vec![
                Token::If,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("yes".to_string()),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_if_else_blocks() {
        assert_eq!(
            lex("if False:\n    print(\"no\")\nelse:\n    print(\"yes\")"),
            Ok(vec![
                Token::If,
                Token::False,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("no".to_string()),
                Token::RightParen,
                Token::Newline,
                Token::Dedent,
                Token::Else,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("yes".to_string()),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_if_elif_else_blocks() {
        assert_eq!(
            lex("if False:\n    pass\nelif True:\n    print(\"elif\")\nelse:\n    pass"),
            Ok(vec![
                Token::If,
                Token::False,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Newline,
                Token::Dedent,
                Token::Elif,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("elif".to_string()),
                Token::RightParen,
                Token::Newline,
                Token::Dedent,
                Token::Else,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_while_break_continue() {
        assert_eq!(
            lex("while True:\n    continue\n    break"),
            Ok(vec![
                Token::While,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Continue,
                Token::Newline,
                Token::Break,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_for_in_loop() {
        assert_eq!(
            lex("for x in range(3):\n    print(x)"),
            Ok(vec![
                Token::For,
                Token::Identifier("x".to_string()),
                Token::In,
                Token::Identifier("range".to_string()),
                Token::LeftParen,
                Token::Number(3),
                Token::RightParen,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Identifier("x".to_string()),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_pass_keyword() {
        assert_eq!(lex("pass"), Ok(vec![Token::Pass, Token::Eof]));
    }

    #[test]
    fn lexes_assert_keyword() {
        assert_eq!(
            lex("assert x, \"message\""),
            Ok(vec![
                Token::Assert,
                Token::Identifier("x".to_string()),
                Token::Comma,
                Token::String("message".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_raise_and_try_except_keywords() {
        assert_eq!(
            lex(
                "try:\n    raise Exception(\"boom\")\nexcept Exception as error:\n    print(error)"
            ),
            Ok(vec![
                Token::Try,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Raise,
                Token::Identifier("Exception".to_string()),
                Token::LeftParen,
                Token::String("boom".to_string()),
                Token::RightParen,
                Token::Newline,
                Token::Dedent,
                Token::Except,
                Token::Identifier("Exception".to_string()),
                Token::As,
                Token::Identifier("error".to_string()),
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Identifier("error".to_string()),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
        assert_eq!(
            lex("try:\n    pass\nexcept* ValueError:\n    pass"),
            Ok(vec![
                Token::Try,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Newline,
                Token::Dedent,
                Token::Except,
                Token::Star,
                Token::Identifier("ValueError".to_string()),
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_with_keyword() {
        assert_eq!(
            lex("with manager() as value:\n    pass"),
            Ok(vec![
                Token::With,
                Token::Identifier("manager".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::As,
                Token::Identifier("value".to_string()),
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_decorator_at_sign() {
        assert_eq!(
            lex("@decorator\ndef f():\n    pass"),
            Ok(vec![
                Token::At,
                Token::Identifier("decorator".to_string()),
                Token::Newline,
                Token::Def,
                Token::Identifier("f".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_function_return_arrow() {
        assert_eq!(
            lex("def f(x: int) -> str:\n    pass"),
            Ok(vec![
                Token::Def,
                Token::Identifier("f".to_string()),
                Token::LeftParen,
                Token::Identifier("x".to_string()),
                Token::Colon,
                Token::Identifier("int".to_string()),
                Token::RightParen,
                Token::Arrow,
                Token::Identifier("str".to_string()),
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_import_keywords() {
        assert_eq!(
            lex("import sys\nfrom time import time"),
            Ok(vec![
                Token::Import,
                Token::Identifier("sys".to_string()),
                Token::Newline,
                Token::From,
                Token::Identifier("time".to_string()),
                Token::Import,
                Token::Identifier("time".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_yield_keyword() {
        assert_eq!(
            lex("yield 1"),
            Ok(vec![Token::Yield, Token::Number(1), Token::Eof])
        );
    }

    #[test]
    fn lexes_async_and_await_keywords() {
        assert_eq!(
            lex("async def f():\n    await g()"),
            Ok(vec![
                Token::Async,
                Token::Def,
                Token::Identifier("f".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Await,
                Token::Identifier("g".to_string()),
                Token::LeftParen,
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn skips_inline_comments() {
        assert_eq!(
            lex("print(1) # one\nprint(2)"),
            Ok(vec![
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(1),
                Token::RightParen,
                Token::Newline,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(2),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_type_comments_and_type_ignores() {
        assert_eq!(
            lex(
                "x = 1  # type: int\n# type: ignore[reason]\npass  # type: ignore whatever\npass  # type: ignorewhatever\npass  # type: ignore@tag"
            ),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(1),
                Token::TypeComment("int".to_string()),
                Token::Newline,
                Token::TypeIgnore("ignore[reason]".to_string()),
                Token::Newline,
                Token::Pass,
                Token::TypeIgnore("ignore whatever".to_string()),
                Token::Newline,
                Token::Pass,
                Token::TypeComment("ignorewhatever".to_string()),
                Token::Newline,
                Token::Pass,
                Token::TypeIgnore("ignore@tag".to_string()),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn skips_comment_only_lines_without_indentation() {
        assert_eq!(
            lex("if True:\n    # comment\n    pass"),
            Ok(vec![
                Token::If,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Newline,
                Token::Indent,
                Token::Pass,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_nested_indentation() {
        assert_eq!(
            lex("if True:\n    if False:\n        print(\"nested\")\n    print(\"done\")"),
            Ok(vec![
                Token::If,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::If,
                Token::False,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("nested".to_string()),
                Token::RightParen,
                Token::Newline,
                Token::Dedent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("done".to_string()),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn ignores_blank_lines_for_indentation() {
        assert_eq!(
            lex("if True:\n    print(1)\n\n    print(2)"),
            Ok(vec![
                Token::If,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(1),
                Token::RightParen,
                Token::Newline,
                Token::Newline,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::Number(2),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn rejects_unmatched_indentation() {
        assert_eq!(
            lex("if True:\n    print(1)\n  print(2)"),
            Err("unindent does not match any outer indentation level".to_string())
        );
    }

    #[test]
    fn rejects_too_many_indentation_levels() {
        fn nested_source(indents: usize) -> String {
            let mut source = String::new();
            for level in 0..indents {
                source.push_str(&"  ".repeat(level));
                source.push_str("if True:\n");
            }
            source.push_str(&"  ".repeat(indents));
            source.push_str("pass\n");
            source
        }

        assert!(lex(&nested_source(MAX_INDENT_LEVELS - 1)).is_ok());
        assert_eq!(
            lex(&nested_source(MAX_INDENT_LEVELS)),
            Err("too many levels of indentation".to_string())
        );
    }

    #[test]
    fn rejects_too_many_nested_parentheses() {
        fn nested(open: char, close: char, depth: usize) -> String {
            format!(
                "{}1{}",
                open.to_string().repeat(depth),
                close.to_string().repeat(depth)
            )
        }

        assert!(lex(&nested('(', ')', MAX_BRACKET_DEPTH)).is_ok());
        for (open, close) in [('(', ')'), ('[', ']'), ('{', '}')] {
            assert_eq!(
                lex(&nested(open, close, MAX_BRACKET_DEPTH + 1)),
                Err("too many nested parentheses".to_string())
            );
        }
    }

    #[test]
    fn rejects_unclosed_bracketed_statements() {
        assert_eq!(lex("(1\n"), Err("EOF in multi-line statement".to_string()));
        assert_eq!(lex("[1"), Err("EOF in multi-line statement".to_string()));
        assert_eq!(lex("{1: 2"), Err("EOF in multi-line statement".to_string()));
    }

    #[test]
    fn rejects_unmatched_closing_brackets() {
        assert_eq!(lex("]"), Err("unmatched ']'".to_string()));
        assert_eq!(
            lex("(]"),
            Err("closing parenthesis ']' does not match opening parenthesis '('".to_string())
        );
    }

    #[test]
    fn lexes_tabs_in_indentation() {
        assert_eq!(
            lex("if True:\n\tprint(\"yes\")"),
            Ok(vec![
                Token::If,
                Token::True,
                Token::Colon,
                Token::Newline,
                Token::Indent,
                Token::Identifier("print".to_string()),
                Token::LeftParen,
                Token::String("yes".to_string()),
                Token::RightParen,
                Token::Dedent,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn lexes_formfeed_as_whitespace() {
        assert_eq!(
            lex("x\x0c=1"),
            Ok(vec![
                Token::Identifier("x".to_string()),
                Token::Equal,
                Token::Number(1),
                Token::Eof,
            ])
        );
    }

    #[test]
    fn rejects_inconsistent_tabs_and_spaces_in_indentation() {
        assert_eq!(
            lex("if True:\n\tprint(1)\n        print(2)"),
            Err("inconsistent use of tabs and spaces in indentation".to_string())
        );
    }

    #[test]
    fn rejects_unknown_character() {
        assert_eq!(lex("print($)"), Err("unexpected character: $".to_string()));
    }

    #[test]
    fn rejects_invalid_non_printable_characters() {
        assert_eq!(
            lex("print\x17(\"Hello\")"),
            Err("invalid non-printable character U+0017".to_string())
        );
        assert_eq!(
            lex("\u{a0}"),
            Err("invalid non-printable character U+00A0".to_string())
        );
    }

    #[test]
    fn rejects_null_bytes_with_cpython_message() {
        assert_eq!(
            lex("print(1)\0"),
            Err("source code cannot contain null bytes".to_string())
        );
    }
}
