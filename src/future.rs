// SPDX-License-Identifier: MIT

//! Future statements and the native `__future__` module data.
//!
//! The feature table, compiler flags, and diagnostics mirror the public
//! behavior of CPython 3.14.6. Mandatory features are accepted as no-ops;
//! `annotations` enables PEP 563 string annotations and `barry_as_FLUFL`
//! switches the inequality spelling to `<>`.

use crate::ast::{
    ComprehensionClause, DictItem, Expr, FStringPart, FunctionParams, ImportFromTargets, Program,
    Stmt, TemplateStringPart,
};

pub(crate) const CO_NESTED: i64 = 0x0010;
pub(crate) const CO_GENERATOR_ALLOWED: i64 = 0;
pub(crate) const CO_FUTURE_DIVISION: i64 = 0x2_0000;
pub(crate) const CO_FUTURE_ABSOLUTE_IMPORT: i64 = 0x4_0000;
pub(crate) const CO_FUTURE_WITH_STATEMENT: i64 = 0x8_0000;
pub(crate) const CO_FUTURE_PRINT_FUNCTION: i64 = 0x10_0000;
pub(crate) const CO_FUTURE_UNICODE_LITERALS: i64 = 0x20_0000;
pub(crate) const CO_FUTURE_BARRY_AS_BDFL: i64 = 0x40_0000;
pub(crate) const CO_FUTURE_GENERATOR_STOP: i64 = 0x80_0000;
pub(crate) const CO_FUTURE_ANNOTATIONS: i64 = 0x100_0000;

/// Future flags that code objects carry and that `compile()` accepts.
pub(crate) const PYCF_MASK: i64 = CO_FUTURE_DIVISION
    | CO_FUTURE_ABSOLUTE_IMPORT
    | CO_FUTURE_WITH_STATEMENT
    | CO_FUTURE_PRINT_FUNCTION
    | CO_FUTURE_UNICODE_LITERALS
    | CO_FUTURE_BARRY_AS_BDFL
    | CO_FUTURE_GENERATOR_STOP
    | CO_FUTURE_ANNOTATIONS;
/// Obsolete flags that `compile()` still accepts and ignores.
pub(crate) const PYCF_MASK_OBSOLETE: i64 = CO_NESTED;

pub(crate) const FUTURE_MODULE_NAME: &str = "__future__";
pub(crate) const FUTURE_IMPORT_POSITION_ERROR: &str =
    "from __future__ imports must occur at the beginning of the file";
pub(crate) const FUTURE_BRACES_ERROR: &str = "not a chance";
pub(crate) const BARRY_AS_BDFL_ERROR: &str = "with Barry as BDFL, use '<>' instead of '!='";

pub(crate) type ReleaseInfo = (i64, i64, i64, &'static str, i64);

pub(crate) struct FutureFeature {
    pub name: &'static str,
    pub optional: ReleaseInfo,
    pub mandatory: Option<ReleaseInfo>,
    pub compiler_flag: i64,
}

/// Every feature in the order of `__future__.all_feature_names`.
pub(crate) const FUTURE_FEATURES: &[FutureFeature] = &[
    FutureFeature {
        name: "nested_scopes",
        optional: (2, 1, 0, "beta", 1),
        mandatory: Some((2, 2, 0, "alpha", 0)),
        compiler_flag: CO_NESTED,
    },
    FutureFeature {
        name: "generators",
        optional: (2, 2, 0, "alpha", 1),
        mandatory: Some((2, 3, 0, "final", 0)),
        compiler_flag: CO_GENERATOR_ALLOWED,
    },
    FutureFeature {
        name: "division",
        optional: (2, 2, 0, "alpha", 2),
        mandatory: Some((3, 0, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_DIVISION,
    },
    FutureFeature {
        name: "absolute_import",
        optional: (2, 5, 0, "alpha", 1),
        mandatory: Some((3, 0, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_ABSOLUTE_IMPORT,
    },
    FutureFeature {
        name: "with_statement",
        optional: (2, 5, 0, "alpha", 1),
        mandatory: Some((2, 6, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_WITH_STATEMENT,
    },
    FutureFeature {
        name: "print_function",
        optional: (2, 6, 0, "alpha", 2),
        mandatory: Some((3, 0, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_PRINT_FUNCTION,
    },
    FutureFeature {
        name: "unicode_literals",
        optional: (2, 6, 0, "alpha", 2),
        mandatory: Some((3, 0, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_UNICODE_LITERALS,
    },
    FutureFeature {
        name: "barry_as_FLUFL",
        optional: (3, 1, 0, "alpha", 2),
        mandatory: Some((4, 0, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_BARRY_AS_BDFL,
    },
    FutureFeature {
        name: "generator_stop",
        optional: (3, 5, 0, "beta", 1),
        mandatory: Some((3, 7, 0, "alpha", 0)),
        compiler_flag: CO_FUTURE_GENERATOR_STOP,
    },
    FutureFeature {
        name: "annotations",
        optional: (3, 7, 0, "beta", 1),
        mandatory: None,
        compiler_flag: CO_FUTURE_ANNOTATIONS,
    },
];

/// The compiler flag constants exposed as `__future__` module attributes.
pub(crate) const FUTURE_COMPILER_FLAG_NAMES: &[(&str, i64)] = &[
    ("CO_NESTED", CO_NESTED),
    ("CO_GENERATOR_ALLOWED", CO_GENERATOR_ALLOWED),
    ("CO_FUTURE_DIVISION", CO_FUTURE_DIVISION),
    ("CO_FUTURE_ABSOLUTE_IMPORT", CO_FUTURE_ABSOLUTE_IMPORT),
    ("CO_FUTURE_WITH_STATEMENT", CO_FUTURE_WITH_STATEMENT),
    ("CO_FUTURE_PRINT_FUNCTION", CO_FUTURE_PRINT_FUNCTION),
    ("CO_FUTURE_UNICODE_LITERALS", CO_FUTURE_UNICODE_LITERALS),
    ("CO_FUTURE_BARRY_AS_BDFL", CO_FUTURE_BARRY_AS_BDFL),
    ("CO_FUTURE_GENERATOR_STOP", CO_FUTURE_GENERATOR_STOP),
    ("CO_FUTURE_ANNOTATIONS", CO_FUTURE_ANNOTATIONS),
];

/// Result of scanning a module body for its leading future statements.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct FutureStatements {
    /// Flags enabled by the future statements themselves.
    pub flags: i64,
    /// Number of leading top-level statements (docstring included) that may
    /// legally be future statements.
    pub prefix_len: usize,
}

pub(crate) fn is_future_import(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::ImportFrom {
            module: Some(module),
            level: 0,
            ..
        } if module == FUTURE_MODULE_NAME
    )
}

/// Scans the leading statements of a module the way CPython's future parser
/// does: an optional docstring followed by consecutive future imports.
pub(crate) fn scan_future_statements(program: &Program) -> Result<FutureStatements, String> {
    let statements = &program.statements;
    let mut index = usize::from(matches!(
        statements.first(),
        Some(Stmt::Expr(Expr::String(_)))
    ));
    let mut flags = 0;
    while let Some(stmt) = statements.get(index) {
        if !is_future_import(stmt) {
            break;
        }
        let Stmt::ImportFrom { targets, .. } = stmt else {
            unreachable!("future imports are from-imports");
        };
        flags |= future_statement_flags(targets)?;
        index += 1;
    }
    Ok(FutureStatements {
        flags,
        prefix_len: index,
    })
}

/// Effective future flags for code compiled from `program` with `inherited`
/// flags. Invalid future statements contribute nothing; compiling the program
/// reports their error.
pub(crate) fn program_future_flags(program: &Program, inherited: i64) -> i64 {
    let declared = scan_future_statements(program)
        .map(|statements| statements.flags)
        .unwrap_or(0);
    (inherited & PYCF_MASK) | declared
}

fn future_statement_flags(targets: &ImportFromTargets) -> Result<i64, String> {
    let aliases = match targets {
        ImportFromTargets::Star => return Err(undefined_feature_error("*")),
        ImportFromTargets::Aliases(aliases) => aliases,
    };
    let mut flags = 0;
    for alias in aliases {
        match alias.name.as_str() {
            "braces" => return Err(FUTURE_BRACES_ERROR.to_string()),
            // Only features that change compilation record a flag; the
            // mandatory ones are accepted as no-ops.
            "annotations" => flags |= CO_FUTURE_ANNOTATIONS,
            "barry_as_FLUFL" => flags |= CO_FUTURE_BARRY_AS_BDFL,
            name if FUTURE_FEATURES.iter().any(|feature| feature.name == name) => {}
            name => return Err(undefined_feature_error(name)),
        }
    }
    Ok(flags)
}

fn undefined_feature_error(name: &str) -> String {
    format!("future feature {name} is not defined")
}

pub(crate) fn is_future_feature_error(message: &str) -> Option<&str> {
    if message == FUTURE_BRACES_ERROR {
        return Some("braces");
    }
    message
        .strip_prefix("future feature ")
        .and_then(|rest| rest.strip_suffix(" is not defined"))
}

/// Validates the expressions that PEP 563 stores as strings. They are never
/// compiled, so the restrictions CPython applies to annotation scopes must be
/// checked here: no `yield`, `await`, or named expressions directly in the
/// annotation, and no asynchronous comprehensions.
pub(crate) fn validate_string_annotations(program: &Program) -> Result<(), String> {
    validate_statements(&program.statements)
}

fn validate_statements(statements: &[Stmt]) -> Result<(), String> {
    for stmt in statements {
        validate_statement(stmt)?;
    }
    Ok(())
}

fn validate_statement(stmt: &Stmt) -> Result<(), String> {
    match stmt {
        Stmt::AnnAssign { annotation, .. } => validate_annotation(annotation),
        Stmt::FunctionDef {
            params,
            body,
            returns,
            ..
        }
        | Stmt::AsyncFunctionDef {
            params,
            body,
            returns,
            ..
        } => {
            validate_parameter_annotations(params)?;
            if let Some(returns) = returns {
                validate_annotation(returns)?;
            }
            validate_statements(body)
        }
        Stmt::ClassDef { body, .. } => validate_statements(body),
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            validate_statements(then_body)?;
            validate_statements(else_body)
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
            validate_statements(body)?;
            validate_statements(else_body)
        }
        Stmt::With { body, .. } | Stmt::AsyncWith { body, .. } => validate_statements(body),
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
            validate_statements(body)?;
            for handler in handlers {
                validate_statements(&handler.body)?;
            }
            validate_statements(else_body)?;
            validate_statements(finally_body)
        }
        Stmt::Match { cases, .. } => {
            for case in cases {
                validate_statements(&case.body)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn validate_parameter_annotations(params: &FunctionParams) -> Result<(), String> {
    // CPython visits positional parameters before positional-only ones.
    for param in params
        .positional
        .iter()
        .chain(params.positional_only.iter())
    {
        if let Some(annotation) = &param.annotation {
            validate_annotation(annotation)?;
        }
    }
    if let Some(annotation) = params.vararg_annotation.as_deref() {
        validate_annotation(annotation)?;
    }
    for param in &params.keyword_only {
        if let Some(annotation) = &param.annotation {
            validate_annotation(annotation)?;
        }
    }
    if let Some(annotation) = params.kwarg_annotation.as_deref() {
        validate_annotation(annotation)?;
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AnnotationScope {
    Annotation,
    Lambda,
    Comprehension(&'static str),
}

fn validate_annotation(annotation: &Expr) -> Result<(), String> {
    validate_annotation_expr(annotation, AnnotationScope::Annotation)
}

fn validate_annotation_expr(expr: &Expr, scope: AnnotationScope) -> Result<(), String> {
    match expr {
        Expr::Yield { value } => {
            yield_allowed(scope)?;
            if let Some(value) = value {
                validate_annotation_expr(value, scope)?;
            }
            Ok(())
        }
        Expr::YieldFrom(value) => {
            yield_allowed(scope)?;
            validate_annotation_expr(value, scope)
        }
        Expr::Await(_) => Err(match scope {
            AnnotationScope::Annotation => "await expression cannot be used within an annotation",
            AnnotationScope::Lambda => "'await' outside async function",
            AnnotationScope::Comprehension(_) => {
                "asynchronous comprehension outside of an asynchronous function"
            }
        }
        .to_string()),
        Expr::NamedExpr { value, .. } => {
            if scope == AnnotationScope::Annotation {
                return Err("named expression cannot be used within an annotation".to_string());
            }
            validate_annotation_expr(value, scope)
        }
        Expr::Lambda { params, body } => {
            for param in params
                .positional_only
                .iter()
                .chain(params.positional.iter())
                .chain(params.keyword_only.iter())
            {
                if let Some(default) = &param.default {
                    validate_annotation_expr(default, scope)?;
                }
            }
            validate_annotation_expr(body, AnnotationScope::Lambda)
        }
        Expr::ListComp { element, clauses } => {
            validate_comprehension(&[element], clauses, scope, "list comprehension")
        }
        Expr::SetComp { element, clauses } => {
            validate_comprehension(&[element], clauses, scope, "set comprehension")
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => validate_comprehension(&[key, value], clauses, scope, "dict comprehension"),
        Expr::GeneratorComp { element, clauses } => {
            validate_comprehension(&[element], clauses, scope, "generator expression")
        }
        _ => {
            let mut result = Ok(());
            for_each_child_expr(expr, &mut |child| {
                if result.is_ok() {
                    result = validate_annotation_expr(child, scope);
                }
            });
            result
        }
    }
}

fn yield_allowed(scope: AnnotationScope) -> Result<(), String> {
    match scope {
        AnnotationScope::Annotation => {
            Err("yield expression cannot be used within an annotation".to_string())
        }
        AnnotationScope::Lambda => Ok(()),
        AnnotationScope::Comprehension(kind) => Err(format!("'yield' inside {kind}")),
    }
}

fn validate_comprehension(
    elements: &[&Expr],
    clauses: &[ComprehensionClause],
    scope: AnnotationScope,
    kind: &'static str,
) -> Result<(), String> {
    let inner = AnnotationScope::Comprehension(kind);
    for (index, clause) in clauses.iter().enumerate() {
        if clause.is_async && kind != "generator expression" {
            return Err(
                "asynchronous comprehension outside of an asynchronous function".to_string(),
            );
        }
        // The first iterable is evaluated in the enclosing scope.
        validate_annotation_expr(&clause.iter, if index == 0 { scope } else { inner })?;
        for condition in &clause.ifs {
            validate_annotation_expr(condition, inner)?;
        }
    }
    for element in elements {
        validate_annotation_expr(element, inner)?;
    }
    Ok(())
}

fn for_each_child_expr(expr: &Expr, visit: &mut dyn FnMut(&Expr)) {
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
        Expr::JoinedString(parts) => for_each_f_string_child(parts, visit),
        Expr::TemplateString(parts) => {
            for part in parts {
                if let TemplateStringPart::Interpolation {
                    value, format_spec, ..
                } = part
                {
                    visit(value);
                    if let Some(format_spec) = format_spec {
                        for_each_f_string_child(format_spec, visit);
                    }
                }
            }
        }
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            visit(value);
            if let Some(format_spec) = format_spec {
                for_each_f_string_child(format_spec, visit);
            }
        }
        Expr::Attribute { object, .. } => visit(object),
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            visit(left);
            visit(right);
        }
        Expr::ChainedComparison { left, comparisons } => {
            visit(left);
            for (_, comparator) in comparisons {
                visit(comparator);
            }
        }
        Expr::Unary { operand, .. } => visit(operand),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            visit(then_branch);
            visit(condition);
            visit(else_branch);
        }
        Expr::NamedExpr { value, .. } => visit(value),
        Expr::Yield { value } => {
            if let Some(value) = value {
                visit(value);
            }
        }
        Expr::YieldFrom(value) | Expr::Await(value) | Expr::Starred(value) => visit(value),
        Expr::List(items) | Expr::Set(items) | Expr::FrozenSet(items) | Expr::Tuple(items) => {
            for item in items {
                visit(item);
            }
        }
        Expr::Dict(items) => {
            for item in items {
                match item {
                    DictItem::Entry { key, value } => {
                        visit(key);
                        visit(value);
                    }
                    DictItem::Unpack(value) => visit(value),
                }
            }
        }
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            visit(element);
            for_each_clause_child(clauses, visit);
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            visit(key);
            visit(value);
            for_each_clause_child(clauses, visit);
        }
        Expr::Subscript { object, index } => {
            visit(object);
            visit(index);
        }
        Expr::SliceLiteral { start, stop, step } => {
            for part in [start, stop, step].into_iter().flatten() {
                visit(part);
            }
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            visit(object);
            for part in [start, stop, step].into_iter().flatten() {
                visit(part);
            }
        }
        Expr::Call { callee, args } => {
            visit(callee);
            for arg in args {
                visit(arg);
            }
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            visit(callee);
            for arg in args {
                visit(arg);
            }
            for (_, value) in keywords {
                visit(value);
            }
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            visit(callee);
            for arg in args {
                match arg {
                    crate::ast::CallArg::Expr(value) | crate::ast::CallArg::Unpack(value) => {
                        visit(value)
                    }
                }
            }
            for keyword in keywords {
                match keyword {
                    crate::ast::CallKeyword::Named(_, value)
                    | crate::ast::CallKeyword::Unpack(value) => visit(value),
                }
            }
        }
        Expr::Lambda { params, body } => {
            for param in params
                .positional_only
                .iter()
                .chain(params.positional.iter())
                .chain(params.keyword_only.iter())
            {
                if let Some(default) = &param.default {
                    visit(default);
                }
            }
            visit(body);
        }
    }
}

fn for_each_f_string_child(parts: &[FStringPart], visit: &mut dyn FnMut(&Expr)) {
    for part in parts {
        if let FStringPart::Formatted {
            value, format_spec, ..
        } = part
        {
            visit(value);
            if let Some(format_spec) = format_spec {
                for_each_f_string_child(format_spec, visit);
            }
        }
    }
}

fn for_each_clause_child(clauses: &[ComprehensionClause], visit: &mut dyn FnMut(&Expr)) {
    for clause in clauses {
        visit(&clause.iter);
        for condition in &clause.ifs {
            visit(condition);
        }
    }
}

/// True when a statement list (not descending into nested scopes) contains an
/// annotated assignment, which makes CPython create `__annotations__` up front
/// under `from __future__ import annotations`.
pub(crate) fn body_has_annotated_assignment(statements: &[Stmt]) -> bool {
    statements.iter().any(|stmt| match stmt {
        Stmt::AnnAssign { .. } => true,
        Stmt::If {
            then_body,
            else_body,
            ..
        } => body_has_annotated_assignment(then_body) || body_has_annotated_assignment(else_body),
        Stmt::While {
            body, else_body, ..
        }
        | Stmt::For {
            body, else_body, ..
        }
        | Stmt::AsyncFor {
            body, else_body, ..
        } => body_has_annotated_assignment(body) || body_has_annotated_assignment(else_body),
        Stmt::With { body, .. } | Stmt::AsyncWith { body, .. } => {
            body_has_annotated_assignment(body)
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
            body_has_annotated_assignment(body)
                || handlers
                    .iter()
                    .any(|handler| body_has_annotated_assignment(&handler.body))
                || body_has_annotated_assignment(else_body)
                || body_has_annotated_assignment(finally_body)
        }
        Stmt::Match { cases, .. } => cases
            .iter()
            .any(|case| body_has_annotated_assignment(&case.body)),
        _ => false,
    })
}
