mod ast;
mod bytecode;
mod compiler;
mod lexer;
mod parser;
mod value;
mod vm;

use compiler::{compile, compile_eval, compile_interactive};
use lexer::{
    decode_source_for_parse, lex_for_parse, lex_with_diagnostics, lex_with_spans_for_parse,
    lex_with_warnings_for_parse,
};
use parser::{parse, parse_eval, parse_func_type, parse_interactive, parse_with_diagnostic};
use vm::Vm;

use crate::ast::{
    CallArg, CallKeyword, ComparisonOp, DictItem, Expr, FStringPart, FunctionParams, Pattern,
    Program, Stmt, Target, TemplateStringPart, TypeParam,
};

pub use lexer::{
    LexError, LexWarning, LexWarningCategory, SourceEncoding, SpannedToken, Token,
    TokenFStringConversion, TokenFStringPart, detect_source_encoding, lex_with_spans,
    tokenize_bytes_with_spans, tokenize_cpython_with_spans, tokenize_with_spans,
};

const MAX_LEADING_UNARY_OPERATORS: usize = 10_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

pub fn run_source(source: &str) -> Result<Vec<String>, String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let stmt = parse(&tokens).map_err(|message| format!("parse error: {message}"))?;
    let instructions = compile(&stmt).map_err(|message| format!("compile error: {message}"))?;
    let mut vm = Vm::new(instructions);

    vm.run()
        .map_err(|message| format!("runtime error: {message}"))
}

pub fn run_source_bytes(source: &[u8]) -> Result<Vec<String>, String> {
    let decoded =
        decode_source_for_parse(source).map_err(|message| format!("decode error: {message}"))?;
    run_source(&decoded)
}

pub fn run_source_with_warnings_as_errors(source: &str) -> Result<Vec<String>, String> {
    reject_too_complex_source(source)?;
    let (tokens, warnings) =
        lex_with_warnings_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    if let Some(warning) = warnings.first() {
        return Err(format!("lex error: {}", warning.message));
    }

    let stmt = parse(&tokens).map_err(|message| format!("parse error: {message}"))?;
    let static_warnings = static_syntax_warnings(&stmt);
    if let Some(warning) = static_warnings.first() {
        return Err(format!("lex error: {}", warning.message));
    }
    let instructions = compile(&stmt).map_err(|message| format!("compile error: {message}"))?;
    let mut vm = Vm::new(instructions);

    vm.run()
        .map_err(|message| format!("runtime error: {message}"))
}

pub fn eval_source(source: &str) -> Result<String, String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let expr = parse_eval(&tokens).map_err(|message| format!("parse error: {message}"))?;
    let instructions =
        compile_eval(&expr).map_err(|message| format!("compile error: {message}"))?;
    let mut vm = Vm::new(instructions);

    vm.run_eval()
        .map(|value| value.to_string())
        .map_err(|message| format!("runtime error: {message}"))
}

pub fn parse_source(source: &str) -> Result<(), String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    parse(&tokens).map_err(|message| format!("parse error: {message}"))?;
    Ok(())
}

pub fn ast_dump_source(source: &str) -> Result<String, String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let program = parse(&tokens).map_err(|message| format!("parse error: {message}"))?;
    Ok(format!("{program:?}"))
}

pub fn parse_eval_source(source: &str) -> Result<(), String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    parse_eval(&tokens).map_err(|message| format!("parse error: {message}"))?;
    Ok(())
}

pub fn ast_dump_eval_source(source: &str) -> Result<String, String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let expr = parse_eval(&tokens).map_err(|message| format!("parse error: {message}"))?;
    Ok(format!("{expr:?}"))
}

pub fn parse_interactive_source(source: &str) -> Result<(), String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    parse_interactive(&tokens).map_err(|message| format!("parse error: {message}"))?;
    Ok(())
}

pub fn ast_dump_interactive_source(source: &str) -> Result<String, String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let program =
        parse_interactive(&tokens).map_err(|message| format!("parse error: {message}"))?;
    Ok(format!("{program:?}"))
}

pub fn compile_source(source: &str) -> Result<(), String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let program = parse(&tokens).map_err(|message| format!("parse error: {message}"))?;
    compile(&program).map_err(|message| format!("compile error: {message}"))?;
    Ok(())
}

pub fn run_interactive_source(source: &str) -> Result<Vec<String>, String> {
    reject_too_complex_source(source)?;
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let program =
        parse_interactive(&tokens).map_err(|message| format!("parse error: {message}"))?;
    let instructions =
        compile_interactive(&program).map_err(|message| format!("compile error: {message}"))?;
    let mut vm = Vm::new(instructions);

    vm.run()
        .map_err(|message| format!("runtime error: {message}"))
}

fn reject_too_complex_source(source: &str) -> Result<(), String> {
    if has_too_many_leading_unary_operators(source) {
        return Err("parse error: too complex".to_string());
    }
    Ok(())
}

fn has_too_many_leading_unary_operators(source: &str) -> bool {
    let mut count = 0usize;
    for ch in source.chars() {
        if ch.is_whitespace() {
            continue;
        }
        if matches!(ch, '+' | '-' | '~') {
            count += 1;
            if count > MAX_LEADING_UNARY_OPERATORS {
                return true;
            }
            continue;
        }
        break;
    }
    false
}

pub fn source_warnings(source: &str) -> Result<Vec<String>, String> {
    let (tokens, mut warnings) =
        lex_with_warnings_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    if warnings.is_empty()
        && let Ok(program) = parse(&tokens)
    {
        warnings.extend(static_syntax_warnings(&program));
    }
    Ok(warnings
        .into_iter()
        .map(|warning| warning.message)
        .collect())
}

pub fn source_warning_diagnostics(source: &str) -> Result<Vec<LexWarning>, String> {
    let (tokens, mut warnings) =
        lex_with_warnings_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    if warnings.is_empty()
        && let Ok(program) = parse(&tokens)
    {
        warnings.extend(static_syntax_warnings(&program));
    }
    Ok(warnings)
}

pub fn source_lex_error_diagnostic(source: &str) -> Option<LexError> {
    lex_with_diagnostics(source).err()
}

pub fn source_warning_as_error_diagnostic(source: &str) -> Result<Option<LexError>, String> {
    let (spanned_tokens, mut warnings) = lex_with_spans_for_parse(source)
        .map_err(|error| format!("lex error: {}", error.message))?;
    let tokens = spanned_tokens
        .iter()
        .map(|spanned| spanned.token.clone())
        .collect::<Vec<_>>();
    if warnings.is_empty()
        && let Ok(program) = parse(&tokens)
    {
        warnings.extend(static_syntax_warnings(&program));
    }
    Ok(warnings.into_iter().next().map(|warning| LexError {
        message: warning.message,
        line: warning.line,
        column: warning.column,
        end_line: warning.end_line,
        end_column: warning.end_column,
    }))
}

fn static_syntax_warnings(program: &Program) -> Vec<LexWarning> {
    let mut warnings = Vec::new();
    collect_finally_control_flow_warnings(&program.statements, false, 0, &mut warnings);
    collect_assert_tuple_warnings(&program.statements, &mut warnings);
    collect_identity_literal_warnings(&program.statements, &mut warnings);
    warnings
}

fn collect_finally_control_flow_warnings(
    statements: &[Stmt],
    in_finally: bool,
    loop_depth_in_finally: usize,
    warnings: &mut Vec<LexWarning>,
) {
    for statement in statements {
        match statement {
            Stmt::Return(_) if in_finally => {
                warnings.push(static_syntax_warning(
                    "'return' in a 'finally' block".to_string(),
                ));
            }
            Stmt::Break if in_finally && loop_depth_in_finally == 0 => {
                warnings.push(static_syntax_warning(
                    "'break' in a 'finally' block".to_string(),
                ));
            }
            Stmt::Continue if in_finally && loop_depth_in_finally == 0 => {
                warnings.push(static_syntax_warning(
                    "'continue' in a 'finally' block".to_string(),
                ));
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_finally_control_flow_warnings(
                    then_body,
                    in_finally,
                    loop_depth_in_finally,
                    warnings,
                );
                collect_finally_control_flow_warnings(
                    else_body,
                    in_finally,
                    loop_depth_in_finally,
                    warnings,
                );
            }
            Stmt::Match { cases, .. } => {
                for case in cases {
                    collect_finally_control_flow_warnings(
                        &case.body,
                        in_finally,
                        loop_depth_in_finally,
                        warnings,
                    );
                }
            }
            Stmt::Try {
                body,
                handlers,
                else_body,
                finally_body,
            }
            | Stmt::TryStar {
                body,
                handlers,
                else_body,
                finally_body,
            } => {
                collect_finally_control_flow_warnings(
                    body,
                    in_finally,
                    loop_depth_in_finally,
                    warnings,
                );
                for handler in handlers {
                    collect_finally_control_flow_warnings(
                        &handler.body,
                        in_finally,
                        loop_depth_in_finally,
                        warnings,
                    );
                }
                collect_finally_control_flow_warnings(
                    else_body,
                    in_finally,
                    loop_depth_in_finally,
                    warnings,
                );
                collect_finally_control_flow_warnings(finally_body, true, 0, warnings);
            }
            Stmt::With { body, .. } | Stmt::AsyncWith { body, .. } => {
                collect_finally_control_flow_warnings(
                    body,
                    in_finally,
                    loop_depth_in_finally,
                    warnings,
                );
            }
            Stmt::While {
                body, else_body, ..
            }
            | Stmt::For {
                body, else_body, ..
            }
            | Stmt::AsyncFor {
                body, else_body, ..
            } => {
                let nested_loop_depth = if in_finally {
                    loop_depth_in_finally + 1
                } else {
                    loop_depth_in_finally
                };
                collect_finally_control_flow_warnings(
                    body,
                    in_finally,
                    nested_loop_depth,
                    warnings,
                );
                collect_finally_control_flow_warnings(
                    else_body,
                    in_finally,
                    loop_depth_in_finally,
                    warnings,
                );
            }
            Stmt::FunctionDef { body, .. } | Stmt::AsyncFunctionDef { body, .. } => {
                if !in_finally {
                    collect_finally_control_flow_warnings(body, false, 0, warnings);
                }
            }
            Stmt::ClassDef { body, .. } => {
                if !in_finally {
                    collect_finally_control_flow_warnings(body, false, 0, warnings);
                }
            }
            Stmt::Pass
            | Stmt::Expr(_)
            | Stmt::Assign { .. }
            | Stmt::AnnAssign { .. }
            | Stmt::TypeAlias { .. }
            | Stmt::AugAssign { .. }
            | Stmt::Delete { .. }
            | Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Return(_)
            | Stmt::Global(_)
            | Stmt::Nonlocal(_)
            | Stmt::Assert { .. }
            | Stmt::Raise { .. }
            | Stmt::Break
            | Stmt::Continue => {}
        }
    }
}

fn static_syntax_warning(message: String) -> LexWarning {
    LexWarning {
        category: LexWarningCategory::SyntaxWarning,
        message,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 1,
    }
}

fn collect_assert_tuple_warnings(statements: &[Stmt], warnings: &mut Vec<LexWarning>) {
    for statement in statements {
        match statement {
            Stmt::Assert {
                condition: crate::ast::Expr::Tuple(elements),
                ..
            } if !elements.is_empty() => {
                warnings.push(static_syntax_warning(
                    "assertion is always true, perhaps remove parentheses?".to_string(),
                ));
            }
            Stmt::If {
                then_body,
                else_body,
                ..
            } => {
                collect_assert_tuple_warnings(then_body, warnings);
                collect_assert_tuple_warnings(else_body, warnings);
            }
            Stmt::Match { cases, .. } => {
                for case in cases {
                    collect_assert_tuple_warnings(&case.body, warnings);
                }
            }
            Stmt::Try {
                body,
                handlers,
                else_body,
                finally_body,
            }
            | Stmt::TryStar {
                body,
                handlers,
                else_body,
                finally_body,
            } => {
                collect_assert_tuple_warnings(body, warnings);
                for handler in handlers {
                    collect_assert_tuple_warnings(&handler.body, warnings);
                }
                collect_assert_tuple_warnings(else_body, warnings);
                collect_assert_tuple_warnings(finally_body, warnings);
            }
            Stmt::With { body, .. }
            | Stmt::AsyncWith { body, .. }
            | Stmt::While { body, .. }
            | Stmt::For { body, .. }
            | Stmt::AsyncFor { body, .. }
            | Stmt::FunctionDef { body, .. }
            | Stmt::AsyncFunctionDef { body, .. }
            | Stmt::ClassDef { body, .. } => collect_assert_tuple_warnings(body, warnings),
            Stmt::Pass
            | Stmt::Expr(_)
            | Stmt::Assign { .. }
            | Stmt::AnnAssign { .. }
            | Stmt::TypeAlias { .. }
            | Stmt::AugAssign { .. }
            | Stmt::Delete { .. }
            | Stmt::Import { .. }
            | Stmt::ImportFrom { .. }
            | Stmt::Return(_)
            | Stmt::Global(_)
            | Stmt::Nonlocal(_)
            | Stmt::Assert { .. }
            | Stmt::Raise { .. }
            | Stmt::Break
            | Stmt::Continue => {}
        }
    }
}

fn collect_identity_literal_warnings(statements: &[Stmt], warnings: &mut Vec<LexWarning>) {
    for statement in statements {
        collect_identity_literal_warnings_stmt(statement, warnings);
    }
}

fn collect_identity_literal_warnings_stmt(statement: &Stmt, warnings: &mut Vec<LexWarning>) {
    match statement {
        Stmt::Expr(expr) => collect_identity_literal_warnings_expr(expr, warnings),
        Stmt::Assign { targets, value } => {
            collect_identity_literal_warnings_targets(targets, warnings);
            collect_identity_literal_warnings_expr(value, warnings);
        }
        Stmt::AnnAssign {
            target,
            annotation,
            value,
            ..
        } => {
            collect_identity_literal_warnings_target(target, warnings);
            collect_identity_literal_warnings_expr(annotation, warnings);
            if let Some(value) = value {
                collect_identity_literal_warnings_expr(value, warnings);
            }
        }
        Stmt::TypeAlias {
            type_params, value, ..
        } => {
            collect_identity_literal_warnings_type_params(type_params, warnings);
            collect_identity_literal_warnings_expr(value, warnings);
        }
        Stmt::AugAssign { target, value, .. } => {
            collect_identity_literal_warnings_target(target, warnings);
            collect_identity_literal_warnings_expr(value, warnings);
        }
        Stmt::Delete { target } => collect_identity_literal_warnings_target(target, warnings),
        Stmt::FunctionDef {
            type_params,
            params,
            body,
            decorators,
            returns,
            ..
        }
        | Stmt::AsyncFunctionDef {
            type_params,
            params,
            body,
            decorators,
            returns,
            ..
        } => {
            collect_identity_literal_warnings_type_params(type_params, warnings);
            collect_identity_literal_warnings_function_params(params, warnings);
            for decorator in decorators {
                collect_identity_literal_warnings_expr(decorator, warnings);
            }
            if let Some(returns) = returns {
                collect_identity_literal_warnings_expr(returns, warnings);
            }
            collect_identity_literal_warnings(body, warnings);
        }
        Stmt::ClassDef {
            type_params,
            bases,
            keywords,
            body,
            decorators,
            ..
        } => {
            collect_identity_literal_warnings_type_params(type_params, warnings);
            for base in bases {
                collect_identity_literal_warnings_call_arg(base, warnings);
            }
            for keyword in keywords {
                collect_identity_literal_warnings_call_keyword(keyword, warnings);
            }
            for decorator in decorators {
                collect_identity_literal_warnings_expr(decorator, warnings);
            }
            collect_identity_literal_warnings(body, warnings);
        }
        Stmt::Return(value) => {
            if let Some(value) = value {
                collect_identity_literal_warnings_expr(value, warnings);
            }
        }
        Stmt::Assert { condition, message } => {
            collect_identity_literal_warnings_expr(condition, warnings);
            if let Some(message) = message {
                collect_identity_literal_warnings_expr(message, warnings);
            }
        }
        Stmt::Raise { value, cause } => {
            if let Some(value) = value {
                collect_identity_literal_warnings_expr(value, warnings);
            }
            if let Some(cause) = cause {
                collect_identity_literal_warnings_expr(cause, warnings);
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_identity_literal_warnings_expr(condition, warnings);
            collect_identity_literal_warnings(then_body, warnings);
            collect_identity_literal_warnings(else_body, warnings);
        }
        Stmt::Match { subject, cases } => {
            collect_identity_literal_warnings_expr(subject, warnings);
            for case in cases {
                collect_identity_literal_warnings_pattern(&case.pattern, warnings);
                if let Some(guard) = &case.guard {
                    collect_identity_literal_warnings_expr(guard, warnings);
                }
                collect_identity_literal_warnings(&case.body, warnings);
            }
        }
        Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
        }
        | Stmt::TryStar {
            body,
            handlers,
            else_body,
            finally_body,
        } => {
            collect_identity_literal_warnings(body, warnings);
            for handler in handlers {
                collect_identity_literal_warnings(&handler.body, warnings);
            }
            collect_identity_literal_warnings(else_body, warnings);
            collect_identity_literal_warnings(finally_body, warnings);
        }
        Stmt::With { items, body } | Stmt::AsyncWith { items, body } => {
            for item in items {
                collect_identity_literal_warnings_expr(&item.context_expr, warnings);
                if let Some(target) = &item.optional_vars {
                    collect_identity_literal_warnings_target(target, warnings);
                }
            }
            collect_identity_literal_warnings(body, warnings);
        }
        Stmt::While {
            condition,
            body,
            else_body,
        } => {
            collect_identity_literal_warnings_expr(condition, warnings);
            collect_identity_literal_warnings(body, warnings);
            collect_identity_literal_warnings(else_body, warnings);
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
        }
        | Stmt::AsyncFor {
            target,
            iter,
            body,
            else_body,
        } => {
            collect_identity_literal_warnings_target(target, warnings);
            collect_identity_literal_warnings_expr(iter, warnings);
            collect_identity_literal_warnings(body, warnings);
            collect_identity_literal_warnings(else_body, warnings);
        }
        Stmt::Pass
        | Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Break
        | Stmt::Continue => {}
    }
}

fn collect_identity_literal_warnings_targets(targets: &[Target], warnings: &mut Vec<LexWarning>) {
    for target in targets {
        collect_identity_literal_warnings_target(target, warnings);
    }
}

fn collect_identity_literal_warnings_target(target: &Target, warnings: &mut Vec<LexWarning>) {
    match target {
        Target::Name(_) => {}
        Target::Attribute { object, .. } => {
            collect_identity_literal_warnings_expr(object, warnings)
        }
        Target::Subscript { object, index } => {
            collect_identity_literal_warnings_expr(object, warnings);
            collect_identity_literal_warnings_expr(index, warnings);
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_identity_literal_warnings_expr(object, warnings);
            collect_identity_literal_warnings_optional_expr(start, warnings);
            collect_identity_literal_warnings_optional_expr(stop, warnings);
            collect_identity_literal_warnings_optional_expr(step, warnings);
        }
        Target::Starred(target) => collect_identity_literal_warnings_target(target, warnings),
        Target::Tuple(targets) | Target::List(targets) => {
            collect_identity_literal_warnings_targets(targets, warnings);
        }
    }
}

fn collect_identity_literal_warnings_expr(expr: &Expr, warnings: &mut Vec<LexWarning>) {
    match expr {
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Bool(_)
        | Expr::None
        | Expr::Ellipsis
        | Expr::Name(_) => {}
        Expr::JoinedString(parts) => {
            for part in parts {
                collect_identity_literal_warnings_f_string_part(part, warnings);
            }
        }
        Expr::TemplateString(parts) => {
            for part in parts {
                collect_identity_literal_warnings_template_string_part(part, warnings);
            }
        }
        Expr::Attribute { object, .. } => collect_identity_literal_warnings_expr(object, warnings),
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            collect_identity_literal_warnings_expr(left, warnings);
            collect_identity_literal_warnings_expr(right, warnings);
        }
        Expr::Comparison { left, op, right } => {
            if let Some(message) = identity_literal_warning_message(left, op, right) {
                warnings.push(static_syntax_warning(message));
            }
            collect_identity_literal_warnings_expr(left, warnings);
            collect_identity_literal_warnings_expr(right, warnings);
        }
        Expr::ChainedComparison { left, comparisons } => {
            let mut left_expr = left.as_ref();
            let mut warned = false;
            for (op, right) in comparisons {
                if !warned {
                    if let Some(message) = identity_literal_warning_message(left_expr, op, right) {
                        warnings.push(static_syntax_warning(message));
                        warned = true;
                    }
                }
                left_expr = right;
            }
            collect_identity_literal_warnings_expr(left, warnings);
            for (_, right) in comparisons {
                collect_identity_literal_warnings_expr(right, warnings);
            }
        }
        Expr::Unary { operand, .. }
        | Expr::NamedExpr { value: operand, .. }
        | Expr::YieldFrom(operand)
        | Expr::Await(operand)
        | Expr::Starred(operand) => collect_identity_literal_warnings_expr(operand, warnings),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_identity_literal_warnings_expr(condition, warnings);
            collect_identity_literal_warnings_expr(then_branch, warnings);
            collect_identity_literal_warnings_expr(else_branch, warnings);
        }
        Expr::Yield { value } => {
            if let Some(value) = value {
                collect_identity_literal_warnings_expr(value, warnings);
            }
        }
        Expr::List(items) | Expr::Set(items) | Expr::FrozenSet(items) | Expr::Tuple(items) => {
            collect_identity_literal_warnings_exprs(items, warnings);
        }
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            collect_identity_literal_warnings_expr(element, warnings);
            collect_identity_literal_warnings_comprehension_clauses(clauses, warnings);
        }
        Expr::Dict(items) => {
            for item in items {
                collect_identity_literal_warnings_dict_item(item, warnings);
            }
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            collect_identity_literal_warnings_expr(key, warnings);
            collect_identity_literal_warnings_expr(value, warnings);
            collect_identity_literal_warnings_comprehension_clauses(clauses, warnings);
        }
        Expr::DictUnpackComp { value, clauses } => {
            collect_identity_literal_warnings_expr(value, warnings);
            collect_identity_literal_warnings_comprehension_clauses(clauses, warnings);
        }
        Expr::Subscript { object, index } => {
            if let Some(message) = missed_comma_subscript_warning(object, index) {
                warnings.push(static_syntax_warning(message));
            }
            collect_identity_literal_warnings_expr(object, warnings);
            collect_identity_literal_warnings_expr(index, warnings);
        }
        Expr::SliceLiteral { start, stop, step } => {
            collect_identity_literal_warnings_optional_boxed_expr(start, warnings);
            collect_identity_literal_warnings_optional_boxed_expr(stop, warnings);
            collect_identity_literal_warnings_optional_boxed_expr(step, warnings);
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            if let Some(message) = missed_comma_subscripter_warning(object) {
                warnings.push(static_syntax_warning(message));
            }
            collect_identity_literal_warnings_expr(object, warnings);
            collect_identity_literal_warnings_optional_boxed_expr(start, warnings);
            collect_identity_literal_warnings_optional_boxed_expr(stop, warnings);
            collect_identity_literal_warnings_optional_boxed_expr(step, warnings);
        }
        Expr::Call { callee, args } => {
            if let Some(message) = missed_comma_call_warning(callee) {
                warnings.push(static_syntax_warning(message));
            }
            collect_identity_literal_warnings_expr(callee, warnings);
            collect_identity_literal_warnings_exprs(args, warnings);
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            if let Some(message) = missed_comma_call_warning(callee) {
                warnings.push(static_syntax_warning(message));
            }
            collect_identity_literal_warnings_expr(callee, warnings);
            collect_identity_literal_warnings_exprs(args, warnings);
            for (_, value) in keywords {
                collect_identity_literal_warnings_expr(value, warnings);
            }
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            if let Some(message) = missed_comma_call_warning(callee) {
                warnings.push(static_syntax_warning(message));
            }
            collect_identity_literal_warnings_expr(callee, warnings);
            for arg in args {
                collect_identity_literal_warnings_call_arg(arg, warnings);
            }
            for keyword in keywords {
                collect_identity_literal_warnings_call_keyword(keyword, warnings);
            }
        }
        Expr::Lambda { params, body } => {
            collect_identity_literal_warnings_function_params(params, warnings);
            collect_identity_literal_warnings_expr(body, warnings);
        }
    }
}

fn collect_identity_literal_warnings_exprs(exprs: &[Expr], warnings: &mut Vec<LexWarning>) {
    for expr in exprs {
        collect_identity_literal_warnings_expr(expr, warnings);
    }
}

fn collect_identity_literal_warnings_optional_expr(
    expr: &Option<Expr>,
    warnings: &mut Vec<LexWarning>,
) {
    if let Some(expr) = expr {
        collect_identity_literal_warnings_expr(expr, warnings);
    }
}

fn collect_identity_literal_warnings_optional_boxed_expr(
    expr: &Option<Box<Expr>>,
    warnings: &mut Vec<LexWarning>,
) {
    if let Some(expr) = expr {
        collect_identity_literal_warnings_expr(expr, warnings);
    }
}

fn collect_identity_literal_warnings_comprehension_clauses(
    clauses: &[crate::ast::ComprehensionClause],
    warnings: &mut Vec<LexWarning>,
) {
    for clause in clauses {
        collect_identity_literal_warnings_target(&clause.target, warnings);
        collect_identity_literal_warnings_expr(&clause.iter, warnings);
        collect_identity_literal_warnings_exprs(&clause.ifs, warnings);
    }
}

fn collect_identity_literal_warnings_dict_item(item: &DictItem, warnings: &mut Vec<LexWarning>) {
    match item {
        DictItem::Entry { key, value } => {
            collect_identity_literal_warnings_expr(key, warnings);
            collect_identity_literal_warnings_expr(value, warnings);
        }
        DictItem::Unpack(value) => collect_identity_literal_warnings_expr(value, warnings),
    }
}

fn collect_identity_literal_warnings_call_arg(arg: &CallArg, warnings: &mut Vec<LexWarning>) {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => {
            collect_identity_literal_warnings_expr(expr, warnings);
        }
    }
}

fn collect_identity_literal_warnings_call_keyword(
    keyword: &CallKeyword,
    warnings: &mut Vec<LexWarning>,
) {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
            collect_identity_literal_warnings_expr(expr, warnings);
        }
    }
}

fn collect_identity_literal_warnings_f_string_part(
    part: &FStringPart,
    warnings: &mut Vec<LexWarning>,
) {
    match part {
        FStringPart::Literal(_) => {}
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            collect_identity_literal_warnings_expr(value, warnings);
            if let Some(format_spec) = format_spec {
                for part in format_spec {
                    collect_identity_literal_warnings_f_string_part(part, warnings);
                }
            }
        }
    }
}

fn collect_identity_literal_warnings_template_string_part(
    part: &TemplateStringPart,
    warnings: &mut Vec<LexWarning>,
) {
    match part {
        TemplateStringPart::Literal(_) => {}
        TemplateStringPart::Interpolation {
            value, format_spec, ..
        } => {
            collect_identity_literal_warnings_expr(value, warnings);
            if let Some(format_spec) = format_spec {
                for part in format_spec {
                    collect_identity_literal_warnings_f_string_part(part, warnings);
                }
            }
        }
    }
}

fn collect_identity_literal_warnings_function_params(
    params: &FunctionParams,
    warnings: &mut Vec<LexWarning>,
) {
    for param in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
    {
        if let Some(annotation) = &param.annotation {
            collect_identity_literal_warnings_expr(annotation, warnings);
        }
        if let Some(default) = &param.default {
            collect_identity_literal_warnings_expr(default, warnings);
        }
    }
    if let Some(annotation) = &params.vararg_annotation {
        collect_identity_literal_warnings_expr(annotation, warnings);
    }
    if let Some(annotation) = &params.kwarg_annotation {
        collect_identity_literal_warnings_expr(annotation, warnings);
    }
}

fn collect_identity_literal_warnings_type_params(
    type_params: &[TypeParam],
    warnings: &mut Vec<LexWarning>,
) {
    for type_param in type_params {
        if let Some(bound) = &type_param.bound {
            collect_identity_literal_warnings_expr(bound, warnings);
        }
        if let Some(default) = &type_param.default {
            collect_identity_literal_warnings_expr(default, warnings);
        }
    }
}

fn collect_identity_literal_warnings_pattern(pattern: &Pattern, warnings: &mut Vec<LexWarning>) {
    match pattern {
        Pattern::Literal(expr) | Pattern::Singleton(expr) | Pattern::Value(expr) => {
            collect_identity_literal_warnings_expr(expr, warnings);
        }
        Pattern::Or(patterns) | Pattern::Sequence(patterns) => {
            for pattern in patterns {
                collect_identity_literal_warnings_pattern(pattern, warnings);
            }
        }
        Pattern::Mapping { entries, .. } => {
            for (key, pattern) in entries {
                collect_identity_literal_warnings_expr(key, warnings);
                collect_identity_literal_warnings_pattern(pattern, warnings);
            }
        }
        Pattern::Class {
            class,
            positional,
            keywords,
        } => {
            collect_identity_literal_warnings_expr(class, warnings);
            for pattern in positional {
                collect_identity_literal_warnings_pattern(pattern, warnings);
            }
            for (_, pattern) in keywords {
                collect_identity_literal_warnings_pattern(pattern, warnings);
            }
        }
        Pattern::As { pattern, .. } => collect_identity_literal_warnings_pattern(pattern, warnings),
        Pattern::Capture(_) | Pattern::Wildcard | Pattern::Star(_) => {}
    }
}

fn identity_literal_warning_message(
    left: &Expr,
    op: &ComparisonOp,
    right: &Expr,
) -> Option<String> {
    if !matches!(op, ComparisonOp::Is | ComparisonOp::IsNot) {
        return None;
    }

    let literal_type =
        identity_warning_literal_type(left).or_else(|| identity_warning_literal_type(right))?;
    let message = match op {
        ComparisonOp::Is => format!("\"is\" with '{literal_type}' literal. Did you mean \"==\"?"),
        ComparisonOp::IsNot => {
            format!("\"is not\" with '{literal_type}' literal. Did you mean \"!=\"?")
        }
        _ => unreachable!("identity operator checked above"),
    };
    Some(message)
}

fn identity_warning_literal_type(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::Number(_) | Expr::BigInt(_) => Some("int"),
        Expr::Float(_) => Some("float"),
        Expr::Imaginary(_) => Some("complex"),
        Expr::String(_) => Some("str"),
        Expr::Bytes(_) => Some("bytes"),
        Expr::Tuple(elements) if elements.iter().all(is_identity_warning_tuple_constant) => {
            Some("tuple")
        }
        Expr::Bool(_) | Expr::None | Expr::Ellipsis => None,
        _ => None,
    }
}

fn is_identity_warning_tuple_constant(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Number(_)
            | Expr::BigInt(_)
            | Expr::Float(_)
            | Expr::Imaginary(_)
            | Expr::String(_)
            | Expr::Bytes(_)
            | Expr::Bool(_)
            | Expr::None
            | Expr::Ellipsis
    )
}

fn missed_comma_call_warning(callee: &Expr) -> Option<String> {
    let type_name = non_callable_static_type(callee)?;
    Some(format!(
        "'{type_name}' object is not callable; perhaps you missed a comma?"
    ))
}

fn non_callable_static_type(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::None => Some("NoneType"),
        Expr::Bool(_) => Some("bool"),
        Expr::Ellipsis => Some("ellipsis"),
        Expr::Number(_) | Expr::BigInt(_) => Some("int"),
        Expr::Float(_) => Some("float"),
        Expr::Imaginary(_) => Some("complex"),
        Expr::String(_) | Expr::JoinedString(_) => Some("str"),
        Expr::Bytes(_) => Some("bytes"),
        Expr::Tuple(_) => Some("tuple"),
        Expr::List(_) | Expr::ListComp { .. } => Some("list"),
        Expr::Set(_) | Expr::SetComp { .. } => Some("set"),
        Expr::Dict(_) | Expr::DictComp { .. } | Expr::DictUnpackComp { .. } => Some("dict"),
        Expr::GeneratorComp { .. } => Some("generator"),
        Expr::TemplateString(_) => Some("string.templatelib.Template"),
        _ => None,
    }
}

fn missed_comma_subscript_warning(object: &Expr, index: &Expr) -> Option<String> {
    missed_comma_subscripter_warning(object).or_else(|| missed_comma_index_warning(object, index))
}

fn missed_comma_subscripter_warning(object: &Expr) -> Option<String> {
    let type_name = non_subscriptable_static_type(object)?;
    Some(format!(
        "'{type_name}' object is not subscriptable; perhaps you missed a comma?"
    ))
}

fn non_subscriptable_static_type(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::None => Some("NoneType"),
        Expr::Bool(_) => Some("bool"),
        Expr::Ellipsis => Some("ellipsis"),
        Expr::Number(_) | Expr::BigInt(_) => Some("int"),
        Expr::Float(_) => Some("float"),
        Expr::Imaginary(_) => Some("complex"),
        Expr::Set(_) | Expr::SetComp { .. } => Some("set"),
        Expr::GeneratorComp { .. } => Some("generator"),
        Expr::Lambda { .. } => Some("function"),
        Expr::TemplateString(_) => Some("string.templatelib.Template"),
        _ => None,
    }
}

fn missed_comma_index_warning(object: &Expr, index: &Expr) -> Option<String> {
    let object_type = index_checked_container_type(object)?;
    let index_type = invalid_static_index_type(index)?;
    Some(format!(
        "{object_type} indices must be integers or slices, not {index_type}; perhaps you missed a comma?"
    ))
}

fn index_checked_container_type(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::String(_) | Expr::JoinedString(_) => Some("str"),
        Expr::Bytes(_) => Some("bytes"),
        Expr::Tuple(_) => Some("tuple"),
        Expr::List(_) | Expr::ListComp { .. } => Some("list"),
        _ => None,
    }
}

fn invalid_static_index_type(expr: &Expr) -> Option<&'static str> {
    match expr {
        Expr::Float(_) => Some("float"),
        Expr::Imaginary(_) => Some("complex"),
        Expr::String(_) | Expr::JoinedString(_) => Some("str"),
        Expr::Bytes(_) => Some("bytes"),
        Expr::None => Some("NoneType"),
        Expr::Ellipsis => Some("ellipsis"),
        Expr::Tuple(_) => Some("tuple"),
        Expr::List(_) | Expr::ListComp { .. } => Some("list"),
        Expr::Set(_) | Expr::SetComp { .. } => Some("set"),
        Expr::Dict(_) | Expr::DictComp { .. } | Expr::DictUnpackComp { .. } => Some("dict"),
        Expr::GeneratorComp { .. } => Some("generator"),
        Expr::Lambda { .. } => Some("function"),
        Expr::TemplateString(_) => Some("string.templatelib.Template"),
        _ => None,
    }
}

pub fn source_parse_error_diagnostic(source: &str) -> Result<Option<ParseError>, String> {
    let (spanned_tokens, _warnings) = lex_with_spans_for_parse(source)
        .map_err(|error| format!("lex error: {}", error.message))?;
    let tokens = spanned_tokens
        .iter()
        .map(|spanned| spanned.token.clone())
        .collect::<Vec<_>>();
    match parse_with_diagnostic(&tokens) {
        Ok(_) => Ok(None),
        Err(message) => Ok(Some(parse_error_diagnostic(
            source,
            message.message,
            &spanned_tokens,
            message.token_index,
        ))),
    }
}

pub fn source_compile_error_diagnostic(source: &str) -> Result<Option<ParseError>, String> {
    let (spanned_tokens, _warnings) = lex_with_spans_for_parse(source)
        .map_err(|error| format!("lex error: {}", error.message))?;
    let tokens = spanned_tokens
        .iter()
        .map(|spanned| spanned.token.clone())
        .collect::<Vec<_>>();
    let program = parse(&tokens).map_err(|message| format!("parse error: {message}"))?;

    match compile(&program) {
        Ok(_) => Ok(None),
        Err(message) => Ok(Some(compile_error_diagnostic(message, &spanned_tokens))),
    }
}

pub fn parse_func_type_source(source: &str) -> Result<String, String> {
    let tokens = lex_for_parse(source).map_err(|message| format!("lex error: {message}"))?;
    let function_type =
        parse_func_type(&tokens).map_err(|message| format!("parse error: {message}"))?;

    Ok(format!("{function_type:?}"))
}

fn compile_error_diagnostic(message: String, spanned_tokens: &[SpannedToken]) -> ParseError {
    let token = compile_error_token(&message, spanned_tokens)
        .or_else(|| {
            spanned_tokens
                .iter()
                .find(|spanned| !matches!(spanned.token, Token::Eof))
        })
        .or_else(|| spanned_tokens.last());

    if let Some(token) = token {
        return ParseError {
            message,
            line: token.line,
            column: token.column,
            end_line: token.end_line,
            end_column: token.end_column,
        };
    }

    ParseError {
        message,
        line: 1,
        column: 1,
        end_line: 1,
        end_column: 1,
    }
}

fn compile_error_token<'a>(
    message: &str,
    spanned_tokens: &'a [SpannedToken],
) -> Option<&'a SpannedToken> {
    if let Some((keyword, name)) = scope_declaration_error_target(message) {
        return find_declaration_token(spanned_tokens, keyword, name);
    }

    let control_flow_token = match message {
        "break outside loop" => Some(ControlFlowKeyword::Break),
        "'continue' not properly in loop" => Some(ControlFlowKeyword::Continue),
        "return outside function" => Some(ControlFlowKeyword::Return),
        "yield outside function" => Some(ControlFlowKeyword::Yield),
        _ => None,
    }?;
    find_control_flow_token(spanned_tokens, control_flow_token)
}

fn scope_declaration_error_target(message: &str) -> Option<(DeclarationKeyword, &str)> {
    let name = quoted_name_in_message(message)?;
    if message.contains("global") {
        return Some((DeclarationKeyword::Global, name));
    }
    if message.contains("nonlocal") {
        return Some((DeclarationKeyword::Nonlocal, name));
    }
    None
}

fn quoted_name_in_message(message: &str) -> Option<&str> {
    let start = message.find('\'')? + 1;
    let end = message[start..].find('\'')?;
    Some(&message[start..start + end])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DeclarationKeyword {
    Global,
    Nonlocal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ControlFlowKeyword {
    Break,
    Continue,
    Return,
    Yield,
}

fn find_declaration_token<'a>(
    spanned_tokens: &'a [SpannedToken],
    keyword: DeclarationKeyword,
    name: &str,
) -> Option<&'a SpannedToken> {
    for (index, spanned) in spanned_tokens.iter().enumerate() {
        if !declaration_keyword_matches(&spanned.token, keyword) {
            continue;
        }

        let mut cursor = index + 1;
        while let Some(candidate) = spanned_tokens.get(cursor) {
            if is_declaration_boundary(&candidate.token) {
                break;
            }
            if matches!(&candidate.token, Token::Identifier(identifier) if identifier == name) {
                return Some(spanned);
            }
            cursor += 1;
        }
    }

    None
}

fn declaration_keyword_matches(token: &Token, keyword: DeclarationKeyword) -> bool {
    matches!(
        (keyword, token),
        (DeclarationKeyword::Global, Token::Global)
            | (DeclarationKeyword::Nonlocal, Token::Nonlocal)
    )
}

fn find_control_flow_token(
    spanned_tokens: &[SpannedToken],
    keyword: ControlFlowKeyword,
) -> Option<&SpannedToken> {
    spanned_tokens
        .iter()
        .find(|spanned| control_flow_keyword_matches(&spanned.token, keyword))
}

fn control_flow_keyword_matches(token: &Token, keyword: ControlFlowKeyword) -> bool {
    matches!(
        (keyword, token),
        (ControlFlowKeyword::Break, Token::Break)
            | (ControlFlowKeyword::Continue, Token::Continue)
            | (ControlFlowKeyword::Return, Token::Return)
            | (ControlFlowKeyword::Yield, Token::Yield)
    )
}

fn is_declaration_boundary(token: &Token) -> bool {
    matches!(
        token,
        Token::Newline | Token::Semicolon | Token::Dedent | Token::Eof
    )
}

fn parse_error_diagnostic(
    source: &str,
    message: String,
    spanned_tokens: &[SpannedToken],
    token_index: Option<usize>,
) -> ParseError {
    let chars = source.chars().collect::<Vec<_>>();
    let (start, end) = parse_error_span(&chars, &message, spanned_tokens, token_index);
    let (line, column) = source_location(&chars, start);
    let (end_line, end_column) = source_location(&chars, end);
    ParseError {
        message,
        line,
        column,
        end_line,
        end_column,
    }
}

fn parse_error_span(
    chars: &[char],
    message: &str,
    spanned_tokens: &[SpannedToken],
    token_index: Option<usize>,
) -> (usize, usize) {
    if let Some(span) = find_invalid_assignment_span(chars, spanned_tokens, message) {
        return span;
    }

    if message == "Generator expression must be parenthesized"
        && let Some(span) = find_generator_expression_span(spanned_tokens, token_index)
    {
        return span;
    }

    if message == "cannot have both 'except' and 'except*' on the same 'try'"
        && let Some(span) = find_except_mixing_span(spanned_tokens, token_index)
    {
        return span;
    }

    if message == "Perhaps you forgot a comma"
        && let Some(span) = find_previous_string_token_span(spanned_tokens, token_index)
    {
        return span;
    }

    if matches!(
        message,
        "cannot use except statement with attribute" | "cannot use attribute as pattern target"
    ) && let Some(span) = find_dotted_attribute_span(spanned_tokens, token_index)
    {
        return span;
    }

    if let Some(found) = message.rsplit_once("found ") {
        if let Some(span) = token_index
            .and_then(|index| token_index_span(spanned_tokens, index))
            .or_else(|| find_found_token_span_in_tokens(spanned_tokens, found.1))
            .or_else(|| find_found_token_span(chars, found.1))
        {
            return span;
        }
    }

    if message_uses_parser_token_index(message)
        && let Some(span) = token_index.and_then(|index| token_index_span(spanned_tokens, index))
    {
        return span;
    }

    let end = chars.len();
    (end, end)
}

fn token_index_span(tokens: &[SpannedToken], index: usize) -> Option<(usize, usize)> {
    tokens.get(index).map(|token| (token.start, token.end))
}

fn find_generator_expression_span(
    spanned_tokens: &[SpannedToken],
    token_index: Option<usize>,
) -> Option<(usize, usize)> {
    let search_start = token_index
        .unwrap_or_else(|| spanned_tokens.len().saturating_sub(1))
        .min(spanned_tokens.len().saturating_sub(1));
    let for_index = (0..=search_start)
        .rev()
        .find(|index| matches!(spanned_tokens[*index].token, Token::For))?;
    let start_index = (0..for_index)
        .rev()
        .find(|index| !generator_span_ignores_token(&spanned_tokens[*index].token))?;
    let start = spanned_tokens[start_index].start;
    let mut depth = 0usize;
    let mut end = spanned_tokens[start_index].end;

    for spanned in &spanned_tokens[start_index..] {
        match &spanned.token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => {
                depth += 1;
                end = spanned.end;
            }
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                if depth == 0 {
                    return Some((start, end));
                }
                depth -= 1;
                end = spanned.end;
            }
            Token::Comma if depth == 0 => return Some((start, end)),
            Token::Newline | Token::Eof if depth == 0 => return Some((start, end)),
            _ => end = spanned.end,
        }
    }

    Some((start, end))
}

fn find_except_mixing_span(
    spanned_tokens: &[SpannedToken],
    token_index: Option<usize>,
) -> Option<(usize, usize)> {
    let index = token_index?;
    let token = spanned_tokens.get(index)?;
    if !matches!(token.token, Token::Except) {
        return None;
    }

    let end = match spanned_tokens.get(index + 1) {
        Some(next) if matches!(next.token, Token::Star) => next.end,
        _ => token.end,
    };
    Some((token.start, end))
}

fn find_previous_string_token_span(
    spanned_tokens: &[SpannedToken],
    token_index: Option<usize>,
) -> Option<(usize, usize)> {
    let search_start = token_index
        .unwrap_or_else(|| spanned_tokens.len().saturating_sub(1))
        .min(spanned_tokens.len().saturating_sub(1));

    (0..=search_start).rev().find_map(|index| {
        let token = spanned_tokens.get(index)?;
        if matches!(
            token.token,
            Token::String(_) | Token::FString(_) | Token::TString(_)
        ) {
            Some((token.start, token.end))
        } else {
            None
        }
    })
}

fn find_dotted_attribute_span(
    spanned_tokens: &[SpannedToken],
    token_index: Option<usize>,
) -> Option<(usize, usize)> {
    let mut index = token_index?;
    let token = spanned_tokens.get(index)?;

    if matches!(token.token, Token::Dot) && index > 0 {
        index -= 1;
    }

    let token = spanned_tokens.get(index)?;
    if !matches!(token.token, Token::Identifier(_)) {
        return None;
    }

    let start = token.start;
    let mut end = token.end;
    let mut cursor = index;

    while matches!(
        (
            spanned_tokens.get(cursor + 1),
            spanned_tokens.get(cursor + 2)
        ),
        (
            Some(SpannedToken {
                token: Token::Dot,
                ..
            }),
            Some(SpannedToken {
                token: Token::Identifier(_),
                ..
            })
        )
    ) {
        end = spanned_tokens[cursor + 2].end;
        cursor += 2;
    }

    Some((start, end))
}

fn generator_span_ignores_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Comma
            | Token::LeftParen
            | Token::Newline
            | Token::Indent
            | Token::Dedent
            | Token::Eof
    )
}

fn message_uses_parser_token_index(message: &str) -> bool {
    matches!(
        message,
        "expected statement after ':'"
            | "expected argument value expression"
            | "expected default value expression"
            | "expected comma between / and *"
            | "expected with item"
            | "expected with item after ','"
            | "expected at least one case block"
            | "expected 'except' or 'finally' after try block"
            | "expected an indented block"
            | "unexpected indent"
            | "cannot have both 'except' and 'except*' on the same 'try'"
            | "cannot use except statement with attribute"
            | "cannot use attribute as pattern target"
            | "expected expression before 'if', but statement is given"
            | "invalid syntax"
            | "Perhaps you forgot a comma"
    ) || message.starts_with("keyword argument repeated")
}

fn find_invalid_assignment_span(
    chars: &[char],
    spanned_tokens: &[SpannedToken],
    message: &str,
) -> Option<(usize, usize)> {
    if message.starts_with("cannot use assignment expressions with ") {
        return find_assignment_left_span_in_tokens(spanned_tokens, AssignmentOperator::Walrus)
            .or_else(|| find_assignment_left_span(chars, AssignmentOperator::Walrus));
    }

    if message == "invalid syntax. Maybe you meant '==' or ':=' instead of '='?"
        || message == "expression cannot contain assignment, perhaps you meant \"==\"?"
        || message == "assignment to yield expression not possible"
        || message.starts_with("cannot assign to ")
    {
        return find_assignment_left_span_in_tokens(spanned_tokens, AssignmentOperator::Equal)
            .or_else(|| find_assignment_left_span(chars, AssignmentOperator::Equal));
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssignmentOperator {
    Equal,
    Walrus,
}

fn find_assignment_left_span(
    chars: &[char],
    operator: AssignmentOperator,
) -> Option<(usize, usize)> {
    let operator_start = match operator {
        AssignmentOperator::Equal => find_last_assignment_equal(chars),
        AssignmentOperator::Walrus => find_last_walrus(chars),
    }?;

    find_left_expr_span_before(chars, operator_start)
}

fn find_assignment_left_span_in_tokens(
    tokens: &[SpannedToken],
    operator: AssignmentOperator,
) -> Option<(usize, usize)> {
    let operator_index = tokens
        .iter()
        .rposition(|token| assignment_operator_matches(&token.token, operator))?;
    if operator_index == 0 {
        return None;
    }

    let mut start_index = 0usize;
    let mut depth = 0usize;
    let mut index = operator_index;
    while index > 0 {
        index -= 1;
        match &tokens[index].token {
            Token::RightParen | Token::RightBracket | Token::RightBrace => depth += 1,
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    start_index = index + 1;
                    break;
                }
            }
            Token::Newline | Token::Semicolon | Token::Comma if depth == 0 => {
                start_index = index + 1;
                break;
            }
            Token::Equal if depth == 0 => {
                start_index = index + 1;
                break;
            }
            _ => {}
        }
    }

    if start_index < operator_index
        && is_statement_intro_token(&tokens[start_index].token)
        && start_index + 1 < operator_index
    {
        start_index += 1;
    }

    if start_index < operator_index {
        Some((tokens[start_index].start, tokens[operator_index - 1].end))
    } else {
        None
    }
}

fn assignment_operator_matches(token: &Token, operator: AssignmentOperator) -> bool {
    matches!(
        (token, operator),
        (Token::Equal, AssignmentOperator::Equal) | (Token::ColonEqual, AssignmentOperator::Walrus)
    )
}

fn is_statement_intro_token(token: &Token) -> bool {
    matches!(
        token,
        Token::If | Token::Elif | Token::While | Token::Assert | Token::Return
    )
}

fn find_last_assignment_equal(chars: &[char]) -> Option<usize> {
    chars.iter().enumerate().rev().find_map(|(index, ch)| {
        if *ch == '=' && is_single_assignment_equal(chars, index) {
            Some(index)
        } else {
            None
        }
    })
}

fn find_last_walrus(chars: &[char]) -> Option<usize> {
    chars.windows(2).rposition(|window| window == [':', '='])
}

fn is_single_assignment_equal(chars: &[char], index: usize) -> bool {
    let previous = index
        .checked_sub(1)
        .and_then(|previous| chars.get(previous));
    let next = chars.get(index + 1);

    !matches!(previous, Some('=' | '!' | '<' | '>' | ':')) && !matches!(next, Some('='))
}

fn find_left_expr_span_before(chars: &[char], boundary: usize) -> Option<(usize, usize)> {
    let mut end = boundary;
    while end > 0 && chars[end - 1].is_whitespace() {
        end -= 1;
    }
    if end == 0 {
        return None;
    }

    let mut start = 0;
    let mut depth = 0usize;
    let mut index = end;
    while index > 0 {
        index -= 1;
        match chars[index] {
            ')' | ']' | '}' => depth += 1,
            '(' | '[' | '{' => {
                if depth > 0 {
                    depth -= 1;
                } else {
                    start = index + 1;
                    break;
                }
            }
            '\n' | '\r' | ';' | ',' if depth == 0 => {
                start = index + 1;
                break;
            }
            '=' if depth == 0 && is_single_assignment_equal(chars, index) => {
                start = index + 1;
                break;
            }
            _ => {}
        }
    }

    while start < end && chars[start].is_whitespace() {
        start += 1;
    }
    let start = skip_statement_intro_keyword(chars, start, end);

    if start < end {
        Some((start, end))
    } else {
        None
    }
}

fn skip_statement_intro_keyword(chars: &[char], start: usize, end: usize) -> usize {
    for keyword in ["if", "elif", "while", "assert", "return"] {
        let keyword_chars = keyword.chars().collect::<Vec<_>>();
        let keyword_end = start + keyword_chars.len();
        if keyword_end < end
            && chars[start..keyword_end] == keyword_chars
            && chars[keyword_end].is_whitespace()
            && word_boundary_before(chars, start)
        {
            let mut next = keyword_end;
            while next < end && chars[next].is_whitespace() {
                next += 1;
            }
            return next;
        }
    }

    start
}

fn find_found_token_span_in_tokens(tokens: &[SpannedToken], found: &str) -> Option<(usize, usize)> {
    tokens
        .iter()
        .find(|spanned| token_matches_found(&spanned.token, found))
        .map(|spanned| (spanned.start, spanned.end))
}

fn token_matches_found(token: &Token, found: &str) -> bool {
    match found {
        "Colon" => matches!(token, Token::Colon),
        "Arrow" => matches!(token, Token::Arrow),
        "Pass" => matches!(token, Token::Pass),
        "Newline" => matches!(token, Token::Newline),
        "Eof" => matches!(token, Token::Eof),
        _ => {
            if let Some(identifier) = found
                .strip_prefix("Identifier(\"")
                .and_then(|rest| rest.strip_suffix("\")"))
            {
                return matches!(token, Token::Identifier(name) if name == identifier);
            }

            if let Some(number) = found
                .strip_prefix("Number(")
                .and_then(|rest| rest.strip_suffix(')'))
            {
                return matches!(token, Token::Number(value) if value.to_string() == number);
            }

            false
        }
    }
}

fn find_found_token_span(chars: &[char], found: &str) -> Option<(usize, usize)> {
    match found {
        "Colon" => find_char_span(chars, ':'),
        "Arrow" => find_str_span(chars, "->"),
        "Pass" => find_word_span(chars, "pass"),
        "Newline" => find_char_span(chars, '\n').or_else(|| find_char_span(chars, '\r')),
        "Eof" => Some((chars.len(), chars.len())),
        _ => {
            if let Some(identifier) = found
                .strip_prefix("Identifier(\"")
                .and_then(|rest| rest.strip_suffix("\")"))
            {
                return find_word_span(chars, identifier);
            }
            if let Some(number) = found
                .strip_prefix("Number(")
                .and_then(|rest| rest.strip_suffix(')'))
            {
                return find_str_span(chars, number);
            }
            None
        }
    }
}

fn find_char_span(chars: &[char], needle: char) -> Option<(usize, usize)> {
    chars
        .iter()
        .position(|ch| *ch == needle)
        .map(|index| (index, index + 1))
}

fn find_str_span(chars: &[char], needle: &str) -> Option<(usize, usize)> {
    let needle = needle.chars().collect::<Vec<_>>();
    if needle.is_empty() || needle.len() > chars.len() {
        return None;
    }

    chars
        .windows(needle.len())
        .position(|window| window == needle.as_slice())
        .map(|index| (index, index + needle.len()))
}

fn find_word_span(chars: &[char], needle: &str) -> Option<(usize, usize)> {
    let needle_chars = needle.chars().collect::<Vec<_>>();
    if needle_chars.is_empty() || needle_chars.len() > chars.len() {
        return None;
    }

    chars
        .windows(needle_chars.len())
        .enumerate()
        .find(|(index, window)| {
            *window == needle_chars.as_slice()
                && word_boundary_before(chars, *index)
                && word_boundary_after(chars, *index + needle_chars.len())
        })
        .map(|(index, _)| (index, index + needle_chars.len()))
}

fn word_boundary_before(chars: &[char], index: usize) -> bool {
    index == 0 || !is_identifier_continue(chars[index - 1])
}

fn word_boundary_after(chars: &[char], index: usize) -> bool {
    index >= chars.len() || !is_identifier_continue(chars[index])
}

fn is_identifier_continue(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphanumeric() || unicode_ident::is_xid_continue(ch)
}

fn source_location(chars: &[char], index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    let mut current = 0usize;

    while current < index && current < chars.len() {
        match chars[current] {
            '\n' => {
                line += 1;
                column = 1;
                current += 1;
            }
            '\r' => {
                line += 1;
                column = 1;
                current += 1;
                if current < index && matches!(chars.get(current), Some('\n')) {
                    current += 1;
                }
            }
            _ => {
                column += 1;
                current += 1;
            }
        }
    }

    (line, column)
}
