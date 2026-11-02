// SPDX-License-Identifier: MIT

//! CPython-shaped `SyntaxError` reports for `compile()`, `exec()`, and
//! `eval()`.
//!
//! The tokenizer, parser, and compiler keep their own internal messages
//! (`run_source` reports them with a `lex error:`/`parse error:`/`compile
//! error:` prefix). A source compiled from Python code instead raises the
//! exception CPython 3.14.6 raises: `SyntaxError`, `IndentationError`, or
//! `TabError`, with CPython's message, its 1-based `lineno`/`offset`/
//! `end_lineno`/`end_offset`, and the source line as `text`. Locations start
//! from the parser's error token and the span rules of
//! `parse_error_diagnostic`, then follow CPython's few fixed conventions
//! (unclosed brackets, indentation errors, compiler errors without `text`).

use super::*;
use crate::parser::ParserDiagnostic;

/// A `SyntaxError` as CPython reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SyntaxErrorReport {
    pub(crate) type_name: &'static str,
    pub(crate) message: String,
    pub(crate) line: i64,
    pub(crate) offset: i64,
    pub(crate) end_line: i64,
    pub(crate) end_offset: i64,
    /// `None` for compiler errors: CPython reads their `text` back from the
    /// file, which a compiled string does not have.
    pub(crate) text: Option<String>,
    /// Whether `args` carries the location tuple. CPython's symbol-table
    /// errors set the location attributes only (`args == (msg,)`).
    pub(crate) location_in_args: bool,
}

/// Errors CPython raises from its code generator, which put the location
/// into `args` (the symbol table's errors do not).
fn codegen_error_message(message: &str) -> bool {
    [
        "'return' outside function",
        "'yield' outside function",
        "'break' outside loop",
        "'continue' not properly in loop",
        "'yield from' inside async function",
        "'return' with value in async generator",
        "starred assignment target must be in a list or tuple",
        "multiple starred expressions in assignment",
        "keyword argument repeated",
    ]
    .iter()
    .any(|prefix| message.starts_with(prefix))
}

/// The source text of 1-based `line`, without its line break.
fn line_text(source: &str, line: usize) -> String {
    source
        .split_inclusive('\n')
        .nth(line.saturating_sub(1))
        .map(|text| text.trim_end_matches(['\n', '\r']).to_string())
        .unwrap_or_default()
}

/// The line as the parser saw it: with the newline the tokenizer adds.
fn parser_line_text(source: &str, line: usize) -> String {
    let mut text = line_text(source, line);
    text.push('\n');
    text
}

fn line_count(source: &str) -> usize {
    source.split_inclusive('\n').count().max(1)
}

pub(crate) fn lex_error_report(source: &str, error: &LexError) -> SyntaxErrorReport {
    let mut report = SyntaxErrorReport {
        type_name: "SyntaxError",
        message: error.message.clone(),
        line: error.line as i64,
        offset: error.column as i64,
        end_line: error.end_line as i64,
        end_offset: error.end_column as i64,
        text: Some(line_text(source, error.line)),
        location_in_args: true,
    };
    let message = error.message.as_str();
    if let Some(translated) = cpython_string_message(message) {
        report.message = translated;
    }
    if message == "inconsistent use of tabs and spaces in indentation" {
        report.type_name = "TabError";
        report.end_offset = -1;
        report.text = Some(parser_line_text(source, error.line));
    } else if message == "unindent does not match any outer indentation level" {
        report.type_name = "IndentationError";
        report.offset = error.end_column as i64;
        report.end_offset = -1;
        report.text = Some(parser_line_text(source, error.line));
    } else if message.starts_with("unterminated ") && message.ends_with("literal") {
        // Single-line literals are detected on their own line, triple-quoted
        // ones at the end of the source.
        let detected = if message.contains("triple-quoted") {
            line_count(source)
        } else {
            error.line
        };
        report.message = format!("{message} (detected at line {detected})");
        report.end_line = report.line;
        report.end_offset = report.offset;
    } else if let Some(rest) = message.strip_prefix("unterminated string literal; ") {
        report.message = format!(
            "unterminated string literal (detected at line {}); {rest}?",
            error.line
        );
        report.end_line = report.line;
        report.end_offset = report.offset;
    } else if message.starts_with("unmatched '") {
        report.end_line = report.line;
        report.end_offset = report.offset;
    } else if message.starts_with("unexpected character: ") {
        // CPython's tokenizer hands the character to the parser as an error
        // token, which fails as plain invalid syntax.
        report.message = "invalid syntax".to_string();
        report.end_line = report.line;
        report.end_offset = report.offset + 1;
        report.text = Some(parser_line_text(source, error.line));
    }
    report
}

/// Where CPython reports an error raised at the end of the input: just past
/// the last character of the last line, with no end column.
fn end_of_input_location(source: &str, report: &mut SyntaxErrorReport) {
    let last = line_count(source);
    let text = line_text(source, last);
    report.line = last as i64;
    report.offset = text.chars().count() as i64 + 1;
    report.end_line = last as i64;
    report.end_offset = -1;
    let mut text = text;
    text.push('\n');
    report.text = Some(text);
}

/// The start rule a source was parsed with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SyntaxInput {
    File,
    Eval,
    Single,
}

/// The report for a parser failure. CPython's `eval` and `single` inputs
/// have no implicit trailing NEWLINE: their source lines keep their own
/// line breaks, and in `eval` an error at the end of the input is at
/// column 0.
pub(crate) fn parse_error_report(
    source: &str,
    tokens: &[SpannedToken],
    diagnostic: ParserDiagnostic,
    input: SyntaxInput,
) -> SyntaxErrorReport {
    let at_end = diagnostic
        .token_index
        .and_then(|index| tokens.get(index))
        .is_some_and(|token| matches!(token.token, Token::Eof));
    let mut report = parse_error_report_for_file_input(source, tokens, diagnostic);
    if input != SyntaxInput::File {
        let last = line_count(source) as i64;
        if report.line == last
            && !source.ends_with('\n')
            && let Some(text) = report.text.as_mut()
        {
            text.truncate(text.trim_end_matches('\n').len());
        }
        if input == SyntaxInput::Eval && at_end && !report.message.ends_with("was never closed") {
            report.offset = 0;
            report.end_line = report.line;
            report.end_offset = 0;
        }
    }
    report
}

fn parse_error_report_for_file_input(
    source: &str,
    tokens: &[SpannedToken],
    diagnostic: ParserDiagnostic,
) -> SyntaxErrorReport {
    let index = diagnostic
        .token_index
        .unwrap_or_else(|| tokens.len().saturating_sub(1))
        .min(tokens.len().saturating_sub(1));
    if let Some(report) = unclosed_bracket_report(source, tokens, index) {
        return report;
    }
    if diagnostic.message.ends_with("' was never closed")
        && let Some(report) = unclosed_bracket_report(source, tokens, tokens.len() - 1)
    {
        return report;
    }

    let internal = diagnostic.message.clone();
    let located = parse_error_diagnostic(source, internal.clone(), tokens, diagnostic.token_index);
    let mut report = SyntaxErrorReport {
        type_name: "SyntaxError",
        message: internal.clone(),
        line: located.line as i64,
        offset: located.column as i64,
        end_line: located.end_line as i64,
        end_offset: located.end_column as i64,
        text: Some(parser_line_text(source, located.line)),
        location_in_args: true,
    };
    let chars = source.chars().collect::<Vec<_>>();
    let end_of_source = source_location(&chars, chars.len());
    if (located.line, located.column) == end_of_source
        && (located.end_line, located.end_column) == end_of_source
        && let Some(token) = tokens.get(index)
    {
        // No span rule matched: like CPython's RAISE_SYNTAX_ERROR, point at
        // the token the parser stopped at.
        set_token_location(source, &mut report, token);
    }
    let at_end = tokens
        .get(index)
        .is_some_and(|token| matches!(token.token, Token::Eof));

    if internal == "unexpected indent" {
        if let Some(token) = tokens.get(index) {
            report.type_name = "IndentationError";
            report.line = token.line as i64;
            report.offset = token.column as i64 - 1;
            report.end_line = token.line as i64;
            report.end_offset = -1;
            report.text = Some(parser_line_text(source, token.line));
        }
        return report;
    }

    let missing_block = internal.starts_with("expected an indented block")
        || (internal == "expected statement after ':'" && at_end);
    if missing_block {
        report.type_name = "IndentationError";
        if let Some(header) = block_header_description(tokens, index) {
            report.message = format!("expected an indented block after {header}");
        } else {
            report.message = "expected an indented block".to_string();
        }
        if at_end {
            end_of_input_location(source, &mut report);
        } else if let Some(token) = tokens.get(index) {
            set_token_location(source, &mut report, token);
        }
        return report;
    }

    report.message = cpython_parse_message(&internal, tokens, index);
    if let Some(unclosed) = unclosed_bracket_report(source, tokens, tokens.len() - 1) {
        // After a generic error CPython tokenizes the rest of the input; a
        // bracket opened on an earlier line and still open at the end
        // replaces the error. A missing-comma check that reads up to the
        // end of the input inside the bracket hits the end there too.
        let error_line = tokens.get(index).map_or(0, |token| token.line) as i64;
        let replaces = match report.message.as_str() {
            "invalid syntax" => unclosed.line < error_line,
            "invalid syntax. Perhaps you forgot a comma?" => missing_comma_span(tokens, index)
                .is_some_and(|(_, end)| {
                    tokens
                        .iter()
                        .skip_while(|token| !std::ptr::eq(*token, end))
                        .nth(1)
                        .is_some_and(|next| matches!(next.token, Token::Eof))
                }),
            _ => false,
        };
        if replaces {
            return unclosed;
        }
    }

    let span = if report.message == "invalid syntax. Perhaps you forgot a comma?"
        && internal != "Perhaps you forgot a comma"
    {
        missing_comma_span(tokens, index)
    } else if report.message.starts_with("cannot delete ") {
        delete_target_span(tokens, index)
    } else if report.message == "expected 'else' after 'if' expression" {
        conditional_without_else_span(tokens, index)
    } else if report.message == "iterable unpacking cannot be used in comprehension" {
        starred_comprehension_element_span(tokens, index)
    } else if report.message == "dict unpacking cannot be used in dict comprehension" {
        let start = logical_line_start(tokens, index);
        tokens[start..index.min(tokens.len())]
            .iter()
            .find(|token| matches!(token.token, Token::DoubleStar))
            .map(|token| (token, token))
    } else if report
        .message
        .ends_with("is an illegal expression for augmented assignment")
        || report
            .message
            .starts_with("Missing parentheses in call to ")
    {
        statement_prefix_span(tokens, index, report.message.starts_with("Missing"))
    } else if report.message.ends_with(" as import target") {
        let start = logical_line_start(tokens, index);
        let line = &tokens[start..];
        let as_index = line
            .iter()
            .rposition(|token| matches!(token.token, Token::As))
            .filter(|position| start + position < index.max(start + 1) + 1);
        as_index.and_then(|position| {
            let first = line.get(position + 1)?;
            let last = line[position + 1..]
                .iter()
                .take_while(|token| {
                    !is_statement_end_token(&token.token)
                        && !matches!(token.token, Token::Comma | Token::RightParen)
                })
                .last()?;
            Some((first, last))
        })
    } else if report.message == "cannot use starred expression here"
        && tokens
            .get(index)
            .is_some_and(|token| matches!(token.token, Token::Star))
    {
        // `(*a if b else c)`: the whole starred expression inside the group.
        let mut depth = 0usize;
        let mut end = None;
        for (position, token) in tokens.iter().enumerate().skip(index + 1) {
            match token.token {
                Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen | Token::RightBracket | Token::RightBrace if depth == 0 => {
                    break;
                }
                Token::RightParen | Token::RightBracket | Token::RightBrace => depth -= 1,
                Token::Newline | Token::Eof => break,
                _ => {}
            }
            end = Some(position);
        }
        end.map(|end| (&tokens[index], &tokens[end]))
    } else if report.message == "expected one or more exception types" {
        let start = logical_line_start(tokens, index);
        tokens.get(start + 2).map(|token| (token, token))
    } else if report.message == "* argument may appear only once" {
        index
            .checked_sub(1)
            .and_then(|before| tokens.get(before))
            .filter(|token| matches!(token.token, Token::Star))
            .map(|token| (token, token))
    } else {
        None
    };
    if let Some(name) = report
        .message
        .strip_prefix("duplicate argument '")
        .and_then(|rest| rest.strip_suffix("' in function definition"))
    {
        // A symbol-table error in CPython: at the repeated parameter, with no
        // source text.
        let start = logical_line_start(tokens, index);
        if let Some(token) = tokens[start..]
            .iter()
            .filter(|token| matches!(&token.token, Token::Identifier(found) if found == name))
            .nth(1)
        {
            report.line = token.line as i64;
            report.offset = token.column as i64;
            report.end_line = token.end_line as i64;
            report.end_offset = token.end_column as i64;
        }
        report.text = None;
        report.location_in_args = false;
        return report;
    }
    if report.message == "starred assignment target must be in a list or tuple"
        && let Some(located) = starred_target_span(&report.message, tokens)
    {
        report.line = located.line as i64;
        report.offset = located.column as i64;
        report.end_line = located.end_line as i64;
        report.end_offset = located.end_column as i64;
        report.text = None;
        return report;
    }
    if report.message.starts_with("cannot assign to __debug__")
        || report.message.starts_with("cannot delete __debug__")
        || report.message.starts_with("keyword argument repeated")
    {
        report.text = None;
        report.location_in_args = codegen_error_message(&report.message);
    }
    if let Some((start, end)) = span {
        report.line = start.line as i64;
        report.offset = start.column as i64;
        report.end_line = end.end_line as i64;
        report.end_offset = end.end_column as i64;
        report.text = Some(parser_line_text(source, start.line));
    } else if at_end {
        end_of_input_location(source, &mut report);
        if report.message == "invalid syntax"
            || report.message == "expected expression after 'else', but statement is given"
        {
            // The tokenizer's implicit NEWLINE.
            report.end_offset = report.offset + 1;
        } else if report.message == "Expected one or more names after 'import'" {
            report.end_offset = report.offset;
        }
    } else if let Some(token) = tokens.get(index)
        && matches!(token.token, Token::Newline)
        && report.line == token.line as i64
        && report.offset == token.column as i64
    {
        set_token_location(source, &mut report, token);
    }
    report
}

pub(crate) fn compile_error_report(
    tokens: &[SpannedToken],
    program: &Program,
    message: &str,
) -> SyntaxErrorReport {
    // CPython says "outside function" when the `await` is in no function at
    // all, and "outside async function" inside a plain `def`.
    let message = if message == "'await' outside async function"
        && !tokens
            .iter()
            .any(|token| matches!(token.token, Token::Def | Token::Lambda))
    {
        "'await' outside function"
    } else {
        message
    };
    let located = if let Some((start, end)) = future_error_span(message, program, tokens)
        .or_else(|| annotation_error_span(message, tokens))
    {
        ParseError {
            message: message.to_string(),
            line: start.line,
            column: start.column,
            end_line: end.end_line,
            end_column: end.end_column,
        }
    } else {
        compile_error_statement_span(message, tokens)
            .unwrap_or_else(|| compile_error_diagnostic(message.to_string(), tokens))
    };
    let message = cpython_compile_message(message);
    SyntaxErrorReport {
        type_name: "SyntaxError",
        location_in_args: codegen_error_message(&message),
        message,
        line: located.line as i64,
        offset: located.column as i64,
        end_line: located.end_line as i64,
        end_offset: located.end_column as i64,
        text: None,
    }
}

/// CPython reports `return`, `break`, `continue`, and `yield` outside their
/// block at the whole statement or expression.
fn compile_error_statement_span(message: &str, tokens: &[SpannedToken]) -> Option<ParseError> {
    let start = match message {
        "break outside loop" => tokens.iter().position(|token| token.token == Token::Break),
        "'continue' not properly in loop" => tokens
            .iter()
            .position(|token| token.token == Token::Continue),
        "return outside function" => tokens.iter().position(|token| token.token == Token::Return),
        "yield outside function" => tokens.iter().position(|token| token.token == Token::Yield),
        "'await' outside function" | "'await' outside async function" => {
            tokens.iter().position(|token| token.token == Token::Await)
        }
        "'yield from' inside async function" => tokens
            .windows(2)
            .position(|pair| pair[0].token == Token::Yield && pair[1].token == Token::From),
        "nonlocal declaration not allowed at module level" => tokens
            .iter()
            .position(|token| token.token == Token::Nonlocal),
        "multiple starred expressions in assignment"
        | "starred assignment target must be in a list or tuple" => {
            return starred_target_span(message, tokens);
        }
        message => {
            // `global`/`nonlocal` declaration errors name the declared name.
            let (keyword, name) = scope_declaration_error_target(message)?;
            let declaration = find_declaration_token(tokens, keyword, name)?;
            tokens
                .iter()
                .position(|token| std::ptr::eq(token, declaration))
        }
    }?;
    let mut depth = 0usize;
    let mut end = start;
    for (index, token) in tokens.iter().enumerate().skip(start + 1) {
        match &token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            token if depth == 0 && is_statement_end_token(token) => break,
            _ => {}
        }
        end = index;
    }
    Some(ParseError {
        message: message.to_string(),
        line: tokens[start].line,
        column: tokens[start].column,
        end_line: tokens[end].end_line,
        end_column: tokens[end].end_column,
    })
}

/// The assignment or `for` target holding the starred item(s): from the
/// start of the statement (or after `for`) to the `=` or `in`.
fn starred_target_span(message: &str, tokens: &[SpannedToken]) -> Option<ParseError> {
    let star = tokens
        .iter()
        .position(|token| matches!(token.token, Token::Star))?;
    let mut start = star;
    while start > 0
        && !matches!(
            tokens[start - 1].token,
            Token::Newline
                | Token::Indent
                | Token::Dedent
                | Token::Semicolon
                | Token::For
                | Token::Equal
                | Token::As
        )
    {
        start -= 1;
    }
    let mut depth = 0usize;
    let mut end = star;
    for (position, token) in tokens.iter().enumerate().skip(start) {
        match &token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Token::Equal | Token::In | Token::Colon if depth == 0 => break,
            token if depth == 0 && is_statement_end_token(token) => break,
            _ => {}
        }
        end = position;
    }
    Some(ParseError {
        message: message.to_string(),
        line: tokens[start].line,
        column: tokens[start].column,
        end_line: tokens[end].end_line,
        end_column: tokens[end].end_column,
    })
}

/// CPython's "'(' was never closed": the parser reached the end of the input
/// inside a bracket. It points at the innermost open bracket.
fn unclosed_bracket_report(
    source: &str,
    tokens: &[SpannedToken],
    index: usize,
) -> Option<SyntaxErrorReport> {
    if !matches!(tokens.get(index)?.token, Token::Eof) {
        return None;
    }
    let mut open: Vec<&SpannedToken> = Vec::new();
    for token in &tokens[..index] {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => open.push(token),
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                open.pop();
            }
            _ => {}
        }
    }
    let bracket = open.last()?;
    let symbol = match bracket.token {
        Token::LeftParen => '(',
        Token::LeftBracket => '[',
        _ => '{',
    };
    let mut text = line_text(source, bracket.line);
    if bracket.line >= line_count(source) {
        text.push('\n');
    }
    Some(SyntaxErrorReport {
        type_name: "SyntaxError",
        message: format!("'{symbol}' was never closed"),
        line: bracket.line as i64,
        offset: bracket.column as i64,
        end_line: bracket.line as i64,
        end_offset: 0,
        text: Some(text),
        location_in_args: true,
    })
}

/// A token's location as CPython reports it; a NEWLINE token covers one
/// column on its own line.
fn set_token_location(source: &str, report: &mut SyntaxErrorReport, token: &SpannedToken) {
    report.line = token.line as i64;
    report.offset = token.column as i64;
    if matches!(token.token, Token::Newline) || token.end_line != token.line {
        report.end_line = token.line as i64;
        report.end_offset = token.column as i64 + 1;
    } else {
        report.end_line = token.end_line as i64;
        report.end_offset = token.end_column as i64;
    }
    report.text = Some(parser_line_text(source, token.line));
}

/// Whether `index` lies inside an open bracket.
fn inside_brackets(tokens: &[SpannedToken], index: usize) -> bool {
    let mut depth = 0i64;
    for token in &tokens[..index.min(tokens.len())] {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth -= 1,
            _ => {}
        }
    }
    depth > 0
}

fn ends_expression(token: &Token) -> bool {
    matches!(
        token,
        Token::Identifier(_)
            | Token::Number(_)
            | Token::BigInt(_)
            | Token::Float(_)
            | Token::Imaginary(_)
            | Token::String(_)
            | Token::Bytes(_)
            | Token::FString(_)
            | Token::TString(_)
            | Token::FStringEnd(_)
            | Token::TStringEnd(_)
            | Token::True
            | Token::False
            | Token::None
            | Token::Ellipsis
            | Token::RightParen
            | Token::RightBracket
            | Token::RightBrace
    )
}

/// CPython's `invalid_expression` "Perhaps you forgot a comma?": two
/// expressions side by side inside brackets. Returns the range from the
/// first expression's start to the second one's end.
fn missing_comma_span(
    tokens: &[SpannedToken],
    index: usize,
) -> Option<(&SpannedToken, &SpannedToken)> {
    let is_boundary = |token: &Token| {
        matches!(
            token,
            Token::Comma
                | Token::Colon
                | Token::Equal
                | Token::ColonEqual
                | Token::Newline
                | Token::Semicolon
                | Token::Eof
                | Token::For
                | Token::If
                | Token::Else
                | Token::In
                | Token::Lambda
                | Token::Yield
        )
    };
    let mut depth = 0usize;
    let mut start = index;
    while start > 0 {
        let previous = &tokens[start - 1].token;
        match previous {
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth += 1,
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            token if depth == 0 && is_boundary(token) => break,
            _ => {}
        }
        start -= 1;
    }
    let mut depth = 0usize;
    let mut end = index;
    for (position, token) in tokens.iter().enumerate().skip(index) {
        match &token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            token if depth == 0 && is_boundary(token) => break,
            _ => {}
        }
        end = position;
    }
    (start < index).then(|| (&tokens[start], &tokens[end]))
}

/// From the start of the statement to the token before the first
/// augmented-assignment operator (or to the end of the statement).
fn statement_prefix_span(
    tokens: &[SpannedToken],
    index: usize,
    whole_statement: bool,
) -> Option<(&SpannedToken, &SpannedToken)> {
    let start = logical_line_start(tokens, index);
    let mut depth = 0usize;
    let mut end = None;
    for (position, token) in tokens.iter().enumerate().skip(start) {
        match &token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1)
            }
            token if depth == 0 && is_statement_end_token(token) => break,
            token if depth == 0 && !whole_statement && is_augmented_assign_token(token) => break,
            _ => {}
        }
        end = Some(position);
    }
    Some((tokens.get(start)?, tokens.get(end?)?))
}

fn is_augmented_assign_token(token: &Token) -> bool {
    matches!(
        token,
        Token::PlusEqual
            | Token::MinusEqual
            | Token::StarEqual
            | Token::AtEqual
            | Token::SlashEqual
            | Token::DoubleSlashEqual
            | Token::PercentEqual
            | Token::DoubleStarEqual
            | Token::PipeEqual
            | Token::CaretEqual
            | Token::AmpersandEqual
            | Token::LeftShiftEqual
            | Token::RightShiftEqual
    )
}

/// The starred element of `[*x for ...]`, `(*x for ...)`, or `f(*x for ...)`.
fn starred_comprehension_element_span(
    tokens: &[SpannedToken],
    index: usize,
) -> Option<(&SpannedToken, &SpannedToken)> {
    let start = logical_line_start(tokens, index);
    for star in start..tokens.len() {
        if !matches!(tokens[star].token, Token::Star)
            || !star.checked_sub(1).is_some_and(|before| {
                matches!(
                    tokens[before].token,
                    Token::LeftParen | Token::LeftBracket | Token::LeftBrace | Token::Comma
                )
            })
        {
            continue;
        }
        let mut depth = 0usize;
        for position in star + 1..tokens.len() {
            match &tokens[position].token {
                Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen | Token::RightBracket | Token::RightBrace | Token::Comma
                    if depth == 0 =>
                {
                    break;
                }
                Token::RightParen | Token::RightBracket | Token::RightBrace => depth -= 1,
                Token::For | Token::Async if depth == 0 => {
                    return Some((&tokens[star], &tokens[position - 1]));
                }
                token if is_statement_end_token(token) => break,
                _ => {}
            }
        }
    }
    None
}

/// The first `del` target CPython rejects: a target that is not a name, an
/// attribute, or a subscript.
fn delete_target_span(
    tokens: &[SpannedToken],
    index: usize,
) -> Option<(&SpannedToken, &SpannedToken)> {
    let del = (0..index.min(tokens.len()))
        .rev()
        .find(|position| matches!(tokens[*position].token, Token::Del))?;
    let mut targets = Vec::new();
    let mut depth = 0usize;
    let mut start = del + 1;
    let mut position = del + 1;
    while let Some(token) = tokens.get(position) {
        match &token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1)
            }
            Token::Comma if depth == 0 => {
                targets.push((start, position));
                start = position + 1;
            }
            token if depth == 0 && is_statement_end_token(token) => break,
            _ => {}
        }
        position += 1;
    }
    if start < position {
        targets.push((start, position));
    }
    let plain = |(first, last): (usize, usize)| {
        let slice = &tokens[first..last];
        matches!(
            slice.first().map(|token| &token.token),
            Some(Token::Identifier(_))
        ) && matches!(
            slice.last().map(|token| &token.token),
            Some(Token::Identifier(_) | Token::RightBracket)
        ) && slice.iter().all(|token| {
            !matches!(
                token.token,
                Token::Plus | Token::Minus | Token::Star | Token::If | Token::Lambda
            )
        })
    };
    let (mut first, mut last) = targets
        .iter()
        .copied()
        .find(|target| target.0 < target.1 && !plain(*target))?;
    // Look inside a parenthesized or bracketed target for its first
    // offending element, as CPython does.
    loop {
        let wrapped = matches!(tokens[first].token, Token::LeftParen | Token::LeftBracket)
            && matches!(
                tokens[last - 1].token,
                Token::RightParen | Token::RightBracket
            )
            && last - first > 2;
        if !wrapped {
            break;
        }
        let mut inner = Vec::new();
        let mut depth = 0usize;
        let mut item_start = first + 1;
        for position in first + 1..last - 1 {
            match tokens[position].token {
                Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen | Token::RightBracket | Token::RightBrace => {
                    depth = depth.saturating_sub(1)
                }
                Token::Comma if depth == 0 => {
                    inner.push((item_start, position));
                    item_start = position + 1;
                }
                _ => {}
            }
        }
        if item_start < last - 1 {
            inner.push((item_start, last - 1));
        }
        let Some(next) = inner
            .into_iter()
            .find(|target| target.0 < target.1 && !plain(*target))
        else {
            break;
        };
        (first, last) = next;
    }
    Some((&tokens[first], &tokens[last - 1]))
}

/// "'if' statement on line 1", "function definition on line 3", ...: the
/// compound statement whose block is missing, found from the start of the
/// logical line that ends before `index`.
fn block_header_description(tokens: &[SpannedToken], index: usize) -> Option<String> {
    let mut start = index.min(tokens.len());
    // Step back over the header's NEWLINE and its tokens.
    while start > 0 && matches!(tokens[start - 1].token, Token::Newline) {
        start -= 1;
    }
    while start > 0
        && !matches!(
            tokens[start - 1].token,
            Token::Newline | Token::Indent | Token::Dedent | Token::Semicolon
        )
    {
        start -= 1;
    }
    let first = tokens.get(start)?;
    let second = tokens.get(start + 1).map(|token| &token.token);
    let name = match (&first.token, second) {
        (Token::If, _) => "'if' statement",
        (Token::Elif, _) => "'elif' statement",
        (Token::Else, _) => "'else' statement",
        (Token::While, _) => "'while' statement",
        (Token::For, _) | (Token::Async, Some(Token::For)) => "'for' statement",
        (Token::With, _) | (Token::Async, Some(Token::With)) => "'with' statement",
        (Token::Try, _) => "'try' statement",
        (Token::Except, Some(Token::Star)) => "'except*' statement",
        (Token::Except, _) => "'except' statement",
        (Token::Finally, _) => "'finally' statement",
        (Token::Def, _) | (Token::Async, Some(Token::Def)) => "function definition",
        (Token::Class, _) => "class definition",
        (Token::Identifier(name), _) if name == "match" => "'match' statement",
        (Token::Identifier(name), _) if name == "case" => "'case' statement",
        _ => return None,
    };
    Some(format!("{name} on line {}", first.line))
}

/// `a if b` without `else`: CPython reports the range from the start of
/// `a` to the end of `b`.
fn conditional_without_else_span(
    tokens: &[SpannedToken],
    index: usize,
) -> Option<(&SpannedToken, &SpannedToken)> {
    let end = tokens.get(index.checked_sub(1)?)?;
    let if_index = (0..index)
        .rev()
        .find(|candidate| matches!(tokens[*candidate].token, Token::If))?;
    let mut depth = 0usize;
    let mut start = if_index;
    while start > 0 {
        let previous = &tokens[start - 1].token;
        match previous {
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth += 1,
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            }
            Token::Comma
            | Token::Equal
            | Token::Colon
            | Token::ColonEqual
            | Token::Newline
            | Token::Indent
            | Token::Dedent
            | Token::Semicolon
            | Token::Return
            | Token::Yield
            | Token::In
            | Token::Lambda
            | Token::Assert
            | Token::Raise
            | Token::Del
            | Token::Await
                if depth == 0 =>
            {
                break;
            }
            _ => {}
        }
        start -= 1;
    }
    Some((tokens.get(start)?, end))
}

/// Whether the logical line holding `index` is a `def` header, so a missing
/// `(` is CPython's forced `&&'('`.
fn in_function_header(tokens: &[SpannedToken], index: usize) -> bool {
    tokens[..index.min(tokens.len())]
        .iter()
        .rev()
        .take_while(|token| {
            !matches!(
                token.token,
                Token::Newline | Token::Indent | Token::Dedent | Token::Semicolon
            )
        })
        .any(|token| token.token == Token::Def)
}

/// CPython's message for a parser failure reported internally as
/// `internal`.
fn cpython_parse_message(internal: &str, tokens: &[SpannedToken], index: usize) -> String {
    if let Some(rest) = internal
        .strip_prefix("f-string expression parse error: ")
        .or_else(|| internal.strip_prefix("t-string expression parse error: "))
    {
        // A replacement field is bracketed: two expressions in a row are a
        // missing comma.
        if let Some(found) = rest.strip_prefix("expected end of input, found ")
            && (found.starts_with("Identifier(")
                || found.starts_with("Number(")
                || found.starts_with("String("))
        {
            return "invalid syntax. Perhaps you forgot a comma?".to_string();
        }
        return cpython_parse_message(rest, tokens, index);
    }
    if let Some(rest) = internal
        .strip_prefix("f-string: ")
        .or_else(|| internal.strip_prefix("t-string: "))
        && rest.starts_with("invalid non-printable character")
    {
        return rest.to_string();
    }
    if internal.starts_with("f-string: unexpected character")
        || internal.starts_with("t-string: unexpected character")
    {
        return format!(
            "{}: expecting a valid expression after '{{'",
            &internal[..8]
        );
    }
    if internal.starts_with("expected 'else' in conditional expression") {
        return "expected 'else' after 'if' expression".to_string();
    }
    if internal == "expected 'except' or 'finally' after try block" {
        return "expected 'except' or 'finally' block".to_string();
    }
    if internal == "expected statement after ':'" {
        return "invalid syntax".to_string();
    }
    // Two expressions side by side inside brackets.
    if (internal.starts_with("expected ") && internal.contains(", found "))
        && inside_brackets(tokens, index)
        && tokens
            .get(index)
            .is_some_and(|token| token_starts_expression(&token.token))
        && index
            .checked_sub(1)
            .and_then(|before| tokens.get(before))
            .is_some_and(|token| ends_expression(&token.token))
    {
        return "invalid syntax. Perhaps you forgot a comma?".to_string();
    }
    if let Some(expected) = internal.strip_prefix("expected ") {
        if expected.starts_with("':'") {
            return "expected ':'".to_string();
        }
        if expected.starts_with("'('") && in_function_header(tokens, index) {
            return "expected '('".to_string();
        }
        if expected.contains(", found ")
            || expected.ends_with(" end of input")
            || expected.starts_with("statement separator")
        {
            return "invalid syntax".to_string();
        }
    }
    if let Some(name) = internal.strip_prefix("duplicate type parameter name: ") {
        return format!("duplicate type parameter '{name}'");
    }
    if let Some(target) = internal
        .strip_prefix("cannot assign to ")
        .and_then(|rest| rest.strip_suffix(" here. Maybe you meant '==' instead of '='?"))
        && plain_assignment_target_message(target, tokens, index)
    {
        return format!("cannot assign to {target}");
    }
    if let Some(message) = cpython_string_message(internal) {
        return message;
    }
    if let Some(rest) = internal.strip_prefix("cannot use except statement with ")
        && except_star_clause(tokens, index)
    {
        return format!("cannot use except* statement with {rest}");
    }
    cpython_parse_message_tail(internal, tokens, index)
}

/// f-string and t-string diagnostics that the lexer and the parser share.
fn cpython_string_message(internal: &str) -> Option<String> {
    for prefix in ["f-string", "t-string"] {
        if let Some(conversion) = internal
            .strip_prefix(prefix)
            .and_then(|rest| rest.strip_prefix(": invalid conversion character: "))
        {
            return Some(
                if conversion
                    .chars()
                    .next()
                    .is_some_and(|ch| ch.is_alphabetic() || ch == '_')
                {
                    format!(
                        "{prefix}: invalid conversion character '{conversion}': expected 's', 'r', or 'a'"
                    )
                } else {
                    format!("{prefix}: invalid conversion character")
                },
            );
        }
        if internal == format!("single '}}' is not allowed in {prefix} literal") {
            return Some(format!("{prefix}: single '}}' is not allowed"));
        }
        if internal == format!("{prefix}: newlines are not allowed in format specifiers") {
            return Some(format!("{internal} for single quoted {prefix}s"));
        }
    }
    None
}

fn cpython_parse_message_tail(internal: &str, tokens: &[SpannedToken], index: usize) -> String {
    if internal == "invalid syntax" || internal.starts_with("expected expression") {
        // A bare `except*` names no type; a bare `except` lacks its ':'.
        let start = logical_line_start(tokens, index);
        let line = &tokens[start..];
        match (
            line.first().map(|token| &token.token),
            line.get(1).map(|token| &token.token),
            line.get(2).map(|token| &token.token),
        ) {
            (Some(Token::Except), Some(Token::Star), Some(Token::Colon | Token::Newline)) => {
                return "expected one or more exception types".to_string();
            }
            (Some(Token::Except), Some(Token::Newline), _) => return "expected ':'".to_string(),
            _ => {}
        }
    }
    match internal {
        "* may appear only once" => "* argument may appear only once".to_string(),
        "compound statement cannot appear in inline suite" => "invalid syntax".to_string(),
        message if message.starts_with("unsupported ") => "invalid syntax".to_string(),
        message => message.to_string(),
    }
}

/// CPython adds "here. Maybe you meant '==' instead of '='?" only through
/// `invalid_named_expression`: one `=` after a `bitwise_or` that is not
/// `True`/`False`/`None`, a tuple, a list, or a generator expression. A
/// chained assignment, a `for` target, or a lambda, comparison, or
/// conditional gets the plain message.
fn plain_assignment_target_message(target: &str, tokens: &[SpannedToken], index: usize) -> bool {
    if matches!(
        target,
        "None"
            | "True"
            | "False"
            | "lambda"
            | "generator expression"
            | "comparison"
            | "conditional expression"
            | "tuple"
            | "list"
    ) {
        return true;
    }
    let index = index.min(tokens.len());
    let mut start = index;
    while start > 0
        && !matches!(
            tokens[start - 1].token,
            Token::Newline | Token::Indent | Token::Dedent | Token::Semicolon
        )
    {
        start -= 1;
    }
    if matches!(
        (
            tokens.get(start).map(|token| &token.token),
            tokens.get(start + 1).map(|token| &token.token)
        ),
        (Some(Token::For), _) | (Some(Token::Async), Some(Token::For))
    ) {
        return true;
    }
    let mut depth = 0usize;
    let mut equals = 0usize;
    for token in &tokens[start..] {
        match token.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1)
            }
            Token::Equal if depth == 0 => equals += 1,
            ref token if depth == 0 && is_statement_end_token(token) => break,
            _ => {}
        }
    }
    equals > 1
}

/// Index of the first token of the logical line holding `index`.
fn logical_line_start(tokens: &[SpannedToken], index: usize) -> usize {
    let mut start = index.min(tokens.len());
    while start > 0
        && !matches!(
            tokens[start - 1].token,
            Token::Newline | Token::Indent | Token::Dedent | Token::Semicolon
        )
    {
        start -= 1;
    }
    start
}

/// Whether the `except` clause the parser stopped in is an `except*`.
fn except_star_clause(tokens: &[SpannedToken], index: usize) -> bool {
    (0..index.min(tokens.len()))
        .rev()
        .find(|position| matches!(tokens[*position].token, Token::Except))
        .is_some_and(|position| {
            matches!(
                tokens.get(position + 1).map(|token| &token.token),
                Some(Token::Star)
            )
        })
}

fn token_starts_expression(token: &Token) -> bool {
    matches!(
        token,
        Token::Identifier(_)
            | Token::Number(_)
            | Token::BigInt(_)
            | Token::Float(_)
            | Token::Imaginary(_)
            | Token::String(_)
            | Token::Bytes(_)
            | Token::FString(_)
            | Token::TString(_)
            | Token::FStringStart { .. }
            | Token::TStringStart { .. }
            | Token::True
            | Token::False
            | Token::None
            | Token::Ellipsis
            | Token::LeftParen
            | Token::LeftBracket
            | Token::LeftBrace
            | Token::Tilde
            | Token::Lambda
            | Token::Await
    )
}

/// CPython's message for a compiler (symbol table or code generation)
/// failure reported internally as `internal`.
fn cpython_compile_message(internal: &str) -> String {
    match internal {
        "yield outside function" => "'yield' outside function".to_string(),
        "return outside function" => "'return' outside function".to_string(),
        "break outside loop" => "'break' outside loop".to_string(),
        "asynchronous comprehension outside async function" => {
            "asynchronous comprehension outside of an asynchronous function".to_string()
        }
        message => {
            if let Some(kind) = message.strip_prefix("yield inside ") {
                return format!("'yield' inside {kind}");
            }
            if let Some(name) = message
                .strip_prefix("name '")
                .and_then(|rest| rest.strip_suffix("' is type parameter and nonlocal"))
            {
                return format!("nonlocal binding not allowed for type parameter '{name}'");
            }
            if let Some(name) = message.strip_prefix("duplicate type parameter name: ") {
                return format!("duplicate type parameter '{name}'");
            }
            message.to_string()
        }
    }
}
