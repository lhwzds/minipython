use crate::ast::{
    BinaryOp, CallArg, CallKeyword, ComparisonOp, ComprehensionClause, DictItem,
    ExceptHandler as AstExceptHandler, Expr, FStringConversion, FStringPart, FunctionParams,
    ImportAlias, ImportFromTargets, LogicalOp, MatchCase, Param, Pattern, Program, Stmt, Target,
    TemplateStringPart, TypeParam, TypeParamKind, UnaryOp, WithItem,
};
use crate::bytecode::{
    CallArgRegister, CallKeywordRegister, ExceptHandler as BytecodeExceptHandler,
    ExceptHandlerNameBinding, FormatConversion, Instruction, Register, TemplatePartRegister,
};
use crate::value::{Value, bytes_value, complex_value, float_value, tuple_value};
use num_bigint::BigInt;
use std::cell::RefCell;
use std::collections::{BTreeSet, HashSet, VecDeque};
use std::rc::Rc;

const MAX_STATIC_BLOCK_DEPTH: usize = 21;

#[derive(Clone, Debug)]
pub struct CompileOptions {
    pub optimize: i64,
    pub allow_top_level_await: bool,
    pub function_first_lines: Vec<usize>,
    pub function_line_sequences: Vec<Vec<usize>>,
    pub function_position_columns: Vec<Vec<Option<(usize, usize)>>>,
    pub function_is_lambdas: Vec<bool>,
    pub generator_expression_line_sequences: Vec<(usize, Vec<usize>)>,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            optimize: -1,
            allow_top_level_await: false,
            function_first_lines: Vec::new(),
            function_line_sequences: Vec::new(),
            function_position_columns: Vec::new(),
            function_is_lambdas: Vec::new(),
            generator_expression_line_sequences: Vec::new(),
        }
    }
}

impl CompileOptions {
    pub fn optimized(optimize: i64) -> Self {
        Self {
            optimize,
            ..Self::default()
        }
    }

    pub fn with_function_first_lines(mut self, lines: Vec<usize>) -> Self {
        self.function_first_lines = lines;
        self
    }

    pub fn with_function_line_sequences(mut self, sequences: Vec<Vec<usize>>) -> Self {
        self.function_line_sequences = sequences;
        self
    }

    pub fn with_function_position_columns(
        mut self,
        columns: Vec<Vec<Option<(usize, usize)>>>,
    ) -> Self {
        self.function_position_columns = columns;
        self
    }

    pub fn with_function_is_lambdas(mut self, is_lambdas: Vec<bool>) -> Self {
        self.function_is_lambdas = is_lambdas;
        self
    }

    pub fn with_generator_expression_line_sequences(
        mut self,
        sequences: Vec<(usize, Vec<usize>)>,
    ) -> Self {
        self.generator_expression_line_sequences = sequences;
        self
    }

    pub fn with_allow_top_level_await(mut self, allow: bool) -> Self {
        self.allow_top_level_await = allow;
        self
    }
}

pub fn compile(program: &Program) -> Result<Vec<Instruction>, String> {
    compile_with_options(program, CompileOptions::default())
}

pub fn compile_with_options(
    program: &Program,
    options: CompileOptions,
) -> Result<Vec<Instruction>, String> {
    validate_lazy_imports(program)?;

    let mut compiler = Compiler::new_root(options);
    validate_module_scope_declarations(&program.statements)?;

    let skip_module_docstring = matches!(
        program.statements.first(),
        Some(Stmt::Expr(Expr::String(_)))
    );
    if let Some(docstring) = compiler.statement_docstring(program.statements.first()) {
        let dst = compiler.alloc_register();
        compiler.instructions.push(Instruction::LoadConst {
            dst,
            value: Value::String(docstring),
        });
        compiler.emit_store_name("__doc__", dst);
    }

    for (index, stmt) in program.statements.iter().enumerate() {
        if index == 0 && skip_module_docstring {
            continue;
        }
        compiler.compile_stmt(stmt)?;
    }
    compiler.instructions.push(Instruction::Halt);

    Ok(compiler.instructions)
}

#[allow(dead_code)]
pub fn compile_interactive(program: &Program) -> Result<Vec<Instruction>, String> {
    compile_interactive_with_options(program, CompileOptions::default())
}

pub fn compile_interactive_with_options(
    program: &Program,
    options: CompileOptions,
) -> Result<Vec<Instruction>, String> {
    validate_lazy_imports(program)?;

    let mut compiler = Compiler::new_root(options);
    validate_module_scope_declarations(&program.statements)?;

    for stmt in &program.statements {
        match stmt {
            Stmt::Expr(expr) => {
                let src = compiler.compile_expr(expr)?;
                compiler.instructions.push(Instruction::Display { src });
            }
            stmt => compiler.compile_stmt(stmt)?,
        }
    }
    compiler.instructions.push(Instruction::Halt);

    Ok(compiler.instructions)
}

pub fn compile_eval(expr: &Expr) -> Result<Vec<Instruction>, String> {
    compile_eval_with_options(expr, CompileOptions::default())
}

pub fn compile_eval_with_options(
    expr: &Expr,
    options: CompileOptions,
) -> Result<Vec<Instruction>, String> {
    let mut compiler = Compiler::new_root(options);

    let src = compiler.compile_expr(expr)?;
    compiler
        .instructions
        .push(Instruction::Return { src: Some(src) });

    Ok(compiler.instructions)
}

pub fn program_contains_top_level_await(program: &Program) -> bool {
    statements_contain_top_level_await(&program.statements)
}

pub fn expr_contains_top_level_await(expr: &Expr) -> bool {
    top_level_expr_contains_await(expr)
}

fn statements_contain_top_level_await(statements: &[Stmt]) -> bool {
    statements.iter().any(stmt_contains_top_level_await)
}

fn stmt_contains_top_level_await(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Pass
        | Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Break
        | Stmt::Continue => false,
        Stmt::Expr(expr) => top_level_expr_contains_await(expr),
        Stmt::Assign { targets, value, .. } => {
            top_level_expr_contains_await(value)
                || targets.iter().any(target_contains_top_level_await)
        }
        Stmt::AnnAssign {
            target,
            annotation,
            value,
            ..
        } => {
            target_contains_top_level_await(target)
                || top_level_expr_contains_await(annotation)
                || value.as_ref().is_some_and(top_level_expr_contains_await)
        }
        Stmt::TypeAlias {
            type_params, value, ..
        } => {
            type_params.iter().any(type_param_contains_top_level_await)
                || top_level_expr_contains_await(value)
        }
        Stmt::AugAssign { target, value, .. } => {
            target_contains_top_level_await(target) || top_level_expr_contains_await(value)
        }
        Stmt::Delete { target } => target_contains_top_level_await(target),
        Stmt::FunctionDef {
            type_params,
            params,
            decorators,
            returns,
            ..
        }
        | Stmt::AsyncFunctionDef {
            type_params,
            params,
            decorators,
            returns,
            ..
        } => {
            type_params.iter().any(type_param_contains_top_level_await)
                || params_contains_top_level_await(params)
                || decorators.iter().any(top_level_expr_contains_await)
                || returns.as_ref().is_some_and(top_level_expr_contains_await)
        }
        Stmt::ClassDef {
            type_params,
            bases,
            keywords,
            decorators,
            ..
        } => {
            type_params.iter().any(type_param_contains_top_level_await)
                || bases.iter().any(call_arg_contains_top_level_await)
                || keywords.iter().any(call_keyword_contains_top_level_await)
                || decorators.iter().any(top_level_expr_contains_await)
        }
        Stmt::Return(value) => value.as_ref().is_some_and(top_level_expr_contains_await),
        Stmt::Assert { condition, message } => {
            top_level_expr_contains_await(condition)
                || message.as_ref().is_some_and(top_level_expr_contains_await)
        }
        Stmt::Raise { value, cause } => {
            value.as_ref().is_some_and(top_level_expr_contains_await)
                || cause.as_ref().is_some_and(top_level_expr_contains_await)
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            top_level_expr_contains_await(condition)
                || statements_contain_top_level_await(then_body)
                || statements_contain_top_level_await(else_body)
        }
        Stmt::Match { subject, cases } => {
            top_level_expr_contains_await(subject)
                || cases.iter().any(match_case_contains_top_level_await)
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
            statements_contain_top_level_await(body)
                || handlers.iter().any(except_handler_contains_top_level_await)
                || statements_contain_top_level_await(else_body)
                || statements_contain_top_level_await(finally_body)
        }
        Stmt::With { items, body, .. } => {
            items.iter().any(with_item_contains_top_level_await)
                || statements_contain_top_level_await(body)
        }
        Stmt::AsyncWith { .. } | Stmt::AsyncFor { .. } => true,
        Stmt::While {
            condition,
            body,
            else_body,
        } => {
            top_level_expr_contains_await(condition)
                || statements_contain_top_level_await(body)
                || statements_contain_top_level_await(else_body)
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            target_contains_top_level_await(target)
                || top_level_expr_contains_await(iter)
                || statements_contain_top_level_await(body)
                || statements_contain_top_level_await(else_body)
        }
    }
}

fn top_level_expr_contains_await(expr: &Expr) -> bool {
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
        | Expr::Name(_) => false,
        Expr::JoinedString(parts) => parts.iter().any(fstring_part_contains_top_level_await),
        Expr::TemplateString(parts) => parts.iter().any(template_part_contains_top_level_await),
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            top_level_expr_contains_await(value)
                || format_spec
                    .as_deref()
                    .is_some_and(|parts| parts.iter().any(fstring_part_contains_top_level_await))
        }
        Expr::Attribute { object, .. }
        | Expr::Unary {
            operand: object, ..
        }
        | Expr::YieldFrom(object)
        | Expr::Starred(object)
        | Expr::Subscript { object, .. } => top_level_expr_contains_await(object),
        Expr::Await(_) => true,
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            top_level_expr_contains_await(left) || top_level_expr_contains_await(right)
        }
        Expr::ChainedComparison { left, comparisons } => {
            top_level_expr_contains_await(left)
                || comparisons
                    .iter()
                    .any(|(_, expr)| top_level_expr_contains_await(expr))
        }
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            top_level_expr_contains_await(condition)
                || top_level_expr_contains_await(then_branch)
                || top_level_expr_contains_await(else_branch)
        }
        Expr::NamedExpr { value, .. } => top_level_expr_contains_await(value),
        Expr::Yield { value } => value.as_deref().is_some_and(top_level_expr_contains_await),
        Expr::List(items) | Expr::Set(items) | Expr::FrozenSet(items) | Expr::Tuple(items) => {
            items.iter().any(top_level_expr_contains_await)
        }
        Expr::ListComp { element, clauses } | Expr::SetComp { element, clauses } => {
            clauses.iter().any(|clause| clause.is_async)
                || top_level_expr_contains_await(element)
                || clauses
                    .iter()
                    .any(comprehension_clause_contains_top_level_await)
        }
        Expr::GeneratorComp { clauses, .. } => clauses
            .first()
            .is_some_and(|clause| top_level_expr_contains_await(&clause.iter)),
        Expr::Dict(entries) => entries.iter().any(dict_item_contains_top_level_await),
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            clauses.iter().any(|clause| clause.is_async)
                || top_level_expr_contains_await(key)
                || top_level_expr_contains_await(value)
                || clauses
                    .iter()
                    .any(comprehension_clause_contains_top_level_await)
        }
        Expr::DictUnpackComp { value, clauses } => {
            clauses.iter().any(|clause| clause.is_async)
                || top_level_expr_contains_await(value)
                || clauses
                    .iter()
                    .any(comprehension_clause_contains_top_level_await)
        }
        Expr::SliceLiteral { start, stop, step } => {
            start.as_deref().is_some_and(top_level_expr_contains_await)
                || stop.as_deref().is_some_and(top_level_expr_contains_await)
                || step.as_deref().is_some_and(top_level_expr_contains_await)
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            top_level_expr_contains_await(object)
                || start.as_deref().is_some_and(top_level_expr_contains_await)
                || stop.as_deref().is_some_and(top_level_expr_contains_await)
                || step.as_deref().is_some_and(top_level_expr_contains_await)
        }
        Expr::Call { callee, args } => {
            top_level_expr_contains_await(callee) || args.iter().any(top_level_expr_contains_await)
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            top_level_expr_contains_await(callee)
                || args.iter().any(top_level_expr_contains_await)
                || keywords
                    .iter()
                    .any(|(_, value)| top_level_expr_contains_await(value))
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            top_level_expr_contains_await(callee)
                || args.iter().any(call_arg_contains_top_level_await)
                || keywords.iter().any(call_keyword_contains_top_level_await)
        }
        Expr::Lambda { params, .. } => params_contains_top_level_await(params),
    }
}

fn target_contains_top_level_await(target: &Target) -> bool {
    match target {
        Target::Name(_) => false,
        Target::Attribute { object, .. } => top_level_expr_contains_await(object),
        Target::Subscript { object, index } => {
            top_level_expr_contains_await(object) || top_level_expr_contains_await(index)
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            top_level_expr_contains_await(object)
                || start.as_ref().is_some_and(top_level_expr_contains_await)
                || stop.as_ref().is_some_and(top_level_expr_contains_await)
                || step.as_ref().is_some_and(top_level_expr_contains_await)
        }
        Target::Starred(target) => target_contains_top_level_await(target),
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(target_contains_top_level_await)
        }
    }
}

fn comprehension_clause_contains_top_level_await(clause: &ComprehensionClause) -> bool {
    target_contains_top_level_await(&clause.target)
        || top_level_expr_contains_await(&clause.iter)
        || clause.ifs.iter().any(top_level_expr_contains_await)
}

fn params_contains_top_level_await(params: &FunctionParams) -> bool {
    params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
        .any(param_contains_top_level_await)
        || params
            .vararg_annotation
            .as_deref()
            .is_some_and(top_level_expr_contains_await)
        || params
            .kwarg_annotation
            .as_deref()
            .is_some_and(top_level_expr_contains_await)
}

fn param_contains_top_level_await(param: &Param) -> bool {
    param
        .annotation
        .as_ref()
        .is_some_and(top_level_expr_contains_await)
        || param
            .default
            .as_ref()
            .is_some_and(top_level_expr_contains_await)
}

fn type_param_contains_top_level_await(type_param: &TypeParam) -> bool {
    type_param
        .bound
        .as_ref()
        .is_some_and(top_level_expr_contains_await)
        || type_param
            .default
            .as_ref()
            .is_some_and(top_level_expr_contains_await)
}

fn call_arg_contains_top_level_await(arg: &CallArg) -> bool {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => top_level_expr_contains_await(expr),
    }
}

fn call_keyword_contains_top_level_await(keyword: &CallKeyword) -> bool {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
            top_level_expr_contains_await(expr)
        }
    }
}

fn dict_item_contains_top_level_await(item: &DictItem) -> bool {
    match item {
        DictItem::Entry { key, value } => {
            top_level_expr_contains_await(key) || top_level_expr_contains_await(value)
        }
        DictItem::Unpack(expr) => top_level_expr_contains_await(expr),
    }
}

fn with_item_contains_top_level_await(item: &WithItem) -> bool {
    top_level_expr_contains_await(&item.context_expr)
        || item
            .optional_vars
            .as_ref()
            .is_some_and(target_contains_top_level_await)
}

fn except_handler_contains_top_level_await(handler: &AstExceptHandler) -> bool {
    handler
        .type_expr
        .as_ref()
        .is_some_and(top_level_expr_contains_await)
        || statements_contain_top_level_await(&handler.body)
}

fn match_case_contains_top_level_await(case: &MatchCase) -> bool {
    pattern_contains_top_level_await(&case.pattern)
        || case
            .guard
            .as_ref()
            .is_some_and(top_level_expr_contains_await)
        || statements_contain_top_level_await(&case.body)
}

fn pattern_contains_top_level_await(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Literal(expr) | Pattern::Singleton(expr) | Pattern::Value(expr) => {
            top_level_expr_contains_await(expr)
        }
        Pattern::Capture(_) | Pattern::Wildcard | Pattern::Star(_) => false,
        Pattern::Or(patterns) | Pattern::Sequence(patterns) => {
            patterns.iter().any(pattern_contains_top_level_await)
        }
        Pattern::Mapping { entries, .. } => entries.iter().any(|(key, pattern)| {
            top_level_expr_contains_await(key) || pattern_contains_top_level_await(pattern)
        }),
        Pattern::Class {
            class,
            positional,
            keywords,
        } => {
            top_level_expr_contains_await(class)
                || positional.iter().any(pattern_contains_top_level_await)
                || keywords
                    .iter()
                    .any(|(_, pattern)| pattern_contains_top_level_await(pattern))
        }
        Pattern::As { pattern, .. } => pattern_contains_top_level_await(pattern),
    }
}

fn fstring_part_contains_top_level_await(part: &FStringPart) -> bool {
    match part {
        FStringPart::Literal(_) => false,
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            top_level_expr_contains_await(value)
                || format_spec
                    .as_ref()
                    .is_some_and(|parts| parts.iter().any(fstring_part_contains_top_level_await))
        }
    }
}

fn template_part_contains_top_level_await(part: &TemplateStringPart) -> bool {
    match part {
        TemplateStringPart::Literal(_) => false,
        TemplateStringPart::Interpolation {
            value, format_spec, ..
        } => {
            top_level_expr_contains_await(value)
                || format_spec
                    .as_ref()
                    .is_some_and(|parts| parts.iter().any(fstring_part_contains_top_level_await))
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LazyImportContext {
    Module,
    Function,
    Class,
    Try,
}

fn validate_lazy_imports(program: &Program) -> Result<(), String> {
    validate_lazy_imports_in_statements(&program.statements, LazyImportContext::Module)
}

fn validate_lazy_imports_in_statements(
    statements: &[Stmt],
    context: LazyImportContext,
) -> Result<(), String> {
    for stmt in statements {
        validate_lazy_import_in_stmt(stmt, context)?;
    }

    Ok(())
}

fn validate_lazy_import_in_stmt(stmt: &Stmt, context: LazyImportContext) -> Result<(), String> {
    match stmt {
        Stmt::Import { is_lazy, .. } => {
            if *is_lazy {
                validate_lazy_import_context(context, false)?;
            }
        }
        Stmt::ImportFrom {
            is_lazy, targets, ..
        } => {
            if *is_lazy {
                validate_lazy_import_context(context, true)?;
                if matches!(targets, ImportFromTargets::Star) {
                    return Err("lazy from ... import * is not allowed".to_string());
                }
            }
        }
        Stmt::FunctionDef { body, .. } | Stmt::AsyncFunctionDef { body, .. } => {
            validate_lazy_imports_in_statements(body, LazyImportContext::Function)?;
        }
        Stmt::ClassDef { body, .. } => {
            validate_lazy_imports_in_statements(body, LazyImportContext::Class)?;
        }
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            validate_lazy_imports_in_statements(then_body, context)?;
            validate_lazy_imports_in_statements(else_body, context)?;
        }
        Stmt::Match { cases, .. } => {
            for case in cases {
                validate_lazy_imports_in_statements(&case.body, context)?;
            }
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
            validate_lazy_imports_in_statements(body, context)?;
            validate_lazy_imports_in_statements(else_body, context)?;
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
            validate_lazy_imports_in_statements(body, LazyImportContext::Try)?;
            for handler in handlers {
                validate_lazy_imports_in_statements(&handler.body, LazyImportContext::Try)?;
            }
            validate_lazy_imports_in_statements(else_body, LazyImportContext::Try)?;
            validate_lazy_imports_in_statements(finally_body, LazyImportContext::Try)?;
        }
        Stmt::With { body, .. } | Stmt::AsyncWith { body, .. } => {
            validate_lazy_imports_in_statements(body, context)?;
        }
        Stmt::Pass
        | Stmt::Expr(_)
        | Stmt::Assign { .. }
        | Stmt::AnnAssign { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::AugAssign { .. }
        | Stmt::Delete { .. }
        | Stmt::Return(_)
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Assert { .. }
        | Stmt::Raise { .. }
        | Stmt::Break
        | Stmt::Continue => {}
    }

    Ok(())
}

fn validate_lazy_import_context(
    context: LazyImportContext,
    is_from_import: bool,
) -> Result<(), String> {
    let prefix = if is_from_import {
        "lazy from ... import"
    } else {
        "lazy import"
    };

    match context {
        LazyImportContext::Module => Ok(()),
        LazyImportContext::Function => Err(format!("{prefix} not allowed inside functions")),
        LazyImportContext::Class => Err(format!("{prefix} not allowed inside classes")),
        LazyImportContext::Try => Err(format!("{prefix} not allowed inside try/except blocks")),
    }
}

struct Compiler {
    instructions: Vec<Instruction>,
    next_register: Register,
    loop_contexts: Vec<LoopContext>,
    finally_contexts: Vec<FinallyContext>,
    function_depth: usize,
    async_function_depth: usize,
    global_names: HashSet<String>,
    nonlocal_names: HashSet<String>,
    local_names: HashSet<String>,
    private_class_name: Option<String>,
    class_scope_all_bindings: HashSet<String>,
    class_scope_prior_bindings: HashSet<String>,
    enclosing_function_bindings: Vec<HashSet<String>>,
    outer_scope_store_names: HashSet<String>,
    comprehension_walrus_enclosing_function_depth: Option<usize>,
    static_block_depth: usize,
    optimize: i64,
    function_first_lines: Rc<RefCell<VecDeque<usize>>>,
    function_line_sequences: Rc<RefCell<VecDeque<Vec<usize>>>>,
    function_position_columns: Rc<RefCell<VecDeque<Vec<Option<(usize, usize)>>>>>,
    function_is_lambdas: Rc<RefCell<VecDeque<bool>>>,
    generator_expression_line_sequences: Rc<RefCell<VecDeque<(usize, Vec<usize>)>>>,
}

struct LoopContext {
    continue_target: usize,
    break_jumps: Vec<usize>,
}

#[derive(Clone, Copy)]
struct LogicalJump {
    instruction: usize,
    src: Register,
}

#[derive(Clone)]
struct FinallyContext {
    prelude: Vec<Instruction>,
    body: Vec<Stmt>,
    trailer: Vec<Instruction>,
}

#[derive(Clone, Copy)]
enum WithExitArgs {
    NoException,
    CurrentException,
}

struct MatchPatternCode {
    failure_jumps: Vec<usize>,
    bindings: Vec<MatchBinding>,
}

struct MatchBinding {
    name: String,
    src: Register,
}

#[derive(Clone, Copy)]
enum TypeParamEvaluation<'a> {
    Deferred { class_name: Option<&'a str> },
}

impl Compiler {
    fn new_root(options: CompileOptions) -> Self {
        Self {
            instructions: Vec::new(),
            next_register: 0,
            loop_contexts: Vec::new(),
            finally_contexts: Vec::new(),
            function_depth: 0,
            async_function_depth: usize::from(options.allow_top_level_await),
            global_names: HashSet::new(),
            nonlocal_names: HashSet::new(),
            local_names: HashSet::new(),
            private_class_name: None,
            class_scope_all_bindings: HashSet::new(),
            class_scope_prior_bindings: HashSet::new(),
            enclosing_function_bindings: Vec::new(),
            outer_scope_store_names: HashSet::new(),
            comprehension_walrus_enclosing_function_depth: None,
            static_block_depth: 0,
            optimize: options.optimize,
            function_first_lines: Rc::new(RefCell::new(VecDeque::from(
                options.function_first_lines,
            ))),
            function_line_sequences: Rc::new(RefCell::new(VecDeque::from(
                options.function_line_sequences,
            ))),
            function_position_columns: Rc::new(RefCell::new(VecDeque::from(
                options.function_position_columns,
            ))),
            function_is_lambdas: Rc::new(RefCell::new(VecDeque::from(options.function_is_lambdas))),
            generator_expression_line_sequences: Rc::new(RefCell::new(VecDeque::from(
                options.generator_expression_line_sequences,
            ))),
        }
    }

    fn statement_docstring(&self, stmt: Option<&Stmt>) -> Option<String> {
        if self.optimize >= 2 {
            return None;
        }

        let Some(Stmt::Expr(Expr::String(value))) = stmt else {
            return None;
        };
        Some(value.clone())
    }

    fn should_skip_optimized_docstring(&self, index: usize, stmt: &Stmt) -> bool {
        self.optimize >= 2 && index == 0 && matches!(stmt, Stmt::Expr(Expr::String(_)))
    }

    fn enter_static_blocks(&mut self, count: usize) -> Result<(), String> {
        if self.static_block_depth + count > MAX_STATIC_BLOCK_DEPTH {
            return Err("too many statically nested blocks".to_string());
        }

        self.static_block_depth += count;
        Ok(())
    }

    fn leave_static_blocks(&mut self, count: usize) {
        self.static_block_depth = self.static_block_depth.saturating_sub(count);
    }

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Pass => {
                self.instructions.push(Instruction::Noop);
                Ok(())
            }
            Stmt::Expr(expr) => {
                let src = self.compile_expr(expr)?;
                self.instructions.push(Instruction::Pop { src });
                Ok(())
            }
            Stmt::Assign { targets, value, .. } => {
                let src = self.compile_expr(value)?;
                for target in targets {
                    self.compile_store_target(target, src)?;
                }
                self.instructions.push(Instruction::Pop { src });
                Ok(())
            }
            Stmt::AnnAssign {
                target,
                annotation,
                value,
                simple,
            } => self.compile_ann_assign_stmt(target, annotation, value.as_ref(), *simple),
            Stmt::TypeAlias {
                name,
                type_params,
                value,
            } => self.compile_type_alias_stmt(name, type_params, value),
            Stmt::AugAssign { target, op, value } => {
                self.compile_aug_assign_stmt(target, op, value)
            }
            Stmt::Delete { target } => self.compile_delete_target(target),
            Stmt::FunctionDef {
                name,
                type_params,
                params,
                body,
                decorators,
                returns,
                ..
            } => self.compile_function_def_stmt(
                name,
                type_params,
                params,
                body,
                decorators,
                returns.as_ref(),
                false,
            ),
            Stmt::AsyncFunctionDef {
                name,
                type_params,
                params,
                body,
                decorators,
                returns,
                ..
            } => self.compile_function_def_stmt(
                name,
                type_params,
                params,
                body,
                decorators,
                returns.as_ref(),
                true,
            ),
            Stmt::ClassDef {
                name,
                type_params,
                bases,
                keywords,
                body,
                decorators,
                ..
            } => self.compile_class_def_stmt(name, type_params, bases, keywords, body, decorators),
            Stmt::Import { aliases, .. } => self.compile_import_stmt(aliases),
            Stmt::ImportFrom {
                module,
                level,
                targets,
                ..
            } => self.compile_import_from_stmt(module.as_deref(), *level, targets),
            Stmt::Return(value) => self.compile_return_stmt(value.as_ref()),
            Stmt::Raise { value, cause } => self.compile_raise_stmt(value.as_ref(), cause.as_ref()),
            Stmt::Global(names) => {
                self.global_names.extend(names.iter().cloned());
                Ok(())
            }
            Stmt::Nonlocal(names) => {
                if names.iter().any(|name| !self.nonlocal_names.contains(name))
                    && self.function_depth == 0
                {
                    return Err("nonlocal declaration not allowed at module level".to_string());
                }
                if names.iter().any(|name| !self.nonlocal_names.contains(name))
                    && self.function_depth == 1
                {
                    let name = names
                        .first()
                        .expect("nonlocal statement always has at least one name");
                    return Err(format!("no binding for nonlocal '{name}' found"));
                }

                self.nonlocal_names.extend(names.iter().cloned());
                Ok(())
            }
            Stmt::Assert { condition, message } => {
                self.compile_assert_stmt(condition, message.as_ref())
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => self.compile_if_stmt(condition, then_body, else_body),
            Stmt::Match { subject, cases } => self.compile_match_stmt(subject, cases),
            Stmt::Try {
                body,
                handlers,
                else_body,
                finally_body,
            } => self.compile_try_stmt(body, handlers, else_body, finally_body),
            Stmt::TryStar {
                body,
                handlers,
                else_body,
                finally_body,
            } => self.compile_try_star_stmt(body, handlers, else_body, finally_body),
            Stmt::With { items, body, .. } => self.compile_with_stmt(items, body),
            Stmt::AsyncWith { items, body, .. } => self.compile_async_with_stmt(items, body),
            Stmt::While {
                condition,
                body,
                else_body,
            } => self.compile_while_stmt(condition, body, else_body),
            Stmt::For {
                target,
                iter,
                body,
                else_body,
                ..
            } => self.compile_for_stmt(target, iter, body, else_body),
            Stmt::AsyncFor {
                target,
                iter,
                body,
                else_body,
                ..
            } => self.compile_async_for_stmt(target, iter, body, else_body),
            Stmt::Break => self.compile_break_stmt(),
            Stmt::Continue => self.compile_continue_stmt(),
        }
    }

    fn compile_if_stmt(
        &mut self,
        condition: &Expr,
        then_body: &[Stmt],
        else_body: &[Stmt],
    ) -> Result<(), String> {
        let condition = self.compile_expr(condition)?;
        let jump_if_false = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        });

        for stmt in then_body {
            self.compile_stmt(stmt)?;
        }

        if else_body.is_empty() {
            let end_target = self.instructions.len();
            self.patch_jump_target(jump_if_false, end_target)?;
            return Ok(());
        }

        let jump_over_else = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let else_target = self.instructions.len();
        self.patch_jump_target(jump_if_false, else_target)?;

        for stmt in else_body {
            self.compile_stmt(stmt)?;
        }

        let end_target = self.instructions.len();
        self.patch_jump_target(jump_over_else, end_target)?;

        Ok(())
    }

    fn compile_match_stmt(&mut self, subject: &Expr, cases: &[MatchCase]) -> Result<(), String> {
        let subject = self.compile_expr(subject)?;
        let mut end_jumps = Vec::new();

        for case in cases {
            let MatchPatternCode {
                mut failure_jumps,
                bindings,
            } = self.compile_match_pattern(subject, &case.pattern)?;

            self.emit_match_bindings(bindings);

            if let Some(guard) = &case.guard {
                let condition = self.compile_expr(guard)?;
                let jump = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse {
                    condition,
                    target: usize::MAX,
                });
                failure_jumps.push(jump);
            }

            for stmt in &case.body {
                self.compile_stmt(stmt)?;
            }

            let jump_to_end = self.instructions.len();
            self.instructions
                .push(Instruction::Jump { target: usize::MAX });
            end_jumps.push(jump_to_end);

            if failure_jumps.is_empty() {
                break;
            }

            for jump in failure_jumps {
                let next_case_target = self.instructions.len();
                self.patch_jump_target(jump, next_case_target)?;
            }
        }

        let end_target = self.instructions.len();
        for jump in end_jumps {
            self.patch_jump_target(jump, end_target)?;
        }

        Ok(())
    }

    fn compile_match_pattern(
        &mut self,
        subject: Register,
        pattern: &Pattern,
    ) -> Result<MatchPatternCode, String> {
        match pattern {
            Pattern::Literal(expected) => {
                if !is_match_literal_expr(expected) {
                    return Err(
                        "patterns may only match literals and attribute lookups".to_string()
                    );
                }
                let expected = self.compile_expr(expected)?;
                let condition = self.alloc_register();
                self.instructions.push(Instruction::Equal {
                    dst: condition,
                    left: subject,
                    right: expected,
                });
                let jump = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse {
                    condition,
                    target: usize::MAX,
                });
                Ok(MatchPatternCode {
                    failure_jumps: vec![jump],
                    bindings: Vec::new(),
                })
            }
            Pattern::Value(expected) => {
                let expected = self.compile_expr(expected)?;
                let condition = self.alloc_register();
                self.instructions.push(Instruction::Equal {
                    dst: condition,
                    left: subject,
                    right: expected,
                });
                let jump = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse {
                    condition,
                    target: usize::MAX,
                });
                Ok(MatchPatternCode {
                    failure_jumps: vec![jump],
                    bindings: Vec::new(),
                })
            }
            Pattern::Singleton(expected) => {
                let expected = self.compile_expr(expected)?;
                let condition = self.alloc_register();
                self.instructions.push(Instruction::Is {
                    dst: condition,
                    left: subject,
                    right: expected,
                });
                let jump = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse {
                    condition,
                    target: usize::MAX,
                });
                Ok(MatchPatternCode {
                    failure_jumps: vec![jump],
                    bindings: Vec::new(),
                })
            }
            Pattern::Capture(name) => Ok(MatchPatternCode {
                failure_jumps: Vec::new(),
                bindings: vec![MatchBinding {
                    name: name.clone(),
                    src: subject,
                }],
            }),
            Pattern::Wildcard => Ok(MatchPatternCode {
                failure_jumps: Vec::new(),
                bindings: Vec::new(),
            }),
            Pattern::Or(alternatives) => self.compile_or_match_pattern(subject, alternatives),
            Pattern::Sequence(patterns) => self.compile_sequence_match_pattern(subject, patterns),
            Pattern::Mapping { entries, rest } => {
                self.compile_mapping_match_pattern(subject, entries, rest.as_deref())
            }
            Pattern::Class {
                class,
                positional,
                keywords,
            } => self.compile_class_match_pattern(subject, class, positional, keywords),
            Pattern::Star(_) => Err("star pattern is only supported inside sequences".to_string()),
            Pattern::As { pattern, name } => {
                let mut compiled = self.compile_match_pattern(subject, pattern)?;
                compiled.bindings.push(MatchBinding {
                    name: name.clone(),
                    src: subject,
                });
                Ok(compiled)
            }
        }
    }

    fn compile_or_match_pattern(
        &mut self,
        subject: Register,
        alternatives: &[Pattern],
    ) -> Result<MatchPatternCode, String> {
        let mut success_jumps = Vec::new();
        let mut binding_names: Option<Vec<String>> = None;
        let mut binding_registers = Vec::new();

        for alternative in alternatives {
            let MatchPatternCode {
                failure_jumps,
                bindings,
            } = self.compile_match_pattern(subject, alternative)?;

            match &binding_names {
                Some(names) => {
                    if !same_match_binding_names(names, &bindings) {
                        return Err("alternative patterns bind different names".to_string());
                    }
                }
                None => {
                    let names = bindings
                        .iter()
                        .map(|binding| binding.name.clone())
                        .collect::<Vec<_>>();
                    binding_registers = names.iter().map(|_| self.alloc_register()).collect();
                    binding_names = Some(names);
                }
            }

            let names = binding_names
                .as_ref()
                .expect("or-pattern binding names are initialized before use");
            self.emit_or_match_binding_moves(names, &binding_registers, bindings)?;

            if failure_jumps.is_empty() {
                let body_target = self.instructions.len();
                for jump in success_jumps {
                    self.patch_jump_target(jump, body_target)?;
                }
                return Ok(MatchPatternCode {
                    failure_jumps: Vec::new(),
                    bindings: build_match_bindings(names, &binding_registers),
                });
            }

            let success_jump = self.instructions.len();
            self.instructions
                .push(Instruction::Jump { target: usize::MAX });
            success_jumps.push(success_jump);

            let next_alternative_target = self.instructions.len();
            for jump in failure_jumps {
                self.patch_jump_target(jump, next_alternative_target)?;
            }
        }

        let next_case_jump = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let body_target = self.instructions.len();
        for jump in success_jumps {
            self.patch_jump_target(jump, body_target)?;
        }
        let names = binding_names.unwrap_or_default();

        Ok(MatchPatternCode {
            failure_jumps: vec![next_case_jump],
            bindings: build_match_bindings(&names, &binding_registers),
        })
    }

    fn emit_or_match_binding_moves(
        &mut self,
        names: &[String],
        registers: &[Register],
        bindings: Vec<MatchBinding>,
    ) -> Result<(), String> {
        for (name, dst) in names.iter().zip(registers) {
            let binding = bindings
                .iter()
                .find(|binding| &binding.name == name)
                .ok_or_else(|| "alternative patterns bind different names".to_string())?;
            self.instructions.push(Instruction::Move {
                dst: *dst,
                src: binding.src,
            });
        }

        Ok(())
    }

    fn compile_class_match_pattern(
        &mut self,
        subject: Register,
        class: &Expr,
        positional: &[Pattern],
        keywords: &[(String, Pattern)],
    ) -> Result<MatchPatternCode, String> {
        let class = self.compile_expr(class)?;
        let condition = self.alloc_register();
        self.instructions.push(Instruction::MatchClass {
            dst: condition,
            subject,
            class,
            positional_count: positional.len(),
            keyword_names: keywords.iter().map(|(name, _)| name.clone()).collect(),
        });
        let jump = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        });

        let mut failure_jumps = vec![jump];
        let mut bindings = Vec::new();

        for (index, pattern) in positional.iter().enumerate() {
            let value = self.alloc_register();
            let found = self.alloc_register();
            self.instructions
                .push(Instruction::LoadMatchClassPositional {
                    dst: value,
                    found,
                    subject,
                    class,
                    index,
                });
            let jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition: found,
                target: usize::MAX,
            });
            failure_jumps.push(jump);

            let mut compiled = self.compile_match_pattern(value, pattern)?;
            failure_jumps.append(&mut compiled.failure_jumps);
            bindings.append(&mut compiled.bindings);
        }

        for (name, pattern) in keywords {
            let value = self.alloc_register();
            let found = self.alloc_register();
            self.instructions.push(Instruction::LoadMatchAttribute {
                dst: value,
                found,
                object: subject,
                name: self.mangle_private_name(name),
            });
            let jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition: found,
                target: usize::MAX,
            });
            failure_jumps.push(jump);

            let mut compiled = self.compile_match_pattern(value, pattern)?;
            failure_jumps.append(&mut compiled.failure_jumps);
            bindings.append(&mut compiled.bindings);
        }

        Ok(MatchPatternCode {
            failure_jumps,
            bindings,
        })
    }

    fn compile_sequence_match_pattern(
        &mut self,
        subject: Register,
        patterns: &[Pattern],
    ) -> Result<MatchPatternCode, String> {
        let star_index = patterns
            .iter()
            .position(|pattern| matches!(pattern, Pattern::Star(_)));
        let min_len = if star_index.is_some() {
            patterns.len() - 1
        } else {
            patterns.len()
        };

        let condition = self.alloc_register();
        self.instructions.push(Instruction::MatchSequence {
            dst: condition,
            src: subject,
            min_len,
            exact: star_index.is_none(),
        });
        let jump = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        });

        let mut failure_jumps = vec![jump];
        let mut bindings = Vec::new();

        if let Some(star_index) = star_index {
            for (index, pattern) in patterns[..star_index].iter().enumerate() {
                let mut compiled =
                    self.compile_sequence_match_item(subject, index as i64, pattern)?;
                failure_jumps.append(&mut compiled.failure_jumps);
                bindings.append(&mut compiled.bindings);
            }

            if let Pattern::Star(name) = &patterns[star_index] {
                if let Some(name) = name {
                    let rest = self.alloc_register();
                    self.instructions.push(Instruction::LoadSequenceRest {
                        dst: rest,
                        src: subject,
                        start: star_index,
                        suffix: patterns.len() - star_index - 1,
                    });
                    bindings.push(MatchBinding {
                        name: name.clone(),
                        src: rest,
                    });
                }
            }

            let suffix = &patterns[star_index + 1..];
            for (offset, pattern) in suffix.iter().enumerate() {
                let index = -((suffix.len() - offset) as i64);
                let mut compiled = self.compile_sequence_match_item(subject, index, pattern)?;
                failure_jumps.append(&mut compiled.failure_jumps);
                bindings.append(&mut compiled.bindings);
            }
        } else {
            for (index, pattern) in patterns.iter().enumerate() {
                let mut compiled =
                    self.compile_sequence_match_item(subject, index as i64, pattern)?;
                failure_jumps.append(&mut compiled.failure_jumps);
                bindings.append(&mut compiled.bindings);
            }
        }

        Ok(MatchPatternCode {
            failure_jumps,
            bindings,
        })
    }

    fn compile_mapping_match_pattern(
        &mut self,
        subject: Register,
        entries: &[(Expr, Pattern)],
        rest: Option<&str>,
    ) -> Result<MatchPatternCode, String> {
        let condition = self.alloc_register();
        self.instructions.push(Instruction::MatchMapping {
            dst: condition,
            src: subject,
        });
        let jump = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        });

        let mut failure_jumps = vec![jump];
        let mut bindings = Vec::new();
        for (key, _) in entries {
            if !is_match_mapping_key_expr(key) {
                return Err("patterns may only match literals and attribute lookups".to_string());
            }
        }
        let key_registers = entries
            .iter()
            .map(|(key, _)| self.compile_expr(key))
            .collect::<Result<Vec<_>, _>>()?;

        if !key_registers.is_empty() {
            let condition = self.alloc_register();
            self.instructions.push(Instruction::MatchMappingKeys {
                dst: condition,
                mapping: subject,
                keys: key_registers.clone(),
            });
            let jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            });
            failure_jumps.push(jump);
        }

        for ((_, pattern), key) in entries.iter().zip(key_registers.iter().copied()) {
            let value = self.alloc_register();
            self.instructions.push(Instruction::LoadSubscript {
                dst: value,
                object: subject,
                index: key,
            });

            let mut compiled = self.compile_match_pattern(value, pattern)?;
            failure_jumps.append(&mut compiled.failure_jumps);
            bindings.append(&mut compiled.bindings);
        }

        if let Some(name) = rest {
            let rest = self.alloc_register();
            self.instructions.push(Instruction::LoadMappingRest {
                dst: rest,
                src: subject,
                keys: key_registers,
            });
            bindings.push(MatchBinding {
                name: name.to_string(),
                src: rest,
            });
        }

        Ok(MatchPatternCode {
            failure_jumps,
            bindings,
        })
    }

    fn compile_sequence_match_item(
        &mut self,
        subject: Register,
        index: i64,
        pattern: &Pattern,
    ) -> Result<MatchPatternCode, String> {
        if matches!(pattern, Pattern::Star(_)) {
            return Err("star pattern is only supported once in a sequence".to_string());
        }

        let index_register = self.alloc_register();
        self.instructions.push(Instruction::LoadConst {
            dst: index_register,
            value: Value::Number(index),
        });
        let item = self.alloc_register();
        self.instructions.push(Instruction::LoadSubscript {
            dst: item,
            object: subject,
            index: index_register,
        });

        self.compile_match_pattern(item, pattern)
    }

    fn emit_match_bindings(&mut self, bindings: Vec<MatchBinding>) {
        for binding in bindings {
            self.emit_store_name(&binding.name, binding.src);
        }
    }

    fn compile_while_stmt(
        &mut self,
        condition: &Expr,
        body: &[Stmt],
        else_body: &[Stmt],
    ) -> Result<(), String> {
        self.enter_static_blocks(1)?;

        let loop_start = self.instructions.len();
        let condition = self.compile_expr(condition)?;
        let jump_if_false = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        });

        self.loop_contexts.push(LoopContext {
            continue_target: loop_start,
            break_jumps: Vec::new(),
        });

        for stmt in body {
            self.compile_stmt(stmt)?;
        }

        let loop_context = self
            .loop_contexts
            .pop()
            .expect("while compilation always pushes a loop context");

        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let else_target = self.instructions.len();
        self.patch_jump_target(jump_if_false, else_target)?;

        for stmt in else_body {
            self.compile_stmt(stmt)?;
        }

        let end_target = self.instructions.len();
        for break_jump in loop_context.break_jumps {
            self.patch_jump_target(break_jump, end_target)?;
        }

        self.leave_static_blocks(1);
        Ok(())
    }

    fn compile_for_stmt(
        &mut self,
        target: &Target,
        iter: &Expr,
        body: &[Stmt],
        else_body: &[Stmt],
    ) -> Result<(), String> {
        self.enter_static_blocks(1)?;

        let iterable = self.compile_expr(iter)?;
        let iterator = self.alloc_register();
        self.instructions.push(Instruction::GetIter {
            dst: iterator,
            src: iterable,
        });

        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let for_iter = self.instructions.len();
        self.instructions.push(Instruction::ForIter {
            iterator,
            dst: item,
            target: usize::MAX,
        });
        self.compile_store_target(target, item)?;

        self.loop_contexts.push(LoopContext {
            continue_target: loop_start,
            break_jumps: Vec::new(),
        });

        for stmt in body {
            self.compile_stmt(stmt)?;
        }

        let loop_context = self
            .loop_contexts
            .pop()
            .expect("for compilation always pushes a loop context");

        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let else_target = self.instructions.len();
        self.patch_jump_target(for_iter, else_target)?;

        for stmt in else_body {
            self.compile_stmt(stmt)?;
        }

        let end_target = self.instructions.len();
        for break_jump in loop_context.break_jumps {
            self.patch_jump_target(break_jump, end_target)?;
        }

        self.leave_static_blocks(1);
        Ok(())
    }

    fn compile_async_for_stmt(
        &mut self,
        target: &Target,
        iter: &Expr,
        body: &[Stmt],
        else_body: &[Stmt],
    ) -> Result<(), String> {
        if self.async_function_depth == 0 {
            return Err("'async for' outside async function".to_string());
        }
        self.enter_static_blocks(1)?;

        let iterable = self.compile_expr(iter)?;
        let iterator = self.alloc_register();
        self.instructions.push(Instruction::GetAsyncIter {
            dst: iterator,
            src: iterable,
        });

        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let async_for_iter = self.instructions.len();
        self.instructions.push(Instruction::AsyncForIter {
            iterator,
            dst: item,
            target: usize::MAX,
        });
        self.compile_store_target(target, item)?;

        self.loop_contexts.push(LoopContext {
            continue_target: loop_start,
            break_jumps: Vec::new(),
        });

        for stmt in body {
            self.compile_stmt(stmt)?;
        }

        let loop_context = self
            .loop_contexts
            .pop()
            .expect("async for compilation always pushes a loop context");

        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let else_target = self.instructions.len();
        self.patch_jump_target(async_for_iter, else_target)?;

        for stmt in else_body {
            self.compile_stmt(stmt)?;
        }

        let end_target = self.instructions.len();
        for break_jump in loop_context.break_jumps {
            self.patch_jump_target(break_jump, end_target)?;
        }

        self.leave_static_blocks(1);
        Ok(())
    }

    fn compile_try_stmt(
        &mut self,
        body: &[Stmt],
        handlers: &[AstExceptHandler],
        else_body: &[Stmt],
        finally_body: &[Stmt],
    ) -> Result<(), String> {
        if finally_body.is_empty() {
            return self.compile_try_except_stmt(body, handlers, else_body, false);
        }

        self.compile_try_finally_stmt(body, handlers, else_body, finally_body, false)
    }

    fn compile_try_star_stmt(
        &mut self,
        body: &[Stmt],
        handlers: &[AstExceptHandler],
        else_body: &[Stmt],
        finally_body: &[Stmt],
    ) -> Result<(), String> {
        reject_except_star_control_flow(handlers)?;

        if finally_body.is_empty() {
            return self.compile_try_except_stmt(body, handlers, else_body, true);
        }

        self.compile_try_finally_stmt(body, handlers, else_body, finally_body, true)
    }

    fn compile_try_except_stmt(
        &mut self,
        body: &[Stmt],
        handlers: &[AstExceptHandler],
        else_body: &[Stmt],
        handlers_are_star: bool,
    ) -> Result<(), String> {
        if handlers.is_empty() {
            return Err("try statement requires except or finally".to_string());
        }
        let static_blocks = 1 + handlers
            .iter()
            .filter(|handler| handler.name.is_some())
            .count();
        self.enter_static_blocks(static_blocks)?;

        let bytecode_handlers = handlers
            .iter()
            .map(|handler| self.bytecode_except_handler(handler, usize::MAX, handlers_are_star))
            .collect::<Result<Vec<_>, _>>()?;
        let setup_except = self.instructions.len();
        self.instructions.push(Instruction::SetupExcept {
            handlers: bytecode_handlers,
        });

        for stmt in body {
            self.compile_stmt(stmt)?;
        }

        self.instructions.push(Instruction::PopExcept);

        for stmt in else_body {
            self.compile_stmt(stmt)?;
        }

        let jump_over_handlers = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let mut handler_jumps = Vec::new();
        for (index, handler) in handlers.iter().enumerate() {
            let handler_target = self.instructions.len();
            self.patch_exception_handler_target(setup_except, index, handler_target)?;

            for stmt in &handler.body {
                self.compile_stmt(stmt)?;
            }

            self.instructions.push(Instruction::ClearException);
            let jump_to_end = self.instructions.len();
            self.instructions
                .push(Instruction::Jump { target: usize::MAX });
            handler_jumps.push(jump_to_end);
        }

        let end_target = self.instructions.len();
        self.patch_jump_target(jump_over_handlers, end_target)?;
        for jump in handler_jumps {
            self.patch_jump_target(jump, end_target)?;
        }

        self.leave_static_blocks(static_blocks);
        Ok(())
    }

    fn bytecode_except_handler(
        &mut self,
        handler: &AstExceptHandler,
        target: usize,
        is_star: bool,
    ) -> Result<BytecodeExceptHandler, String> {
        let (type_names, type_register) = match handler.type_expr.as_ref() {
            None => (None, None),
            Some(type_expr) => match except_type_names_from_expr(Some(type_expr)) {
                Ok(type_names) => (type_names, None),
                Err(_) => (None, Some(self.compile_expr(type_expr)?)),
            },
        };

        Ok(BytecodeExceptHandler {
            type_names,
            type_register,
            name: handler.name.clone(),
            name_binding: handler
                .name
                .as_ref()
                .map(|name| self.except_handler_name_binding(name)),
            target,
            is_star,
        })
    }

    fn except_handler_name_binding(&self, name: &str) -> ExceptHandlerNameBinding {
        if self.name_is_declared_nonlocal(name) {
            ExceptHandlerNameBinding::Nonlocal
        } else if self.name_is_declared_global(name) {
            ExceptHandlerNameBinding::Global
        } else {
            ExceptHandlerNameBinding::Local
        }
    }

    fn compile_try_finally_stmt(
        &mut self,
        body: &[Stmt],
        handlers: &[AstExceptHandler],
        else_body: &[Stmt],
        finally_body: &[Stmt],
        handlers_are_star: bool,
    ) -> Result<(), String> {
        let static_blocks = 2 + handlers
            .iter()
            .filter(|handler| handler.name.is_some())
            .count();
        self.enter_static_blocks(static_blocks)?;

        let mut bytecode_handlers = handlers
            .iter()
            .map(|handler| self.bytecode_except_handler(handler, usize::MAX, handlers_are_star))
            .collect::<Result<Vec<_>, _>>()?;
        let finally_handler_index = bytecode_handlers.len();
        bytecode_handlers.push(BytecodeExceptHandler {
            type_names: None,
            type_register: None,
            name: None,
            name_binding: None,
            target: usize::MAX,
            is_star: false,
        });
        let setup_except = self.instructions.len();
        self.instructions.push(Instruction::SetupExcept {
            handlers: bytecode_handlers,
        });

        self.finally_contexts.push(FinallyContext {
            prelude: vec![Instruction::PopExcept],
            body: finally_body.to_vec(),
            trailer: Vec::new(),
        });
        for stmt in body {
            self.compile_stmt(stmt)?;
        }
        self.finally_contexts
            .pop()
            .expect("try body compilation always pushes a finally context");

        self.instructions.push(Instruction::PopExcept);

        self.finally_contexts.push(FinallyContext {
            prelude: Vec::new(),
            body: finally_body.to_vec(),
            trailer: Vec::new(),
        });
        for stmt in else_body {
            self.compile_stmt(stmt)?;
        }
        self.finally_contexts
            .pop()
            .expect("try else compilation always pushes a finally context");

        for stmt in finally_body {
            self.compile_stmt(stmt)?;
        }

        let jump_over_handlers = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let mut handler_jumps = Vec::new();
        for (index, handler) in handlers.iter().enumerate() {
            let handler_target = self.instructions.len();
            self.patch_exception_handler_target(setup_except, index, handler_target)?;

            self.finally_contexts.push(FinallyContext {
                prelude: vec![Instruction::ClearException],
                body: finally_body.to_vec(),
                trailer: Vec::new(),
            });
            for stmt in &handler.body {
                self.compile_stmt(stmt)?;
            }
            self.finally_contexts
                .pop()
                .expect("except handler compilation always pushes a finally context");

            self.instructions.push(Instruction::ClearException);
            for stmt in finally_body {
                self.compile_stmt(stmt)?;
            }

            let jump_to_end = self.instructions.len();
            self.instructions
                .push(Instruction::Jump { target: usize::MAX });
            handler_jumps.push(jump_to_end);
        }

        let finally_handler_target = self.instructions.len();
        self.patch_exception_handler_target(
            setup_except,
            finally_handler_index,
            finally_handler_target,
        )?;

        for stmt in finally_body {
            self.compile_stmt(stmt)?;
        }
        self.instructions.push(Instruction::Raise {
            src: None,
            cause: None,
        });

        let end_target = self.instructions.len();
        self.patch_jump_target(jump_over_handlers, end_target)?;
        for jump in handler_jumps {
            self.patch_jump_target(jump, end_target)?;
        }

        self.leave_static_blocks(static_blocks);
        Ok(())
    }

    fn compile_with_stmt(&mut self, items: &[WithItem], body: &[Stmt]) -> Result<(), String> {
        self.compile_with_items(items, 0, body)
    }

    fn compile_async_with_stmt(&mut self, items: &[WithItem], body: &[Stmt]) -> Result<(), String> {
        if self.async_function_depth == 0 {
            return Err("'async with' outside async function".to_string());
        }

        self.compile_async_with_items(items, 0, body)
    }

    fn compile_with_items(
        &mut self,
        items: &[WithItem],
        index: usize,
        body: &[Stmt],
    ) -> Result<(), String> {
        let Some(item) = items.get(index) else {
            for stmt in body {
                self.compile_stmt(stmt)?;
            }
            return Ok(());
        };
        self.enter_static_blocks(1)?;

        let manager = self.compile_expr(&item.context_expr)?;
        let exit = self.alloc_register();
        self.instructions.push(Instruction::LoadContextManagerExit {
            dst: exit,
            manager,
            is_async: false,
        });
        let enter = self.alloc_register();
        self.instructions
            .push(Instruction::LoadContextManagerEnter {
                dst: enter,
                manager,
                is_async: false,
            });
        let entered = self.alloc_register();
        self.instructions.push(Instruction::Call {
            dst: entered,
            callee: enter,
            args: Vec::new(),
        });

        let setup_except = self.instructions.len();
        self.instructions.push(Instruction::SetupExcept {
            handlers: vec![BytecodeExceptHandler {
                type_names: None,
                type_register: None,
                name: None,
                name_binding: None,
                target: usize::MAX,
                is_star: false,
            }],
        });

        let (normal_exit_trailer, _) = self.build_with_exit_call(exit, WithExitArgs::NoException);
        self.finally_contexts.push(FinallyContext {
            prelude: vec![Instruction::PopExcept],
            body: Vec::new(),
            trailer: normal_exit_trailer.clone(),
        });

        if let Some(target) = &item.optional_vars {
            self.compile_store_target(target, entered)?;
        }

        self.compile_with_items(items, index + 1, body)?;
        self.finally_contexts
            .pop()
            .expect("with compilation always pushes a finally context");

        self.instructions.push(Instruction::PopExcept);
        self.instructions.extend(normal_exit_trailer);
        let jump_over_handler = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let handler_target = self.instructions.len();
        self.patch_exception_handler_target(setup_except, 0, handler_target)?;
        let suppress = self.emit_with_exit_call(exit, WithExitArgs::CurrentException);
        let jump_to_reraise = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition: suppress,
            target: usize::MAX,
        });
        self.instructions.push(Instruction::ClearException);
        let jump_to_end_after_suppress = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let reraise_target = self.instructions.len();
        self.patch_jump_target(jump_to_reraise, reraise_target)?;
        self.instructions.push(Instruction::Raise {
            src: None,
            cause: None,
        });

        let end_target = self.instructions.len();
        self.patch_jump_target(jump_over_handler, end_target)?;
        self.patch_jump_target(jump_to_end_after_suppress, end_target)?;

        self.leave_static_blocks(1);
        Ok(())
    }

    fn compile_async_with_items(
        &mut self,
        items: &[WithItem],
        index: usize,
        body: &[Stmt],
    ) -> Result<(), String> {
        let Some(item) = items.get(index) else {
            for stmt in body {
                self.compile_stmt(stmt)?;
            }
            return Ok(());
        };
        self.enter_static_blocks(1)?;

        let manager = self.compile_expr(&item.context_expr)?;
        let exit = self.alloc_register();
        self.instructions.push(Instruction::LoadContextManagerExit {
            dst: exit,
            manager,
            is_async: true,
        });
        let enter = self.alloc_register();
        self.instructions
            .push(Instruction::LoadContextManagerEnter {
                dst: enter,
                manager,
                is_async: true,
            });
        let enter_awaitable = self.alloc_register();
        self.instructions.push(Instruction::Call {
            dst: enter_awaitable,
            callee: enter,
            args: Vec::new(),
        });
        let entered = self.alloc_register();
        self.instructions.push(Instruction::AwaitContextManager {
            dst: entered,
            src: enter_awaitable,
            is_exit: false,
        });

        let setup_except = self.instructions.len();
        self.instructions.push(Instruction::SetupExcept {
            handlers: vec![BytecodeExceptHandler {
                type_names: None,
                type_register: None,
                name: None,
                name_binding: None,
                target: usize::MAX,
                is_star: false,
            }],
        });

        let (normal_exit_trailer, _) =
            self.build_async_with_exit_call(exit, WithExitArgs::NoException);
        self.finally_contexts.push(FinallyContext {
            prelude: vec![Instruction::PopExcept],
            body: Vec::new(),
            trailer: normal_exit_trailer.clone(),
        });

        if let Some(target) = &item.optional_vars {
            self.compile_store_target(target, entered)?;
        }

        self.compile_async_with_items(items, index + 1, body)?;
        self.finally_contexts
            .pop()
            .expect("async with compilation always pushes a finally context");

        self.instructions.push(Instruction::PopExcept);
        self.instructions.extend(normal_exit_trailer);
        let jump_over_handler = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let handler_target = self.instructions.len();
        self.patch_exception_handler_target(setup_except, 0, handler_target)?;
        let suppress = self.emit_async_with_exit_call(exit, WithExitArgs::CurrentException);
        let jump_to_reraise = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition: suppress,
            target: usize::MAX,
        });
        self.instructions.push(Instruction::ClearException);
        let jump_to_end_after_suppress = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let reraise_target = self.instructions.len();
        self.patch_jump_target(jump_to_reraise, reraise_target)?;
        self.instructions.push(Instruction::Raise {
            src: None,
            cause: None,
        });

        let end_target = self.instructions.len();
        self.patch_jump_target(jump_over_handler, end_target)?;
        self.patch_jump_target(jump_to_end_after_suppress, end_target)?;

        self.leave_static_blocks(1);
        Ok(())
    }

    fn compile_store_target(&mut self, target: &Target, src: Register) -> Result<(), String> {
        match target {
            Target::Name(name) => {
                self.emit_store_name(name, src);
                Ok(())
            }
            Target::Attribute { object, name } => {
                let object = self.compile_expr(object)?;
                self.instructions.push(Instruction::StoreAttribute {
                    object,
                    name: self.mangle_private_name(name),
                    src,
                });
                Ok(())
            }
            Target::Subscript { object, index } => {
                let object_register = self.compile_expr(object)?;
                let index = self.compile_expr(index)?;
                self.instructions.push(Instruction::StoreSubscript {
                    object: object_register,
                    index,
                    src,
                });

                Ok(())
            }
            Target::Slice {
                object,
                start,
                stop,
                step,
            } => {
                let object_register = self.compile_expr(object)?;
                let start = start
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let stop = stop
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let step = step
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                self.instructions.push(Instruction::StoreSlice {
                    object: object_register,
                    start,
                    stop,
                    step,
                    src,
                });

                Ok(())
            }
            Target::Tuple(targets) | Target::List(targets) => {
                self.compile_store_sequence_targets(targets, src)
            }
            Target::Starred(_) => {
                Err("starred assignment target must be in a list or tuple".to_string())
            }
        }
    }

    fn compile_store_sequence_targets(
        &mut self,
        targets: &[Target],
        src: Register,
    ) -> Result<(), String> {
        let starred_indices = targets
            .iter()
            .enumerate()
            .filter_map(|(index, target)| matches!(target, Target::Starred(_)).then_some(index))
            .collect::<Vec<_>>();

        if starred_indices.is_empty() {
            let dst = targets
                .iter()
                .map(|_| self.alloc_register())
                .collect::<Vec<_>>();
            self.instructions.push(Instruction::UnpackSequence {
                src,
                dst: dst.clone(),
            });

            for (target, src) in targets.iter().zip(dst) {
                self.compile_store_target(target, src)?;
            }

            return Ok(());
        }

        if starred_indices.len() > 1 {
            return Err("multiple starred expressions in assignment".to_string());
        }

        let starred_index = starred_indices[0];
        let before = (0..starred_index)
            .map(|_| self.alloc_register())
            .collect::<Vec<_>>();
        let rest = self.alloc_register();
        let after_count = targets.len() - starred_index - 1;
        let after = (0..after_count)
            .map(|_| self.alloc_register())
            .collect::<Vec<_>>();

        self.instructions.push(Instruction::UnpackSequenceEx {
            src,
            before: before.clone(),
            rest,
            after: after.clone(),
        });

        for (target, register) in targets[..starred_index].iter().zip(before) {
            self.compile_store_target(target, register)?;
        }

        let Target::Starred(rest_target) = &targets[starred_index] else {
            unreachable!("starred index points to a starred target");
        };
        self.compile_store_target(rest_target, rest)?;

        for (target, register) in targets[starred_index + 1..].iter().zip(after) {
            self.compile_store_target(target, register)?;
        }

        Ok(())
    }

    fn compile_ann_assign_stmt(
        &mut self,
        target: &Target,
        annotation: &Expr,
        value: Option<&Expr>,
        simple: bool,
    ) -> Result<(), String> {
        if let Some(value) = value {
            let src = self.compile_expr(value)?;
            self.compile_store_target(target, src)?;
            self.instructions.push(Instruction::Pop { src });
        } else {
            self.compile_annotation_target_side_effects(target)?;
        }

        if self.function_depth > 0 {
            return Ok(());
        }

        let annotation = self.compile_expr(annotation)?;
        if simple && let Target::Name(name) = target {
            self.instructions.push(Instruction::StoreAnnotation {
                name: self.mangle_private_name(name),
                annotation,
            });
        }

        Ok(())
    }

    fn compile_annotation_target_side_effects(&mut self, target: &Target) -> Result<(), String> {
        match target {
            Target::Name(_) => {}
            Target::Attribute { object, .. } => {
                let _ = self.compile_expr(object)?;
            }
            Target::Subscript { object, index } => {
                let _ = self.compile_expr(object)?;
                let _ = self.compile_expr(index)?;
            }
            Target::Slice {
                object,
                start,
                stop,
                step,
            } => {
                let _ = self.compile_expr(object)?;
                if let Some(start) = start {
                    let _ = self.compile_expr(start)?;
                }
                if let Some(stop) = stop {
                    let _ = self.compile_expr(stop)?;
                }
                if let Some(step) = step {
                    let _ = self.compile_expr(step)?;
                }
            }
            Target::Starred(_) | Target::Tuple(_) | Target::List(_) => {}
        }

        Ok(())
    }

    fn compile_aug_assign_stmt(
        &mut self,
        target: &Target,
        op: &BinaryOp,
        value: &Expr,
    ) -> Result<(), String> {
        match target {
            Target::Name(name) => {
                let left = self.alloc_register();
                self.emit_load_name(left, name);
                let right = self.compile_expr(value)?;
                let dst = self.alloc_register();
                self.compile_augmented_binary_instruction(op, left, right, dst);
                self.emit_store_name(name, dst);
                self.instructions.push(Instruction::Pop { src: left });
                self.instructions.push(Instruction::Pop { src: right });
                self.instructions.push(Instruction::Pop { src: dst });
            }
            Target::Attribute { object, name } => {
                let object = self.compile_expr(object)?;
                let left = self.alloc_register();
                self.instructions.push(Instruction::LoadAttribute {
                    dst: left,
                    object,
                    name: self.mangle_private_name(name),
                });
                let right = self.compile_expr(value)?;
                let dst = self.alloc_register();
                self.compile_augmented_binary_instruction(op, left, right, dst);
                self.instructions.push(Instruction::StoreAttribute {
                    object,
                    name: self.mangle_private_name(name),
                    src: dst,
                });
                self.instructions.push(Instruction::Pop { src: object });
                self.instructions.push(Instruction::Pop { src: left });
                self.instructions.push(Instruction::Pop { src: right });
                self.instructions.push(Instruction::Pop { src: dst });
            }
            Target::Subscript { object, index } => {
                let object_register = self.compile_expr(object)?;
                let index_register = self.compile_expr(index)?;
                let left = self.alloc_register();
                self.instructions.push(Instruction::LoadSubscript {
                    dst: left,
                    object: object_register,
                    index: index_register,
                });
                let right = self.compile_expr(value)?;
                let dst = self.alloc_register();
                self.compile_augmented_binary_instruction(op, left, right, dst);
                self.instructions.push(Instruction::StoreSubscript {
                    object: object_register,
                    index: index_register,
                    src: dst,
                });
                self.instructions.push(Instruction::Pop {
                    src: object_register,
                });
                self.instructions.push(Instruction::Pop {
                    src: index_register,
                });
                self.instructions.push(Instruction::Pop { src: left });
                self.instructions.push(Instruction::Pop { src: right });
                self.instructions.push(Instruction::Pop { src: dst });
            }
            Target::Slice {
                object,
                start,
                stop,
                step,
            } => {
                let object_register = self.compile_expr(object)?;
                let start = start
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let stop = stop
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let step = step
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let left = self.alloc_register();
                self.instructions.push(Instruction::LoadSlice {
                    dst: left,
                    object: object_register,
                    start,
                    stop,
                    step,
                });
                let right = self.compile_expr(value)?;
                let dst = self.alloc_register();
                self.compile_augmented_binary_instruction(op, left, right, dst);
                self.instructions.push(Instruction::StoreSlice {
                    object: object_register,
                    start,
                    stop,
                    step,
                    src: dst,
                });
                self.instructions.push(Instruction::Pop {
                    src: object_register,
                });
                if let Some(start) = start {
                    self.instructions.push(Instruction::Pop { src: start });
                }
                if let Some(stop) = stop {
                    self.instructions.push(Instruction::Pop { src: stop });
                }
                if let Some(step) = step {
                    self.instructions.push(Instruction::Pop { src: step });
                }
                self.instructions.push(Instruction::Pop { src: left });
                self.instructions.push(Instruction::Pop { src: right });
                self.instructions.push(Instruction::Pop { src: dst });
            }
            Target::Tuple(_) | Target::List(_) | Target::Starred(_) => {
                return Err(
                    "augmented assignment target must be a name, attribute, or subscript"
                        .to_string(),
                );
            }
        }

        Ok(())
    }

    fn compile_delete_target(&mut self, target: &Target) -> Result<(), String> {
        match target {
            Target::Name(name) => {
                self.emit_delete_name(name);
                Ok(())
            }
            Target::Attribute { object, name } => {
                let object = self.compile_expr(object)?;
                self.instructions.push(Instruction::DeleteAttribute {
                    object,
                    name: self.mangle_private_name(name),
                });
                Ok(())
            }
            Target::Subscript { object, index } => {
                let object_register = self.compile_expr(object)?;
                let index = self.compile_expr(index)?;
                self.instructions.push(Instruction::DeleteSubscript {
                    object: object_register,
                    index,
                });

                Ok(())
            }
            Target::Slice {
                object,
                start,
                stop,
                step,
            } => {
                let object_register = self.compile_expr(object)?;
                let start = start
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let stop = stop
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let step = step
                    .as_ref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                self.instructions.push(Instruction::DeleteSlice {
                    object: object_register,
                    start,
                    stop,
                    step,
                });

                Ok(())
            }
            Target::Tuple(targets) | Target::List(targets) => {
                for target in targets {
                    self.compile_delete_target(target)?;
                }
                Ok(())
            }
            Target::Starred(_) => Err("cannot delete starred target".to_string()),
        }
    }

    fn compile_function_def_stmt(
        &mut self,
        name: &str,
        type_params: &[TypeParam],
        params: &FunctionParams,
        body: &[Stmt],
        decorators: &[Expr],
        returns: Option<&Expr>,
        is_async: bool,
    ) -> Result<(), String> {
        let decorators = self.compile_decorator_exprs(decorators)?;
        let type_param_names = type_param_name_set(type_params);
        let class_name = self.private_class_name.clone();
        let type_params = self.compile_type_params(
            type_params,
            TypeParamEvaluation::Deferred {
                class_name: class_name.as_deref(),
            },
        )?;
        let is_generator = statements_contain_yield(body);
        if is_async && statements_contain_yield_from(body) {
            return Err("'yield from' inside async function".to_string());
        }
        if is_async && is_generator && statements_contain_return_value(body) {
            return Err("'return' with value in async generator".to_string());
        }
        let first_line = self.next_function_first_line();
        let line_sequence = self.next_function_line_sequence(first_line);
        let position_columns = self.next_function_position_columns();
        self.next_function_is_lambda();
        let docstring = self.statement_docstring(body.first());
        let (body_instructions, is_generator) =
            self.compile_function_body(params, body, is_async, &type_param_names)?;
        let dst = self.compile_function_value(
            name,
            &type_params,
            &[],
            params,
            returns,
            docstring,
            body_instructions,
            is_generator,
            is_async,
            first_line,
            line_sequence,
            position_columns,
        )?;
        let dst = self.apply_decorators(dst, &decorators)?;
        self.emit_store_name(name, dst);

        Ok(())
    }

    fn compile_type_alias_stmt(
        &mut self,
        name: &str,
        type_params: &[TypeParam],
        value: &Expr,
    ) -> Result<(), String> {
        let class_name = self.private_class_name.clone();
        let type_params = self.compile_type_params(
            type_params,
            TypeParamEvaluation::Deferred {
                class_name: class_name.as_deref(),
            },
        )?;
        let class_name = self.private_class_name.clone();
        let value = self.compile_deferred_type_param_expr(
            value,
            &type_params,
            class_name.as_deref(),
            false,
        )?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::MakeTypeAlias {
            dst,
            name: name.to_string(),
            type_params: type_params.iter().map(|(_, register)| *register).collect(),
            value,
        });
        self.emit_store_name(name, dst);

        Ok(())
    }

    fn compile_class_def_stmt(
        &mut self,
        name: &str,
        type_params: &[TypeParam],
        bases: &[CallArg],
        keywords: &[CallKeyword],
        body: &[Stmt],
        decorators: &[Expr],
    ) -> Result<(), String> {
        let decorators = self.compile_decorator_exprs(decorators)?;
        let type_param_names = type_param_name_set(type_params);
        let type_params = self.compile_type_params(
            type_params,
            TypeParamEvaluation::Deferred {
                class_name: Some(name),
            },
        )?;
        let bases = self.compile_type_scoped_call_args(bases, &type_params)?;
        let keywords = self.compile_type_scoped_call_keywords(keywords, &type_params)?;
        let scope =
            analyze_class_scope(body, &self.enclosing_function_bindings, &type_param_names)?;
        let static_attributes = collect_class_static_attributes(body);
        let mut class_compiler = self.new_class_body_compiler();
        class_compiler.global_names = scope.global_names;
        class_compiler.nonlocal_names = scope.nonlocal_names;
        class_compiler.class_scope_all_bindings = scope.local_bindings;
        class_compiler.private_class_name = Some(name.to_string());
        class_compiler.enclosing_function_bindings = self.enclosing_function_bindings.clone();

        for (index, stmt) in body.iter().enumerate() {
            if self.should_skip_optimized_docstring(index, stmt) {
                continue;
            }
            class_compiler.compile_stmt(stmt)?;
            class_compiler.add_class_scope_prior_bindings(stmt);
        }
        class_compiler.instructions.push(Instruction::Halt);
        let docstring = self.statement_docstring(body.first());

        let dst = self.alloc_register();
        self.instructions.push(Instruction::MakeClass {
            dst,
            name: name.to_string(),
            type_params: type_params.iter().map(|(_, register)| *register).collect(),
            bases,
            keywords,
            static_attributes,
            docstring,
            body: class_compiler.instructions,
        });
        let dst = self.apply_decorators(dst, &decorators)?;
        self.emit_store_name(name, dst);

        Ok(())
    }

    fn compile_type_params(
        &mut self,
        type_params: &[TypeParam],
        evaluation: TypeParamEvaluation<'_>,
    ) -> Result<Vec<(String, Register)>, String> {
        let mut compiled = Vec::new();

        for type_param in type_params {
            let dst = self.alloc_register();
            self.instructions.push(Instruction::MakeTypeParam {
                dst,
                kind: type_param_kind_label(&type_param.kind).to_string(),
                name: type_param.name.clone(),
                bound: None,
                default: None,
            });
            compiled.push((type_param.name.clone(), dst));

            let bound = type_param
                .bound
                .as_ref()
                .map(|expr| {
                    self.compile_type_param_metadata_expr(
                        expr,
                        &compiled,
                        evaluation,
                        matches!(expr, Expr::Tuple(_)),
                    )
                })
                .transpose()?;
            let default = type_param
                .default
                .as_ref()
                .map(|expr| {
                    self.compile_type_param_metadata_expr(expr, &compiled, evaluation, false)
                })
                .transpose()?;
            if bound.is_some() || default.is_some() {
                self.instructions.push(Instruction::UpdateTypeParam {
                    target: dst,
                    bound,
                    default,
                });
            }
        }

        Ok(compiled)
    }

    fn compile_type_param_metadata_expr(
        &mut self,
        expr: &Expr,
        type_params: &[(String, Register)],
        evaluation: TypeParamEvaluation<'_>,
        is_constraint_tuple: bool,
    ) -> Result<Register, String> {
        match evaluation {
            TypeParamEvaluation::Deferred { class_name } => self.compile_deferred_type_param_expr(
                expr,
                type_params,
                class_name,
                is_constraint_tuple,
            ),
        }
    }

    fn compile_deferred_type_param_expr(
        &mut self,
        expr: &Expr,
        type_params: &[(String, Register)],
        class_name: Option<&str>,
        is_constraint_tuple: bool,
    ) -> Result<Register, String> {
        let body = self.compile_deferred_type_scoped_expr_body(expr, type_params)?;
        let dst = self.alloc_register();
        self.instructions
            .push(Instruction::MakeDeferredTypeParamExpr {
                dst,
                body,
                type_params: type_params.iter().map(|(_, register)| *register).collect(),
                class_name: class_name.map(str::to_string),
                is_constraint_tuple,
            });
        Ok(dst)
    }

    fn compile_deferred_type_scoped_expr_body(
        &self,
        expr: &Expr,
        type_params: &[(String, Register)],
    ) -> Result<Vec<Instruction>, String> {
        let mut compiler = self.new_deferred_type_param_expr_compiler();
        let deferred_type_params = type_params
            .iter()
            .map(|(name, _)| {
                let dst = compiler.alloc_register();
                compiler.instructions.push(Instruction::LoadNonlocal {
                    dst,
                    name: name.clone(),
                });
                (name.clone(), dst)
            })
            .collect::<Vec<_>>();
        let src = match expr {
            Expr::Starred(value) => {
                compiler.compile_type_scoped_unpack_expr(value, &deferred_type_params)?
            }
            expr => compiler.compile_type_scoped_expr(expr, &deferred_type_params)?,
        };
        compiler
            .instructions
            .push(Instruction::Return { src: Some(src) });
        Ok(compiler.instructions)
    }

    fn compile_decorator_exprs(&mut self, decorators: &[Expr]) -> Result<Vec<Register>, String> {
        decorators
            .iter()
            .map(|decorator| self.compile_expr(decorator))
            .collect()
    }

    fn apply_decorators(
        &mut self,
        mut decorated: Register,
        decorators: &[Register],
    ) -> Result<Register, String> {
        for decorator in decorators.iter().rev() {
            let dst = self.alloc_register();
            self.instructions.push(Instruction::Call {
                dst,
                callee: *decorator,
                args: vec![decorated],
            });
            decorated = dst;
        }

        Ok(decorated)
    }

    fn compile_import_stmt(&mut self, aliases: &[ImportAlias]) -> Result<(), String> {
        for alias in aliases {
            let dst = self.alloc_register();
            self.instructions.push(Instruction::ImportModule {
                dst,
                name: alias.name.clone(),
                return_root: alias.asname.is_none(),
                level: 0,
            });
            self.emit_store_name(import_binding_name(alias), dst);
        }

        Ok(())
    }

    fn compile_import_from_stmt(
        &mut self,
        module: Option<&str>,
        level: usize,
        targets: &ImportFromTargets,
    ) -> Result<(), String> {
        let module = module.unwrap_or("");
        let module_register = self.alloc_register();
        self.instructions.push(Instruction::ImportModule {
            dst: module_register,
            name: module.to_string(),
            return_root: false,
            level,
        });

        match targets {
            ImportFromTargets::Star => {
                self.instructions.push(Instruction::ImportStar {
                    module: module_register,
                });
            }
            ImportFromTargets::Aliases(aliases) => {
                for alias in aliases {
                    let dst = self.alloc_register();
                    self.instructions.push(Instruction::ImportFrom {
                        dst,
                        module: module_register,
                        name: alias.name.clone(),
                    });
                    self.emit_store_name(import_binding_name(alias), dst);
                }
            }
        }

        Ok(())
    }

    fn compile_function_body(
        &mut self,
        params: &FunctionParams,
        body: &[Stmt],
        is_async: bool,
        type_param_names: &HashSet<String>,
    ) -> Result<(Vec<Instruction>, bool), String> {
        let is_generator = statements_contain_yield(body);
        let scope = analyze_function_scope(
            params,
            body,
            &self.enclosing_function_bindings,
            type_param_names,
        )?;
        let mut function_compiler = self.new_nested_function_compiler();
        if is_generator {
            function_compiler.static_block_depth = 1;
        }
        if is_async {
            function_compiler.async_function_depth = self.async_function_depth + 1;
        }
        function_compiler.global_names = scope.global_names;
        function_compiler.nonlocal_names = scope.nonlocal_names;
        function_compiler.local_names = scope.local_bindings.clone();
        function_compiler.enclosing_function_bindings = std::iter::once(scope.local_bindings)
            .chain(self.enclosing_function_bindings.iter().cloned())
            .collect();

        for (index, stmt) in body.iter().enumerate() {
            if function_compiler.should_skip_optimized_docstring(index, stmt) {
                continue;
            }
            function_compiler.compile_stmt(stmt)?;
        }
        function_compiler
            .instructions
            .push(Instruction::ImplicitReturn);

        Ok((function_compiler.instructions, is_generator))
    }

    fn compile_lambda_expr(
        &mut self,
        params: &FunctionParams,
        body: &Expr,
        closure_bindings: &[(String, Register)],
    ) -> Result<Register, String> {
        let is_generator = expr_contains_yield(body);
        let mut function_compiler = self.new_nested_function_compiler();
        if is_generator {
            function_compiler.static_block_depth = 1;
        }
        function_compiler.local_names = function_param_names(params);
        let src = function_compiler.compile_expr(body)?;
        function_compiler
            .instructions
            .push(Instruction::Return { src: Some(src) });

        let (first_line, line_sequence, position_columns) = self.next_lambda_position_metadata();
        self.compile_function_value(
            "<lambda>",
            &[],
            closure_bindings,
            params,
            None,
            None,
            function_compiler.instructions,
            is_generator,
            false,
            first_line,
            line_sequence,
            position_columns,
        )
    }

    fn compile_function_value(
        &mut self,
        name: &str,
        type_params: &[(String, Register)],
        closure_bindings: &[(String, Register)],
        params: &FunctionParams,
        returns: Option<&Expr>,
        docstring: Option<String>,
        body: Vec<Instruction>,
        is_generator: bool,
        is_async: bool,
        first_line: usize,
        line_sequence: Vec<usize>,
        position_columns: Vec<Option<(usize, usize)>>,
    ) -> Result<Register, String> {
        let defaults = params
            .positional_only
            .iter()
            .chain(params.positional.iter())
            .filter_map(|param| {
                param.default.as_ref().map(|default| {
                    Ok((
                        self.mangle_private_name(&param.name),
                        self.compile_expr(default)?,
                    ))
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let keyword_defaults = params
            .keyword_only
            .iter()
            .filter_map(|param| {
                param.default.as_ref().map(|default| {
                    Ok((
                        self.mangle_private_name(&param.name),
                        self.compile_expr(default)?,
                    ))
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let annotations = self.compile_function_annotations(params, returns, type_params)?;

        let dst = self.alloc_register();
        let mut closure_bindings = self.expanded_type_param_closure_bindings(closure_bindings);
        closure_bindings.extend(self.expanded_type_param_closure_bindings(type_params));

        self.instructions.push(Instruction::MakeFunction {
            dst,
            name: name.to_string(),
            type_params: type_params.iter().map(|(_, register)| *register).collect(),
            closure_bindings,
            positional_only: params
                .positional_only
                .iter()
                .map(|param| self.mangle_private_name(&param.name))
                .collect(),
            params: params
                .positional
                .iter()
                .map(|param| self.mangle_private_name(&param.name))
                .collect(),
            defaults,
            vararg: params
                .vararg
                .as_ref()
                .map(|name| self.mangle_private_name(name)),
            keyword_only: params
                .keyword_only
                .iter()
                .map(|param| self.mangle_private_name(&param.name))
                .collect(),
            keyword_defaults,
            kwarg: params
                .kwarg
                .as_ref()
                .map(|name| self.mangle_private_name(name)),
            annotations,
            docstring,
            body,
            is_generator,
            is_async,
            first_line,
            line_sequence,
            position_columns,
        });

        Ok(dst)
    }

    fn expanded_type_param_closure_bindings(
        &self,
        closure_bindings: &[(String, Register)],
    ) -> Vec<(String, Register)> {
        let mut expanded = Vec::new();
        for (name, register) in closure_bindings {
            expanded.push((name.clone(), *register));
            let mangled = self.mangle_private_name(name);
            if mangled != *name {
                expanded.push((mangled, *register));
            }
        }
        expanded
    }

    fn compile_function_annotations(
        &mut self,
        params: &FunctionParams,
        returns: Option<&Expr>,
        type_params: &[(String, Register)],
    ) -> Result<Vec<(String, Register)>, String> {
        let mut annotations = Vec::new();

        for param in params
            .positional_only
            .iter()
            .chain(params.positional.iter())
        {
            if let Some(annotation) = &param.annotation {
                let annotation = self.compile_type_scoped_expr(annotation, type_params)?;
                annotations.push((self.mangle_private_name(&param.name), annotation));
            }
        }

        if let (Some(name), Some(annotation)) =
            (&params.vararg, params.vararg_annotation.as_deref())
        {
            let annotation = self.compile_type_scoped_expr(annotation, type_params)?;
            annotations.push((self.mangle_private_name(name), annotation));
        }

        for param in &params.keyword_only {
            if let Some(annotation) = &param.annotation {
                let annotation = self.compile_type_scoped_expr(annotation, type_params)?;
                annotations.push((self.mangle_private_name(&param.name), annotation));
            }
        }

        if let (Some(name), Some(annotation)) = (&params.kwarg, params.kwarg_annotation.as_deref())
        {
            let annotation = self.compile_type_scoped_expr(annotation, type_params)?;
            annotations.push((self.mangle_private_name(name), annotation));
        }

        if let Some(returns) = returns {
            let annotation = self.compile_type_scoped_expr(returns, type_params)?;
            annotations.push(("return".to_string(), annotation));
        }

        Ok(annotations)
    }

    fn compile_type_scoped_expr(
        &mut self,
        expr: &Expr,
        type_params: &[(String, Register)],
    ) -> Result<Register, String> {
        match expr {
            Expr::Name(name) => {
                if let Some((_, register)) = type_params
                    .iter()
                    .find(|(type_param_name, _)| type_param_name == name)
                {
                    return Ok(*register);
                }
                if self.name_is_future_class_scope_binding(name) {
                    let dst = self.alloc_register();
                    self.instructions.push(Instruction::LoadGlobal {
                        dst,
                        name: self.mangle_private_name(name),
                    });
                    return Ok(dst);
                }
                self.compile_expr(expr)
            }
            Expr::Attribute { object, name } => {
                let object = self.compile_type_scoped_expr(object, type_params)?;
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadAttribute {
                    dst,
                    object,
                    name: self.mangle_private_name(name),
                });
                Ok(dst)
            }
            Expr::Subscript { object, index } => {
                let object = self.compile_type_scoped_expr(object, type_params)?;
                let index = self.compile_type_scoped_subscript_index(index, type_params)?;
                let dst = self.alloc_register();
                self.instructions
                    .push(Instruction::LoadSubscript { dst, object, index });
                Ok(dst)
            }
            Expr::SliceLiteral { start, stop, step } => self.compile_slice_literal(
                start.as_deref(),
                stop.as_deref(),
                step.as_deref(),
                Some(type_params),
            ),
            Expr::Slice {
                object,
                start,
                stop,
                step,
            } => {
                let object = self.compile_type_scoped_expr(object, type_params)?;
                let start = start
                    .as_deref()
                    .map(|expr| self.compile_type_scoped_expr(expr, type_params))
                    .transpose()?;
                let stop = stop
                    .as_deref()
                    .map(|expr| self.compile_type_scoped_expr(expr, type_params))
                    .transpose()?;
                let step = step
                    .as_deref()
                    .map(|expr| self.compile_type_scoped_expr(expr, type_params))
                    .transpose()?;
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadSlice {
                    dst,
                    object,
                    start,
                    stop,
                    step,
                });
                Ok(dst)
            }
            Expr::Binary { left, op, right } => {
                let left = self.compile_type_scoped_expr(left, type_params)?;
                let right = self.compile_type_scoped_expr(right, type_params)?;
                let dst = self.alloc_register();
                self.compile_binary_instruction(op, left, right, dst);
                Ok(dst)
            }
            Expr::Tuple(items) => self.compile_tuple_display(items, Some(type_params)),
            Expr::List(items) => self.compile_list_display(items, Some(type_params)),
            Expr::Set(items) => self.compile_set_display(items, Some(type_params)),
            Expr::Dict(entries) => self.compile_dict_display(entries, Some(type_params)),
            Expr::Call { callee, args } => {
                let callee = self.compile_type_scoped_expr(callee, type_params)?;
                let args = args
                    .iter()
                    .map(|arg| self.compile_type_scoped_expr(arg, type_params))
                    .collect::<Result<Vec<_>, String>>()?;
                let dst = self.alloc_register();
                self.instructions
                    .push(Instruction::Call { dst, callee, args });
                Ok(dst)
            }
            Expr::KeywordCall {
                callee,
                args,
                keywords,
            } => {
                let callee = self.compile_type_scoped_expr(callee, type_params)?;
                let args = args
                    .iter()
                    .map(|arg| self.compile_type_scoped_expr(arg, type_params))
                    .collect::<Result<Vec<_>, String>>()?;
                let keywords = keywords
                    .iter()
                    .map(|(name, value)| {
                        Ok((
                            name.clone(),
                            self.compile_type_scoped_expr(value, type_params)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let dst = self.alloc_register();
                self.instructions.push(Instruction::CallKeyword {
                    dst,
                    callee,
                    args,
                    keywords,
                });
                Ok(dst)
            }
            Expr::UnpackCall {
                callee,
                args,
                keywords,
            } => {
                let callee = self.compile_type_scoped_expr(callee, type_params)?;
                let args = args
                    .iter()
                    .map(|arg| match arg {
                        CallArg::Expr(expr) => Ok(CallArgRegister::Value(
                            self.compile_type_scoped_expr(expr, type_params)?,
                        )),
                        CallArg::Unpack(expr) => Ok(CallArgRegister::Unpack(
                            self.compile_type_scoped_expr(expr, type_params)?,
                        )),
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let keywords = keywords
                    .iter()
                    .map(|keyword| match keyword {
                        CallKeyword::Named(name, expr) => Ok(CallKeywordRegister::Named(
                            name.clone(),
                            self.compile_type_scoped_expr(expr, type_params)?,
                        )),
                        CallKeyword::Unpack(expr) => Ok(CallKeywordRegister::Unpack(
                            self.compile_type_scoped_expr(expr, type_params)?,
                        )),
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let dst = self.alloc_register();
                self.instructions.push(Instruction::CallUnpack {
                    dst,
                    callee,
                    args,
                    keywords,
                });
                Ok(dst)
            }
            Expr::Lambda { params, body } => self.compile_lambda_expr(params, body, type_params),
            Expr::ListComp { element, clauses } => {
                self.compile_type_scoped_list_comp_expr(element, clauses, type_params)
            }
            Expr::GeneratorComp { element, clauses } => {
                self.compile_generator_comp_expr(element, clauses, Some(type_params))
            }
            Expr::Starred(value) => self.compile_type_scoped_expr(value, type_params),
            _ => self.compile_expr(expr),
        }
    }

    fn compile_type_scoped_subscript_index(
        &mut self,
        index: &Expr,
        type_params: &[(String, Register)],
    ) -> Result<Register, String> {
        match index {
            Expr::Tuple(items) => {
                let items = items
                    .iter()
                    .map(|item| match item {
                        Expr::Starred(value) => {
                            self.compile_type_scoped_unpack_expr(value, type_params)
                        }
                        item => self.compile_type_scoped_expr(item, type_params),
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let dst = self.alloc_register();
                self.instructions
                    .push(Instruction::BuildTuple { dst, items });
                Ok(dst)
            }
            Expr::Starred(value) => self.compile_type_scoped_unpack_expr(value, type_params),
            index => self.compile_type_scoped_expr(index, type_params),
        }
    }

    fn compile_type_scoped_unpack_expr(
        &mut self,
        value: &Expr,
        type_params: &[(String, Register)],
    ) -> Result<Register, String> {
        let value = self.compile_type_scoped_expr(value, type_params)?;
        let dst = self.alloc_register();
        self.instructions
            .push(Instruction::BuildUnpack { dst, value });
        Ok(dst)
    }

    fn compile_type_scoped_call_args(
        &mut self,
        args: &[CallArg],
        type_params: &[(String, Register)],
    ) -> Result<Vec<CallArgRegister>, String> {
        args.iter()
            .map(|arg| match arg {
                CallArg::Expr(expr) => Ok(CallArgRegister::Value(
                    self.compile_type_scoped_expr(expr, type_params)?,
                )),
                CallArg::Unpack(expr) => Ok(CallArgRegister::Unpack(
                    self.compile_type_scoped_expr(expr, type_params)?,
                )),
            })
            .collect()
    }

    fn compile_type_scoped_call_keywords(
        &mut self,
        keywords: &[CallKeyword],
        type_params: &[(String, Register)],
    ) -> Result<Vec<CallKeywordRegister>, String> {
        keywords
            .iter()
            .map(|keyword| match keyword {
                CallKeyword::Named(name, expr) => Ok(CallKeywordRegister::Named(
                    name.clone(),
                    self.compile_type_scoped_expr(expr, type_params)?,
                )),
                CallKeyword::Unpack(expr) => Ok(CallKeywordRegister::Unpack(
                    self.compile_type_scoped_expr(expr, type_params)?,
                )),
            })
            .collect()
    }

    fn compile_maybe_type_scoped_expr(
        &mut self,
        expr: &Expr,
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        match type_params {
            Some(type_params) => self.compile_type_scoped_expr(expr, type_params),
            None => self.compile_expr(expr),
        }
    }

    fn compile_slice_literal(
        &mut self,
        start: Option<&Expr>,
        stop: Option<&Expr>,
        step: Option<&Expr>,
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        let start = start
            .map(|expr| self.compile_maybe_type_scoped_expr(expr, type_params))
            .transpose()?;
        let stop = stop
            .map(|expr| self.compile_maybe_type_scoped_expr(expr, type_params))
            .transpose()?;
        let step = step
            .map(|expr| self.compile_maybe_type_scoped_expr(expr, type_params))
            .transpose()?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::BuildSlice {
            dst,
            start,
            stop,
            step,
        });
        Ok(dst)
    }

    fn compile_list_display(
        &mut self,
        elements: &[Expr],
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        if !elements
            .iter()
            .any(|element| matches!(element, Expr::Starred(_)))
        {
            let items = elements
                .iter()
                .map(|element| self.compile_maybe_type_scoped_expr(element, type_params))
                .collect::<Result<Vec<_>, String>>()?;
            let dst = self.alloc_register();
            self.instructions.push(Instruction::BuildList {
                dst,
                items: items.clone(),
            });
            if type_params.is_none() {
                for item in items {
                    self.instructions.push(Instruction::Pop { src: item });
                }
            }
            return Ok(dst);
        }

        let list = self.alloc_register();
        self.instructions.push(Instruction::BuildList {
            dst: list,
            items: Vec::new(),
        });

        for element in elements {
            match element {
                Expr::Starred(value) => {
                    let iterable = self.compile_maybe_type_scoped_expr(value, type_params)?;
                    self.instructions
                        .push(Instruction::ListExtend { list, iterable });
                    if type_params.is_none() {
                        self.instructions.push(Instruction::Pop { src: iterable });
                    }
                }
                element => {
                    let item = self.compile_maybe_type_scoped_expr(element, type_params)?;
                    self.instructions
                        .push(Instruction::ListAppend { list, item });
                    if type_params.is_none() {
                        self.instructions.push(Instruction::Pop { src: item });
                    }
                }
            }
        }

        Ok(list)
    }

    fn compile_tuple_display(
        &mut self,
        elements: &[Expr],
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        if !elements
            .iter()
            .any(|element| matches!(element, Expr::Starred(_)))
        {
            if let Some(value) = compile_time_tuple_constant_value(elements) {
                let dst = self.alloc_register();
                self.instructions
                    .push(Instruction::LoadConst { dst, value });
                return Ok(dst);
            }

            let items = elements
                .iter()
                .map(|element| self.compile_maybe_type_scoped_expr(element, type_params))
                .collect::<Result<Vec<_>, String>>()?;
            let dst = self.alloc_register();
            self.instructions.push(Instruction::BuildTuple {
                dst,
                items: items.clone(),
            });
            if type_params.is_none() {
                for item in items {
                    self.instructions.push(Instruction::Pop { src: item });
                }
            }
            return Ok(dst);
        }

        let list = self.compile_list_display(elements, type_params)?;
        let dst = self.alloc_register();
        self.instructions
            .push(Instruction::BuildTupleFromList { dst, list });
        if type_params.is_none() {
            self.instructions.push(Instruction::Pop { src: list });
        }
        Ok(dst)
    }

    fn compile_set_display(
        &mut self,
        elements: &[Expr],
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        if !elements
            .iter()
            .any(|element| matches!(element, Expr::Starred(_)))
        {
            let items = elements
                .iter()
                .map(|element| self.compile_maybe_type_scoped_expr(element, type_params))
                .collect::<Result<Vec<_>, String>>()?;
            let dst = self.alloc_register();
            self.instructions.push(Instruction::BuildSet {
                dst,
                items: items.clone(),
            });
            if type_params.is_none() {
                for item in items {
                    self.instructions.push(Instruction::Pop { src: item });
                }
            }
            return Ok(dst);
        }

        let set = self.alloc_register();
        self.instructions.push(Instruction::BuildSet {
            dst: set,
            items: Vec::new(),
        });

        for element in elements {
            match element {
                Expr::Starred(value) => {
                    let iterable = self.compile_maybe_type_scoped_expr(value, type_params)?;
                    self.instructions
                        .push(Instruction::SetUpdate { set, iterable });
                    if type_params.is_none() {
                        self.instructions.push(Instruction::Pop { src: iterable });
                    }
                }
                element => {
                    let item = self.compile_maybe_type_scoped_expr(element, type_params)?;
                    self.instructions.push(Instruction::SetAdd { set, item });
                    if type_params.is_none() {
                        self.instructions.push(Instruction::Pop { src: item });
                    }
                }
            }
        }

        Ok(set)
    }

    fn compile_frozen_set_display(&mut self, elements: &[Expr]) -> Result<Register, String> {
        let items = elements
            .iter()
            .map(|element| self.compile_expr(element))
            .collect::<Result<Vec<_>, _>>()?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::BuildFrozenSet {
            dst,
            items: items.clone(),
        });
        for item in items {
            self.instructions.push(Instruction::Pop { src: item });
        }
        Ok(dst)
    }

    fn compile_dict_display(
        &mut self,
        entries: &[DictItem],
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        if entries
            .iter()
            .all(|entry| matches!(entry, DictItem::Entry { .. }))
        {
            let entries = entries
                .iter()
                .map(|entry| match entry {
                    DictItem::Entry { key, value } => Ok((
                        self.compile_maybe_type_scoped_expr(key, type_params)?,
                        self.compile_maybe_type_scoped_expr(value, type_params)?,
                    )),
                    DictItem::Unpack(_) => unreachable!("all dict entries are key/value pairs"),
                })
                .collect::<Result<Vec<_>, String>>()?;
            let dst = self.alloc_register();
            self.instructions.push(Instruction::BuildDict {
                dst,
                entries: entries.clone(),
            });
            if type_params.is_none() {
                for (key, value) in entries {
                    self.instructions.push(Instruction::Pop { src: key });
                    self.instructions.push(Instruction::Pop { src: value });
                }
            }
            return Ok(dst);
        }

        let dict = self.alloc_register();
        self.instructions.push(Instruction::BuildDict {
            dst: dict,
            entries: Vec::new(),
        });

        for entry in entries {
            match entry {
                DictItem::Entry { key, value } => {
                    let key = self.compile_maybe_type_scoped_expr(key, type_params)?;
                    let value = self.compile_maybe_type_scoped_expr(value, type_params)?;
                    self.instructions
                        .push(Instruction::DictSetItem { dict, key, value });
                    if type_params.is_none() {
                        self.instructions.push(Instruction::Pop { src: key });
                        self.instructions.push(Instruction::Pop { src: value });
                    }
                }
                DictItem::Unpack(expr) => {
                    let src = self.compile_maybe_type_scoped_expr(expr, type_params)?;
                    self.instructions
                        .push(Instruction::DictUpdate { dict, src });
                    if type_params.is_none() {
                        self.instructions.push(Instruction::Pop { src });
                    }
                }
            }
        }

        Ok(dict)
    }

    fn new_nested_function_compiler(&self) -> Compiler {
        Compiler {
            instructions: Vec::new(),
            next_register: 0,
            loop_contexts: Vec::new(),
            finally_contexts: Vec::new(),
            function_depth: self.function_depth + 1,
            async_function_depth: 0,
            global_names: HashSet::new(),
            nonlocal_names: HashSet::new(),
            local_names: HashSet::new(),
            private_class_name: self.private_class_name.clone(),
            class_scope_all_bindings: HashSet::new(),
            class_scope_prior_bindings: HashSet::new(),
            enclosing_function_bindings: self.enclosing_function_bindings.clone(),
            outer_scope_store_names: HashSet::new(),
            comprehension_walrus_enclosing_function_depth: None,
            static_block_depth: 0,
            optimize: self.optimize,
            function_first_lines: self.function_first_lines.clone(),
            function_line_sequences: self.function_line_sequences.clone(),
            function_position_columns: self.function_position_columns.clone(),
            function_is_lambdas: self.function_is_lambdas.clone(),
            generator_expression_line_sequences: self.generator_expression_line_sequences.clone(),
        }
    }

    fn new_class_body_compiler(&self) -> Compiler {
        Compiler {
            instructions: Vec::new(),
            next_register: 0,
            loop_contexts: Vec::new(),
            finally_contexts: Vec::new(),
            function_depth: 0,
            async_function_depth: 0,
            global_names: HashSet::new(),
            nonlocal_names: HashSet::new(),
            local_names: HashSet::new(),
            private_class_name: None,
            class_scope_all_bindings: HashSet::new(),
            class_scope_prior_bindings: HashSet::new(),
            enclosing_function_bindings: Vec::new(),
            outer_scope_store_names: HashSet::new(),
            comprehension_walrus_enclosing_function_depth: None,
            static_block_depth: 0,
            optimize: self.optimize,
            function_first_lines: self.function_first_lines.clone(),
            function_line_sequences: self.function_line_sequences.clone(),
            function_position_columns: self.function_position_columns.clone(),
            function_is_lambdas: self.function_is_lambdas.clone(),
            generator_expression_line_sequences: self.generator_expression_line_sequences.clone(),
        }
    }

    fn new_deferred_type_param_expr_compiler(&self) -> Compiler {
        Compiler {
            instructions: Vec::new(),
            next_register: 0,
            loop_contexts: Vec::new(),
            finally_contexts: Vec::new(),
            function_depth: 0,
            async_function_depth: self.async_function_depth,
            global_names: self.global_names.clone(),
            nonlocal_names: self.nonlocal_names.clone(),
            local_names: HashSet::new(),
            private_class_name: self.private_class_name.clone(),
            class_scope_all_bindings: self.class_scope_all_bindings.clone(),
            class_scope_prior_bindings: self.class_scope_prior_bindings.clone(),
            enclosing_function_bindings: self.enclosing_function_bindings.clone(),
            outer_scope_store_names: HashSet::new(),
            comprehension_walrus_enclosing_function_depth: None,
            static_block_depth: self.static_block_depth,
            optimize: self.optimize,
            function_first_lines: self.function_first_lines.clone(),
            function_line_sequences: self.function_line_sequences.clone(),
            function_position_columns: self.function_position_columns.clone(),
            function_is_lambdas: self.function_is_lambdas.clone(),
            generator_expression_line_sequences: self.generator_expression_line_sequences.clone(),
        }
    }

    fn comprehension_walrus_enclosing_function_depth(&self) -> usize {
        self.comprehension_walrus_enclosing_function_depth
            .unwrap_or(self.function_depth)
    }

    fn next_function_first_line(&self) -> usize {
        self.function_first_lines
            .borrow_mut()
            .pop_front()
            .unwrap_or(1)
    }

    fn next_function_line_sequence(&self, first_line: usize) -> Vec<usize> {
        self.function_line_sequences
            .borrow_mut()
            .pop_front()
            .filter(|sequence| !sequence.is_empty())
            .unwrap_or_else(|| vec![first_line])
    }

    fn next_function_position_columns(&self) -> Vec<Option<(usize, usize)>> {
        self.function_position_columns
            .borrow_mut()
            .pop_front()
            .unwrap_or_default()
    }

    fn next_function_is_lambda(&self) -> bool {
        self.function_is_lambdas
            .borrow_mut()
            .pop_front()
            .unwrap_or(false)
    }

    fn next_lambda_position_metadata(&self) -> (usize, Vec<usize>, Vec<Option<(usize, usize)>>) {
        if self
            .function_is_lambdas
            .borrow()
            .front()
            .copied()
            .unwrap_or(false)
        {
            let first_line = self.next_function_first_line();
            let line_sequence = self.next_function_line_sequence(first_line);
            let position_columns = self.next_function_position_columns();
            self.next_function_is_lambda();
            (first_line, line_sequence, position_columns)
        } else {
            (1, vec![1], Vec::new())
        }
    }

    fn next_generator_expression_line_sequence(&self) -> (usize, Vec<usize>) {
        self.generator_expression_line_sequences
            .borrow_mut()
            .pop_front()
            .filter(|(_, sequence)| !sequence.is_empty())
            .unwrap_or_else(|| (1, vec![1]))
    }

    fn compile_return_stmt(&mut self, value: Option<&Expr>) -> Result<(), String> {
        if self.function_depth == 0 {
            return Err("return outside function".to_string());
        }

        let src = value.map(|value| self.compile_expr(value)).transpose()?;
        self.compile_pending_finalizers()?;
        self.instructions.push(Instruction::Return { src });

        Ok(())
    }

    fn compile_raise_stmt(
        &mut self,
        value: Option<&Expr>,
        cause: Option<&Expr>,
    ) -> Result<(), String> {
        let src = value.map(|value| self.compile_expr(value)).transpose()?;
        let cause = cause.map(|cause| self.compile_expr(cause)).transpose()?;
        self.instructions.push(Instruction::Raise { src, cause });

        Ok(())
    }

    fn compile_yield_expr(&mut self, value: Option<&Expr>) -> Result<Register, String> {
        if self.function_depth == 0 {
            return Err("yield outside function".to_string());
        }

        let dst = self.alloc_register();
        let src = value.map(|value| self.compile_expr(value)).transpose()?;
        self.instructions.push(Instruction::Yield {
            src,
            resume_dst: Some(dst),
        });

        Ok(dst)
    }

    fn compile_yield_from_expr(&mut self, iterable: &Expr) -> Result<Register, String> {
        if self.function_depth == 0 {
            return Err("yield outside function".to_string());
        }

        let iterable = self.compile_expr(iterable)?;
        let iterator = self.alloc_register();
        self.instructions.push(Instruction::GetIter {
            dst: iterator,
            src: iterable,
        });

        let result = self.alloc_register();
        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let for_iter = self.instructions.len();
        self.instructions.push(Instruction::ForIterValue {
            iterator,
            dst: item,
            completion: result,
            track_yield_from: true,
            target: usize::MAX,
        });
        self.instructions.push(Instruction::Yield {
            src: Some(item),
            resume_dst: None,
        });
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;

        Ok(result)
    }

    fn compile_await_expr(&mut self, expr: &Expr) -> Result<Register, String> {
        if self.async_function_depth == 0 {
            return Err("'await' outside async function".to_string());
        }

        let src = self.compile_expr(expr)?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::Await { dst, src });
        Ok(dst)
    }

    fn compile_assert_stmt(
        &mut self,
        condition: &Expr,
        message: Option<&Expr>,
    ) -> Result<(), String> {
        let condition = self.compile_expr(condition)?;
        let message = message
            .map(|message| self.compile_expr(message))
            .transpose()?;
        self.instructions
            .push(Instruction::Assert { condition, message });

        Ok(())
    }

    fn compile_binary_instruction(
        &mut self,
        op: &BinaryOp,
        left: Register,
        right: Register,
        dst: Register,
    ) {
        match op {
            BinaryOp::Add => {
                self.instructions
                    .push(Instruction::Add { dst, left, right });
            }
            BinaryOp::Subtract => {
                self.instructions
                    .push(Instruction::Subtract { dst, left, right });
            }
            BinaryOp::Multiply => {
                self.instructions
                    .push(Instruction::Multiply { dst, left, right });
            }
            BinaryOp::MatrixMultiply => {
                self.instructions
                    .push(Instruction::MatrixMultiply { dst, left, right });
            }
            BinaryOp::TrueDivide => {
                self.instructions
                    .push(Instruction::TrueDivide { dst, left, right });
            }
            BinaryOp::FloorDivide => {
                self.instructions
                    .push(Instruction::FloorDivide { dst, left, right });
            }
            BinaryOp::Modulo => {
                self.instructions
                    .push(Instruction::Modulo { dst, left, right });
            }
            BinaryOp::Power => {
                self.instructions
                    .push(Instruction::Power { dst, left, right });
            }
            BinaryOp::BitOr => {
                self.instructions
                    .push(Instruction::BitOr { dst, left, right });
            }
            BinaryOp::BitXor => {
                self.instructions
                    .push(Instruction::BitXor { dst, left, right });
            }
            BinaryOp::BitAnd => {
                self.instructions
                    .push(Instruction::BitAnd { dst, left, right });
            }
            BinaryOp::LeftShift => {
                self.instructions
                    .push(Instruction::LeftShift { dst, left, right });
            }
            BinaryOp::RightShift => {
                self.instructions
                    .push(Instruction::RightShift { dst, left, right });
            }
        }
    }

    fn compile_augmented_binary_instruction(
        &mut self,
        op: &BinaryOp,
        left: Register,
        right: Register,
        dst: Register,
    ) {
        match op {
            BinaryOp::Add => {
                self.instructions
                    .push(Instruction::InPlaceAdd { dst, left, right });
            }
            BinaryOp::Subtract => {
                self.instructions
                    .push(Instruction::InPlaceSubtract { dst, left, right });
            }
            BinaryOp::Multiply => {
                self.instructions
                    .push(Instruction::InPlaceMultiply { dst, left, right });
            }
            BinaryOp::MatrixMultiply => {
                self.instructions
                    .push(Instruction::InPlaceMatrixMultiply { dst, left, right });
            }
            BinaryOp::FloorDivide => {
                self.instructions
                    .push(Instruction::InPlaceFloorDivide { dst, left, right });
            }
            BinaryOp::BitOr => {
                self.instructions
                    .push(Instruction::InPlaceBitOr { dst, left, right });
            }
            BinaryOp::BitXor => {
                self.instructions
                    .push(Instruction::InPlaceBitXor { dst, left, right });
            }
            BinaryOp::BitAnd => {
                self.instructions
                    .push(Instruction::InPlaceBitAnd { dst, left, right });
            }
            _ => self.compile_binary_instruction(op, left, right, dst),
        }
    }

    fn compile_break_stmt(&mut self) -> Result<(), String> {
        if self.loop_contexts.is_empty() {
            return Err("break outside loop".to_string());
        }

        self.compile_pending_finalizers()?;
        let jump_index = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });
        self.loop_contexts
            .last_mut()
            .expect("loop context exists after empty check")
            .break_jumps
            .push(jump_index);

        Ok(())
    }

    fn compile_continue_stmt(&mut self) -> Result<(), String> {
        let continue_target = self
            .loop_contexts
            .last()
            .map(|context| context.continue_target)
            .ok_or_else(|| "'continue' not properly in loop".to_string())?;

        self.compile_pending_finalizers()?;
        self.instructions.push(Instruction::Jump {
            target: continue_target,
        });

        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<Register, String> {
        match expr {
            Expr::Number(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::Number(*value),
                });
                Ok(dst)
            }
            Expr::BigInt(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::BigInt(parse_big_int_literal(value)?),
                });
                Ok(dst)
            }
            Expr::Float(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: float_value(parse_float_literal(value)?),
                });
                Ok(dst)
            }
            Expr::Imaginary(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: complex_value(0.0, parse_float_literal(value)?),
                });
                Ok(dst)
            }
            Expr::String(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::String(value.clone()),
                });
                Ok(dst)
            }
            Expr::Bytes(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: bytes_value(value.clone()),
                });
                Ok(dst)
            }
            Expr::JoinedString(parts) => self.compile_joined_string_expr(parts),
            Expr::TemplateString(parts) => self.compile_template_string_expr(parts),
            Expr::TemplateInterpolation {
                value,
                expression,
                conversion,
                format_spec,
            } => self.compile_template_interpolation_expr(
                value,
                expression,
                *conversion,
                format_spec.as_deref(),
            ),
            Expr::Bool(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::Bool(*value),
                });
                Ok(dst)
            }
            Expr::None => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::None,
                });
                Ok(dst)
            }
            Expr::Ellipsis => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::Ellipsis,
                });
                Ok(dst)
            }
            Expr::Binary { left, op, right } => {
                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;
                let dst = self.alloc_register();
                self.compile_binary_instruction(op, left, right, dst);

                Ok(dst)
            }
            Expr::Comparison { left, op, right } => {
                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;
                let dst = self.alloc_register();
                self.compile_comparison_instruction(op, left, right, dst);

                Ok(dst)
            }
            Expr::ChainedComparison { left, comparisons } => {
                self.compile_chained_comparison_expr(left, comparisons)
            }
            Expr::Unary { op, operand } => match op {
                UnaryOp::Not => self.compile_not_expr(operand),
                UnaryOp::Positive => self.compile_positive_expr(operand),
                UnaryOp::Negative => self.compile_negate_expr(operand),
                UnaryOp::Invert => self.compile_invert_expr(operand),
            },
            Expr::Logical { left, op, right } => self.compile_logical_expr(left, op, right),
            Expr::IfExpression {
                condition,
                then_branch,
                else_branch,
            } => self.compile_if_expression(condition, then_branch, else_branch),
            Expr::NamedExpr { name, value } => {
                let src = self.compile_expr(value)?;
                if self.outer_scope_store_names.contains(name) {
                    self.instructions.push(Instruction::StoreOuterName {
                        name: self.mangle_private_name(name),
                        src,
                    });
                } else {
                    self.emit_store_name(name, src);
                }
                Ok(src)
            }
            Expr::Yield { value } => self.compile_yield_expr(value.as_deref()),
            Expr::YieldFrom(iterable) => self.compile_yield_from_expr(iterable),
            Expr::Await(expr) => self.compile_await_expr(expr),
            Expr::Starred(_) => Err("starred expression cannot be used here".to_string()),
            Expr::List(elements) => self.compile_list_display(elements, None),
            Expr::ListComp { element, clauses } => self.compile_list_comp_expr(element, clauses),
            Expr::Set(elements) => self.compile_set_display(elements, None),
            Expr::FrozenSet(elements) => self.compile_frozen_set_display(elements),
            Expr::SetComp { element, clauses } => self.compile_set_comp_expr(element, clauses),
            Expr::GeneratorComp { element, clauses } => {
                self.compile_generator_comp_expr(element, clauses, None)
            }
            Expr::Tuple(elements) => self.compile_tuple_display(elements, None),
            Expr::Dict(entries) => self.compile_dict_display(entries, None),
            Expr::DictComp {
                key,
                value,
                clauses,
            } => self.compile_dict_comp_expr(key, value, clauses),
            Expr::DictUnpackComp { value, clauses } => {
                self.compile_dict_unpack_comp_expr(value, clauses)
            }
            Expr::Subscript { object, index } => {
                let object = self.compile_expr(object)?;
                let index = self.compile_expr(index)?;
                let dst = self.alloc_register();
                self.instructions
                    .push(Instruction::LoadSubscript { dst, object, index });
                Ok(dst)
            }
            Expr::SliceLiteral { start, stop, step } => {
                self.compile_slice_literal(start.as_deref(), stop.as_deref(), step.as_deref(), None)
            }
            Expr::Slice {
                object,
                start,
                stop,
                step,
            } => {
                let object = self.compile_expr(object)?;
                let start = start
                    .as_deref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let stop = stop
                    .as_deref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let step = step
                    .as_deref()
                    .map(|expr| self.compile_expr(expr))
                    .transpose()?;
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadSlice {
                    dst,
                    object,
                    start,
                    stop,
                    step,
                });
                Ok(dst)
            }
            Expr::Name(name) => {
                let dst = self.alloc_register();
                self.emit_load_name(dst, name);
                Ok(dst)
            }
            Expr::Attribute { object, name } => {
                let object = self.compile_expr(object)?;
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadAttribute {
                    dst,
                    object,
                    name: self.mangle_private_name(name),
                });
                Ok(dst)
            }
            Expr::Call { callee, args } => {
                let callee = self.compile_expr(callee)?;
                let args = args
                    .iter()
                    .map(|arg| self.compile_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let dst = self.alloc_register();

                self.instructions
                    .push(Instruction::Call { dst, callee, args });
                Ok(dst)
            }
            Expr::KeywordCall {
                callee,
                args,
                keywords,
            } => {
                let callee = self.compile_expr(callee)?;
                let args = args
                    .iter()
                    .map(|arg| self.compile_expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let keywords = keywords
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), self.compile_expr(value)?)))
                    .collect::<Result<Vec<_>, String>>()?;
                let dst = self.alloc_register();

                self.instructions.push(Instruction::CallKeyword {
                    dst,
                    callee,
                    args,
                    keywords,
                });
                Ok(dst)
            }
            Expr::UnpackCall {
                callee,
                args,
                keywords,
            } => {
                let callee = self.compile_expr(callee)?;
                let args = args
                    .iter()
                    .map(|arg| match arg {
                        CallArg::Expr(expr) => Ok(CallArgRegister::Value(self.compile_expr(expr)?)),
                        CallArg::Unpack(expr) => {
                            Ok(CallArgRegister::Unpack(self.compile_expr(expr)?))
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let keywords = keywords
                    .iter()
                    .map(|keyword| match keyword {
                        CallKeyword::Named(name, expr) => Ok(CallKeywordRegister::Named(
                            name.clone(),
                            self.compile_expr(expr)?,
                        )),
                        CallKeyword::Unpack(expr) => {
                            Ok(CallKeywordRegister::Unpack(self.compile_expr(expr)?))
                        }
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                let dst = self.alloc_register();

                self.instructions.push(Instruction::CallUnpack {
                    dst,
                    callee,
                    args,
                    keywords,
                });
                Ok(dst)
            }
            Expr::Lambda { params, body } => self.compile_lambda_expr(params, body, &[]),
        }
    }

    fn compile_joined_string_expr(&mut self, parts: &[FStringPart]) -> Result<Register, String> {
        let dst = self.alloc_register();
        self.instructions.push(Instruction::LoadConst {
            dst,
            value: Value::String(String::new()),
        });

        for part in parts {
            let part = match part {
                FStringPart::Literal(value) => {
                    let register = self.alloc_register();
                    self.instructions.push(Instruction::LoadConst {
                        dst: register,
                        value: Value::String(value.clone()),
                    });
                    register
                }
                FStringPart::Formatted {
                    value,
                    conversion,
                    format_spec,
                } => {
                    let src = self.compile_expr(value)?;
                    let format_spec = format_spec
                        .as_deref()
                        .map(|parts| self.compile_joined_string_expr(parts))
                        .transpose()?;
                    let register = self.alloc_register();
                    self.instructions.push(Instruction::FormatValue {
                        dst: register,
                        src,
                        conversion: conversion.map(f_string_conversion_to_bytecode),
                        format_spec,
                    });
                    register
                }
            };

            self.instructions.push(Instruction::Add {
                dst,
                left: dst,
                right: part,
            });
        }

        Ok(dst)
    }

    fn compile_template_string_expr(
        &mut self,
        parts: &[TemplateStringPart],
    ) -> Result<Register, String> {
        let mut compiled_parts = Vec::new();

        for part in parts {
            match part {
                TemplateStringPart::Literal(value) => {
                    compiled_parts.push(TemplatePartRegister::Literal(value.clone()));
                }
                TemplateStringPart::Interpolation {
                    value,
                    expression,
                    conversion,
                    format_spec,
                } => {
                    let value = self.compile_expr(value)?;
                    let format_spec = format_spec
                        .as_deref()
                        .map(|parts| self.compile_joined_string_expr(parts))
                        .transpose()?;
                    compiled_parts.push(TemplatePartRegister::Interpolation {
                        value,
                        expression: expression.clone(),
                        conversion: conversion.map(f_string_conversion_to_bytecode),
                        format_spec,
                    });
                }
            }
        }

        let dst = self.alloc_register();
        self.instructions.push(Instruction::BuildTemplate {
            dst,
            parts: compiled_parts,
        });
        Ok(dst)
    }

    fn compile_template_interpolation_expr(
        &mut self,
        value: &Expr,
        expression: &str,
        conversion: Option<FStringConversion>,
        format_spec: Option<&[FStringPart]>,
    ) -> Result<Register, String> {
        let value = self.compile_expr(value)?;
        let format_spec = format_spec
            .map(|parts| self.compile_joined_string_expr(parts))
            .transpose()?;
        let dst = self.alloc_register();
        self.instructions
            .push(Instruction::BuildTemplateInterpolation {
                dst,
                value,
                expression: expression.to_string(),
                conversion: conversion.map(f_string_conversion_to_bytecode),
                format_spec,
            });
        Ok(dst)
    }

    fn compile_if_expression(
        &mut self,
        condition: &Expr,
        then_branch: &Expr,
        else_branch: &Expr,
    ) -> Result<Register, String> {
        let condition = self.compile_expr(condition)?;
        let dst = self.alloc_register();

        let jump_to_else = self.instructions.len();
        self.instructions.push(Instruction::JumpIfFalse {
            condition,
            target: usize::MAX,
        });

        let then_branch = self.compile_expr(then_branch)?;
        self.instructions.push(Instruction::Move {
            dst,
            src: then_branch,
        });

        let jump_to_end = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: usize::MAX });

        let else_target = self.instructions.len();
        self.patch_jump_target(jump_to_else, else_target)?;

        let else_branch = self.compile_expr(else_branch)?;
        self.instructions.push(Instruction::Move {
            dst,
            src: else_branch,
        });

        let end_target = self.instructions.len();
        self.patch_jump_target(jump_to_end, end_target)?;

        Ok(dst)
    }

    fn compile_list_comp_expr(
        &mut self,
        element: &Expr,
        clauses: &[ComprehensionClause],
    ) -> Result<Register, String> {
        if clauses.is_empty() {
            return Err("list comprehension requires at least one for clause".to_string());
        }
        self.reject_async_comprehension_outside_async_function(clauses)?;
        self.reject_await_comprehension_outside_async_function(&[element], clauses)?;
        if comprehension_inner_contains_yield(&[element], clauses) {
            return Err("yield inside list comprehension".to_string());
        }
        let is_async = generator_comprehension_is_async(element, clauses);

        let first_iter = self.compile_expr(&clauses[0].iter)?;
        let comprehension_enclosing_depth = self.comprehension_walrus_enclosing_function_depth();
        let mut list_compiler = self.new_nested_function_compiler();
        list_compiler.async_function_depth = self.async_function_depth;
        list_compiler.comprehension_walrus_enclosing_function_depth =
            Some(comprehension_enclosing_depth);
        list_compiler.configure_comprehension_scope(
            &[element],
            clauses,
            comprehension_enclosing_depth,
            &self.global_names,
            &self.nonlocal_names,
        );
        let list = list_compiler.alloc_register();
        list_compiler.instructions.push(Instruction::BuildList {
            dst: list,
            items: Vec::new(),
        });
        list_compiler.compile_list_comp_clause_from_first_iter(list, element, clauses, 0, ".0")?;
        list_compiler
            .instructions
            .push(Instruction::Return { src: Some(list) });

        let callee = self.compile_comprehension_function(
            "<listcomp>",
            list_compiler.instructions,
            is_async,
        )?;
        Ok(self.emit_comprehension_call(callee, first_iter, is_async))
    }

    fn compile_type_scoped_list_comp_expr(
        &mut self,
        element: &Expr,
        clauses: &[ComprehensionClause],
        type_params: &[(String, Register)],
    ) -> Result<Register, String> {
        if clauses.is_empty() {
            return Err("list comprehension requires at least one for clause".to_string());
        }
        self.reject_async_comprehension_outside_async_function(clauses)?;
        self.reject_await_comprehension_outside_async_function(&[element], clauses)?;
        if comprehension_inner_contains_yield(&[element], clauses) {
            return Err("yield inside list comprehension".to_string());
        }
        let is_async = generator_comprehension_is_async(element, clauses);

        let first_iter = self.compile_type_scoped_expr(&clauses[0].iter, type_params)?;
        let comprehension_enclosing_depth = self.comprehension_walrus_enclosing_function_depth();
        let mut list_compiler = self.new_nested_function_compiler();
        list_compiler.async_function_depth = self.async_function_depth;
        list_compiler.comprehension_walrus_enclosing_function_depth =
            Some(comprehension_enclosing_depth);
        list_compiler.configure_comprehension_scope(
            &[element],
            clauses,
            comprehension_enclosing_depth,
            &self.global_names,
            &self.nonlocal_names,
        );
        let list = list_compiler.alloc_register();
        list_compiler.instructions.push(Instruction::BuildList {
            dst: list,
            items: Vec::new(),
        });
        list_compiler.compile_list_comp_clause_from_first_iter(list, element, clauses, 0, ".0")?;
        list_compiler
            .instructions
            .push(Instruction::Return { src: Some(list) });

        let params = FunctionParams {
            positional: vec![Param {
                name: ".0".to_string(),
                annotation: None,
                default: None,
                type_comment: None,
            }],
            ..FunctionParams::default()
        };
        let callee = self.compile_function_value(
            "<listcomp>",
            &[],
            type_params,
            &params,
            None,
            None,
            list_compiler.instructions,
            false,
            is_async,
            1,
            vec![1],
            Vec::new(),
        )?;
        Ok(self.emit_comprehension_call(callee, first_iter, is_async))
    }

    fn configure_comprehension_scope(
        &mut self,
        head_exprs: &[&Expr],
        clauses: &[ComprehensionClause],
        enclosing_function_depth: usize,
        enclosing_global_names: &HashSet<String>,
        enclosing_nonlocal_names: &HashSet<String>,
    ) {
        self.local_names.insert(".0".to_string());
        for clause in clauses {
            collect_comprehension_target_locals(&clause.target, &mut self.local_names);
        }
        for name in comprehension_named_expression_bindings(head_exprs, clauses) {
            if enclosing_global_names.contains(&name) || enclosing_function_depth == 0 {
                self.global_names.insert(name);
            } else if enclosing_nonlocal_names.contains(&name) {
                self.nonlocal_names.insert(name);
            } else {
                self.outer_scope_store_names.insert(name);
            }
        }
    }

    fn compile_comprehension_function(
        &mut self,
        name: &str,
        body: Vec<Instruction>,
        is_async: bool,
    ) -> Result<Register, String> {
        let params = FunctionParams {
            positional: vec![Param {
                name: ".0".to_string(),
                annotation: None,
                default: None,
                type_comment: None,
            }],
            ..FunctionParams::default()
        };
        self.compile_function_value(
            name,
            &[],
            &[],
            &params,
            None,
            None,
            body,
            false,
            is_async,
            1,
            vec![1],
            Vec::new(),
        )
    }

    fn emit_comprehension_call(
        &mut self,
        callee: Register,
        first_iter: Register,
        is_async: bool,
    ) -> Register {
        let dst = self.alloc_register();
        let call_dst = if is_async { self.alloc_register() } else { dst };
        self.instructions.push(Instruction::Call {
            dst: call_dst,
            callee,
            args: vec![first_iter],
        });
        if is_async {
            self.instructions
                .push(Instruction::Await { dst, src: call_dst });
            self.instructions.push(Instruction::Pop { src: call_dst });
        }
        self.instructions.push(Instruction::Pop { src: first_iter });
        self.instructions.push(Instruction::Pop { src: callee });
        dst
    }

    fn reject_async_comprehension_outside_async_function(
        &self,
        clauses: &[ComprehensionClause],
    ) -> Result<(), String> {
        if self.async_function_depth == 0 && clauses.iter().any(|clause| clause.is_async) {
            Err("asynchronous comprehension outside async function".to_string())
        } else {
            Ok(())
        }
    }

    fn reject_await_comprehension_outside_async_function(
        &self,
        result_exprs: &[&Expr],
        clauses: &[ComprehensionClause],
    ) -> Result<(), String> {
        if self.async_function_depth == 0
            && comprehension_inner_contains_await(result_exprs, clauses)
        {
            Err("asynchronous comprehension outside async function".to_string())
        } else {
            Ok(())
        }
    }

    fn compile_comprehension_iter(
        &mut self,
        clause: &ComprehensionClause,
        iterable: Register,
    ) -> (usize, usize, Register) {
        let iterator = self.alloc_register();
        if clause.is_async {
            self.instructions.push(Instruction::GetAsyncIter {
                dst: iterator,
                src: iterable,
            });
        } else {
            self.instructions.push(Instruction::GetIter {
                dst: iterator,
                src: iterable,
            });
        }

        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let iter_instruction = self.instructions.len();
        if clause.is_async {
            self.instructions.push(Instruction::AsyncForIter {
                iterator,
                dst: item,
                target: usize::MAX,
            });
        } else {
            self.instructions.push(Instruction::ForIter {
                iterator,
                dst: item,
                target: usize::MAX,
            });
        }

        (loop_start, iter_instruction, item)
    }

    fn compile_list_extend_from_expr(
        &mut self,
        list: Register,
        value: &Expr,
    ) -> Result<(), String> {
        let iterable = self.compile_expr(value)?;
        let iterator = self.alloc_register();
        self.instructions.push(Instruction::GetIter {
            dst: iterator,
            src: iterable,
        });

        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let for_iter = self.instructions.len();
        self.instructions.push(Instruction::ForIter {
            iterator,
            dst: item,
            target: usize::MAX,
        });
        self.instructions
            .push(Instruction::ListAppend { list, item });
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;

        Ok(())
    }

    fn compile_list_comp_clause_from_first_iter(
        &mut self,
        list: Register,
        element: &Expr,
        clauses: &[ComprehensionClause],
        index: usize,
        first_iter_name: &str,
    ) -> Result<(), String> {
        let clause = clauses
            .get(index)
            .ok_or_else(|| format!("missing list comprehension clause at index {index}"))?;
        let iterable = if index == 0 {
            let dst = self.alloc_register();
            self.emit_load_name(dst, first_iter_name);
            dst
        } else {
            self.compile_expr(&clause.iter)?
        };
        let (loop_start, for_iter, item) = self.compile_comprehension_iter(clause, iterable);
        self.compile_store_target(&clause.target, item)?;

        let mut false_jumps = Vec::new();
        for condition in &clause.ifs {
            let condition = self.compile_expr(condition)?;
            let false_jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            });
            false_jumps.push(false_jump);
        }

        if index + 1 == clauses.len() {
            match element {
                Expr::Starred(value) => self.compile_list_extend_from_expr(list, value)?,
                element => {
                    let item = self.compile_expr(element)?;
                    self.instructions
                        .push(Instruction::ListAppend { list, item });
                }
            }
        } else {
            self.compile_list_comp_clause_from_first_iter(
                list,
                element,
                clauses,
                index + 1,
                first_iter_name,
            )?;
        }

        let continue_target = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;
        for false_jump in false_jumps {
            self.patch_jump_target(false_jump, continue_target)?;
        }

        Ok(())
    }

    fn compile_set_comp_expr(
        &mut self,
        element: &Expr,
        clauses: &[ComprehensionClause],
    ) -> Result<Register, String> {
        if clauses.is_empty() {
            return Err("set comprehension requires at least one for clause".to_string());
        }
        self.reject_async_comprehension_outside_async_function(clauses)?;
        self.reject_await_comprehension_outside_async_function(&[element], clauses)?;
        if comprehension_inner_contains_yield(&[element], clauses) {
            return Err("yield inside set comprehension".to_string());
        }
        let is_async = generator_comprehension_is_async(element, clauses);

        let first_iter = self.compile_expr(&clauses[0].iter)?;
        let comprehension_enclosing_depth = self.comprehension_walrus_enclosing_function_depth();
        let mut set_compiler = self.new_nested_function_compiler();
        set_compiler.async_function_depth = self.async_function_depth;
        set_compiler.comprehension_walrus_enclosing_function_depth =
            Some(comprehension_enclosing_depth);
        set_compiler.configure_comprehension_scope(
            &[element],
            clauses,
            comprehension_enclosing_depth,
            &self.global_names,
            &self.nonlocal_names,
        );
        let set = set_compiler.alloc_register();
        set_compiler.instructions.push(Instruction::BuildSet {
            dst: set,
            items: Vec::new(),
        });
        set_compiler.compile_set_comp_clause_from_first_iter(set, element, clauses, 0, ".0")?;
        set_compiler
            .instructions
            .push(Instruction::Return { src: Some(set) });

        let callee =
            self.compile_comprehension_function("<setcomp>", set_compiler.instructions, is_async)?;
        Ok(self.emit_comprehension_call(callee, first_iter, is_async))
    }

    fn compile_set_comp_clause_from_first_iter(
        &mut self,
        set: Register,
        element: &Expr,
        clauses: &[ComprehensionClause],
        index: usize,
        first_iter_name: &str,
    ) -> Result<(), String> {
        let clause = clauses
            .get(index)
            .ok_or_else(|| format!("missing set comprehension clause at index {index}"))?;
        let iterable = if index == 0 {
            let dst = self.alloc_register();
            self.emit_load_name(dst, first_iter_name);
            dst
        } else {
            self.compile_expr(&clause.iter)?
        };
        let (loop_start, for_iter, item) = self.compile_comprehension_iter(clause, iterable);
        self.compile_store_target(&clause.target, item)?;

        let mut false_jumps = Vec::new();
        for condition in &clause.ifs {
            let condition = self.compile_expr(condition)?;
            let false_jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            });
            false_jumps.push(false_jump);
        }

        if index + 1 == clauses.len() {
            match element {
                Expr::Starred(value) => self.compile_set_update_from_expr(set, value)?,
                element => {
                    let item = self.compile_expr(element)?;
                    self.instructions.push(Instruction::SetAdd { set, item });
                }
            }
        } else {
            self.compile_set_comp_clause_from_first_iter(
                set,
                element,
                clauses,
                index + 1,
                first_iter_name,
            )?;
        }

        let continue_target = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;
        for false_jump in false_jumps {
            self.patch_jump_target(false_jump, continue_target)?;
        }

        Ok(())
    }

    fn compile_set_update_from_expr(&mut self, set: Register, value: &Expr) -> Result<(), String> {
        let iterable = self.compile_expr(value)?;
        let iterator = self.alloc_register();
        self.instructions.push(Instruction::GetIter {
            dst: iterator,
            src: iterable,
        });

        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let for_iter = self.instructions.len();
        self.instructions.push(Instruction::ForIter {
            iterator,
            dst: item,
            target: usize::MAX,
        });
        self.instructions.push(Instruction::SetAdd { set, item });
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;

        Ok(())
    }

    fn compile_generator_comp_expr(
        &mut self,
        element: &Expr,
        clauses: &[ComprehensionClause],
        type_params: Option<&[(String, Register)]>,
    ) -> Result<Register, String> {
        if clauses.is_empty() {
            return Err("generator expression requires at least one for clause".to_string());
        }
        if comprehension_inner_contains_yield(&[element], clauses) {
            return Err("yield inside generator expression".to_string());
        }
        let is_async = generator_comprehension_is_async(element, clauses);
        let (first_line, line_sequence) = self.next_generator_expression_line_sequence();

        let first_iter = self.compile_maybe_type_scoped_expr(&clauses[0].iter, type_params)?;
        let comprehension_enclosing_depth = self.comprehension_walrus_enclosing_function_depth();
        let mut generator_compiler = self.new_nested_function_compiler();
        generator_compiler.comprehension_walrus_enclosing_function_depth =
            Some(comprehension_enclosing_depth);
        if is_async {
            generator_compiler.async_function_depth = self.async_function_depth + 1;
        }
        generator_compiler.local_names.insert(".0".to_string());
        for name in comprehension_named_expression_bindings(&[element], clauses) {
            if self.name_is_declared_global(&name) || comprehension_enclosing_depth == 0 {
                generator_compiler.global_names.insert(name);
            } else if self.name_is_declared_nonlocal(&name) {
                generator_compiler.nonlocal_names.insert(name);
            } else {
                generator_compiler.outer_scope_store_names.insert(name);
            }
        }
        generator_compiler.compile_generator_comp_clause(element, clauses, 0, ".0")?;
        generator_compiler
            .instructions
            .push(Instruction::Return { src: None });

        let params = FunctionParams {
            positional: vec![Param {
                name: ".0".to_string(),
                annotation: None,
                default: None,
                type_comment: None,
            }],
            ..FunctionParams::default()
        };
        let callee = self.compile_function_value(
            "<genexpr>",
            &[],
            type_params.unwrap_or(&[]),
            &params,
            None,
            None,
            generator_compiler.instructions,
            true,
            is_async,
            first_line,
            line_sequence,
            Vec::new(),
        )?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::Call {
            dst,
            callee,
            args: vec![first_iter],
        });

        Ok(dst)
    }

    fn compile_generator_comp_clause(
        &mut self,
        element: &Expr,
        clauses: &[ComprehensionClause],
        index: usize,
        first_iter_name: &str,
    ) -> Result<(), String> {
        let clause = clauses
            .get(index)
            .ok_or_else(|| format!("missing generator expression clause at index {index}"))?;
        let iterable = if index == 0 {
            let dst = self.alloc_register();
            self.emit_load_name(dst, first_iter_name);
            dst
        } else {
            self.compile_expr(&clause.iter)?
        };
        let (loop_start, for_iter, item) = self.compile_comprehension_iter(clause, iterable);
        self.compile_store_target(&clause.target, item)?;

        let mut false_jumps = Vec::new();
        for condition in &clause.ifs {
            let condition = self.compile_expr(condition)?;
            let false_jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            });
            false_jumps.push(false_jump);
        }

        if index + 1 == clauses.len() {
            match element {
                Expr::Starred(value) => self.compile_starred_generator_yield(value)?,
                element => {
                    let item = self.compile_expr(element)?;
                    self.instructions.push(Instruction::Yield {
                        src: Some(item),
                        resume_dst: None,
                    });
                }
            }
        } else {
            self.compile_generator_comp_clause(element, clauses, index + 1, first_iter_name)?;
        }

        let continue_target = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;
        for false_jump in false_jumps {
            self.patch_jump_target(false_jump, continue_target)?;
        }

        Ok(())
    }

    fn compile_starred_generator_yield(&mut self, value: &Expr) -> Result<(), String> {
        let iterable = self.compile_expr(value)?;
        let iterator = self.alloc_register();
        self.instructions.push(Instruction::GetIter {
            dst: iterator,
            src: iterable,
        });

        let loop_start = self.instructions.len();
        let item = self.alloc_register();
        let for_iter = self.instructions.len();
        self.instructions.push(Instruction::ForIter {
            iterator,
            dst: item,
            target: usize::MAX,
        });
        self.instructions.push(Instruction::Yield {
            src: Some(item),
            resume_dst: None,
        });
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;

        Ok(())
    }

    fn compile_dict_comp_expr(
        &mut self,
        key: &Expr,
        value: &Expr,
        clauses: &[ComprehensionClause],
    ) -> Result<Register, String> {
        if clauses.is_empty() {
            return Err("dict comprehension requires at least one for clause".to_string());
        }
        self.reject_async_comprehension_outside_async_function(clauses)?;
        self.reject_await_comprehension_outside_async_function(&[key, value], clauses)?;
        if comprehension_inner_contains_yield(&[key, value], clauses) {
            return Err("yield inside dict comprehension".to_string());
        }
        let is_async = generator_comprehension_is_async(key, clauses)
            || comprehension_inner_contains_await(&[value], clauses);

        let first_iter = self.compile_expr(&clauses[0].iter)?;
        let comprehension_enclosing_depth = self.comprehension_walrus_enclosing_function_depth();
        let mut dict_compiler = self.new_nested_function_compiler();
        dict_compiler.async_function_depth = self.async_function_depth;
        dict_compiler.comprehension_walrus_enclosing_function_depth =
            Some(comprehension_enclosing_depth);
        dict_compiler.configure_comprehension_scope(
            &[key, value],
            clauses,
            comprehension_enclosing_depth,
            &self.global_names,
            &self.nonlocal_names,
        );
        let dict = dict_compiler.alloc_register();
        dict_compiler.instructions.push(Instruction::BuildDict {
            dst: dict,
            entries: Vec::new(),
        });
        dict_compiler
            .compile_dict_comp_clause_from_first_iter(dict, key, value, clauses, 0, ".0")?;
        dict_compiler
            .instructions
            .push(Instruction::Return { src: Some(dict) });

        let callee = self.compile_comprehension_function(
            "<dictcomp>",
            dict_compiler.instructions,
            is_async,
        )?;
        Ok(self.emit_comprehension_call(callee, first_iter, is_async))
    }

    fn compile_dict_comp_clause_from_first_iter(
        &mut self,
        dict: Register,
        key: &Expr,
        value: &Expr,
        clauses: &[ComprehensionClause],
        index: usize,
        first_iter_name: &str,
    ) -> Result<(), String> {
        let clause = clauses
            .get(index)
            .ok_or_else(|| format!("missing dict comprehension clause at index {index}"))?;
        let iterable = if index == 0 {
            let dst = self.alloc_register();
            self.emit_load_name(dst, first_iter_name);
            dst
        } else {
            self.compile_expr(&clause.iter)?
        };
        let (loop_start, for_iter, item) = self.compile_comprehension_iter(clause, iterable);
        self.compile_store_target(&clause.target, item)?;

        let mut false_jumps = Vec::new();
        for condition in &clause.ifs {
            let condition = self.compile_expr(condition)?;
            let false_jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            });
            false_jumps.push(false_jump);
        }

        if index + 1 == clauses.len() {
            let key = self.compile_expr(key)?;
            let value = self.compile_expr(value)?;
            self.instructions
                .push(Instruction::DictSetItem { dict, key, value });
        } else {
            self.compile_dict_comp_clause_from_first_iter(
                dict,
                key,
                value,
                clauses,
                index + 1,
                first_iter_name,
            )?;
        }

        let continue_target = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;
        for false_jump in false_jumps {
            self.patch_jump_target(false_jump, continue_target)?;
        }

        Ok(())
    }

    fn compile_dict_unpack_comp_expr(
        &mut self,
        value: &Expr,
        clauses: &[ComprehensionClause],
    ) -> Result<Register, String> {
        if clauses.is_empty() {
            return Err(
                "dict unpacking comprehension requires at least one for clause".to_string(),
            );
        }
        self.reject_async_comprehension_outside_async_function(clauses)?;
        self.reject_await_comprehension_outside_async_function(&[value], clauses)?;
        if comprehension_inner_contains_yield(&[value], clauses) {
            return Err("yield inside dict comprehension".to_string());
        }
        let is_async = generator_comprehension_is_async(value, clauses);

        let first_iter = self.compile_expr(&clauses[0].iter)?;
        let comprehension_enclosing_depth = self.comprehension_walrus_enclosing_function_depth();
        let mut dict_compiler = self.new_nested_function_compiler();
        dict_compiler.async_function_depth = self.async_function_depth;
        dict_compiler.comprehension_walrus_enclosing_function_depth =
            Some(comprehension_enclosing_depth);
        dict_compiler.configure_comprehension_scope(
            &[value],
            clauses,
            comprehension_enclosing_depth,
            &self.global_names,
            &self.nonlocal_names,
        );
        let dict = dict_compiler.alloc_register();
        dict_compiler.instructions.push(Instruction::BuildDict {
            dst: dict,
            entries: Vec::new(),
        });
        dict_compiler
            .compile_dict_unpack_comp_clause_from_first_iter(dict, value, clauses, 0, ".0")?;
        dict_compiler
            .instructions
            .push(Instruction::Return { src: Some(dict) });

        let callee = self.compile_comprehension_function(
            "<dictcomp>",
            dict_compiler.instructions,
            is_async,
        )?;
        Ok(self.emit_comprehension_call(callee, first_iter, is_async))
    }

    fn compile_dict_unpack_comp_clause_from_first_iter(
        &mut self,
        dict: Register,
        value: &Expr,
        clauses: &[ComprehensionClause],
        index: usize,
        first_iter_name: &str,
    ) -> Result<(), String> {
        let clause = clauses.get(index).ok_or_else(|| {
            format!("missing dict unpacking comprehension clause at index {index}")
        })?;
        let iterable = if index == 0 {
            let dst = self.alloc_register();
            self.emit_load_name(dst, first_iter_name);
            dst
        } else {
            self.compile_expr(&clause.iter)?
        };
        let (loop_start, for_iter, item) = self.compile_comprehension_iter(clause, iterable);
        self.compile_store_target(&clause.target, item)?;

        let mut false_jumps = Vec::new();
        for condition in &clause.ifs {
            let condition = self.compile_expr(condition)?;
            let false_jump = self.instructions.len();
            self.instructions.push(Instruction::JumpIfFalse {
                condition,
                target: usize::MAX,
            });
            false_jumps.push(false_jump);
        }

        if index + 1 == clauses.len() {
            let src = self.compile_expr(value)?;
            self.instructions
                .push(Instruction::DictUpdate { dict, src });
        } else {
            self.compile_dict_unpack_comp_clause_from_first_iter(
                dict,
                value,
                clauses,
                index + 1,
                first_iter_name,
            )?;
        }

        let continue_target = self.instructions.len();
        self.instructions
            .push(Instruction::Jump { target: loop_start });

        let end_target = self.instructions.len();
        self.patch_jump_target(for_iter, end_target)?;
        for false_jump in false_jumps {
            self.patch_jump_target(false_jump, continue_target)?;
        }

        Ok(())
    }

    fn compile_not_expr(&mut self, operand: &Expr) -> Result<Register, String> {
        let src = self.compile_expr(operand)?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::Not { dst, src });
        Ok(dst)
    }

    fn compile_chained_comparison_expr(
        &mut self,
        left: &Expr,
        comparisons: &[(ComparisonOp, Expr)],
    ) -> Result<Register, String> {
        if comparisons.is_empty() {
            return Err("chained comparison must contain at least one comparison".to_string());
        }

        let mut left = self.compile_expr(left)?;
        let dst = self.alloc_register();
        let mut false_jumps = Vec::new();

        for (index, (op, right_expr)) in comparisons.iter().enumerate() {
            let right = self.compile_expr(right_expr)?;
            self.compile_comparison_instruction(op, left, right, dst);

            if index + 1 < comparisons.len() {
                let false_jump = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse {
                    condition: dst,
                    target: usize::MAX,
                });
                false_jumps.push(false_jump);
                left = right;
            }
        }

        let end_target = self.instructions.len();
        for false_jump in false_jumps {
            self.patch_jump_target(false_jump, end_target)?;
        }

        Ok(dst)
    }

    fn compile_comparison_instruction(
        &mut self,
        op: &ComparisonOp,
        left: Register,
        right: Register,
        dst: Register,
    ) {
        match op {
            ComparisonOp::Equal => {
                self.instructions
                    .push(Instruction::Equal { dst, left, right });
            }
            ComparisonOp::NotEqual => {
                self.instructions
                    .push(Instruction::NotEqual { dst, left, right });
            }
            ComparisonOp::Less => {
                self.instructions
                    .push(Instruction::Less { dst, left, right });
            }
            ComparisonOp::LessEqual => {
                self.instructions
                    .push(Instruction::LessEqual { dst, left, right });
            }
            ComparisonOp::Greater => {
                self.instructions
                    .push(Instruction::Greater { dst, left, right });
            }
            ComparisonOp::GreaterEqual => {
                self.instructions
                    .push(Instruction::GreaterEqual { dst, left, right });
            }
            ComparisonOp::In => {
                self.instructions.push(Instruction::Contains {
                    dst,
                    needle: left,
                    haystack: right,
                });
            }
            ComparisonOp::NotIn => {
                self.instructions.push(Instruction::Contains {
                    dst,
                    needle: left,
                    haystack: right,
                });
                self.instructions.push(Instruction::Not { dst, src: dst });
            }
            ComparisonOp::Is => {
                self.instructions.push(Instruction::Is { dst, left, right });
            }
            ComparisonOp::IsNot => {
                self.instructions.push(Instruction::Is { dst, left, right });
                self.instructions.push(Instruction::Not { dst, src: dst });
            }
        }
    }

    fn compile_negate_expr(&mut self, operand: &Expr) -> Result<Register, String> {
        if let Some(value) = negated_literal_constant_value(operand)? {
            return Ok(self.compile_const_value(value));
        }

        let src = self.compile_expr(operand)?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::Negate { dst, src });
        Ok(dst)
    }

    fn compile_const_value(&mut self, value: Value) -> Register {
        let dst = self.alloc_register();
        self.instructions
            .push(Instruction::LoadConst { dst, value });
        dst
    }

    fn compile_positive_expr(&mut self, operand: &Expr) -> Result<Register, String> {
        let src = self.compile_expr(operand)?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::Positive { dst, src });
        Ok(dst)
    }

    fn compile_invert_expr(&mut self, operand: &Expr) -> Result<Register, String> {
        let src = self.compile_expr(operand)?;
        let dst = self.alloc_register();
        self.instructions.push(Instruction::Invert { dst, src });
        Ok(dst)
    }

    fn compile_logical_expr(
        &mut self,
        left: &Expr,
        op: &LogicalOp,
        right: &Expr,
    ) -> Result<Register, String> {
        let expr = Expr::Logical {
            left: Box::new(left.clone()),
            op: op.clone(),
            right: Box::new(right.clone()),
        };
        let dst = self.alloc_register();
        let mut end_jumps = Vec::new();
        self.compile_logical_value_into(&expr, dst, &mut end_jumps)?;
        self.patch_jumps_to_here(&end_jumps)?;
        Ok(dst)
    }

    fn compile_logical_value_into(
        &mut self,
        expr: &Expr,
        dst: Register,
        end_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        match expr {
            Expr::Logical {
                left,
                op: LogicalOp::And,
                right,
            } => {
                let mut left_true = Vec::new();
                let mut left_false = Vec::new();
                self.compile_logical_condition(left, &mut left_true, &mut left_false)?;
                self.patch_logical_jumps_to_here(&left_true)?;
                self.compile_logical_value_into(right, dst, end_jumps)?;
                self.emit_logical_value_blocks(dst, &left_false, end_jumps)
            }
            Expr::Logical {
                left,
                op: LogicalOp::Or,
                right,
            } => {
                let mut left_true = Vec::new();
                let mut left_false = Vec::new();
                self.compile_logical_condition(left, &mut left_true, &mut left_false)?;
                self.patch_logical_jumps_to_here(&left_false)?;
                self.compile_logical_value_into(right, dst, end_jumps)?;
                self.emit_logical_value_blocks(dst, &left_true, end_jumps)
            }
            expr => {
                let src = self.compile_expr(expr)?;
                self.instructions.push(Instruction::Move { dst, src });
                let jump = self.instructions.len();
                self.instructions
                    .push(Instruction::Jump { target: usize::MAX });
                end_jumps.push(jump);
                Ok(())
            }
        }
    }

    fn compile_logical_condition(
        &mut self,
        expr: &Expr,
        true_jumps: &mut Vec<LogicalJump>,
        false_jumps: &mut Vec<LogicalJump>,
    ) -> Result<(), String> {
        match expr {
            Expr::Logical {
                left,
                op: LogicalOp::And,
                right,
            } => {
                let mut left_true = Vec::new();
                self.compile_logical_condition(left, &mut left_true, false_jumps)?;
                self.patch_logical_jumps_to_here(&left_true)?;
                self.compile_logical_condition(right, true_jumps, false_jumps)
            }
            Expr::Logical {
                left,
                op: LogicalOp::Or,
                right,
            } => {
                let mut left_false = Vec::new();
                self.compile_logical_condition(left, true_jumps, &mut left_false)?;
                self.patch_logical_jumps_to_here(&left_false)?;
                self.compile_logical_condition(right, true_jumps, false_jumps)
            }
            expr => {
                let src = self.compile_expr(expr)?;
                let jump_to_false = self.instructions.len();
                self.instructions.push(Instruction::JumpIfFalse {
                    condition: src,
                    target: usize::MAX,
                });
                false_jumps.push(LogicalJump {
                    instruction: jump_to_false,
                    src,
                });

                let jump_to_true = self.instructions.len();
                self.instructions
                    .push(Instruction::Jump { target: usize::MAX });
                true_jumps.push(LogicalJump {
                    instruction: jump_to_true,
                    src,
                });
                Ok(())
            }
        }
    }

    fn emit_logical_value_blocks(
        &mut self,
        dst: Register,
        jumps: &[LogicalJump],
        end_jumps: &mut Vec<usize>,
    ) -> Result<(), String> {
        for jump in jumps {
            self.patch_jump_target(jump.instruction, self.instructions.len())?;
            self.instructions
                .push(Instruction::Move { dst, src: jump.src });
            let end_jump = self.instructions.len();
            self.instructions
                .push(Instruction::Jump { target: usize::MAX });
            end_jumps.push(end_jump);
        }
        Ok(())
    }

    fn patch_logical_jumps_to_here(&mut self, jumps: &[LogicalJump]) -> Result<(), String> {
        self.patch_jumps_to_here(
            &jumps
                .iter()
                .map(|jump| jump.instruction)
                .collect::<Vec<_>>(),
        )
    }

    fn patch_jumps_to_here(&mut self, jumps: &[usize]) -> Result<(), String> {
        let target = self.instructions.len();
        for jump in jumps {
            self.patch_jump_target(*jump, target)?;
        }
        Ok(())
    }

    fn alloc_register(&mut self) -> Register {
        let register = self.next_register;
        self.next_register += 1;
        register
    }

    fn mangle_private_name(&self, name: &str) -> String {
        mangle_private_name(self.private_class_name.as_deref(), name)
    }

    fn emit_load_name(&mut self, dst: Register, name: &str) {
        if name == "__debug__" {
            self.instructions.push(Instruction::LoadConst {
                dst,
                value: Value::Bool(self.optimize <= 0),
            });
            return;
        }

        let emitted_name = self.mangle_private_name(name);
        if self.name_is_declared_nonlocal(name) {
            self.instructions.push(Instruction::LoadNonlocal {
                dst,
                name: emitted_name,
            });
        } else if self.name_is_declared_global(name) {
            self.instructions.push(Instruction::LoadGlobal {
                dst,
                name: emitted_name,
            });
        } else if self.function_depth > 0 && self.local_names.contains(name) {
            self.instructions.push(Instruction::LoadLocal {
                dst,
                name: emitted_name,
            });
        } else {
            self.instructions.push(Instruction::LoadName {
                dst,
                name: emitted_name,
            });
        }
    }

    fn emit_store_name(&mut self, name: &str, src: Register) {
        let emitted_name = self.mangle_private_name(name);
        if self.name_is_declared_nonlocal(name) {
            self.instructions.push(Instruction::StoreNonlocal {
                name: emitted_name,
                src,
            });
        } else if self.name_is_declared_global(name) {
            self.instructions.push(Instruction::StoreGlobal {
                name: emitted_name,
                src,
            });
        } else {
            self.instructions.push(Instruction::StoreName {
                name: emitted_name,
                src,
            });
        }
    }

    fn emit_delete_name(&mut self, name: &str) {
        let emitted_name = self.mangle_private_name(name);
        if self.name_is_declared_nonlocal(name) {
            self.instructions
                .push(Instruction::DeleteNonlocal { name: emitted_name });
        } else if self.name_is_declared_global(name) {
            self.instructions
                .push(Instruction::DeleteGlobal { name: emitted_name });
        } else {
            self.instructions
                .push(Instruction::DeleteName { name: emitted_name });
        }
    }

    fn name_is_declared_global(&self, name: &str) -> bool {
        self.global_names.contains(name)
    }

    fn name_is_declared_nonlocal(&self, name: &str) -> bool {
        self.nonlocal_names.contains(name)
    }

    fn name_is_future_class_scope_binding(&self, name: &str) -> bool {
        self.private_class_name.is_some()
            && self.class_scope_all_bindings.contains(name)
            && !self.class_scope_prior_bindings.contains(name)
            && !self.name_is_declared_global(name)
            && !self.name_is_declared_nonlocal(name)
    }

    fn add_class_scope_prior_bindings(&mut self, stmt: &Stmt) {
        if self.private_class_name.is_none() {
            return;
        }

        let mut bindings = HashSet::new();
        collect_stmt_bindings(stmt, &mut bindings);
        bindings.retain(|name| {
            self.class_scope_all_bindings.contains(name)
                && !self.name_is_declared_global(name)
                && !self.name_is_declared_nonlocal(name)
        });
        self.class_scope_prior_bindings.extend(bindings);
    }

    fn compile_pending_finalizers(&mut self) -> Result<(), String> {
        let saved_contexts = self.finally_contexts.clone();

        for index in (0..saved_contexts.len()).rev() {
            let context = saved_contexts[index].clone();
            self.finally_contexts = saved_contexts[..index].to_vec();
            self.instructions.extend(context.prelude);

            for stmt in &context.body {
                self.compile_stmt(stmt)?;
            }

            self.instructions.extend(context.trailer);
        }

        self.finally_contexts = saved_contexts;
        Ok(())
    }

    fn emit_with_exit_call(&mut self, exit: Register, args: WithExitArgs) -> Register {
        let (instructions, result) = self.build_with_exit_call(exit, args);
        self.instructions.extend(instructions);
        result
    }

    fn emit_async_with_exit_call(&mut self, exit: Register, args: WithExitArgs) -> Register {
        let (instructions, result) = self.build_async_with_exit_call(exit, args);
        self.instructions.extend(instructions);
        result
    }

    fn build_with_exit_call(
        &mut self,
        exit: Register,
        args: WithExitArgs,
    ) -> (Vec<Instruction>, Register) {
        let type_arg = self.alloc_register();
        let value_arg = self.alloc_register();
        let traceback_arg = self.alloc_register();
        let mut instructions = Vec::new();

        match args {
            WithExitArgs::NoException => {
                for dst in [type_arg, value_arg, traceback_arg] {
                    instructions.push(Instruction::LoadConst {
                        dst,
                        value: Value::None,
                    });
                }
            }
            WithExitArgs::CurrentException => {
                instructions.push(Instruction::LoadCurrentException {
                    type_dst: type_arg,
                    value_dst: value_arg,
                    traceback_dst: traceback_arg,
                });
            }
        }

        let result = self.alloc_register();
        instructions.push(Instruction::Call {
            dst: result,
            callee: exit,
            args: vec![type_arg, value_arg, traceback_arg],
        });

        (instructions, result)
    }

    fn build_async_with_exit_call(
        &mut self,
        exit: Register,
        args: WithExitArgs,
    ) -> (Vec<Instruction>, Register) {
        let (mut instructions, awaitable) = self.build_with_exit_call(exit, args);
        let result = self.alloc_register();
        instructions.push(Instruction::AwaitContextManager {
            dst: result,
            src: awaitable,
            is_exit: true,
        });

        (instructions, result)
    }

    fn patch_jump_target(&mut self, instruction_index: usize, target: usize) -> Result<(), String> {
        match self.instructions.get_mut(instruction_index) {
            Some(Instruction::JumpIfFalse {
                target: jump_target,
                ..
            }) => {
                *jump_target = target;
                Ok(())
            }
            Some(Instruction::Jump {
                target: jump_target,
            }) => {
                *jump_target = target;
                Ok(())
            }
            Some(Instruction::ForIter {
                target: jump_target,
                ..
            })
            | Some(Instruction::AsyncForIter {
                target: jump_target,
                ..
            })
            | Some(Instruction::ForIterValue {
                target: jump_target,
                ..
            }) => {
                *jump_target = target;
                Ok(())
            }
            Some(instruction) => Err(format!("cannot patch jump target on {instruction:?}")),
            None => Err(format!(
                "cannot patch missing instruction at index {instruction_index}"
            )),
        }
    }

    fn patch_exception_handler_target(
        &mut self,
        instruction_index: usize,
        handler_index: usize,
        target: usize,
    ) -> Result<(), String> {
        match self.instructions.get_mut(instruction_index) {
            Some(Instruction::SetupExcept { handlers }) => {
                let handler = handlers.get_mut(handler_index).ok_or_else(|| {
                    format!("cannot patch missing exception handler at index {handler_index}")
                })?;
                handler.target = target;
                Ok(())
            }
            Some(instruction) => Err(format!(
                "cannot patch exception handler target on {instruction:?}"
            )),
            None => Err(format!(
                "cannot patch missing instruction at index {instruction_index}"
            )),
        }
    }
}

fn compile_time_tuple_constant_value(elements: &[Expr]) -> Option<Value> {
    elements
        .iter()
        .map(compile_time_constant_value)
        .collect::<Option<Vec<_>>>()
        .map(tuple_value)
}

fn compile_time_constant_value(expr: &Expr) -> Option<Value> {
    match expr {
        Expr::Number(value) => Some(Value::Number(*value)),
        Expr::BigInt(value) => parse_big_int_literal(value).ok().map(Value::BigInt),
        Expr::Float(value) => parse_float_literal(value).ok().map(float_value),
        Expr::Imaginary(value) => parse_float_literal(value)
            .ok()
            .map(|imag| complex_value(0.0, imag)),
        Expr::String(value) => Some(Value::String(value.clone())),
        Expr::Bytes(value) => Some(bytes_value(value.clone())),
        Expr::Bool(value) => Some(Value::Bool(*value)),
        Expr::None => Some(Value::None),
        Expr::Ellipsis => Some(Value::Ellipsis),
        Expr::Tuple(elements) => compile_time_tuple_constant_value(elements),
        _ => None,
    }
}

fn collect_class_static_attributes(body: &[Stmt]) -> Vec<String> {
    let mut attributes = BTreeSet::new();
    collect_static_attributes_from_statements(body, &mut attributes);
    attributes.into_iter().collect()
}

fn collect_static_attributes_from_statements(
    statements: &[Stmt],
    attributes: &mut BTreeSet<String>,
) {
    for stmt in statements {
        collect_static_attributes_from_stmt(stmt, attributes);
    }
}

fn collect_static_attributes_from_stmt(stmt: &Stmt, attributes: &mut BTreeSet<String>) {
    match stmt {
        Stmt::Pass
        | Stmt::Import { .. }
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Break
        | Stmt::Continue => {}
        Stmt::Expr(expr) => collect_static_attributes_from_expr(expr, attributes),
        Stmt::Assign { targets, value, .. } => {
            collect_static_attributes_from_expr(value, attributes);
            for target in targets {
                collect_static_attributes_from_store_target(target, attributes);
            }
        }
        Stmt::AnnAssign {
            target,
            annotation,
            value,
            ..
        } => {
            collect_static_attributes_from_store_target(target, attributes);
            collect_static_attributes_from_expr(annotation, attributes);
            if let Some(value) = value {
                collect_static_attributes_from_expr(value, attributes);
            }
        }
        Stmt::TypeAlias {
            type_params, value, ..
        } => {
            collect_static_attributes_from_type_params(type_params, attributes);
            collect_static_attributes_from_expr(value, attributes);
        }
        Stmt::AugAssign { target, value, .. } => {
            collect_static_attributes_from_store_target(target, attributes);
            collect_static_attributes_from_expr(value, attributes);
        }
        Stmt::Delete { target } => {
            collect_static_attributes_from_target_value_exprs(target, attributes);
        }
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
            for decorator in decorators {
                collect_static_attributes_from_expr(decorator, attributes);
            }
            collect_static_attributes_from_type_params(type_params, attributes);
            collect_static_attributes_from_params(params, attributes);
            if let Some(returns) = returns {
                collect_static_attributes_from_expr(returns, attributes);
            }
            collect_static_attributes_from_statements(body, attributes);
        }
        Stmt::ClassDef {
            type_params,
            bases,
            keywords,
            decorators,
            ..
        } => {
            for decorator in decorators {
                collect_static_attributes_from_expr(decorator, attributes);
            }
            collect_static_attributes_from_type_params(type_params, attributes);
            for base in bases {
                collect_static_attributes_from_call_arg(base, attributes);
            }
            for keyword in keywords {
                collect_static_attributes_from_call_keyword(keyword, attributes);
            }
        }
        Stmt::ImportFrom { .. } => {}
        Stmt::Return(value) => {
            if let Some(value) = value {
                collect_static_attributes_from_expr(value, attributes);
            }
        }
        Stmt::Assert { condition, message } => {
            collect_static_attributes_from_expr(condition, attributes);
            if let Some(message) = message {
                collect_static_attributes_from_expr(message, attributes);
            }
        }
        Stmt::Raise { value, cause } => {
            if let Some(value) = value {
                collect_static_attributes_from_expr(value, attributes);
            }
            if let Some(cause) = cause {
                collect_static_attributes_from_expr(cause, attributes);
            }
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            collect_static_attributes_from_expr(condition, attributes);
            collect_static_attributes_from_statements(then_body, attributes);
            collect_static_attributes_from_statements(else_body, attributes);
        }
        Stmt::Match { subject, cases } => {
            collect_static_attributes_from_expr(subject, attributes);
            for case in cases {
                collect_static_attributes_from_match_case(case, attributes);
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
            collect_static_attributes_from_statements(body, attributes);
            for handler in handlers {
                if let Some(type_expr) = &handler.type_expr {
                    collect_static_attributes_from_expr(type_expr, attributes);
                }
                collect_static_attributes_from_statements(&handler.body, attributes);
            }
            collect_static_attributes_from_statements(else_body, attributes);
            collect_static_attributes_from_statements(finally_body, attributes);
        }
        Stmt::With { items, body, .. } | Stmt::AsyncWith { items, body, .. } => {
            for item in items {
                collect_static_attributes_from_with_item(item, attributes);
            }
            collect_static_attributes_from_statements(body, attributes);
        }
        Stmt::While {
            condition,
            body,
            else_body,
        } => {
            collect_static_attributes_from_expr(condition, attributes);
            collect_static_attributes_from_statements(body, attributes);
            collect_static_attributes_from_statements(else_body, attributes);
        }
        Stmt::For {
            target,
            iter,
            body,
            else_body,
            ..
        }
        | Stmt::AsyncFor {
            target,
            iter,
            body,
            else_body,
            ..
        } => {
            collect_static_attributes_from_store_target(target, attributes);
            collect_static_attributes_from_expr(iter, attributes);
            collect_static_attributes_from_statements(body, attributes);
            collect_static_attributes_from_statements(else_body, attributes);
        }
    }
}

fn collect_static_attributes_from_match_case(case: &MatchCase, attributes: &mut BTreeSet<String>) {
    collect_static_attributes_from_pattern(&case.pattern, attributes);
    if let Some(guard) = &case.guard {
        collect_static_attributes_from_expr(guard, attributes);
    }
    collect_static_attributes_from_statements(&case.body, attributes);
}

fn collect_static_attributes_from_pattern(pattern: &Pattern, attributes: &mut BTreeSet<String>) {
    match pattern {
        Pattern::Literal(expr) | Pattern::Singleton(expr) | Pattern::Value(expr) => {
            collect_static_attributes_from_expr(expr, attributes);
        }
        Pattern::Capture(_) | Pattern::Wildcard | Pattern::Star(_) => {}
        Pattern::Or(patterns) | Pattern::Sequence(patterns) => {
            for pattern in patterns {
                collect_static_attributes_from_pattern(pattern, attributes);
            }
        }
        Pattern::Mapping { entries, .. } => {
            for (key, pattern) in entries {
                collect_static_attributes_from_expr(key, attributes);
                collect_static_attributes_from_pattern(pattern, attributes);
            }
        }
        Pattern::Class {
            class,
            positional,
            keywords,
        } => {
            collect_static_attributes_from_expr(class, attributes);
            for pattern in positional {
                collect_static_attributes_from_pattern(pattern, attributes);
            }
            for (_, pattern) in keywords {
                collect_static_attributes_from_pattern(pattern, attributes);
            }
        }
        Pattern::As { pattern, .. } => {
            collect_static_attributes_from_pattern(pattern, attributes);
        }
    }
}

fn collect_static_attributes_from_with_item(item: &WithItem, attributes: &mut BTreeSet<String>) {
    collect_static_attributes_from_expr(&item.context_expr, attributes);
    if let Some(target) = &item.optional_vars {
        collect_static_attributes_from_store_target(target, attributes);
    }
}

fn collect_static_attributes_from_store_target(target: &Target, attributes: &mut BTreeSet<String>) {
    match target {
        Target::Name(_) => {}
        Target::Attribute { object, name } => {
            if matches!(object.as_ref(), Expr::Name(object_name) if object_name == "self") {
                attributes.insert(name.clone());
            }
            collect_static_attributes_from_expr(object, attributes);
        }
        Target::Subscript { object, index } => {
            collect_static_attributes_from_expr(object, attributes);
            collect_static_attributes_from_expr(index, attributes);
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_static_attributes_from_expr(object, attributes);
            collect_optional_static_attribute_expr(start.as_ref(), attributes);
            collect_optional_static_attribute_expr(stop.as_ref(), attributes);
            collect_optional_static_attribute_expr(step.as_ref(), attributes);
        }
        Target::Starred(target) => collect_static_attributes_from_store_target(target, attributes),
        Target::Tuple(targets) | Target::List(targets) => {
            for target in targets {
                collect_static_attributes_from_store_target(target, attributes);
            }
        }
    }
}

fn collect_static_attributes_from_target_value_exprs(
    target: &Target,
    attributes: &mut BTreeSet<String>,
) {
    match target {
        Target::Name(_) => {}
        Target::Attribute { object, .. } => {
            collect_static_attributes_from_expr(object, attributes);
        }
        Target::Subscript { object, index } => {
            collect_static_attributes_from_expr(object, attributes);
            collect_static_attributes_from_expr(index, attributes);
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_static_attributes_from_expr(object, attributes);
            collect_optional_static_attribute_expr(start.as_ref(), attributes);
            collect_optional_static_attribute_expr(stop.as_ref(), attributes);
            collect_optional_static_attribute_expr(step.as_ref(), attributes);
        }
        Target::Starred(target) => {
            collect_static_attributes_from_target_value_exprs(target, attributes)
        }
        Target::Tuple(targets) | Target::List(targets) => {
            for target in targets {
                collect_static_attributes_from_target_value_exprs(target, attributes);
            }
        }
    }
}

fn collect_static_attributes_from_expr(expr: &Expr, attributes: &mut BTreeSet<String>) {
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
                collect_static_attributes_from_f_string_part(part, attributes);
            }
        }
        Expr::TemplateString(parts) => {
            for part in parts {
                collect_static_attributes_from_template_string_part(part, attributes);
            }
        }
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            collect_static_attributes_from_expr(value, attributes);
            if let Some(format_spec) = format_spec {
                for part in format_spec {
                    collect_static_attributes_from_f_string_part(part, attributes);
                }
            }
        }
        Expr::Attribute { object, .. } => collect_static_attributes_from_expr(object, attributes),
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            collect_static_attributes_from_expr(left, attributes);
            collect_static_attributes_from_expr(right, attributes);
        }
        Expr::ChainedComparison { left, comparisons } => {
            collect_static_attributes_from_expr(left, attributes);
            for (_, expr) in comparisons {
                collect_static_attributes_from_expr(expr, attributes);
            }
        }
        Expr::Unary { operand, .. }
        | Expr::Await(operand)
        | Expr::Starred(operand)
        | Expr::YieldFrom(operand) => collect_static_attributes_from_expr(operand, attributes),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_static_attributes_from_expr(condition, attributes);
            collect_static_attributes_from_expr(then_branch, attributes);
            collect_static_attributes_from_expr(else_branch, attributes);
        }
        Expr::NamedExpr { value, .. } => collect_static_attributes_from_expr(value, attributes),
        Expr::Yield { value } => {
            if let Some(value) = value {
                collect_static_attributes_from_expr(value, attributes);
            }
        }
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => {
            for element in elements {
                collect_static_attributes_from_expr(element, attributes);
            }
        }
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            collect_static_attributes_from_expr(element, attributes);
            collect_static_attributes_from_comprehension_clauses(clauses, attributes);
        }
        Expr::Dict(items) => {
            for item in items {
                match item {
                    DictItem::Entry { key, value } => {
                        collect_static_attributes_from_expr(key, attributes);
                        collect_static_attributes_from_expr(value, attributes);
                    }
                    DictItem::Unpack(expr) => collect_static_attributes_from_expr(expr, attributes),
                }
            }
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            collect_static_attributes_from_expr(key, attributes);
            collect_static_attributes_from_expr(value, attributes);
            collect_static_attributes_from_comprehension_clauses(clauses, attributes);
        }
        Expr::DictUnpackComp { value, clauses } => {
            collect_static_attributes_from_expr(value, attributes);
            collect_static_attributes_from_comprehension_clauses(clauses, attributes);
        }
        Expr::Subscript { object, index } => {
            collect_static_attributes_from_expr(object, attributes);
            collect_static_attributes_from_expr(index, attributes);
        }
        Expr::SliceLiteral { start, stop, step } => {
            collect_optional_boxed_static_attribute_expr(start.as_ref(), attributes);
            collect_optional_boxed_static_attribute_expr(stop.as_ref(), attributes);
            collect_optional_boxed_static_attribute_expr(step.as_ref(), attributes);
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_static_attributes_from_expr(object, attributes);
            collect_optional_boxed_static_attribute_expr(start.as_ref(), attributes);
            collect_optional_boxed_static_attribute_expr(stop.as_ref(), attributes);
            collect_optional_boxed_static_attribute_expr(step.as_ref(), attributes);
        }
        Expr::Call { callee, args } | Expr::KeywordCall { callee, args, .. } => {
            collect_static_attributes_from_expr(callee, attributes);
            for arg in args {
                collect_static_attributes_from_expr(arg, attributes);
            }
            if let Expr::KeywordCall { keywords, .. } = expr {
                for (_, value) in keywords {
                    collect_static_attributes_from_expr(value, attributes);
                }
            }
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            collect_static_attributes_from_expr(callee, attributes);
            for arg in args {
                collect_static_attributes_from_call_arg(arg, attributes);
            }
            for keyword in keywords {
                collect_static_attributes_from_call_keyword(keyword, attributes);
            }
        }
        Expr::Lambda { params, body } => {
            collect_static_attributes_from_params(params, attributes);
            collect_static_attributes_from_expr(body, attributes);
        }
    }
}

fn collect_static_attributes_from_comprehension_clauses(
    clauses: &[ComprehensionClause],
    attributes: &mut BTreeSet<String>,
) {
    for clause in clauses {
        collect_static_attributes_from_store_target(&clause.target, attributes);
        collect_static_attributes_from_expr(&clause.iter, attributes);
        for condition in &clause.ifs {
            collect_static_attributes_from_expr(condition, attributes);
        }
    }
}

fn collect_static_attributes_from_params(
    params: &FunctionParams,
    attributes: &mut BTreeSet<String>,
) {
    for param in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
    {
        collect_static_attributes_from_param(param, attributes);
    }
    collect_optional_boxed_static_attribute_expr(params.vararg_annotation.as_ref(), attributes);
    collect_optional_boxed_static_attribute_expr(params.kwarg_annotation.as_ref(), attributes);
}

fn collect_static_attributes_from_param(param: &Param, attributes: &mut BTreeSet<String>) {
    if let Some(annotation) = &param.annotation {
        collect_static_attributes_from_expr(annotation, attributes);
    }
    if let Some(default) = &param.default {
        collect_static_attributes_from_expr(default, attributes);
    }
}

fn collect_static_attributes_from_type_params(
    type_params: &[TypeParam],
    attributes: &mut BTreeSet<String>,
) {
    for type_param in type_params {
        if let Some(bound) = &type_param.bound {
            collect_static_attributes_from_expr(bound, attributes);
        }
        if let Some(default) = &type_param.default {
            collect_static_attributes_from_expr(default, attributes);
        }
    }
}

fn collect_static_attributes_from_call_arg(arg: &CallArg, attributes: &mut BTreeSet<String>) {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => {
            collect_static_attributes_from_expr(expr, attributes);
        }
    }
}

fn collect_static_attributes_from_call_keyword(
    keyword: &CallKeyword,
    attributes: &mut BTreeSet<String>,
) {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
            collect_static_attributes_from_expr(expr, attributes);
        }
    }
}

fn collect_static_attributes_from_f_string_part(
    part: &FStringPart,
    attributes: &mut BTreeSet<String>,
) {
    match part {
        FStringPart::Literal(_) => {}
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            collect_static_attributes_from_expr(value, attributes);
            if let Some(parts) = format_spec {
                for part in parts {
                    collect_static_attributes_from_f_string_part(part, attributes);
                }
            }
        }
    }
}

fn collect_static_attributes_from_template_string_part(
    part: &TemplateStringPart,
    attributes: &mut BTreeSet<String>,
) {
    match part {
        TemplateStringPart::Literal(_) => {}
        TemplateStringPart::Interpolation {
            value, format_spec, ..
        } => {
            collect_static_attributes_from_expr(value, attributes);
            if let Some(parts) = format_spec {
                for part in parts {
                    collect_static_attributes_from_f_string_part(part, attributes);
                }
            }
        }
    }
}

fn collect_optional_static_attribute_expr(expr: Option<&Expr>, attributes: &mut BTreeSet<String>) {
    if let Some(expr) = expr {
        collect_static_attributes_from_expr(expr, attributes);
    }
}

fn collect_optional_boxed_static_attribute_expr(
    expr: Option<&Box<Expr>>,
    attributes: &mut BTreeSet<String>,
) {
    if let Some(expr) = expr {
        collect_static_attributes_from_expr(expr, attributes);
    }
}

fn parse_float_literal(value: &str) -> Result<f64, String> {
    value
        .replace('_', "")
        .parse::<f64>()
        .map_err(|_| format!("invalid float literal: {value}"))
}

fn parse_big_int_literal(value: &str) -> Result<BigInt, String> {
    value
        .parse::<BigInt>()
        .map_err(|_| format!("invalid int literal: {value}"))
}

fn negated_literal_constant_value(expr: &Expr) -> Result<Option<Value>, String> {
    let value = match expr {
        Expr::Number(value) => match value.checked_neg() {
            Some(value) => Value::Number(value),
            None => Value::BigInt(-BigInt::from(*value)),
        },
        Expr::BigInt(value) => Value::BigInt(-parse_big_int_literal(value)?),
        Expr::Float(value) => float_value(-parse_float_literal(value)?),
        Expr::Imaginary(value) => complex_value(-0.0, -parse_float_literal(value)?),
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn f_string_conversion_to_bytecode(conversion: FStringConversion) -> FormatConversion {
    match conversion {
        FStringConversion::Str => FormatConversion::Str,
        FStringConversion::Repr => FormatConversion::Repr,
        FStringConversion::Ascii => FormatConversion::Ascii,
    }
}

fn mangle_private_name(private_class_name: Option<&str>, name: &str) -> String {
    let Some(class_name) = private_class_name else {
        return name.to_string();
    };
    if !name.starts_with("__") || name.ends_with("__") || name.contains('.') {
        return name.to_string();
    }

    let stripped_class_name = class_name.trim_start_matches('_');
    if stripped_class_name.is_empty() {
        return name.to_string();
    }

    format!("_{stripped_class_name}{name}")
}

#[derive(Default)]
struct ScopeDeclarationAnalysis {
    global_names: HashSet<String>,
    nonlocal_names: HashSet<String>,
    local_bindings: HashSet<String>,
}

struct ScopeDeclarationTracker {
    is_module: bool,
    parameter_names: HashSet<String>,
    type_parameter_names: HashSet<String>,
    used_names: HashSet<String>,
    assigned_names: HashSet<String>,
    global_names: HashSet<String>,
    nonlocal_names: HashSet<String>,
}

fn validate_module_scope_declarations(statements: &[Stmt]) -> Result<(), String> {
    let mut tracker = ScopeDeclarationTracker::new(HashSet::new(), HashSet::new(), true);
    tracker.visit_statements(statements)?;
    if !tracker.nonlocal_names.is_empty() {
        return Err("nonlocal declaration not allowed at module level".to_string());
    }
    Ok(())
}

fn analyze_function_scope(
    params: &FunctionParams,
    statements: &[Stmt],
    enclosing_function_bindings: &[HashSet<String>],
    type_parameter_names: &HashSet<String>,
) -> Result<ScopeDeclarationAnalysis, String> {
    let parameter_names = function_param_names(params);
    let mut tracker =
        ScopeDeclarationTracker::new(parameter_names.clone(), type_parameter_names.clone(), false);
    tracker.visit_statements(statements)?;
    for name in &tracker.nonlocal_names {
        if !enclosing_function_bindings
            .iter()
            .any(|bindings| bindings.contains(name))
        {
            return Err(format!("no binding for nonlocal '{name}' found"));
        }
    }

    let mut local_bindings = parameter_names;
    collect_statement_bindings(statements, &mut local_bindings);
    for name in tracker
        .global_names
        .iter()
        .chain(tracker.nonlocal_names.iter())
    {
        local_bindings.remove(name);
    }

    Ok(ScopeDeclarationAnalysis {
        global_names: tracker.global_names,
        nonlocal_names: tracker.nonlocal_names,
        local_bindings,
    })
}

fn analyze_class_scope(
    statements: &[Stmt],
    enclosing_function_bindings: &[HashSet<String>],
    type_parameter_names: &HashSet<String>,
) -> Result<ScopeDeclarationAnalysis, String> {
    let mut tracker =
        ScopeDeclarationTracker::new(HashSet::new(), type_parameter_names.clone(), false);
    tracker.visit_statements(statements)?;
    for name in &tracker.nonlocal_names {
        if !enclosing_function_bindings
            .iter()
            .any(|bindings| bindings.contains(name))
        {
            return Err(format!("no binding for nonlocal '{name}' found"));
        }
    }

    let mut local_bindings = HashSet::new();
    collect_statement_bindings(statements, &mut local_bindings);
    for name in tracker
        .global_names
        .iter()
        .chain(tracker.nonlocal_names.iter())
    {
        local_bindings.remove(name);
    }

    Ok(ScopeDeclarationAnalysis {
        global_names: tracker.global_names,
        nonlocal_names: tracker.nonlocal_names,
        local_bindings,
    })
}

impl ScopeDeclarationTracker {
    fn new(
        parameter_names: HashSet<String>,
        type_parameter_names: HashSet<String>,
        is_module: bool,
    ) -> Self {
        Self {
            is_module,
            parameter_names,
            type_parameter_names,
            used_names: HashSet::new(),
            assigned_names: HashSet::new(),
            global_names: HashSet::new(),
            nonlocal_names: HashSet::new(),
        }
    }

    fn visit_statements(&mut self, statements: &[Stmt]) -> Result<(), String> {
        for stmt in statements {
            self.visit_stmt(stmt)?;
        }
        Ok(())
    }

    fn visit_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Global(names) => {
                for name in names {
                    self.declare_global(name)?;
                }
            }
            Stmt::Nonlocal(names) => {
                for name in names {
                    self.declare_nonlocal(name)?;
                }
            }
            Stmt::Pass | Stmt::Break | Stmt::Continue => {}
            Stmt::Expr(expr) => self.visit_expr(expr),
            Stmt::Assign { targets, value, .. } => {
                self.visit_expr(value);
                for target in targets {
                    self.visit_target_assignment(target);
                }
            }
            Stmt::AnnAssign {
                target,
                annotation,
                value,
                simple,
            } => {
                self.visit_expr(annotation);
                if let Some(value) = value {
                    self.visit_expr(value);
                }
                if *simple {
                    self.validate_annotation_target_scope(target)?;
                }
                if *simple || value.is_some() || !matches!(target, Target::Name(_)) {
                    self.visit_target_assignment(target);
                }
            }
            Stmt::TypeAlias {
                name,
                type_params,
                value,
            } => {
                self.visit_type_params(type_params);
                self.visit_expr(value);
                self.assigned_names.insert(name.clone());
            }
            Stmt::AugAssign { target, value, .. } => {
                self.visit_expr(value);
                self.visit_target_assignment(target);
            }
            Stmt::Delete { target } => self.visit_target_assignment(target),
            Stmt::FunctionDef {
                name,
                type_params,
                params,
                decorators,
                returns,
                ..
            }
            | Stmt::AsyncFunctionDef {
                name,
                type_params,
                params,
                decorators,
                returns,
                ..
            } => {
                for decorator in decorators {
                    self.visit_expr(decorator);
                }
                self.visit_type_params(type_params);
                self.visit_param_runtime_exprs(params);
                if let Some(returns) = returns {
                    self.visit_expr(returns);
                }
                self.assigned_names.insert(name.clone());
            }
            Stmt::ClassDef {
                name,
                type_params,
                bases,
                keywords,
                decorators,
                ..
            } => {
                for decorator in decorators {
                    self.visit_expr(decorator);
                }
                self.visit_type_params(type_params);
                for base in bases {
                    self.visit_call_arg(base);
                }
                for keyword in keywords {
                    self.visit_call_keyword(keyword);
                }
                self.assigned_names.insert(name.clone());
            }
            Stmt::Import { aliases, .. } => {
                for alias in aliases {
                    self.assigned_names
                        .insert(import_binding_name(alias).to_string());
                }
            }
            Stmt::ImportFrom { targets, .. } => match targets {
                ImportFromTargets::Star => {}
                ImportFromTargets::Aliases(aliases) => {
                    for alias in aliases {
                        self.assigned_names
                            .insert(import_binding_name(alias).to_string());
                    }
                }
            },
            Stmt::Return(value) => {
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            Stmt::Assert { condition, message } => {
                self.visit_expr(condition);
                if let Some(message) = message {
                    self.visit_expr(message);
                }
            }
            Stmt::Raise { value, cause } => {
                if let Some(value) = value {
                    self.visit_expr(value);
                }
                if let Some(cause) = cause {
                    self.visit_expr(cause);
                }
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.visit_expr(condition);
                self.visit_statements(then_body)?;
                self.visit_statements(else_body)?;
            }
            Stmt::Match { subject, cases } => {
                self.visit_expr(subject);
                for case in cases {
                    self.visit_pattern_bindings(&case.pattern);
                    if let Some(guard) = &case.guard {
                        self.visit_expr(guard);
                    }
                    self.visit_statements(&case.body)?;
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
                self.visit_statements(body)?;
                for handler in handlers {
                    if let Some(name) = &handler.name {
                        self.assigned_names.insert(name.clone());
                    }
                    self.visit_statements(&handler.body)?;
                }
                self.visit_statements(else_body)?;
                self.visit_statements(finally_body)?;
            }
            Stmt::With { items, body, .. } | Stmt::AsyncWith { items, body, .. } => {
                for item in items {
                    self.visit_expr(&item.context_expr);
                    if let Some(target) = &item.optional_vars {
                        self.visit_target_assignment(target);
                    }
                }
                self.visit_statements(body)?;
            }
            Stmt::While {
                condition,
                body,
                else_body,
            } => {
                self.visit_expr(condition);
                self.visit_statements(body)?;
                self.visit_statements(else_body)?;
            }
            Stmt::For {
                target,
                iter,
                body,
                else_body,
                ..
            }
            | Stmt::AsyncFor {
                target,
                iter,
                body,
                else_body,
                ..
            } => {
                self.visit_expr(iter);
                self.visit_target_assignment(target);
                self.visit_statements(body)?;
                self.visit_statements(else_body)?;
            }
        }
        Ok(())
    }

    fn declare_global(&mut self, name: &str) -> Result<(), String> {
        if self.parameter_names.contains(name) {
            return Err(format!("name '{name}' is parameter and global"));
        }
        if self.assigned_names.contains(name) {
            return Err(format!(
                "name '{name}' is assigned to before global declaration"
            ));
        }
        if self.used_names.contains(name) {
            return Err(format!("name '{name}' is used prior to global declaration"));
        }
        if self.nonlocal_names.contains(name) {
            return Err(format!("name '{name}' is nonlocal and global"));
        }
        self.global_names.insert(name.to_string());
        Ok(())
    }

    fn declare_nonlocal(&mut self, name: &str) -> Result<(), String> {
        if self.type_parameter_names.contains(name) {
            return Err(format!("name '{name}' is type parameter and nonlocal"));
        }
        if self.parameter_names.contains(name) {
            return Err(format!("name '{name}' is parameter and nonlocal"));
        }
        if self.assigned_names.contains(name) {
            return Err(format!(
                "name '{name}' is assigned to before nonlocal declaration"
            ));
        }
        if self.used_names.contains(name) {
            return Err(format!(
                "name '{name}' is used prior to nonlocal declaration"
            ));
        }
        if self.global_names.contains(name) {
            return Err(format!("name '{name}' is nonlocal and global"));
        }
        self.nonlocal_names.insert(name.to_string());
        Ok(())
    }

    fn validate_annotation_target_scope(&self, target: &Target) -> Result<(), String> {
        let Target::Name(name) = target else {
            return Ok(());
        };
        if !self.is_module && self.global_names.contains(name) {
            return Err(format!("annotated name '{name}' can't be global"));
        }
        if self.nonlocal_names.contains(name) {
            return Err(format!("annotated name '{name}' can't be nonlocal"));
        }
        Ok(())
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(name) => {
                self.used_names.insert(name.clone());
            }
            Expr::Attribute { object, .. } => self.visit_expr(object),
            Expr::Binary { left, right, .. }
            | Expr::Comparison { left, right, .. }
            | Expr::Logical { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::ChainedComparison { left, comparisons } => {
                self.visit_expr(left);
                for (_, right) in comparisons {
                    self.visit_expr(right);
                }
            }
            Expr::Unary { operand, .. }
            | Expr::YieldFrom(operand)
            | Expr::Await(operand)
            | Expr::Starred(operand) => self.visit_expr(operand),
            Expr::IfExpression {
                condition,
                then_branch,
                else_branch,
            } => {
                self.visit_expr(condition);
                self.visit_expr(then_branch);
                self.visit_expr(else_branch);
            }
            Expr::NamedExpr { name, value } => {
                self.visit_expr(value);
                self.assigned_names.insert(name.clone());
            }
            Expr::Yield { value } => {
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            Expr::List(items) | Expr::Set(items) | Expr::FrozenSet(items) | Expr::Tuple(items) => {
                for item in items {
                    self.visit_expr(item);
                }
            }
            Expr::ListComp { clauses, .. }
            | Expr::SetComp { clauses, .. }
            | Expr::GeneratorComp { clauses, .. }
            | Expr::DictComp { clauses, .. }
            | Expr::DictUnpackComp { clauses, .. } => {
                if let Some(first_clause) = clauses.first() {
                    self.visit_expr(&first_clause.iter);
                }
            }
            Expr::Dict(entries) => {
                for entry in entries {
                    match entry {
                        DictItem::Entry { key, value } => {
                            self.visit_expr(key);
                            self.visit_expr(value);
                        }
                        DictItem::Unpack(value) => self.visit_expr(value),
                    }
                }
            }
            Expr::Subscript { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            Expr::SliceLiteral { start, stop, step } => {
                self.visit_optional_expr(start.as_deref());
                self.visit_optional_expr(stop.as_deref());
                self.visit_optional_expr(step.as_deref());
            }
            Expr::Slice {
                object,
                start,
                stop,
                step,
            } => {
                self.visit_expr(object);
                self.visit_optional_expr(start.as_deref());
                self.visit_optional_expr(stop.as_deref());
                self.visit_optional_expr(step.as_deref());
            }
            Expr::Call { callee, args } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::KeywordCall {
                callee,
                args,
                keywords,
            } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_expr(arg);
                }
                for (_, value) in keywords {
                    self.visit_expr(value);
                }
            }
            Expr::UnpackCall {
                callee,
                args,
                keywords,
            } => {
                self.visit_expr(callee);
                for arg in args {
                    self.visit_call_arg(arg);
                }
                for keyword in keywords {
                    self.visit_call_keyword(keyword);
                }
            }
            Expr::Lambda { params, .. } => self.visit_param_runtime_exprs(params),
            Expr::JoinedString(parts) => self.visit_f_string_parts(parts),
            Expr::TemplateString(parts) => {
                for part in parts {
                    match part {
                        TemplateStringPart::Literal(_) => {}
                        TemplateStringPart::Interpolation {
                            value, format_spec, ..
                        } => {
                            self.visit_expr(value);
                            if let Some(format_spec) = format_spec {
                                self.visit_f_string_parts(format_spec);
                            }
                        }
                    }
                }
            }
            Expr::TemplateInterpolation {
                value, format_spec, ..
            } => {
                self.visit_expr(value);
                if let Some(format_spec) = format_spec {
                    self.visit_f_string_parts(format_spec);
                }
            }
            Expr::Number(_)
            | Expr::BigInt(_)
            | Expr::Float(_)
            | Expr::Imaginary(_)
            | Expr::String(_)
            | Expr::Bytes(_)
            | Expr::Bool(_)
            | Expr::None
            | Expr::Ellipsis => {}
        }
    }

    fn visit_optional_expr(&mut self, expr: Option<&Expr>) {
        if let Some(expr) = expr {
            self.visit_expr(expr);
        }
    }

    fn visit_call_arg(&mut self, arg: &CallArg) {
        match arg {
            CallArg::Expr(expr) | CallArg::Unpack(expr) => self.visit_expr(expr),
        }
    }

    fn visit_call_keyword(&mut self, keyword: &CallKeyword) {
        match keyword {
            CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => self.visit_expr(expr),
        }
    }

    fn visit_f_string_parts(&mut self, parts: &[FStringPart]) {
        for part in parts {
            match part {
                FStringPart::Literal(_) => {}
                FStringPart::Formatted {
                    value, format_spec, ..
                } => {
                    self.visit_expr(value);
                    if let Some(format_spec) = format_spec {
                        self.visit_f_string_parts(format_spec);
                    }
                }
            }
        }
    }

    fn visit_type_params(&mut self, type_params: &[TypeParam]) {
        for type_param in type_params {
            if let Some(bound) = &type_param.bound {
                self.visit_expr(bound);
            }
            if let Some(default) = &type_param.default {
                self.visit_expr(default);
            }
        }
    }

    fn visit_param_runtime_exprs(&mut self, params: &FunctionParams) {
        for param in params
            .positional_only
            .iter()
            .chain(params.positional.iter())
        {
            self.visit_optional_expr(param.annotation.as_ref());
            self.visit_optional_expr(param.default.as_ref());
        }
        if let Some(annotation) = params.vararg_annotation.as_deref() {
            self.visit_expr(annotation);
        }
        for param in &params.keyword_only {
            self.visit_optional_expr(param.annotation.as_ref());
            self.visit_optional_expr(param.default.as_ref());
        }
        if let Some(annotation) = params.kwarg_annotation.as_deref() {
            self.visit_expr(annotation);
        }
    }

    fn visit_target_assignment(&mut self, target: &Target) {
        match target {
            Target::Name(name) => {
                self.assigned_names.insert(name.clone());
            }
            Target::Attribute { object, .. } => self.visit_expr(object),
            Target::Subscript { object, index } => {
                self.visit_expr(object);
                self.visit_expr(index);
            }
            Target::Slice {
                object,
                start,
                stop,
                step,
            } => {
                self.visit_expr(object);
                self.visit_optional_expr(start.as_ref());
                self.visit_optional_expr(stop.as_ref());
                self.visit_optional_expr(step.as_ref());
            }
            Target::Starred(target) => self.visit_target_assignment(target),
            Target::Tuple(targets) | Target::List(targets) => {
                for target in targets {
                    self.visit_target_assignment(target);
                }
            }
        }
    }

    fn visit_pattern_bindings(&mut self, pattern: &Pattern) {
        match pattern {
            Pattern::Capture(name) => {
                self.assigned_names.insert(name.clone());
            }
            Pattern::Mapping { entries, rest } => {
                for (key, value) in entries {
                    self.visit_expr(key);
                    self.visit_pattern_bindings(value);
                }
                if let Some(rest) = rest {
                    self.assigned_names.insert(rest.clone());
                }
            }
            Pattern::Class {
                class,
                positional,
                keywords,
            } => {
                self.visit_expr(class);
                for pattern in positional {
                    self.visit_pattern_bindings(pattern);
                }
                for (_, pattern) in keywords {
                    self.visit_pattern_bindings(pattern);
                }
            }
            Pattern::Or(patterns) | Pattern::Sequence(patterns) => {
                for pattern in patterns {
                    self.visit_pattern_bindings(pattern);
                }
            }
            Pattern::Star(Some(name)) => {
                self.assigned_names.insert(name.clone());
            }
            Pattern::As { pattern, name } => {
                self.visit_pattern_bindings(pattern);
                self.assigned_names.insert(name.clone());
            }
            Pattern::Literal(expr) | Pattern::Singleton(expr) | Pattern::Value(expr) => {
                self.visit_expr(expr)
            }
            Pattern::Wildcard | Pattern::Star(None) => {}
        }
    }
}

fn function_param_names(params: &FunctionParams) -> HashSet<String> {
    let mut names = HashSet::new();
    for param in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
    {
        names.insert(param.name.clone());
    }
    if let Some(name) = &params.vararg {
        names.insert(name.clone());
    }
    for param in &params.keyword_only {
        names.insert(param.name.clone());
    }
    if let Some(name) = &params.kwarg {
        names.insert(name.clone());
    }
    names
}

fn type_param_name_set(type_params: &[TypeParam]) -> HashSet<String> {
    type_params
        .iter()
        .map(|type_param| type_param.name.clone())
        .collect()
}

fn collect_statement_bindings(statements: &[Stmt], names: &mut HashSet<String>) {
    for stmt in statements {
        collect_stmt_bindings(stmt, names);
    }
}

fn collect_stmt_bindings(stmt: &Stmt, names: &mut HashSet<String>) {
    match stmt {
        Stmt::Assign { targets, value, .. } => {
            collect_expr_bindings(value, names);
            for target in targets {
                collect_target_bindings(target, names);
            }
        }
        Stmt::AnnAssign {
            target,
            value,
            simple,
            ..
        } => {
            if let Some(value) = value {
                collect_expr_bindings(value, names);
            }
            if *simple || value.is_some() || !matches!(target, Target::Name(_)) {
                collect_target_bindings(target, names);
            }
        }
        Stmt::TypeAlias { name, value, .. } => {
            collect_expr_bindings(value, names);
            names.insert(name.clone());
        }
        Stmt::AugAssign { target, value, .. } => {
            collect_expr_bindings(value, names);
            collect_target_bindings(target, names);
        }
        Stmt::Delete { target } => collect_target_bindings(target, names),
        Stmt::FunctionDef { name, .. }
        | Stmt::AsyncFunctionDef { name, .. }
        | Stmt::ClassDef { name, .. } => {
            names.insert(name.clone());
        }
        Stmt::Import { aliases, .. } => {
            for alias in aliases {
                names.insert(import_binding_name(alias).to_string());
            }
        }
        Stmt::ImportFrom { targets, .. } => match targets {
            ImportFromTargets::Star => {}
            ImportFromTargets::Aliases(aliases) => {
                for alias in aliases {
                    names.insert(import_binding_name(alias).to_string());
                }
            }
        },
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            collect_statement_bindings(then_body, names);
            collect_statement_bindings(else_body, names);
        }
        Stmt::Match { cases, .. } => {
            for case in cases {
                collect_pattern_bindings(&case.pattern, names);
                if let Some(guard) = &case.guard {
                    collect_expr_bindings(guard, names);
                }
                collect_statement_bindings(&case.body, names);
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
            collect_statement_bindings(body, names);
            for handler in handlers {
                if let Some(name) = &handler.name {
                    names.insert(name.clone());
                }
                collect_statement_bindings(&handler.body, names);
            }
            collect_statement_bindings(else_body, names);
            collect_statement_bindings(finally_body, names);
        }
        Stmt::With { items, body, .. } | Stmt::AsyncWith { items, body, .. } => {
            for item in items {
                if let Some(target) = &item.optional_vars {
                    collect_target_bindings(target, names);
                }
            }
            collect_statement_bindings(body, names);
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
            if let Stmt::For { target, iter, .. } | Stmt::AsyncFor { target, iter, .. } = stmt {
                collect_expr_bindings(iter, names);
                collect_target_bindings(target, names);
            }
            collect_statement_bindings(body, names);
            collect_statement_bindings(else_body, names);
        }
        Stmt::Expr(expr) => collect_expr_bindings(expr, names),
        Stmt::Return(Some(expr)) => collect_expr_bindings(expr, names),
        Stmt::Raise { value, cause } => {
            if let Some(value) = value {
                collect_expr_bindings(value, names);
            }
            if let Some(cause) = cause {
                collect_expr_bindings(cause, names);
            }
        }
        Stmt::Assert { condition, message } => {
            collect_expr_bindings(condition, names);
            if let Some(message) = message {
                collect_expr_bindings(message, names);
            }
        }
        Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Pass
        | Stmt::Return(None)
        | Stmt::Break
        | Stmt::Continue => {}
    }
}

fn comprehension_named_expression_bindings(
    head_exprs: &[&Expr],
    clauses: &[ComprehensionClause],
) -> HashSet<String> {
    let mut names = HashSet::new();

    for expr in head_exprs {
        collect_expr_bindings(expr, &mut names);
    }

    for clause in clauses {
        for condition in &clause.ifs {
            collect_expr_bindings(condition, &mut names);
        }
    }

    names
}

fn collect_expr_bindings(expr: &Expr, names: &mut HashSet<String>) {
    match expr {
        Expr::NamedExpr { name, value } => {
            collect_expr_bindings(value, names);
            names.insert(name.clone());
        }
        Expr::Attribute { object, .. } => collect_expr_bindings(object, names),
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            collect_expr_bindings(left, names);
            collect_expr_bindings(right, names);
        }
        Expr::ChainedComparison { left, comparisons } => {
            collect_expr_bindings(left, names);
            for (_, right) in comparisons {
                collect_expr_bindings(right, names);
            }
        }
        Expr::Unary { operand, .. }
        | Expr::YieldFrom(operand)
        | Expr::Await(operand)
        | Expr::Starred(operand) => collect_expr_bindings(operand, names),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_expr_bindings(condition, names);
            collect_expr_bindings(then_branch, names);
            collect_expr_bindings(else_branch, names);
        }
        Expr::Yield { value } => {
            if let Some(value) = value {
                collect_expr_bindings(value, names);
            }
        }
        Expr::List(items) | Expr::Set(items) | Expr::FrozenSet(items) | Expr::Tuple(items) => {
            for item in items {
                collect_expr_bindings(item, names);
            }
        }
        Expr::Dict(entries) => {
            for entry in entries {
                match entry {
                    DictItem::Entry { key, value } => {
                        collect_expr_bindings(key, names);
                        collect_expr_bindings(value, names);
                    }
                    DictItem::Unpack(value) => collect_expr_bindings(value, names),
                }
            }
        }
        Expr::Subscript { object, index } => {
            collect_expr_bindings(object, names);
            collect_expr_bindings(index, names);
        }
        Expr::SliceLiteral { start, stop, step } => {
            collect_optional_expr_bindings(start.as_deref(), names);
            collect_optional_expr_bindings(stop.as_deref(), names);
            collect_optional_expr_bindings(step.as_deref(), names);
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_expr_bindings(object, names);
            collect_optional_expr_bindings(start.as_deref(), names);
            collect_optional_expr_bindings(stop.as_deref(), names);
            collect_optional_expr_bindings(step.as_deref(), names);
        }
        Expr::Call { callee, args } => {
            collect_expr_bindings(callee, names);
            for arg in args {
                collect_expr_bindings(arg, names);
            }
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            collect_expr_bindings(callee, names);
            for arg in args {
                collect_expr_bindings(arg, names);
            }
            for (_, value) in keywords {
                collect_expr_bindings(value, names);
            }
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            collect_expr_bindings(callee, names);
            for arg in args {
                match arg {
                    CallArg::Expr(expr) | CallArg::Unpack(expr) => {
                        collect_expr_bindings(expr, names);
                    }
                }
            }
            for keyword in keywords {
                match keyword {
                    CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
                        collect_expr_bindings(expr, names);
                    }
                }
            }
        }
        Expr::Lambda { params, .. } => collect_param_runtime_bindings(params, names),
        Expr::JoinedString(parts) => collect_f_string_bindings(parts, names),
        Expr::TemplateString(parts) => {
            for part in parts {
                match part {
                    TemplateStringPart::Literal(_) => {}
                    TemplateStringPart::Interpolation {
                        value, format_spec, ..
                    } => {
                        collect_expr_bindings(value, names);
                        if let Some(format_spec) = format_spec {
                            collect_f_string_bindings(format_spec, names);
                        }
                    }
                }
            }
        }
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            collect_expr_bindings(value, names);
            if let Some(format_spec) = format_spec {
                collect_f_string_bindings(format_spec, names);
            }
        }
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            if let Some(first_clause) = clauses.first() {
                collect_expr_bindings(&first_clause.iter, names);
            }
            for name in comprehension_named_expression_bindings(&[element], clauses) {
                names.insert(name);
            }
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            if let Some(first_clause) = clauses.first() {
                collect_expr_bindings(&first_clause.iter, names);
            }
            for name in comprehension_named_expression_bindings(&[key, value], clauses) {
                names.insert(name);
            }
        }
        Expr::DictUnpackComp { value, clauses } => {
            if let Some(first_clause) = clauses.first() {
                collect_expr_bindings(&first_clause.iter, names);
            }
            for name in comprehension_named_expression_bindings(&[value], clauses) {
                names.insert(name);
            }
        }
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
    }
}

fn collect_optional_expr_bindings(expr: Option<&Expr>, names: &mut HashSet<String>) {
    if let Some(expr) = expr {
        collect_expr_bindings(expr, names);
    }
}

fn collect_param_runtime_bindings(params: &FunctionParams, names: &mut HashSet<String>) {
    for param in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
    {
        collect_optional_expr_bindings(param.annotation.as_ref(), names);
        collect_optional_expr_bindings(param.default.as_ref(), names);
    }
    collect_optional_expr_bindings(params.vararg_annotation.as_deref(), names);
    for param in &params.keyword_only {
        collect_optional_expr_bindings(param.annotation.as_ref(), names);
        collect_optional_expr_bindings(param.default.as_ref(), names);
    }
    collect_optional_expr_bindings(params.kwarg_annotation.as_deref(), names);
}

fn collect_f_string_bindings(parts: &[FStringPart], names: &mut HashSet<String>) {
    for part in parts {
        match part {
            FStringPart::Literal(_) => {}
            FStringPart::Formatted {
                value, format_spec, ..
            } => {
                collect_expr_bindings(value, names);
                if let Some(format_spec) = format_spec {
                    collect_f_string_bindings(format_spec, names);
                }
            }
        }
    }
}

fn collect_target_bindings(target: &Target, names: &mut HashSet<String>) {
    match target {
        Target::Name(name) => {
            names.insert(name.clone());
        }
        Target::Starred(target) => collect_target_bindings(target, names),
        Target::Tuple(targets) | Target::List(targets) => {
            for target in targets {
                collect_target_bindings(target, names);
            }
        }
        Target::Attribute { object, .. } => collect_expr_bindings(object, names),
        Target::Subscript { object, index } => {
            collect_expr_bindings(object, names);
            collect_expr_bindings(index, names);
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_expr_bindings(object, names);
            collect_optional_expr_bindings(start.as_ref(), names);
            collect_optional_expr_bindings(stop.as_ref(), names);
            collect_optional_expr_bindings(step.as_ref(), names);
        }
    }
}

fn collect_comprehension_target_locals(target: &Target, names: &mut HashSet<String>) {
    match target {
        Target::Name(name) => {
            names.insert(name.clone());
        }
        Target::Starred(target) => collect_comprehension_target_locals(target, names),
        Target::Tuple(targets) | Target::List(targets) => {
            for target in targets {
                collect_comprehension_target_locals(target, names);
            }
        }
        Target::Attribute { .. } | Target::Subscript { .. } | Target::Slice { .. } => {}
    }
}

fn collect_pattern_bindings(pattern: &Pattern, names: &mut HashSet<String>) {
    match pattern {
        Pattern::Capture(name) | Pattern::Star(Some(name)) => {
            names.insert(name.clone());
        }
        Pattern::As { pattern, name } => {
            collect_pattern_bindings(pattern, names);
            names.insert(name.clone());
        }
        Pattern::Or(patterns) | Pattern::Sequence(patterns) => {
            for pattern in patterns {
                collect_pattern_bindings(pattern, names);
            }
        }
        Pattern::Mapping { entries, rest } => {
            for (_, pattern) in entries {
                collect_pattern_bindings(pattern, names);
            }
            if let Some(rest) = rest {
                names.insert(rest.clone());
            }
        }
        Pattern::Class {
            positional,
            keywords,
            ..
        } => {
            for pattern in positional {
                collect_pattern_bindings(pattern, names);
            }
            for (_, pattern) in keywords {
                collect_pattern_bindings(pattern, names);
            }
        }
        Pattern::Literal(_)
        | Pattern::Singleton(_)
        | Pattern::Value(_)
        | Pattern::Wildcard
        | Pattern::Star(None) => {}
    }
}

fn same_match_binding_names(expected: &[String], bindings: &[MatchBinding]) -> bool {
    expected.len() == bindings.len()
        && expected
            .iter()
            .all(|name| bindings.iter().any(|binding| &binding.name == name))
}

fn build_match_bindings(names: &[String], registers: &[Register]) -> Vec<MatchBinding> {
    names
        .iter()
        .zip(registers)
        .map(|(name, src)| MatchBinding {
            name: name.clone(),
            src: *src,
        })
        .collect()
}

fn is_match_literal_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Bool(_)
        | Expr::None => true,
        Expr::Unary { op, operand } => {
            matches!(op, UnaryOp::Negative)
                && matches!(
                    operand.as_ref(),
                    Expr::Number(_) | Expr::BigInt(_) | Expr::Float(_) | Expr::Imaginary(_)
                )
        }
        Expr::Binary { left, op, right } => {
            matches!(op, BinaryOp::Add | BinaryOp::Subtract)
                && is_match_signed_real_expr(left)
                && matches!(right.as_ref(), Expr::Imaginary(_))
        }
        _ => false,
    }
}

fn is_match_mapping_key_expr(expr: &Expr) -> bool {
    is_match_literal_expr(expr) || matches!(expr, Expr::Attribute { .. })
}

fn is_match_signed_real_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_) | Expr::BigInt(_) | Expr::Float(_) => true,
        Expr::Unary { op, operand } => {
            matches!(op, UnaryOp::Negative)
                && matches!(
                    operand.as_ref(),
                    Expr::Number(_) | Expr::BigInt(_) | Expr::Float(_)
                )
        }
        _ => false,
    }
}

fn import_binding_name(alias: &ImportAlias) -> &str {
    alias
        .asname
        .as_deref()
        .unwrap_or_else(|| import_root_name(&alias.name))
}

fn import_root_name(name: &str) -> &str {
    name.split('.')
        .next()
        .expect("import name always has at least one part")
}

fn type_param_kind_label(kind: &TypeParamKind) -> &'static str {
    match kind {
        TypeParamKind::TypeVar => "TypeVar",
        TypeParamKind::TypeVarTuple => "TypeVarTuple",
        TypeParamKind::ParamSpec => "ParamSpec",
    }
}

fn statements_contain_yield(statements: &[Stmt]) -> bool {
    statements.iter().any(stmt_contains_yield)
}

fn statements_contain_yield_from(statements: &[Stmt]) -> bool {
    statements.iter().any(stmt_contains_yield_from)
}

fn statements_contain_return_value(statements: &[Stmt]) -> bool {
    statements.iter().any(stmt_contains_return_value)
}

fn stmt_contains_return_value(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(Some(_)) => true,
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            statements_contain_return_value(then_body) || statements_contain_return_value(else_body)
        }
        Stmt::Match { cases, .. } => cases
            .iter()
            .any(|case| statements_contain_return_value(&case.body)),
        Stmt::While {
            body, else_body, ..
        }
        | Stmt::For {
            body, else_body, ..
        }
        | Stmt::AsyncFor {
            body, else_body, ..
        } => statements_contain_return_value(body) || statements_contain_return_value(else_body),
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
            statements_contain_return_value(body)
                || handlers
                    .iter()
                    .any(|handler| statements_contain_return_value(&handler.body))
                || statements_contain_return_value(else_body)
                || statements_contain_return_value(finally_body)
        }
        Stmt::With { body, .. } | Stmt::AsyncWith { body, .. } => {
            statements_contain_return_value(body)
        }
        Stmt::FunctionDef { .. }
        | Stmt::AsyncFunctionDef { .. }
        | Stmt::ClassDef { .. }
        | Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Pass
        | Stmt::Delete { .. }
        | Stmt::Expr(_)
        | Stmt::Assign { .. }
        | Stmt::AnnAssign { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::AugAssign { .. }
        | Stmt::Return(None)
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Assert { .. }
        | Stmt::Raise { .. }
        | Stmt::Break
        | Stmt::Continue => false,
    }
}

fn stmt_contains_yield(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr) => expr_contains_yield(expr),
        Stmt::Assign { value, .. } | Stmt::AugAssign { value, .. } => expr_contains_yield(value),
        Stmt::TypeAlias {
            type_params, value, ..
        } => type_params_contain_yield(type_params) || expr_contains_yield(value),
        Stmt::AnnAssign {
            target,
            annotation,
            value,
            ..
        } => {
            target_contains_yield(target)
                || expr_contains_yield(annotation)
                || value.as_ref().is_some_and(expr_contains_yield)
        }
        Stmt::Return(Some(value)) => expr_contains_yield(value),
        Stmt::Raise { value, cause } => {
            value.as_ref().is_some_and(expr_contains_yield)
                || cause.as_ref().is_some_and(expr_contains_yield)
        }
        Stmt::Assert { condition, message } => {
            expr_contains_yield(condition) || message.as_ref().is_some_and(expr_contains_yield)
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_contains_yield(condition)
                || statements_contain_yield(then_body)
                || statements_contain_yield(else_body)
        }
        Stmt::Match { subject, cases } => {
            expr_contains_yield(subject)
                || cases.iter().any(|case| {
                    case.guard.as_ref().is_some_and(expr_contains_yield)
                        || statements_contain_yield(&case.body)
                })
        }
        Stmt::While {
            condition,
            body,
            else_body,
        } => {
            expr_contains_yield(condition)
                || statements_contain_yield(body)
                || statements_contain_yield(else_body)
        }
        Stmt::For {
            iter,
            body,
            else_body,
            ..
        }
        | Stmt::AsyncFor {
            iter,
            body,
            else_body,
            ..
        } => {
            expr_contains_yield(iter)
                || statements_contain_yield(body)
                || statements_contain_yield(else_body)
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
            statements_contain_yield(body)
                || handlers
                    .iter()
                    .any(|handler| statements_contain_yield(&handler.body))
                || statements_contain_yield(else_body)
                || statements_contain_yield(finally_body)
        }
        Stmt::With { items, body, .. } | Stmt::AsyncWith { items, body, .. } => {
            items.iter().any(|item| {
                expr_contains_yield(&item.context_expr)
                    || item
                        .optional_vars
                        .as_ref()
                        .is_some_and(target_contains_yield)
            }) || statements_contain_yield(body)
        }
        Stmt::FunctionDef { type_params, .. } | Stmt::AsyncFunctionDef { type_params, .. } => {
            type_params_contain_yield(type_params)
        }
        Stmt::ClassDef {
            type_params,
            bases,
            keywords,
            ..
        } => {
            type_params_contain_yield(type_params)
                || call_args_contain_yield(bases)
                || call_keywords_contain_yield(keywords)
        }
        Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Pass
        | Stmt::Delete { .. }
        | Stmt::Return(None)
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Break
        | Stmt::Continue => false,
    }
}

fn stmt_contains_yield_from(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(expr) => expr_contains_yield_from(expr),
        Stmt::Assign { value, .. } | Stmt::AugAssign { value, .. } => {
            expr_contains_yield_from(value)
        }
        Stmt::AnnAssign {
            target,
            annotation,
            value,
            ..
        } => {
            target_contains_yield_from(target)
                || expr_contains_yield_from(annotation)
                || value.as_ref().is_some_and(expr_contains_yield_from)
        }
        Stmt::TypeAlias {
            type_params, value, ..
        } => type_params_contain_yield_from(type_params) || expr_contains_yield_from(value),
        Stmt::Return(Some(value)) => expr_contains_yield_from(value),
        Stmt::Raise { value, cause } => {
            value.as_ref().is_some_and(expr_contains_yield_from)
                || cause.as_ref().is_some_and(expr_contains_yield_from)
        }
        Stmt::Assert { condition, message } => {
            expr_contains_yield_from(condition)
                || message.as_ref().is_some_and(expr_contains_yield_from)
        }
        Stmt::If {
            condition,
            then_body,
            else_body,
        } => {
            expr_contains_yield_from(condition)
                || statements_contain_yield_from(then_body)
                || statements_contain_yield_from(else_body)
        }
        Stmt::Match { subject, cases } => {
            expr_contains_yield_from(subject)
                || cases.iter().any(|case| {
                    case.guard.as_ref().is_some_and(expr_contains_yield_from)
                        || statements_contain_yield_from(&case.body)
                })
        }
        Stmt::While {
            condition,
            body,
            else_body,
        } => {
            expr_contains_yield_from(condition)
                || statements_contain_yield_from(body)
                || statements_contain_yield_from(else_body)
        }
        Stmt::For {
            iter,
            body,
            else_body,
            ..
        }
        | Stmt::AsyncFor {
            iter,
            body,
            else_body,
            ..
        } => {
            expr_contains_yield_from(iter)
                || statements_contain_yield_from(body)
                || statements_contain_yield_from(else_body)
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
            statements_contain_yield_from(body)
                || handlers
                    .iter()
                    .any(|handler| statements_contain_yield_from(&handler.body))
                || statements_contain_yield_from(else_body)
                || statements_contain_yield_from(finally_body)
        }
        Stmt::With { items, body, .. } | Stmt::AsyncWith { items, body, .. } => {
            items.iter().any(|item| {
                expr_contains_yield_from(&item.context_expr)
                    || item
                        .optional_vars
                        .as_ref()
                        .is_some_and(target_contains_yield_from)
            }) || statements_contain_yield_from(body)
        }
        Stmt::FunctionDef { type_params, .. } | Stmt::AsyncFunctionDef { type_params, .. } => {
            type_params_contain_yield_from(type_params)
        }
        Stmt::ClassDef {
            type_params,
            bases,
            keywords,
            ..
        } => {
            type_params_contain_yield_from(type_params)
                || call_args_contain_yield_from(bases)
                || call_keywords_contain_yield_from(keywords)
        }
        Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Pass
        | Stmt::Delete { .. }
        | Stmt::Return(None)
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Break
        | Stmt::Continue => false,
    }
}

fn reject_except_star_control_flow(handlers: &[AstExceptHandler]) -> Result<(), String> {
    if handlers
        .iter()
        .any(|handler| statements_contain_except_star_control_flow(&handler.body))
    {
        return Err(
            "'break', 'continue' and 'return' cannot appear in an except* block".to_string(),
        );
    }

    Ok(())
}

fn except_type_names_from_expr(type_expr: Option<&Expr>) -> Result<Option<Vec<String>>, String> {
    let Some(type_expr) = type_expr else {
        return Ok(None);
    };

    match type_expr {
        Expr::Tuple(elements) => {
            if elements.is_empty() {
                return Err("empty exception type tuple".to_string());
            }
            elements
                .iter()
                .map(except_type_name_from_expr)
                .collect::<Result<Vec<_>, _>>()
                .map(Some)
        }
        expr => except_type_name_from_expr(expr).map(|name| Some(vec![name])),
    }
}

fn except_type_name_from_expr(expr: &Expr) -> Result<String, String> {
    match expr {
        Expr::Name(name) => Ok(name.clone()),
        Expr::Attribute { object, name } => {
            let object = except_type_name_from_expr(object)?;
            Ok(format!("{object}.{name}"))
        }
        expr => Err(format!("unsupported except type expression: {expr:?}")),
    }
}

fn statements_contain_except_star_control_flow(statements: &[Stmt]) -> bool {
    statements
        .iter()
        .any(stmt_contains_except_star_control_flow)
}

fn stmt_contains_except_star_control_flow(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) | Stmt::Break | Stmt::Continue => true,
        Stmt::If {
            then_body,
            else_body,
            ..
        } => {
            statements_contain_except_star_control_flow(then_body)
                || statements_contain_except_star_control_flow(else_body)
        }
        Stmt::Match { cases, .. } => cases
            .iter()
            .any(|case| statements_contain_except_star_control_flow(&case.body)),
        Stmt::While {
            body, else_body, ..
        }
        | Stmt::For {
            body, else_body, ..
        }
        | Stmt::AsyncFor {
            body, else_body, ..
        } => {
            statements_contain_except_star_control_flow(body)
                || statements_contain_except_star_control_flow(else_body)
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
            statements_contain_except_star_control_flow(body)
                || handlers
                    .iter()
                    .any(|handler| statements_contain_except_star_control_flow(&handler.body))
                || statements_contain_except_star_control_flow(else_body)
                || statements_contain_except_star_control_flow(finally_body)
        }
        Stmt::With { body, .. } | Stmt::AsyncWith { body, .. } => {
            statements_contain_except_star_control_flow(body)
        }
        Stmt::FunctionDef { .. } | Stmt::AsyncFunctionDef { .. } | Stmt::ClassDef { .. } => false,
        Stmt::Pass
        | Stmt::Expr(_)
        | Stmt::Assign { .. }
        | Stmt::AnnAssign { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::AugAssign { .. }
        | Stmt::Delete { .. }
        | Stmt::Import { .. }
        | Stmt::ImportFrom { .. }
        | Stmt::Global(_)
        | Stmt::Nonlocal(_)
        | Stmt::Assert { .. }
        | Stmt::Raise { .. } => false,
    }
}

fn type_params_contain_yield(type_params: &[TypeParam]) -> bool {
    type_params.iter().any(|type_param| {
        type_param.bound.as_ref().is_some_and(expr_contains_yield)
            || type_param.default.as_ref().is_some_and(expr_contains_yield)
    })
}

fn call_args_contain_yield(args: &[CallArg]) -> bool {
    args.iter().any(|arg| match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => expr_contains_yield(expr),
    })
}

fn call_keywords_contain_yield(keywords: &[CallKeyword]) -> bool {
    keywords.iter().any(|keyword| match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => expr_contains_yield(expr),
    })
}

fn expr_contains_yield(expr: &Expr) -> bool {
    match expr {
        Expr::Yield { .. } | Expr::YieldFrom(_) => true,
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            expr_contains_yield(left) || expr_contains_yield(right)
        }
        Expr::ChainedComparison { left, comparisons } => {
            expr_contains_yield(left)
                || comparisons
                    .iter()
                    .any(|(_, expr)| expr_contains_yield(expr))
        }
        Expr::Unary { operand, .. } | Expr::Await(operand) | Expr::Starred(operand) => {
            expr_contains_yield(operand)
        }
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_yield(condition)
                || expr_contains_yield(then_branch)
                || expr_contains_yield(else_branch)
        }
        Expr::NamedExpr { value, .. } => expr_contains_yield(value),
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => elements.iter().any(expr_contains_yield),
        Expr::JoinedString(parts) => parts.iter().any(|part| match part {
            FStringPart::Literal(_) => false,
            FStringPart::Formatted {
                value, format_spec, ..
            } => {
                expr_contains_yield(value)
                    || format_spec
                        .as_deref()
                        .is_some_and(f_string_parts_contain_yield)
            }
        }),
        Expr::TemplateString(parts) => parts.iter().any(|part| match part {
            TemplateStringPart::Literal(_) => false,
            TemplateStringPart::Interpolation {
                value, format_spec, ..
            } => {
                expr_contains_yield(value)
                    || format_spec
                        .as_deref()
                        .is_some_and(f_string_parts_contain_yield)
            }
        }),
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            expr_contains_yield(value)
                || format_spec
                    .as_deref()
                    .is_some_and(f_string_parts_contain_yield)
        }
        Expr::ListComp { element, clauses } | Expr::SetComp { element, clauses } => {
            comprehension_contains_yield(element, clauses)
        }
        Expr::GeneratorComp { .. } => false,
        Expr::Dict(entries) => entries.iter().any(|entry| match entry {
            DictItem::Entry { key, value } => {
                expr_contains_yield(key) || expr_contains_yield(value)
            }
            DictItem::Unpack(expr) => expr_contains_yield(expr),
        }),
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            expr_contains_yield(key)
                || expr_contains_yield(value)
                || clauses.iter().any(|clause| {
                    target_contains_yield(&clause.target)
                        || expr_contains_yield(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_yield)
                })
        }
        Expr::DictUnpackComp { value, clauses } => {
            expr_contains_yield(value)
                || clauses.iter().any(|clause| {
                    target_contains_yield(&clause.target)
                        || expr_contains_yield(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_yield)
                })
        }
        Expr::Subscript { object, index } => {
            expr_contains_yield(object) || expr_contains_yield(index)
        }
        Expr::SliceLiteral { start, stop, step } => {
            start.as_deref().is_some_and(expr_contains_yield)
                || stop.as_deref().is_some_and(expr_contains_yield)
                || step.as_deref().is_some_and(expr_contains_yield)
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_contains_yield(object)
                || start.as_deref().is_some_and(expr_contains_yield)
                || stop.as_deref().is_some_and(expr_contains_yield)
                || step.as_deref().is_some_and(expr_contains_yield)
        }
        Expr::Attribute { object, .. } => expr_contains_yield(object),
        Expr::Call { callee, args } => {
            expr_contains_yield(callee) || args.iter().any(expr_contains_yield)
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            expr_contains_yield(callee)
                || args.iter().any(expr_contains_yield)
                || keywords.iter().any(|(_, expr)| expr_contains_yield(expr))
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            expr_contains_yield(callee)
                || args.iter().any(|arg| match arg {
                    CallArg::Expr(expr) | CallArg::Unpack(expr) => expr_contains_yield(expr),
                })
                || keywords.iter().any(|keyword| match keyword {
                    CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
                        expr_contains_yield(expr)
                    }
                })
        }
        Expr::Lambda { .. }
        | Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Bool(_)
        | Expr::None
        | Expr::Ellipsis
        | Expr::Name(_) => false,
    }
}

fn f_string_parts_contain_yield(parts: &[FStringPart]) -> bool {
    parts.iter().any(|part| match part {
        FStringPart::Literal(_) => false,
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            expr_contains_yield(value)
                || format_spec
                    .as_deref()
                    .is_some_and(f_string_parts_contain_yield)
        }
    })
}

fn comprehension_contains_yield(element: &Expr, clauses: &[ComprehensionClause]) -> bool {
    expr_contains_yield(element)
        || clauses.iter().any(|clause| {
            target_contains_yield(&clause.target)
                || expr_contains_yield(&clause.iter)
                || clause.ifs.iter().any(expr_contains_yield)
        })
}

fn comprehension_inner_contains_yield(
    result_exprs: &[&Expr],
    clauses: &[ComprehensionClause],
) -> bool {
    result_exprs.iter().any(|expr| expr_contains_yield(expr))
        || clauses.iter().enumerate().any(|(index, clause)| {
            target_contains_yield(&clause.target)
                || (index > 0 && expr_contains_yield(&clause.iter))
                || clause.ifs.iter().any(expr_contains_yield)
        })
}

fn comprehension_inner_contains_await(
    result_exprs: &[&Expr],
    clauses: &[ComprehensionClause],
) -> bool {
    result_exprs
        .iter()
        .any(|expr| expr_contains_await_for_comprehension(expr))
        || clauses.iter().enumerate().any(|(index, clause)| {
            target_contains_await_for_comprehension(&clause.target)
                || (index > 0 && expr_contains_await_for_comprehension(&clause.iter))
                || clause.ifs.iter().any(expr_contains_await_for_comprehension)
        })
}

fn generator_comprehension_is_async(element: &Expr, clauses: &[ComprehensionClause]) -> bool {
    clauses.iter().any(|clause| clause.is_async)
        || comprehension_inner_contains_await(&[element], clauses)
}

fn expr_contains_await_for_comprehension(expr: &Expr) -> bool {
    match expr {
        Expr::Await(_) => true,
        Expr::Yield { value } => value
            .as_deref()
            .is_some_and(expr_contains_await_for_comprehension),
        Expr::YieldFrom(value) | Expr::Unary { operand: value, .. } | Expr::Starred(value) => {
            expr_contains_await_for_comprehension(value)
        }
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            expr_contains_await_for_comprehension(left)
                || expr_contains_await_for_comprehension(right)
        }
        Expr::ChainedComparison { left, comparisons } => {
            expr_contains_await_for_comprehension(left)
                || comparisons
                    .iter()
                    .any(|(_, expr)| expr_contains_await_for_comprehension(expr))
        }
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_await_for_comprehension(condition)
                || expr_contains_await_for_comprehension(then_branch)
                || expr_contains_await_for_comprehension(else_branch)
        }
        Expr::NamedExpr { value, .. } => expr_contains_await_for_comprehension(value),
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => elements.iter().any(expr_contains_await_for_comprehension),
        Expr::JoinedString(parts) => parts
            .iter()
            .any(f_string_part_contains_await_for_comprehension),
        Expr::TemplateString(parts) => parts
            .iter()
            .any(template_string_part_contains_await_for_comprehension),
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            expr_contains_await_for_comprehension(value)
                || format_spec.as_deref().is_some_and(|parts| {
                    parts
                        .iter()
                        .any(f_string_part_contains_await_for_comprehension)
                })
        }
        Expr::ListComp { element, clauses } | Expr::SetComp { element, clauses } => {
            expr_contains_await_for_comprehension(element)
                || clauses.iter().any(|clause| {
                    target_contains_await_for_comprehension(&clause.target)
                        || expr_contains_await_for_comprehension(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_await_for_comprehension)
                })
        }
        Expr::GeneratorComp { .. } => false,
        Expr::Dict(entries) => entries.iter().any(|entry| match entry {
            DictItem::Entry { key, value } => {
                expr_contains_await_for_comprehension(key)
                    || expr_contains_await_for_comprehension(value)
            }
            DictItem::Unpack(value) => expr_contains_await_for_comprehension(value),
        }),
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            expr_contains_await_for_comprehension(key)
                || expr_contains_await_for_comprehension(value)
                || clauses.iter().any(|clause| {
                    target_contains_await_for_comprehension(&clause.target)
                        || expr_contains_await_for_comprehension(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_await_for_comprehension)
                })
        }
        Expr::DictUnpackComp { value, clauses } => {
            expr_contains_await_for_comprehension(value)
                || clauses.iter().any(|clause| {
                    target_contains_await_for_comprehension(&clause.target)
                        || expr_contains_await_for_comprehension(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_await_for_comprehension)
                })
        }
        Expr::Subscript { object, index } => {
            expr_contains_await_for_comprehension(object)
                || expr_contains_await_for_comprehension(index)
        }
        Expr::SliceLiteral { start, stop, step } => {
            start
                .as_deref()
                .is_some_and(expr_contains_await_for_comprehension)
                || stop
                    .as_deref()
                    .is_some_and(expr_contains_await_for_comprehension)
                || step
                    .as_deref()
                    .is_some_and(expr_contains_await_for_comprehension)
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_contains_await_for_comprehension(object)
                || start
                    .as_deref()
                    .is_some_and(expr_contains_await_for_comprehension)
                || stop
                    .as_deref()
                    .is_some_and(expr_contains_await_for_comprehension)
                || step
                    .as_deref()
                    .is_some_and(expr_contains_await_for_comprehension)
        }
        Expr::Attribute { object, .. } => expr_contains_await_for_comprehension(object),
        Expr::Call { callee, args } => {
            expr_contains_await_for_comprehension(callee)
                || args.iter().any(expr_contains_await_for_comprehension)
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            expr_contains_await_for_comprehension(callee)
                || args.iter().any(expr_contains_await_for_comprehension)
                || keywords
                    .iter()
                    .any(|(_, expr)| expr_contains_await_for_comprehension(expr))
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            expr_contains_await_for_comprehension(callee)
                || args.iter().any(|arg| match arg {
                    CallArg::Expr(expr) | CallArg::Unpack(expr) => {
                        expr_contains_await_for_comprehension(expr)
                    }
                })
                || keywords.iter().any(|keyword| match keyword {
                    CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
                        expr_contains_await_for_comprehension(expr)
                    }
                })
        }
        Expr::Lambda { params, .. } => function_params_contain_await_for_comprehension(params),
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Bool(_)
        | Expr::None
        | Expr::Ellipsis
        | Expr::Name(_) => false,
    }
}

fn f_string_part_contains_await_for_comprehension(part: &FStringPart) -> bool {
    match part {
        FStringPart::Literal(_) => false,
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            expr_contains_await_for_comprehension(value)
                || format_spec.as_deref().is_some_and(|parts| {
                    parts
                        .iter()
                        .any(f_string_part_contains_await_for_comprehension)
                })
        }
    }
}

fn template_string_part_contains_await_for_comprehension(part: &TemplateStringPart) -> bool {
    match part {
        TemplateStringPart::Literal(_) => false,
        TemplateStringPart::Interpolation {
            value, format_spec, ..
        } => {
            expr_contains_await_for_comprehension(value)
                || format_spec.as_deref().is_some_and(|parts| {
                    parts
                        .iter()
                        .any(f_string_part_contains_await_for_comprehension)
                })
        }
    }
}

fn function_params_contain_await_for_comprehension(params: &FunctionParams) -> bool {
    params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
        .any(param_contains_await_for_comprehension)
        || params
            .vararg_annotation
            .as_deref()
            .is_some_and(expr_contains_await_for_comprehension)
        || params
            .kwarg_annotation
            .as_deref()
            .is_some_and(expr_contains_await_for_comprehension)
}

fn param_contains_await_for_comprehension(param: &Param) -> bool {
    param
        .annotation
        .as_ref()
        .is_some_and(expr_contains_await_for_comprehension)
        || param
            .default
            .as_ref()
            .is_some_and(expr_contains_await_for_comprehension)
}

fn target_contains_await_for_comprehension(target: &Target) -> bool {
    match target {
        Target::Attribute { object, .. } => expr_contains_await_for_comprehension(object),
        Target::Subscript { object, index } => {
            expr_contains_await_for_comprehension(object)
                || expr_contains_await_for_comprehension(index)
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_contains_await_for_comprehension(object)
                || start
                    .as_ref()
                    .is_some_and(expr_contains_await_for_comprehension)
                || stop
                    .as_ref()
                    .is_some_and(expr_contains_await_for_comprehension)
                || step
                    .as_ref()
                    .is_some_and(expr_contains_await_for_comprehension)
        }
        Target::Starred(target) => target_contains_await_for_comprehension(target),
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(target_contains_await_for_comprehension)
        }
        Target::Name(_) => false,
    }
}

fn target_contains_yield(target: &Target) -> bool {
    match target {
        Target::Attribute { object, .. } => expr_contains_yield(object),
        Target::Subscript { object, index } => {
            expr_contains_yield(object) || expr_contains_yield(index)
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_contains_yield(object)
                || start.as_ref().is_some_and(expr_contains_yield)
                || stop.as_ref().is_some_and(expr_contains_yield)
                || step.as_ref().is_some_and(expr_contains_yield)
        }
        Target::Starred(target) => target_contains_yield(target),
        Target::Tuple(targets) | Target::List(targets) => targets.iter().any(target_contains_yield),
        Target::Name(_) => false,
    }
}

fn type_params_contain_yield_from(type_params: &[TypeParam]) -> bool {
    type_params.iter().any(|type_param| {
        type_param
            .bound
            .as_ref()
            .is_some_and(expr_contains_yield_from)
            || type_param
                .default
                .as_ref()
                .is_some_and(expr_contains_yield_from)
    })
}

fn call_args_contain_yield_from(args: &[CallArg]) -> bool {
    args.iter().any(|arg| match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => expr_contains_yield_from(expr),
    })
}

fn call_keywords_contain_yield_from(keywords: &[CallKeyword]) -> bool {
    keywords.iter().any(|keyword| match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => expr_contains_yield_from(expr),
    })
}

fn expr_contains_yield_from(expr: &Expr) -> bool {
    match expr {
        Expr::YieldFrom(_) => true,
        Expr::Yield { value } => value.as_deref().is_some_and(expr_contains_yield_from),
        Expr::Binary { left, right, .. }
        | Expr::Comparison { left, right, .. }
        | Expr::Logical { left, right, .. } => {
            expr_contains_yield_from(left) || expr_contains_yield_from(right)
        }
        Expr::ChainedComparison { left, comparisons } => {
            expr_contains_yield_from(left)
                || comparisons
                    .iter()
                    .any(|(_, expr)| expr_contains_yield_from(expr))
        }
        Expr::Unary { operand, .. } | Expr::Await(operand) | Expr::Starred(operand) => {
            expr_contains_yield_from(operand)
        }
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_contains_yield_from(condition)
                || expr_contains_yield_from(then_branch)
                || expr_contains_yield_from(else_branch)
        }
        Expr::NamedExpr { value, .. } => expr_contains_yield_from(value),
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => elements.iter().any(expr_contains_yield_from),
        Expr::JoinedString(parts) => parts.iter().any(f_string_part_contains_yield_from),
        Expr::TemplateString(parts) => parts.iter().any(|part| match part {
            TemplateStringPart::Literal(_) => false,
            TemplateStringPart::Interpolation {
                value, format_spec, ..
            } => {
                expr_contains_yield_from(value)
                    || format_spec
                        .as_deref()
                        .is_some_and(f_string_parts_contain_yield_from)
            }
        }),
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            expr_contains_yield_from(value)
                || format_spec
                    .as_deref()
                    .is_some_and(f_string_parts_contain_yield_from)
        }
        Expr::ListComp { element, clauses } | Expr::SetComp { element, clauses } => {
            comprehension_contains_yield_from(element, clauses)
        }
        Expr::GeneratorComp { .. } => false,
        Expr::Dict(entries) => entries.iter().any(|entry| match entry {
            DictItem::Entry { key, value } => {
                expr_contains_yield_from(key) || expr_contains_yield_from(value)
            }
            DictItem::Unpack(expr) => expr_contains_yield_from(expr),
        }),
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            expr_contains_yield_from(key)
                || expr_contains_yield_from(value)
                || clauses.iter().any(|clause| {
                    target_contains_yield_from(&clause.target)
                        || expr_contains_yield_from(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_yield_from)
                })
        }
        Expr::DictUnpackComp { value, clauses } => {
            expr_contains_yield_from(value)
                || clauses.iter().any(|clause| {
                    target_contains_yield_from(&clause.target)
                        || expr_contains_yield_from(&clause.iter)
                        || clause.ifs.iter().any(expr_contains_yield_from)
                })
        }
        Expr::Subscript { object, index } => {
            expr_contains_yield_from(object) || expr_contains_yield_from(index)
        }
        Expr::SliceLiteral { start, stop, step } => {
            start.as_deref().is_some_and(expr_contains_yield_from)
                || stop.as_deref().is_some_and(expr_contains_yield_from)
                || step.as_deref().is_some_and(expr_contains_yield_from)
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_contains_yield_from(object)
                || start.as_deref().is_some_and(expr_contains_yield_from)
                || stop.as_deref().is_some_and(expr_contains_yield_from)
                || step.as_deref().is_some_and(expr_contains_yield_from)
        }
        Expr::Attribute { object, .. } => expr_contains_yield_from(object),
        Expr::Call { callee, args } => {
            expr_contains_yield_from(callee) || args.iter().any(expr_contains_yield_from)
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            expr_contains_yield_from(callee)
                || args.iter().any(expr_contains_yield_from)
                || keywords
                    .iter()
                    .any(|(_, expr)| expr_contains_yield_from(expr))
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            expr_contains_yield_from(callee)
                || args.iter().any(|arg| match arg {
                    CallArg::Expr(expr) | CallArg::Unpack(expr) => expr_contains_yield_from(expr),
                })
                || keywords.iter().any(|keyword| match keyword {
                    CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
                        expr_contains_yield_from(expr)
                    }
                })
        }
        Expr::Lambda { .. }
        | Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Bool(_)
        | Expr::None
        | Expr::Ellipsis
        | Expr::Name(_) => false,
    }
}

fn f_string_part_contains_yield_from(part: &FStringPart) -> bool {
    match part {
        FStringPart::Literal(_) => false,
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            expr_contains_yield_from(value)
                || format_spec
                    .as_deref()
                    .is_some_and(f_string_parts_contain_yield_from)
        }
    }
}

fn f_string_parts_contain_yield_from(parts: &[FStringPart]) -> bool {
    parts.iter().any(f_string_part_contains_yield_from)
}

fn comprehension_contains_yield_from(element: &Expr, clauses: &[ComprehensionClause]) -> bool {
    expr_contains_yield_from(element)
        || clauses.iter().any(|clause| {
            target_contains_yield_from(&clause.target)
                || expr_contains_yield_from(&clause.iter)
                || clause.ifs.iter().any(expr_contains_yield_from)
        })
}

fn target_contains_yield_from(target: &Target) -> bool {
    match target {
        Target::Attribute { object, .. } => expr_contains_yield_from(object),
        Target::Subscript { object, index } => {
            expr_contains_yield_from(object) || expr_contains_yield_from(index)
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_contains_yield_from(object)
                || start.as_ref().is_some_and(expr_contains_yield_from)
                || stop.as_ref().is_some_and(expr_contains_yield_from)
                || step.as_ref().is_some_and(expr_contains_yield_from)
        }
        Target::Starred(target) => target_contains_yield_from(target),
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(target_contains_yield_from)
        }
        Target::Name(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::compile;
    use crate::ast::{
        BinaryOp, ComparisonOp, ComprehensionClause, DictItem, ExceptHandler as AstExceptHandler,
        Expr, FStringConversion, FStringPart, LogicalOp, MatchCase, Pattern, Program, Stmt, Target,
        UnaryOp,
    };
    use crate::bytecode::{
        ExceptHandler as BytecodeExceptHandler, ExceptHandlerNameBinding, FormatConversion,
        Instruction,
    };
    use crate::value::{Value, complex_value, float_value, tuple_value};

    #[test]
    fn compiles_print_number_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("print".to_string())),
                args: vec![Expr::Number(123)],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(123)
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_float_literal_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Float("1.5".to_string()))],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: float_value(1.5)
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_imaginary_literal_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Imaginary("1.5".to_string()))],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: complex_value(0.0, 1.5)
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_addition_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("print".to_string())),
                args: vec![Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                }],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(2)
                },
                Instruction::Add {
                    dst: 3,
                    left: 1,
                    right: 2
                },
                Instruction::Call {
                    dst: 4,
                    callee: 0,
                    args: vec![3]
                },
                Instruction::Pop { src: 4 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_arithmetic_binary_ops_to_bytecode() {
        let cases = [
            (
                BinaryOp::Subtract,
                Instruction::Subtract {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::Multiply,
                Instruction::Multiply {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::MatrixMultiply,
                Instruction::MatrixMultiply {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::TrueDivide,
                Instruction::TrueDivide {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::FloorDivide,
                Instruction::FloorDivide {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::Modulo,
                Instruction::Modulo {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::Power,
                Instruction::Power {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::BitOr,
                Instruction::BitOr {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::BitXor,
                Instruction::BitXor {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::BitAnd,
                Instruction::BitAnd {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::LeftShift,
                Instruction::LeftShift {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
            (
                BinaryOp::RightShift,
                Instruction::RightShift {
                    dst: 2,
                    left: 0,
                    right: 1,
                },
            ),
        ];

        for (op, instruction) in cases {
            let program = Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(6)),
                    op,
                    right: Box::new(Expr::Number(2)),
                })],
            };

            assert_eq!(
                compile(&program),
                Ok(vec![
                    Instruction::LoadConst {
                        dst: 0,
                        value: Value::Number(6)
                    },
                    Instruction::LoadConst {
                        dst: 1,
                        value: Value::Number(2)
                    },
                    instruction,
                    Instruction::Pop { src: 2 },
                    Instruction::Halt,
                ])
            );
        }
    }

    #[test]
    fn compiles_ellipsis_literal_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Ellipsis)],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Ellipsis
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_positive_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Unary {
                op: UnaryOp::Positive,
                operand: Box::new(Expr::Number(1)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::Positive { dst: 1, src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_negative_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Unary {
                op: UnaryOp::Negative,
                operand: Box::new(Expr::Number(1)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(-1)
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_bitwise_invert_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Unary {
                op: UnaryOp::Invert,
                operand: Box::new(Expr::Number(1)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::Invert { dst: 1, src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_conditional_expression_to_short_circuit_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::IfExpression {
                condition: Box::new(Expr::Bool(true)),
                then_branch: Box::new(Expr::Number(1)),
                else_branch: Box::new(Expr::Number(2)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(true)
                },
                Instruction::JumpIfFalse {
                    condition: 0,
                    target: 5
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(1)
                },
                Instruction::Move { dst: 1, src: 2 },
                Instruction::Jump { target: 7 },
                Instruction::LoadConst {
                    dst: 3,
                    value: Value::Number(2)
                },
                Instruction::Move { dst: 1, src: 3 },
                Instruction::Pop { src: 1 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_assert_statement_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Assert {
                condition: Expr::Bool(false),
                message: Some(Expr::String("message".to_string())),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(false)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::String("message".to_string())
                },
                Instruction::Assert {
                    condition: 0,
                    message: Some(1)
                },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_raise_statement_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Raise {
                value: Some(Expr::Call {
                    callee: Box::new(Expr::Name("Exception".to_string())),
                    args: vec![Expr::String("boom".to_string())],
                }),
                cause: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "Exception".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::String("boom".to_string())
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Raise {
                    src: Some(2),
                    cause: None
                },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_raise_from_statement_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Raise {
                value: Some(Expr::Call {
                    callee: Box::new(Expr::Name("ValueError".to_string())),
                    args: vec![Expr::String("bad".to_string())],
                }),
                cause: Some(Expr::Call {
                    callee: Box::new(Expr::Name("Exception".to_string())),
                    args: vec![Expr::String("root".to_string())],
                }),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "ValueError".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::String("bad".to_string())
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::LoadName {
                    dst: 3,
                    name: "Exception".to_string()
                },
                Instruction::LoadConst {
                    dst: 4,
                    value: Value::String("root".to_string())
                },
                Instruction::Call {
                    dst: 5,
                    callee: 3,
                    args: vec![4]
                },
                Instruction::Raise {
                    src: Some(2),
                    cause: Some(5)
                },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_try_except_statement_to_handler_bytecode() {
        let program = Program {
            statements: vec![Stmt::Try {
                body: vec![Stmt::Raise {
                    value: Some(Expr::Call {
                        callee: Box::new(Expr::Name("Exception".to_string())),
                        args: vec![Expr::String("boom".to_string())],
                    }),
                    cause: None,
                }],
                handlers: vec![AstExceptHandler {
                    type_expr: Some(Expr::Name("Exception".to_string())),
                    name: Some("error".to_string()),
                    body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::Name("error".to_string())],
                    })],
                }],
                else_body: Vec::new(),
                finally_body: Vec::new(),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::SetupExcept {
                    handlers: vec![BytecodeExceptHandler {
                        type_names: Some(vec!["Exception".to_string()]),
                        type_register: None,
                        name: Some("error".to_string()),
                        name_binding: Some(ExceptHandlerNameBinding::Local),
                        target: 7,
                        is_star: false,
                    }]
                },
                Instruction::LoadName {
                    dst: 0,
                    name: "Exception".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::String("boom".to_string())
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Raise {
                    src: Some(2),
                    cause: None
                },
                Instruction::PopExcept,
                Instruction::Jump { target: 13 },
                Instruction::LoadName {
                    dst: 3,
                    name: "print".to_string()
                },
                Instruction::LoadName {
                    dst: 4,
                    name: "error".to_string()
                },
                Instruction::Call {
                    dst: 5,
                    callee: 3,
                    args: vec![4]
                },
                Instruction::Pop { src: 5 },
                Instruction::ClearException,
                Instruction::Jump { target: 13 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_multiple_call_arguments() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("print".to_string())),
                args: vec![Expr::Number(1), Expr::Number(2)],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(2)
                },
                Instruction::Call {
                    dst: 3,
                    callee: 0,
                    args: vec![1, 2]
                },
                Instruction::Pop { src: 3 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_unknown_callable_to_runtime_lookup() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("unknown".to_string())),
                args: vec![Expr::Number(1)],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "unknown".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_list_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::List(vec![
                Expr::Number(1),
                Expr::Binary {
                    left: Box::new(Expr::Number(2)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(3)),
                },
            ]))],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(3)
                },
                Instruction::Add {
                    dst: 3,
                    left: 1,
                    right: 2
                },
                Instruction::BuildList {
                    dst: 4,
                    items: vec![0, 3]
                },
                Instruction::Pop { src: 0 },
                Instruction::Pop { src: 3 },
                Instruction::Pop { src: 4 },
                Instruction::Halt,
            ])
        );
    }

    fn assert_comprehension_function_call(
        instructions: &[Instruction],
        function_name: &str,
        body_matches: impl Fn(&[Instruction]),
    ) {
        match instructions {
            [
                Instruction::LoadName {
                    dst: first_iter,
                    name,
                },
                Instruction::MakeFunction {
                    dst: callee,
                    name: actual_function_name,
                    params,
                    body,
                    is_generator,
                    ..
                },
                Instruction::Call {
                    dst,
                    callee: call_callee,
                    args,
                },
                Instruction::Pop {
                    src: popped_first_iter,
                },
                Instruction::Pop { src: popped_callee },
                Instruction::Pop { src: popped_result },
                Instruction::Halt,
            ] => {
                assert_eq!(name, "items");
                assert_eq!(actual_function_name, function_name);
                assert_eq!(params, &vec![".0".to_string()]);
                assert!(!is_generator);
                assert_eq!(call_callee, callee);
                assert_eq!(args, &vec![*first_iter]);
                assert_eq!(popped_first_iter, first_iter);
                assert_eq!(popped_callee, callee);
                assert_eq!(popped_result, dst);
                assert!(body.iter().any(|instruction| {
                    matches!(instruction, Instruction::LoadLocal { name, .. } if name == ".0")
                }));
                assert!(body.iter().any(|instruction| {
                    matches!(instruction, Instruction::StoreName { name, .. } if name == "x")
                }));
                assert!(body.iter().any(|instruction| matches!(
                    instruction,
                    Instruction::Return { src: Some(_) }
                )));
                body_matches(body);
            }
            instructions => panic!("unexpected comprehension bytecode: {instructions:?}"),
        }
    }

    #[test]
    fn compiles_list_comprehension_to_loop_and_append_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::ListComp {
                element: Box::new(Expr::Binary {
                    left: Box::new(Expr::Name("x".to_string())),
                    op: BinaryOp::Multiply,
                    right: Box::new(Expr::Number(2)),
                }),
                clauses: vec![ComprehensionClause {
                    is_async: false,
                    target: Target::Name("x".to_string()),
                    iter: Expr::Name("items".to_string()),
                    ifs: vec![Expr::Comparison {
                        left: Box::new(Expr::Name("x".to_string())),
                        op: ComparisonOp::Greater,
                        right: Box::new(Expr::Number(1)),
                    }],
                }],
            })],
        };

        let instructions = compile(&program).unwrap();
        assert_comprehension_function_call(&instructions, "<listcomp>", |body| {
            assert!(body.iter().any(|instruction| {
                matches!(instruction, Instruction::BuildList { items, .. } if items.is_empty())
            }));
            assert!(
                body.iter()
                    .any(|instruction| matches!(instruction, Instruction::ListAppend { .. }))
            );
        });
    }

    #[test]
    fn compiles_set_comprehension_to_loop_and_add_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::SetComp {
                element: Box::new(Expr::Binary {
                    left: Box::new(Expr::Name("x".to_string())),
                    op: BinaryOp::Multiply,
                    right: Box::new(Expr::Number(2)),
                }),
                clauses: vec![ComprehensionClause {
                    is_async: false,
                    target: Target::Name("x".to_string()),
                    iter: Expr::Name("items".to_string()),
                    ifs: vec![Expr::Comparison {
                        left: Box::new(Expr::Name("x".to_string())),
                        op: ComparisonOp::Greater,
                        right: Box::new(Expr::Number(1)),
                    }],
                }],
            })],
        };

        let instructions = compile(&program).unwrap();
        assert_comprehension_function_call(&instructions, "<setcomp>", |body| {
            assert!(body.iter().any(|instruction| {
                matches!(instruction, Instruction::BuildSet { items, .. } if items.is_empty())
            }));
            assert!(
                body.iter()
                    .any(|instruction| matches!(instruction, Instruction::SetAdd { .. }))
            );
        });
    }

    #[test]
    fn compiles_generator_expression_to_generator_function_call() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::GeneratorComp {
                element: Box::new(Expr::Binary {
                    left: Box::new(Expr::Name("x".to_string())),
                    op: BinaryOp::Multiply,
                    right: Box::new(Expr::Number(2)),
                }),
                clauses: vec![ComprehensionClause {
                    is_async: false,
                    target: Target::Name("x".to_string()),
                    iter: Expr::Name("items".to_string()),
                    ifs: vec![Expr::Comparison {
                        left: Box::new(Expr::Name("x".to_string())),
                        op: ComparisonOp::Greater,
                        right: Box::new(Expr::Number(1)),
                    }],
                }],
            })],
        };

        let instructions = compile(&program).unwrap();
        match &instructions[..] {
            [
                Instruction::LoadName {
                    dst: first_iter,
                    name,
                },
                Instruction::MakeFunction {
                    dst: callee,
                    name: function_name,
                    params,
                    body,
                    is_generator,
                    ..
                },
                Instruction::Call {
                    dst,
                    callee: call_callee,
                    args,
                },
                Instruction::Pop { src },
                Instruction::Halt,
            ] => {
                assert_eq!(name, "items");
                assert_eq!(function_name, "<genexpr>");
                assert_eq!(params, &vec![".0".to_string()]);
                assert!(*is_generator);
                assert_eq!(args, &vec![*first_iter]);
                assert_eq!(call_callee, callee);
                assert_eq!(src, dst);
                assert!(body.iter().any(|instruction| {
                    matches!(instruction, Instruction::Yield { src: Some(_), .. })
                }));
                assert!(body.iter().any(|instruction| {
                    matches!(instruction, Instruction::LoadLocal { name, .. } if name == ".0")
                }));
            }
            instructions => panic!("unexpected generator expression bytecode: {instructions:?}"),
        }
    }

    #[test]
    fn compiles_dict_comprehension_to_loop_and_set_item_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::DictComp {
                key: Box::new(Expr::Name("x".to_string())),
                value: Box::new(Expr::Binary {
                    left: Box::new(Expr::Name("x".to_string())),
                    op: BinaryOp::Multiply,
                    right: Box::new(Expr::Number(2)),
                }),
                clauses: vec![ComprehensionClause {
                    is_async: false,
                    target: Target::Name("x".to_string()),
                    iter: Expr::Name("items".to_string()),
                    ifs: vec![Expr::Comparison {
                        left: Box::new(Expr::Name("x".to_string())),
                        op: ComparisonOp::Greater,
                        right: Box::new(Expr::Number(1)),
                    }],
                }],
            })],
        };

        let instructions = compile(&program).unwrap();
        assert_comprehension_function_call(&instructions, "<dictcomp>", |body| {
            assert!(body.iter().any(|instruction| {
                matches!(instruction, Instruction::BuildDict { entries, .. } if entries.is_empty())
            }));
            assert!(
                body.iter()
                    .any(|instruction| matches!(instruction, Instruction::DictSetItem { .. }))
            );
        });
    }

    #[test]
    fn compiles_tuple_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Tuple(vec![
                Expr::Number(1),
                Expr::Binary {
                    left: Box::new(Expr::Number(2)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(3)),
                },
            ]))],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(3)
                },
                Instruction::Add {
                    dst: 3,
                    left: 1,
                    right: 2
                },
                Instruction::BuildTuple {
                    dst: 4,
                    items: vec![0, 3]
                },
                Instruction::Pop { src: 0 },
                Instruction::Pop { src: 3 },
                Instruction::Pop { src: 4 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_dict_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Dict(vec![
                DictItem::Entry {
                    key: Expr::String("a".to_string()),
                    value: Expr::Number(1),
                },
                DictItem::Entry {
                    key: Expr::String("b".to_string()),
                    value: Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(3)),
                    },
                },
            ]))],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::String("a".to_string())
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::String("b".to_string())
                },
                Instruction::LoadConst {
                    dst: 3,
                    value: Value::Number(2)
                },
                Instruction::LoadConst {
                    dst: 4,
                    value: Value::Number(3)
                },
                Instruction::Add {
                    dst: 5,
                    left: 3,
                    right: 4
                },
                Instruction::BuildDict {
                    dst: 6,
                    entries: vec![(0, 1), (2, 5)]
                },
                Instruction::Pop { src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Pop { src: 2 },
                Instruction::Pop { src: 5 },
                Instruction::Pop { src: 6 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_dict_unpack_expression_to_update_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Dict(vec![
                DictItem::Unpack(Expr::Name("base".to_string())),
                DictItem::Entry {
                    key: Expr::String("x".to_string()),
                    value: Expr::Number(1),
                },
                DictItem::Unpack(Expr::Name("override".to_string())),
            ]))],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::BuildDict {
                    dst: 0,
                    entries: vec![]
                },
                Instruction::LoadName {
                    dst: 1,
                    name: "base".to_string()
                },
                Instruction::DictUpdate { dict: 0, src: 1 },
                Instruction::Pop { src: 1 },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::String("x".to_string())
                },
                Instruction::LoadConst {
                    dst: 3,
                    value: Value::Number(1)
                },
                Instruction::DictSetItem {
                    dict: 0,
                    key: 2,
                    value: 3
                },
                Instruction::Pop { src: 2 },
                Instruction::Pop { src: 3 },
                Instruction::LoadName {
                    dst: 4,
                    name: "override".to_string()
                },
                Instruction::DictUpdate { dict: 0, src: 4 },
                Instruction::Pop { src: 4 },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_subscript_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Subscript {
                object: Box::new(Expr::Name("items".to_string())),
                index: Box::new(Expr::Number(0)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "items".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(0)
                },
                Instruction::LoadSubscript {
                    dst: 2,
                    object: 0,
                    index: 1
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_slice_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Slice {
                object: Box::new(Expr::Name("items".to_string())),
                start: Some(Box::new(Expr::Number(1))),
                stop: Some(Box::new(Expr::Number(3))),
                step: None,
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "items".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(3)
                },
                Instruction::LoadSlice {
                    dst: 3,
                    object: 0,
                    start: Some(1),
                    stop: Some(2),
                    step: None,
                },
                Instruction::Pop { src: 3 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_assignment_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Assign {
                targets: vec![Target::Name("x".to_string())],
                value: Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                },
                type_comment: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::Add {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 2
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_chained_assignment_to_single_value_register() {
        let program = Program {
            statements: vec![Stmt::Assign {
                targets: vec![Target::Name("a".to_string()), Target::Name("b".to_string())],
                value: Expr::Number(3),
                type_comment: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(3)
                },
                Instruction::StoreName {
                    name: "a".to_string(),
                    src: 0
                },
                Instruction::StoreName {
                    name: "b".to_string(),
                    src: 0
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_slice_literal_subscript_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Subscript {
                object: Box::new(Expr::Name("items".to_string())),
                index: Box::new(Expr::SliceLiteral {
                    start: Some(Box::new(Expr::Number(1))),
                    stop: Some(Box::new(Expr::Number(3))),
                    step: None,
                }),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "items".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(3)
                },
                Instruction::BuildSlice {
                    dst: 3,
                    start: Some(1),
                    stop: Some(2),
                    step: None,
                },
                Instruction::LoadSubscript {
                    dst: 4,
                    object: 0,
                    index: 3
                },
                Instruction::Pop { src: 4 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_named_expression_to_store_and_value_register() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::NamedExpr {
                name: "x".to_string(),
                value: Box::new(Expr::Number(3)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(3)
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 0
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_augmented_assignment_to_load_operate_store_bytecode() {
        let program = Program {
            statements: vec![Stmt::AugAssign {
                target: Target::Name("x".to_string()),
                op: BinaryOp::Add,
                value: Expr::Number(2),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "x".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::InPlaceAdd {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 2
                },
                Instruction::Pop { src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_multiply_augmented_assignment_to_in_place_bytecode() {
        let program = Program {
            statements: vec![Stmt::AugAssign {
                target: Target::Name("x".to_string()),
                op: BinaryOp::Multiply,
                value: Expr::Number(3),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "x".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(3)
                },
                Instruction::InPlaceMultiply {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 2
                },
                Instruction::Pop { src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_floor_divide_augmented_assignment_to_in_place_bytecode() {
        let program = Program {
            statements: vec![Stmt::AugAssign {
                target: Target::Name("x".to_string()),
                op: BinaryOp::FloorDivide,
                value: Expr::Number(3),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "x".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(3)
                },
                Instruction::InPlaceFloorDivide {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 2
                },
                Instruction::Pop { src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_tuple_unpack_assignment_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Assign {
                targets: vec![Target::Tuple(vec![
                    Target::Name("a".to_string()),
                    Target::Name("b".to_string()),
                ])],
                value: Expr::Tuple(vec![Expr::Number(1), Expr::Number(2)]),
                type_comment: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: tuple_value(vec![Value::Number(1), Value::Number(2)])
                },
                Instruction::UnpackSequence {
                    src: 0,
                    dst: vec![1, 2]
                },
                Instruction::StoreName {
                    name: "a".to_string(),
                    src: 1
                },
                Instruction::StoreName {
                    name: "b".to_string(),
                    src: 2
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_starred_unpack_assignment_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Assign {
                targets: vec![Target::Tuple(vec![
                    Target::Name("a".to_string()),
                    Target::Starred(Box::new(Target::Name("rest".to_string()))),
                    Target::Name("b".to_string()),
                ])],
                value: Expr::Tuple(vec![
                    Expr::Number(1),
                    Expr::Number(2),
                    Expr::Number(3),
                    Expr::Number(4),
                ]),
                type_comment: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: tuple_value(vec![
                        Value::Number(1),
                        Value::Number(2),
                        Value::Number(3),
                        Value::Number(4)
                    ])
                },
                Instruction::UnpackSequenceEx {
                    src: 0,
                    before: vec![1],
                    rest: 2,
                    after: vec![3],
                },
                Instruction::StoreName {
                    name: "a".to_string(),
                    src: 1
                },
                Instruction::StoreName {
                    name: "rest".to_string(),
                    src: 2
                },
                Instruction::StoreName {
                    name: "b".to_string(),
                    src: 3
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_string_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("print".to_string())),
                args: vec![Expr::String("hello".to_string())],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::String("hello".to_string())
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_joined_string_to_format_and_concat_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("print".to_string())),
                args: vec![Expr::JoinedString(vec![
                    FStringPart::Literal("hello ".to_string()),
                    FStringPart::Formatted {
                        value: Box::new(Expr::Name("name".to_string())),
                        conversion: Some(FStringConversion::Repr),
                        format_spec: None,
                    },
                ])],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::String(String::new())
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::String("hello ".to_string())
                },
                Instruction::Add {
                    dst: 1,
                    left: 1,
                    right: 2
                },
                Instruction::LoadName {
                    dst: 3,
                    name: "name".to_string()
                },
                Instruction::FormatValue {
                    dst: 4,
                    src: 3,
                    conversion: Some(FormatConversion::Repr),
                    format_spec: None
                },
                Instruction::Add {
                    dst: 1,
                    left: 1,
                    right: 4
                },
                Instruction::Call {
                    dst: 5,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Pop { src: 5 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_boolean_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Call {
                callee: Box::new(Expr::Name("print".to_string())),
                args: vec![Expr::Bool(true)],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Bool(true)
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_none_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::None)],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::None
                },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_equality_comparison_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Comparison {
                left: Box::new(Expr::Number(1)),
                op: ComparisonOp::Equal,
                right: Box::new(Expr::Number(2)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::Equal {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_ordering_comparison_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Comparison {
                left: Box::new(Expr::Number(1)),
                op: ComparisonOp::Less,
                right: Box::new(Expr::Number(2)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::Less {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_membership_comparison_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Comparison {
                left: Box::new(Expr::Number(1)),
                op: ComparisonOp::In,
                right: Box::new(Expr::List(vec![Expr::Number(1)])),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::BuildList {
                    dst: 2,
                    items: vec![1]
                },
                Instruction::Pop { src: 1 },
                Instruction::Contains {
                    dst: 3,
                    needle: 0,
                    haystack: 2
                },
                Instruction::Pop { src: 3 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_not_in_comparison_to_contains_then_not() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Comparison {
                left: Box::new(Expr::Number(1)),
                op: ComparisonOp::NotIn,
                right: Box::new(Expr::List(vec![Expr::Number(2)])),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(2)
                },
                Instruction::BuildList {
                    dst: 2,
                    items: vec![1]
                },
                Instruction::Pop { src: 1 },
                Instruction::Contains {
                    dst: 3,
                    needle: 0,
                    haystack: 2
                },
                Instruction::Not { dst: 3, src: 3 },
                Instruction::Pop { src: 3 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_identity_comparison_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Comparison {
                left: Box::new(Expr::Name("x".to_string())),
                op: ComparisonOp::Is,
                right: Box::new(Expr::None),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "x".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::None
                },
                Instruction::Is {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_match_singleton_pattern_to_identity_check() {
        let program = Program {
            statements: vec![Stmt::Match {
                subject: Expr::Name("x".to_string()),
                cases: vec![MatchCase {
                    pattern: Pattern::Singleton(Expr::Bool(true)),
                    guard: None,
                    body: vec![Stmt::Expr(Expr::String("hit".to_string()))],
                }],
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "x".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Bool(true)
                },
                Instruction::Is {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::JumpIfFalse {
                    condition: 2,
                    target: 7
                },
                Instruction::LoadConst {
                    dst: 3,
                    value: Value::String("hit".to_string())
                },
                Instruction::Pop { src: 3 },
                Instruction::Jump { target: 7 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_is_not_comparison_to_is_then_not() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Comparison {
                left: Box::new(Expr::Name("x".to_string())),
                op: ComparisonOp::IsNot,
                right: Box::new(Expr::None),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "x".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::None
                },
                Instruction::Is {
                    dst: 2,
                    left: 0,
                    right: 1
                },
                Instruction::Not { dst: 2, src: 2 },
                Instruction::Pop { src: 2 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_chained_comparison_to_short_circuit_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::ChainedComparison {
                left: Box::new(Expr::Number(1)),
                comparisons: vec![
                    (ComparisonOp::Less, Expr::Number(2)),
                    (ComparisonOp::Less, Expr::Number(3)),
                ],
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(1)
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Number(2)
                },
                Instruction::Less {
                    dst: 1,
                    left: 0,
                    right: 2
                },
                Instruction::JumpIfFalse {
                    condition: 1,
                    target: 6
                },
                Instruction::LoadConst {
                    dst: 3,
                    value: Value::Number(3)
                },
                Instruction::Less {
                    dst: 1,
                    left: 2,
                    right: 3
                },
                Instruction::Pop { src: 1 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_not_expression_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(Expr::Bool(true)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(true)
                },
                Instruction::Not { dst: 1, src: 0 },
                Instruction::Pop { src: 1 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_and_expression_to_short_circuit_bytecode() {
        let program = Program {
            statements: vec![Stmt::Expr(Expr::Logical {
                left: Box::new(Expr::Bool(true)),
                op: LogicalOp::And,
                right: Box::new(Expr::Bool(false)),
            })],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Bool(true)
                },
                Instruction::JumpIfFalse {
                    condition: 1,
                    target: 6
                },
                Instruction::Jump { target: 3 },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::Bool(false)
                },
                Instruction::Move { dst: 0, src: 2 },
                Instruction::Jump { target: 8 },
                Instruction::Move { dst: 0, src: 1 },
                Instruction::Jump { target: 8 },
                Instruction::Pop { src: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_if_statement_to_jump_bytecode() {
        let program = Program {
            statements: vec![Stmt::If {
                condition: Expr::Bool(true),
                then_body: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::String("yes".to_string())],
                })],
                else_body: Vec::new(),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(true)
                },
                Instruction::JumpIfFalse {
                    condition: 0,
                    target: 6
                },
                Instruction::LoadName {
                    dst: 1,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::String("yes".to_string())
                },
                Instruction::Call {
                    dst: 3,
                    callee: 1,
                    args: vec![2]
                },
                Instruction::Pop { src: 3 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_if_else_statement_to_jump_bytecode() {
        let program = Program {
            statements: vec![Stmt::If {
                condition: Expr::Bool(false),
                then_body: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::String("yes".to_string())],
                })],
                else_body: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::String("no".to_string())],
                })],
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(false)
                },
                Instruction::JumpIfFalse {
                    condition: 0,
                    target: 7
                },
                Instruction::LoadName {
                    dst: 1,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::String("yes".to_string())
                },
                Instruction::Call {
                    dst: 3,
                    callee: 1,
                    args: vec![2]
                },
                Instruction::Pop { src: 3 },
                Instruction::Jump { target: 11 },
                Instruction::LoadName {
                    dst: 4,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 5,
                    value: Value::String("no".to_string())
                },
                Instruction::Call {
                    dst: 6,
                    callee: 4,
                    args: vec![5]
                },
                Instruction::Pop { src: 6 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_while_statement_to_loop_bytecode() {
        let program = Program {
            statements: vec![Stmt::While {
                condition: Expr::Bool(true),
                body: vec![Stmt::Pass],
                else_body: Vec::new(),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(true)
                },
                Instruction::JumpIfFalse {
                    condition: 0,
                    target: 4
                },
                Instruction::Noop,
                Instruction::Jump { target: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_break_and_continue_to_loop_jumps() {
        let program = Program {
            statements: vec![Stmt::While {
                condition: Expr::Bool(true),
                body: vec![Stmt::Continue, Stmt::Break],
                else_body: Vec::new(),
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(true)
                },
                Instruction::JumpIfFalse {
                    condition: 0,
                    target: 5
                },
                Instruction::Jump { target: 0 },
                Instruction::Jump { target: 5 },
                Instruction::Jump { target: 0 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn rejects_break_and_continue_outside_loop() {
        assert_eq!(
            compile(&Program {
                statements: vec![Stmt::Break],
            }),
            Err("break outside loop".to_string())
        );
        assert_eq!(
            compile(&Program {
                statements: vec![Stmt::Continue],
            }),
            Err("'continue' not properly in loop".to_string())
        );
    }

    #[test]
    fn compiles_while_else_after_condition_false_target() {
        let program = Program {
            statements: vec![Stmt::While {
                condition: Expr::Bool(false),
                body: vec![Stmt::Pass],
                else_body: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::String("done".to_string())],
                })],
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Bool(false)
                },
                Instruction::JumpIfFalse {
                    condition: 0,
                    target: 4
                },
                Instruction::Noop,
                Instruction::Jump { target: 0 },
                Instruction::LoadName {
                    dst: 1,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 2,
                    value: Value::String("done".to_string())
                },
                Instruction::Call {
                    dst: 3,
                    callee: 1,
                    args: vec![2]
                },
                Instruction::Pop { src: 3 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_for_statement_to_iterator_bytecode() {
        let program = Program {
            statements: vec![Stmt::For {
                target: Target::Name("x".to_string()),
                iter: Expr::Call {
                    callee: Box::new(Expr::Name("range".to_string())),
                    args: vec![Expr::Number(3)],
                },
                body: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Name("x".to_string())],
                })],
                else_body: Vec::new(),
                type_comment: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "range".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(3)
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::GetIter { dst: 3, src: 2 },
                Instruction::ForIter {
                    iterator: 3,
                    dst: 4,
                    target: 11
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 4
                },
                Instruction::LoadName {
                    dst: 5,
                    name: "print".to_string()
                },
                Instruction::LoadName {
                    dst: 6,
                    name: "x".to_string()
                },
                Instruction::Call {
                    dst: 7,
                    callee: 5,
                    args: vec![6]
                },
                Instruction::Pop { src: 7 },
                Instruction::Jump { target: 4 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_for_else_after_iterator_exhaustion_target() {
        let program = Program {
            statements: vec![Stmt::For {
                target: Target::Name("x".to_string()),
                iter: Expr::Call {
                    callee: Box::new(Expr::Name("range".to_string())),
                    args: vec![Expr::Number(0)],
                },
                body: vec![Stmt::Pass],
                else_body: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::String("done".to_string())],
                })],
                type_comment: None,
            }],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "range".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(0)
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::GetIter { dst: 3, src: 2 },
                Instruction::ForIter {
                    iterator: 3,
                    dst: 4,
                    target: 8
                },
                Instruction::StoreName {
                    name: "x".to_string(),
                    src: 4
                },
                Instruction::Noop,
                Instruction::Jump { target: 4 },
                Instruction::LoadName {
                    dst: 5,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 6,
                    value: Value::String("done".to_string())
                },
                Instruction::Call {
                    dst: 7,
                    callee: 5,
                    args: vec![6]
                },
                Instruction::Pop { src: 7 },
                Instruction::Halt,
            ])
        );
    }

    #[test]
    fn compiles_pass_to_noop_bytecode() {
        let program = Program {
            statements: vec![Stmt::Pass],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![Instruction::Noop, Instruction::Halt])
        );
    }

    #[test]
    fn compiles_multiple_statements() {
        let program = Program {
            statements: vec![
                Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Number(1)],
                }),
                Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Number(2)],
                }),
            ],
        };

        assert_eq!(
            compile(&program),
            Ok(vec![
                Instruction::LoadName {
                    dst: 0,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 1,
                    value: Value::Number(1)
                },
                Instruction::Call {
                    dst: 2,
                    callee: 0,
                    args: vec![1]
                },
                Instruction::Pop { src: 2 },
                Instruction::LoadName {
                    dst: 3,
                    name: "print".to_string()
                },
                Instruction::LoadConst {
                    dst: 4,
                    value: Value::Number(2)
                },
                Instruction::Call {
                    dst: 5,
                    callee: 3,
                    args: vec![4]
                },
                Instruction::Pop { src: 5 },
                Instruction::Halt,
            ])
        );
    }
}
