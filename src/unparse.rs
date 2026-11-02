// SPDX-License-Identifier: PSF-2.0 AND MIT
// Derived from CPython 3.14.6 Python/ast_unparse.c (Copyright (c) 2001 Python Software Foundation; All Rights Reserved).
// Changes: translated to Rust and adapted to minipython's VM; see the repository history.

//! Expression unparser used for PEP 563 string annotations.
//!
//! `from __future__ import annotations` stores every annotation as the text
//! CPython's compiler produces from the annotation's AST. The rules below
//! reproduce that observable text: normalized operator spacing, parentheses
//! chosen from operator precedence, tuples inside subscripts without
//! parentheses, constants rendered with `repr()`, and f-strings rebuilt from
//! their parts. CPython 3.14.6 is the behavior oracle for this module.

use crate::ast::{
    BinaryOp, CallArg, CallKeyword, ComparisonOp, ComprehensionClause, DictItem, Expr,
    FStringConversion, FStringPart, FunctionParams, LogicalOp, Target, TemplateStringPart, UnaryOp,
};
use crate::value::{format_float_display, repr_bytes, repr_string};

const PR_TUPLE: u8 = 0;
const PR_TEST: u8 = 1;
const PR_OR: u8 = 2;
const PR_AND: u8 = 3;
const PR_NOT: u8 = 4;
const PR_CMP: u8 = 5;
const PR_EXPR: u8 = 6;
const PR_BOR: u8 = PR_EXPR;
const PR_BXOR: u8 = 7;
const PR_BAND: u8 = 8;
const PR_SHIFT: u8 = 9;
const PR_ARITH: u8 = 10;
const PR_TERM: u8 = 11;
const PR_FACTOR: u8 = 12;
const PR_POWER: u8 = 13;
const PR_AWAIT: u8 = 14;
const PR_ATOM: u8 = 15;

/// Text used for an infinite float constant, `1e(DBL_MAX_10_EXP + 1)`.
const INFINITY_TEXT: &str = "1e309";

/// Renders an annotation expression the way CPython stores it under
/// `from __future__ import annotations`.
pub(crate) fn annotation_string(expr: &Expr) -> String {
    let mut out = String::new();
    write_expr(&mut out, expr, PR_TEST);
    out
}

fn open_if(out: &mut String, condition: bool) {
    if condition {
        out.push('(');
    }
}

fn close_if(out: &mut String, condition: bool) {
    if condition {
        out.push(')');
    }
}

fn write_expr(out: &mut String, expr: &Expr, level: u8) {
    match expr {
        Expr::Number(value) => out.push_str(&value.to_string()),
        Expr::BigInt(value) => out.push_str(&big_int_text(value)),
        Expr::Float(value) => out.push_str(&float_text(value)),
        Expr::Imaginary(value) => out.push_str(&imaginary_text(value)),
        Expr::String(value) => out.push_str(&repr_string(value)),
        Expr::Bytes(value) => out.push_str(&repr_bytes(value)),
        Expr::JoinedString(parts) => {
            let body = f_string_body(parts);
            out.push('f');
            out.push_str(&repr_string(&body));
        }
        Expr::TemplateString(parts) => {
            let body = template_string_body(parts);
            out.push('t');
            out.push_str(&repr_string(&body));
        }
        Expr::TemplateInterpolation {
            expression,
            conversion,
            format_spec,
            ..
        } => write_interpolation(out, expression, *conversion, format_spec.as_deref()),
        Expr::Bool(true) => out.push_str("True"),
        Expr::Bool(false) => out.push_str("False"),
        Expr::None => out.push_str("None"),
        Expr::Ellipsis => out.push_str("..."),
        Expr::Name(name) => out.push_str(name),
        Expr::Attribute { object, name } => write_attribute(out, object, name),
        Expr::Binary { left, op, right } => write_binary(out, left, op, right, level),
        Expr::Comparison { left, op, right } => {
            write_comparison(out, left, [(op, right.as_ref())], level)
        }
        Expr::ChainedComparison { left, comparisons } => write_comparison(
            out,
            left,
            comparisons.iter().map(|(op, comparator)| (op, comparator)),
            level,
        ),
        Expr::Unary { op, operand } => {
            let (text, precedence) = match op {
                UnaryOp::Invert => ("~", PR_FACTOR),
                UnaryOp::Not => ("not ", PR_NOT),
                UnaryOp::Positive => ("+", PR_FACTOR),
                UnaryOp::Negative => ("-", PR_FACTOR),
            };
            open_if(out, level > precedence);
            out.push_str(text);
            write_expr(out, operand, precedence);
            close_if(out, level > precedence);
        }
        Expr::Logical { op, .. } => {
            let (text, precedence) = match op {
                LogicalOp::And => (" and ", PR_AND),
                LogicalOp::Or => (" or ", PR_OR),
            };
            let mut values = Vec::new();
            collect_logical_operands(expr, op, &mut values);
            open_if(out, level > precedence);
            for (index, value) in values.into_iter().enumerate() {
                if index > 0 {
                    out.push_str(text);
                }
                write_expr(out, value, precedence + 1);
            }
            close_if(out, level > precedence);
        }
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            open_if(out, level > PR_TEST);
            write_expr(out, then_branch, PR_TEST + 1);
            out.push_str(" if ");
            write_expr(out, condition, PR_TEST + 1);
            out.push_str(" else ");
            write_expr(out, else_branch, PR_TEST);
            close_if(out, level > PR_TEST);
        }
        Expr::NamedExpr { name, value } => {
            open_if(out, level > PR_TUPLE);
            out.push_str(name);
            out.push_str(" := ");
            write_expr(out, value, PR_ATOM);
            close_if(out, level > PR_TUPLE);
        }
        Expr::Yield { value } => match value {
            None => out.push_str("(yield)"),
            Some(value) => {
                out.push_str("(yield ");
                write_expr(out, value, PR_TEST);
                out.push(')');
            }
        },
        Expr::YieldFrom(value) => {
            out.push_str("(yield from ");
            write_expr(out, value, PR_TEST);
            out.push(')');
        }
        Expr::Await(value) => {
            open_if(out, level > PR_AWAIT);
            out.push_str("await ");
            write_expr(out, value, PR_ATOM);
            close_if(out, level > PR_AWAIT);
        }
        Expr::Starred(value) => {
            out.push('*');
            write_expr(out, value, PR_EXPR);
        }
        Expr::List(items) => {
            out.push('[');
            write_comma_separated(out, items, PR_TEST);
            out.push(']');
        }
        Expr::Set(items) | Expr::FrozenSet(items) => {
            out.push('{');
            write_comma_separated(out, items, PR_TEST);
            out.push('}');
        }
        Expr::Tuple(items) => write_tuple(out, items, level),
        Expr::Dict(items) => {
            out.push('{');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                match item {
                    DictItem::Entry { key, value } => {
                        write_expr(out, key, PR_TEST);
                        out.push_str(": ");
                        write_expr(out, value, PR_TEST);
                    }
                    DictItem::Unpack(value) => {
                        out.push_str("**");
                        write_expr(out, value, PR_EXPR);
                    }
                }
            }
            out.push('}');
        }
        Expr::ListComp { element, clauses } => {
            out.push('[');
            write_expr(out, element, PR_TEST);
            write_comprehension_clauses(out, clauses);
            out.push(']');
        }
        Expr::SetComp { element, clauses } => {
            out.push('{');
            write_expr(out, element, PR_TEST);
            write_comprehension_clauses(out, clauses);
            out.push('}');
        }
        Expr::GeneratorComp { element, clauses } => {
            out.push('(');
            write_expr(out, element, PR_TEST);
            write_comprehension_clauses(out, clauses);
            out.push(')');
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            out.push('{');
            write_expr(out, key, PR_TEST);
            out.push_str(": ");
            write_expr(out, value, PR_TEST);
            write_comprehension_clauses(out, clauses);
            out.push('}');
        }
        Expr::Subscript { object, index } => {
            write_expr(out, object, PR_ATOM);
            out.push('[');
            write_expr(out, index, PR_TUPLE);
            out.push(']');
        }
        Expr::SliceLiteral { start, stop, step } => {
            write_slice(out, start.as_deref(), stop.as_deref(), step.as_deref())
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            write_expr(out, object, PR_ATOM);
            out.push('[');
            write_slice(out, start.as_deref(), stop.as_deref(), step.as_deref());
            out.push(']');
        }
        Expr::Call { callee, args } => {
            let args = args.iter().map(CallArgRef::Positional).collect::<Vec<_>>();
            write_call(out, callee, &args, &[]);
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            let args = args.iter().map(CallArgRef::Positional).collect::<Vec<_>>();
            let keywords = keywords
                .iter()
                .map(|(name, value)| CallKeywordRef::Named(name, value))
                .collect::<Vec<_>>();
            write_call(out, callee, &args, &keywords);
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            let args = args
                .iter()
                .map(|arg| match arg {
                    CallArg::Expr(value) => CallArgRef::Positional(value),
                    CallArg::Unpack(value) => CallArgRef::Unpack(value),
                })
                .collect::<Vec<_>>();
            let keywords = keywords
                .iter()
                .map(|keyword| match keyword {
                    CallKeyword::Named(name, value) => CallKeywordRef::Named(name, value),
                    CallKeyword::Unpack(value) => CallKeywordRef::Unpack(value),
                })
                .collect::<Vec<_>>();
            write_call(out, callee, &args, &keywords);
        }
        Expr::Lambda { params, body } => {
            open_if(out, level > PR_TEST);
            let positional = params.positional_only.len() + params.positional.len();
            // CPython only separates `lambda` from its parameters with a space
            // when there is at least one positional parameter.
            out.push_str(if positional > 0 { "lambda " } else { "lambda" });
            write_parameters(out, params);
            out.push_str(": ");
            write_expr(out, body, PR_TEST);
            close_if(out, level > PR_TEST);
        }
    }
}

fn write_attribute(out: &mut String, object: &Expr, name: &str) {
    write_expr(out, object, PR_ATOM);
    // Integers need a space so that the period is not read as a decimal
    // point.
    if matches!(object, Expr::Number(_) | Expr::BigInt(_)) {
        out.push_str(" .");
    } else {
        out.push('.');
    }
    out.push_str(name);
}

fn collect_logical_operands<'a>(expr: &'a Expr, op: &LogicalOp, values: &mut Vec<&'a Expr>) {
    match expr {
        Expr::Logical {
            left,
            op: inner,
            right,
        } if inner == op => {
            collect_logical_operands(left, op, values);
            values.push(right);
        }
        _ => values.push(expr),
    }
}

fn write_binary(out: &mut String, left: &Expr, op: &BinaryOp, right: &Expr, level: u8) {
    let (text, precedence) = match op {
        BinaryOp::Add => (" + ", PR_ARITH),
        BinaryOp::Subtract => (" - ", PR_ARITH),
        BinaryOp::Multiply => (" * ", PR_TERM),
        BinaryOp::MatrixMultiply => (" @ ", PR_TERM),
        BinaryOp::TrueDivide => (" / ", PR_TERM),
        BinaryOp::Modulo => (" % ", PR_TERM),
        BinaryOp::LeftShift => (" << ", PR_SHIFT),
        BinaryOp::RightShift => (" >> ", PR_SHIFT),
        BinaryOp::BitOr => (" | ", PR_BOR),
        BinaryOp::BitXor => (" ^ ", PR_BXOR),
        BinaryOp::BitAnd => (" & ", PR_BAND),
        BinaryOp::FloorDivide => (" // ", PR_TERM),
        BinaryOp::Power => (" ** ", PR_POWER),
    };
    let right_associative = matches!(op, BinaryOp::Power);
    open_if(out, level > precedence);
    write_expr(out, left, precedence + u8::from(right_associative));
    out.push_str(text);
    write_expr(out, right, precedence + u8::from(!right_associative));
    close_if(out, level > precedence);
}

fn write_comparison<'a>(
    out: &mut String,
    left: &Expr,
    comparisons: impl IntoIterator<Item = (&'a ComparisonOp, &'a Expr)>,
    level: u8,
) {
    open_if(out, level > PR_CMP);
    write_expr(out, left, PR_CMP + 1);
    for (op, comparator) in comparisons {
        out.push_str(match op {
            ComparisonOp::Equal => " == ",
            ComparisonOp::NotEqual => " != ",
            ComparisonOp::Less => " < ",
            ComparisonOp::LessEqual => " <= ",
            ComparisonOp::Greater => " > ",
            ComparisonOp::GreaterEqual => " >= ",
            ComparisonOp::Is => " is ",
            ComparisonOp::IsNot => " is not ",
            ComparisonOp::In => " in ",
            ComparisonOp::NotIn => " not in ",
        });
        write_expr(out, comparator, PR_CMP + 1);
    }
    close_if(out, level > PR_CMP);
}

fn write_comma_separated(out: &mut String, items: &[Expr], level: u8) {
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        write_expr(out, item, level);
    }
}

fn write_tuple(out: &mut String, items: &[Expr], level: u8) {
    if items.is_empty() {
        out.push_str("()");
        return;
    }
    open_if(out, level > PR_TUPLE);
    write_comma_separated(out, items, PR_TEST);
    if items.len() == 1 {
        out.push(',');
    }
    close_if(out, level > PR_TUPLE);
}

fn write_slice(out: &mut String, start: Option<&Expr>, stop: Option<&Expr>, step: Option<&Expr>) {
    if let Some(start) = start {
        write_expr(out, start, PR_TEST);
    }
    out.push(':');
    if let Some(stop) = stop {
        write_expr(out, stop, PR_TEST);
    }
    if let Some(step) = step {
        out.push(':');
        write_expr(out, step, PR_TEST);
    }
}

enum CallArgRef<'a> {
    Positional(&'a Expr),
    Unpack(&'a Expr),
}

enum CallKeywordRef<'a> {
    Named(&'a str, &'a Expr),
    Unpack(&'a Expr),
}

fn write_call(
    out: &mut String,
    callee: &Expr,
    args: &[CallArgRef<'_>],
    keywords: &[CallKeywordRef<'_>],
) {
    write_expr(out, callee, PR_ATOM);
    if let ([CallArgRef::Positional(generator @ Expr::GeneratorComp { .. })], []) = (args, keywords)
    {
        // A lone generator expression argument shares the call parentheses.
        write_expr(out, generator, PR_TEST);
        return;
    }
    out.push('(');
    let mut first = true;
    for arg in args {
        if !first {
            out.push_str(", ");
        }
        first = false;
        match arg {
            CallArgRef::Positional(value) => write_expr(out, value, PR_TEST),
            CallArgRef::Unpack(value) => {
                out.push('*');
                write_expr(out, value, PR_EXPR);
            }
        }
    }
    for keyword in keywords {
        if !first {
            out.push_str(", ");
        }
        first = false;
        match keyword {
            CallKeywordRef::Named(name, value) => {
                out.push_str(name);
                out.push('=');
                write_expr(out, value, PR_TEST);
            }
            CallKeywordRef::Unpack(value) => {
                out.push_str("**");
                write_expr(out, value, PR_TEST);
            }
        }
    }
    out.push(')');
}

fn write_comprehension_clauses(out: &mut String, clauses: &[ComprehensionClause]) {
    for clause in clauses {
        out.push_str(if clause.is_async {
            " async for "
        } else {
            " for "
        });
        write_target(out, &clause.target, PR_TUPLE);
        out.push_str(" in ");
        write_expr(out, &clause.iter, PR_TEST + 1);
        for condition in &clause.ifs {
            out.push_str(" if ");
            write_expr(out, condition, PR_TEST + 1);
        }
    }
}

fn write_target(out: &mut String, target: &Target, level: u8) {
    match target {
        Target::Name(name) => out.push_str(name),
        Target::Attribute { object, name } => write_attribute(out, object, name),
        Target::Subscript { object, index } => {
            write_expr(out, object, PR_ATOM);
            out.push('[');
            write_expr(out, index, PR_TUPLE);
            out.push(']');
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            write_expr(out, object, PR_ATOM);
            out.push('[');
            write_slice(out, start.as_ref(), stop.as_ref(), step.as_ref());
            out.push(']');
        }
        Target::Starred(inner) => {
            out.push('*');
            write_target(out, inner, PR_EXPR);
        }
        Target::Tuple(items) => {
            if items.is_empty() {
                out.push_str("()");
                return;
            }
            open_if(out, level > PR_TUPLE);
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_target(out, item, PR_TEST);
            }
            if items.len() == 1 {
                out.push(',');
            }
            close_if(out, level > PR_TUPLE);
        }
        Target::List(items) => {
            out.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                write_target(out, item, PR_TEST);
            }
            out.push(']');
        }
    }
}

fn write_parameters(out: &mut String, params: &FunctionParams) {
    let mut first = true;
    let mut separator = |out: &mut String| {
        if !first {
            out.push_str(", ");
        }
        first = false;
    };
    let positional_only = params.positional_only.len();
    for (index, param) in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .enumerate()
    {
        separator(out);
        write_parameter(out, &param.name, param.annotation.as_ref());
        if let Some(default) = &param.default {
            out.push('=');
            write_expr(out, default, PR_TEST);
        }
        if positional_only > 0 && index + 1 == positional_only {
            out.push_str(", /");
        }
    }
    if params.vararg.is_some() || !params.keyword_only.is_empty() {
        separator(out);
        out.push('*');
        if let Some(vararg) = &params.vararg {
            write_parameter(out, vararg, params.vararg_annotation.as_deref());
        }
    }
    for param in &params.keyword_only {
        separator(out);
        write_parameter(out, &param.name, param.annotation.as_ref());
        if let Some(default) = &param.default {
            out.push('=');
            write_expr(out, default, PR_TEST);
        }
    }
    if let Some(kwarg) = &params.kwarg {
        separator(out);
        out.push_str("**");
        write_parameter(out, kwarg, params.kwarg_annotation.as_deref());
    }
}

fn write_parameter(out: &mut String, name: &str, annotation: Option<&Expr>) {
    out.push_str(name);
    if let Some(annotation) = annotation {
        out.push_str(": ");
        write_expr(out, annotation, PR_TEST);
    }
}

fn f_string_body(parts: &[FStringPart]) -> String {
    let mut body = String::new();
    for part in parts {
        match part {
            FStringPart::Literal(text) => push_escaped_braces(&mut body, text),
            FStringPart::Formatted {
                value,
                conversion,
                format_spec,
            } => write_replacement_field(&mut body, value, *conversion, format_spec.as_deref()),
        }
    }
    body
}

fn template_string_body(parts: &[TemplateStringPart]) -> String {
    let mut body = String::new();
    for part in parts {
        match part {
            TemplateStringPart::Literal(text) => push_escaped_braces(&mut body, text),
            TemplateStringPart::Interpolation {
                expression,
                conversion,
                format_spec,
                ..
            } => write_interpolation(&mut body, expression, *conversion, format_spec.as_deref()),
        }
    }
    body
}

fn push_escaped_braces(out: &mut String, text: &str) {
    for ch in text.chars() {
        match ch {
            '{' => out.push_str("{{"),
            '}' => out.push_str("}}"),
            ch => out.push(ch),
        }
    }
}

fn write_replacement_field(
    out: &mut String,
    value: &Expr,
    conversion: Option<FStringConversion>,
    format_spec: Option<&[FStringPart]>,
) {
    // The grammar allows a tuple here, but a level above PR_TEST keeps a
    // lambda's ':' from being read as the start of a format spec.
    let mut expression = String::new();
    write_expr(&mut expression, value, PR_TEST + 1);
    out.push_str(if expression.starts_with('{') {
        "{ "
    } else {
        "{"
    });
    out.push_str(&expression);
    write_conversion_and_format_spec(out, conversion, format_spec);
}

/// Template string interpolations keep their source text (without trailing
/// whitespace) instead of an unparsed expression.
fn write_interpolation(
    out: &mut String,
    expression: &str,
    conversion: Option<FStringConversion>,
    format_spec: Option<&[FStringPart]>,
) {
    out.push('{');
    out.push_str(strip_trailing_comment(expression).trim_end());
    write_conversion_and_format_spec(out, conversion, format_spec);
}

/// Drops a `#` comment that ends an interpolation's source text, ignoring
/// `#` characters inside string literals.
fn strip_trailing_comment(expression: &str) -> &str {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, ch) in expression.char_indices() {
        match quote {
            Some(open) => {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == open {
                    quote = None;
                }
            }
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '#' => return &expression[..index],
                _ => {}
            },
        }
    }
    expression
}

fn write_conversion_and_format_spec(
    out: &mut String,
    conversion: Option<FStringConversion>,
    format_spec: Option<&[FStringPart]>,
) {
    if let Some(conversion) = conversion {
        out.push_str(match conversion {
            FStringConversion::Str => "!s",
            FStringConversion::Repr => "!r",
            FStringConversion::Ascii => "!a",
        });
    }
    if let Some(format_spec) = format_spec {
        out.push(':');
        out.push_str(&f_string_body(format_spec));
    }
    out.push('}');
}

fn big_int_text(value: &str) -> String {
    let digits = value.replace('_', "");
    match digits.parse::<num_bigint::BigInt>() {
        Ok(number) => number.to_string(),
        Err(_) => digits,
    }
}

fn parse_float_text(value: &str) -> Option<f64> {
    value.replace('_', "").parse::<f64>().ok()
}

fn float_text(value: &str) -> String {
    match parse_float_text(value) {
        Some(number) => float_repr(number),
        None => value.to_string(),
    }
}

fn float_repr(value: f64) -> String {
    if value.is_infinite() {
        let sign = if value.is_sign_negative() { "-" } else { "" };
        return format!("{sign}{INFINITY_TEXT}");
    }
    format_float_display(value)
}

fn imaginary_text(value: &str) -> String {
    let digits = value.trim_end_matches(['j', 'J']);
    let Some(imag) = parse_float_text(digits) else {
        return value.to_string();
    };
    // A complex repr prints its float part without a trailing ".0".
    let text = float_repr(imag);
    let text = text.strip_suffix(".0").unwrap_or(&text);
    format!("{text}j")
}
