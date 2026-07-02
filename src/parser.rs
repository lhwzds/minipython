use crate::ast::{
    BinaryOp, CallArg, CallKeyword, ComparisonOp, ComprehensionClause, DictItem, ExceptHandler,
    Expr, FStringConversion, FStringPart, FunctionParams, FunctionType, ImportAlias,
    ImportFromTargets, LogicalOp, MatchCase, Param, Pattern, Program, Stmt, Target,
    TemplateStringPart, TypeParam, TypeParamKind, UnaryOp, WithItem,
};
use crate::lexer::{Token, TokenFStringConversion, TokenFStringPart, lex};
use std::collections::HashSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParserDiagnostic {
    pub message: String,
    pub token_index: Option<usize>,
}

pub fn parse(tokens: &[Token]) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    let program = parser.parse_program()?;
    parser.expect_eof()?;
    Ok(program)
}

pub fn parse_with_diagnostic(tokens: &[Token]) -> Result<Program, ParserDiagnostic> {
    let mut parser = Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(program) => program,
        Err(message) => return Err(parser.error_diagnostic(message)),
    };
    match parser.expect_eof() {
        Ok(()) => Ok(program),
        Err(message) => Err(parser.error_diagnostic(message)),
    }
}

pub fn parse_eval(tokens: &[Token]) -> Result<Expr, String> {
    let mut parser = Parser::new(tokens);
    parser.skip_newlines();
    let expr = parser.parse_expression_list_until_statement_boundary()?;
    parser.skip_newlines();
    parser.expect_eof()?;
    Ok(expr)
}

pub fn parse_interactive(tokens: &[Token]) -> Result<Program, String> {
    let mut parser = Parser::new(tokens);
    let program = parser.parse_interactive_program()?;
    parser.expect_eof()?;
    Ok(program)
}

pub fn parse_func_type(tokens: &[Token]) -> Result<FunctionType, String> {
    let mut parser = Parser::new(tokens);
    let function_type = parser.parse_func_type_input()?;
    parser.skip_newlines();
    parser.expect_eof()?;
    Ok(function_type)
}

const MAX_UNARY_OPERATOR_DEPTH: usize = 10_000;

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
    comprehension_iter_depth: usize,
    class_body_depth: usize,
}

struct CallArguments {
    args: Vec<CallArg>,
    keywords: Vec<CallKeyword>,
    has_unpack: bool,
}

#[derive(Clone, Copy)]
enum ParameterListEnd {
    RightParen,
    Colon,
}

#[derive(Clone, Copy)]
enum ParameterTypeCommentTarget {
    Positional(usize),
    KeywordOnly(usize),
    Vararg,
    Kwarg,
}

#[derive(Clone, Copy)]
enum TypeExpressionMarker {
    Plain,
    Star,
    DoubleStar,
}

impl Parser<'_> {
    fn new(tokens: &[Token]) -> Parser<'_> {
        Parser {
            tokens,
            current: 0,
            comprehension_iter_depth: 0,
            class_body_depth: 0,
        }
    }

    fn error_diagnostic(&self, message: String) -> ParserDiagnostic {
        ParserDiagnostic {
            token_index: self.error_token_index(&message),
            message,
        }
    }

    fn error_token_index(&self, message: &str) -> Option<usize> {
        if self.tokens.is_empty() {
            return None;
        }

        if let Some((_, found)) = message.rsplit_once("found ") {
            if self
                .tokens
                .get(self.current)
                .is_some_and(|token| parser_token_matches_found(token, found))
            {
                return Some(self.current);
            }

            if self.current > 0
                && self
                    .tokens
                    .get(self.current - 1)
                    .is_some_and(|token| parser_token_matches_found(token, found))
            {
                return Some(self.current - 1);
            }
        }

        if self.current < self.tokens.len() {
            Some(self.current)
        } else {
            self.tokens.len().checked_sub(1)
        }
    }

    fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        self.skip_newlines();
        while !matches!(self.peek(), Some(Token::Eof) | None) {
            let statement = self.parse_statement()?;
            let has_own_boundary = statement_has_own_boundary(&statement);
            statements.push(statement);

            match self.peek() {
                Some(Token::Semicolon) => self.skip_statement_separators(),
                Some(Token::Newline) => self.skip_newlines(),
                Some(Token::TypeComment(_) | Token::TypeIgnore(_)) => {
                    self.skip_type_comment_tokens();
                    self.skip_newlines();
                }
                Some(Token::Eof) | None => {}
                Some(_) if has_own_boundary => {}
                Some(token) => {
                    let previous = statements.last().expect("statement was just pushed");
                    if let Some(error) = self.former_statement_boundary_error(previous) {
                        return Err(error);
                    }
                    return Err(format!(
                        "expected statement separator or end of input, found {token:?}"
                    ));
                }
            }
        }

        Ok(Program { statements })
    }

    fn parse_interactive_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        self.skip_newlines();
        if matches!(self.peek(), Some(Token::Eof) | None) {
            return Ok(Program {
                statements: vec![Stmt::Pass],
            });
        }

        let statement = self.parse_statement()?;
        let first_has_own_boundary = statement_has_own_boundary(&statement);
        statements.push(statement);

        if first_has_own_boundary && !self.interactive_compound_statement_has_terminator() {
            return Err("unexpected EOF while parsing".to_string());
        }

        if !first_has_own_boundary {
            while matches!(self.peek(), Some(Token::Semicolon)) {
                self.advance();
                if matches!(
                    self.peek(),
                    Some(Token::Newline)
                        | Some(Token::TypeComment(_) | Token::TypeIgnore(_))
                        | Some(Token::Eof)
                        | None
                ) {
                    break;
                }

                let statement = self.parse_statement()?;
                if statement_has_own_boundary(&statement) {
                    return Err("compound statement cannot follow ';'".to_string());
                }
                statements.push(statement);
            }
        }

        self.skip_newlines();

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        if matches!(self.peek(), Some(Token::Indent)) {
            return Err("unexpected indent".to_string());
        }

        if matches!(self.peek(), Some(Token::If)) {
            return self.parse_if_statement();
        }

        if matches!(self.peek(), Some(Token::While)) {
            return self.parse_while_statement();
        }

        if matches!(self.peek(), Some(Token::For)) {
            return self.parse_for_statement();
        }

        if matches!(self.peek(), Some(Token::Async)) {
            return self.parse_async_statement(Vec::new());
        }

        if matches!(self.peek(), Some(Token::Try)) {
            return self.parse_try_statement();
        }

        if matches!(self.peek(), Some(Token::With)) {
            return self.parse_with_statement();
        }

        if self.starts_match_statement() {
            return self.parse_match_statement();
        }

        if self.starts_lazy_import_statement() {
            return self.parse_lazy_import_statement();
        }

        if matches!(self.peek(), Some(Token::Import)) {
            return self.parse_import_statement(false);
        }

        if matches!(self.peek(), Some(Token::From)) {
            return self.parse_import_from_statement(false);
        }

        if matches!(self.peek(), Some(Token::At)) {
            return self.parse_decorated_statement();
        }

        if matches!(self.peek(), Some(Token::Def)) {
            return self.parse_function_def_statement(Vec::new());
        }

        if matches!(self.peek(), Some(Token::Class)) {
            return self.parse_class_def_statement(Vec::new());
        }

        if self.starts_type_alias_statement() {
            return self.parse_type_alias_statement();
        }

        if matches!(self.peek(), Some(Token::Return)) {
            return self.parse_return_statement();
        }

        if matches!(self.peek(), Some(Token::Yield)) {
            return self.parse_yield_statement();
        }

        if matches!(self.peek(), Some(Token::Raise)) {
            return self.parse_raise_statement();
        }

        if matches!(self.peek(), Some(Token::Del)) {
            return self.parse_delete_statement();
        }

        if matches!(self.peek(), Some(Token::Global)) {
            return self.parse_global_statement();
        }

        if matches!(self.peek(), Some(Token::Nonlocal)) {
            return self.parse_nonlocal_statement();
        }

        if matches!(self.peek(), Some(Token::Assert)) {
            return self.parse_assert_statement();
        }

        if matches!(self.peek(), Some(Token::Pass)) {
            self.advance();
            return Ok(Stmt::Pass);
        }

        if matches!(self.peek(), Some(Token::Break)) {
            self.advance();
            return Ok(Stmt::Break);
        }

        if matches!(self.peek(), Some(Token::Continue)) {
            self.advance();
            return Ok(Stmt::Continue);
        }

        let statement_start = self.current;
        let annotation_target_is_simple = matches!(
            (
                self.tokens.get(statement_start),
                self.tokens.get(statement_start + 1)
            ),
            (Some(Token::Identifier(_)), Some(Token::Colon))
        );
        match self.parse_assignment_target() {
            Ok(target) => {
                if matches!(self.peek(), Some(Token::Equal)) {
                    validate_store_target(&target)?;
                    let (targets, value) = self.parse_assignment_targets_and_value(target)?;
                    let type_comment = self.take_type_comment();
                    return Ok(Stmt::Assign {
                        targets,
                        value,
                        type_comment,
                    });
                }

                if matches!(self.peek(), Some(Token::Colon)) {
                    validate_store_target(&target)?;
                    validate_annotation_target(&target)?;
                    self.advance();
                    let annotation = self.parse_expression()?;
                    let value = if matches!(self.peek(), Some(Token::Equal)) {
                        self.advance();
                        Some(self.parse_expression_list_until_statement_boundary()?)
                    } else {
                        None
                    };
                    return Ok(Stmt::AnnAssign {
                        target,
                        annotation,
                        value,
                        simple: annotation_target_is_simple,
                    });
                }

                if let Some(op) = self.match_aug_assign_operator() {
                    validate_aug_assign_target(&target)?;
                    let value = self.parse_expression_list_until_statement_boundary()?;
                    return Ok(Stmt::AugAssign { target, op, value });
                }
            }
            Err(error) if is_assignment_target_syntax_error(&error) => return Err(error),
            Err(_) => {}
        }
        self.current = statement_start;

        let expr = self.parse_expression_list_until_statement_boundary()?;
        if matches!(self.peek(), Some(Token::Equal)) {
            return Err(invalid_expression_assignment_message(&expr));
        }
        if self.match_aug_assign_operator().is_some() {
            return Err(format!(
                "'{}' is an illegal expression for augmented assignment",
                invalid_named_expression_target_name(&expr)
            ));
        }
        if matches!(self.peek(), Some(Token::Colon)) {
            return Err(invalid_annotation_assignment_message(&expr));
        }
        Ok(Stmt::Expr(expr))
    }

    fn parse_assignment_targets_and_value(
        &mut self,
        first_target: Target,
    ) -> Result<(Vec<Target>, Expr), String> {
        let mut targets = vec![first_target];

        loop {
            self.expect_equal()?;
            let value_start = self.current;

            if let Ok(next_target) = self.parse_assignment_target() {
                if matches!(self.peek(), Some(Token::Equal)) {
                    validate_store_target(&next_target)?;
                    targets.push(next_target);
                    continue;
                }
            }

            self.current = value_start;
            let value = self.parse_expression_list_until_statement_boundary()?;
            if matches!(self.peek(), Some(Token::Equal)) {
                return Err(invalid_expression_assignment_message(&value));
            }
            return Ok((targets, value));
        }
    }

    fn parse_if_statement(&mut self) -> Result<Stmt, String> {
        self.expect_if()?;
        let condition = self.parse_named_expression()?;
        self.expect_colon()?;
        let then_body = self.parse_block_after_colon()?;
        let else_body = self.parse_if_tail()?;

        Ok(Stmt::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_elif_statement(&mut self) -> Result<Stmt, String> {
        self.expect_elif()?;
        let condition = self.parse_named_expression()?;
        self.expect_colon()?;
        let then_body = self.parse_block_after_colon()?;
        let else_body = self.parse_if_tail()?;

        Ok(Stmt::If {
            condition,
            then_body,
            else_body,
        })
    }

    fn parse_if_tail(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(self.peek(), Some(Token::Elif)) {
            return Ok(vec![self.parse_elif_statement()?]);
        }

        if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            self.expect_colon()?;
            return self.parse_block_after_colon();
        }

        Ok(Vec::new())
    }

    fn parse_while_statement(&mut self) -> Result<Stmt, String> {
        self.expect_while()?;
        let condition = self.parse_named_expression()?;
        self.expect_colon()?;
        let body = self.parse_block_after_colon()?;
        let else_body = if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            self.expect_colon()?;
            self.parse_block_after_colon()?
        } else {
            Vec::new()
        };

        Ok(Stmt::While {
            condition,
            body,
            else_body,
        })
    }

    fn parse_for_statement(&mut self) -> Result<Stmt, String> {
        self.expect_for()?;
        let target = self.parse_assignment_target()?;
        validate_store_target(&target)?;
        self.expect_in()?;
        let iter = self.parse_expression_list_until_colon()?;
        self.expect_colon()?;
        let type_comment = self.take_type_comment();
        let body = self.parse_block_after_colon()?;
        let else_body = if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            self.expect_colon()?;
            self.parse_block_after_colon()?
        } else {
            Vec::new()
        };

        Ok(Stmt::For {
            target,
            iter,
            body,
            else_body,
            type_comment,
        })
    }

    fn parse_try_statement(&mut self) -> Result<Stmt, String> {
        self.expect_try()?;
        self.expect_colon()?;
        let body = self.parse_block_after_colon()?;
        let mut handlers = Vec::new();
        let mut is_star_try: Option<bool> = None;

        while matches!(self.peek(), Some(Token::Except)) {
            let handler_start = self.current;
            let (handler, is_star_handler) = self.parse_except_handler()?;
            match is_star_try {
                Some(existing) if existing != is_star_handler => {
                    self.current = handler_start;
                    return Err(
                        "cannot have both 'except' and 'except*' on the same 'try'".to_string()
                    );
                }
                None => is_star_try = Some(is_star_handler),
                Some(_) => {}
            }
            handlers.push(handler);
        }

        if handlers
            .iter()
            .take(handlers.len().saturating_sub(1))
            .any(|handler| handler.type_expr.is_none())
        {
            return Err("default 'except:' must be last".to_string());
        }

        let else_body = if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            self.expect_colon()?;
            self.parse_block_after_colon()?
        } else {
            Vec::new()
        };

        let finally_body = if matches!(self.peek(), Some(Token::Finally)) {
            self.advance();
            self.expect_colon()?;
            self.parse_block_after_colon()?
        } else {
            Vec::new()
        };

        if handlers.is_empty() && finally_body.is_empty() {
            return Err("expected 'except' or 'finally' after try block".to_string());
        }

        if is_star_try == Some(true) {
            return Ok(Stmt::TryStar {
                body,
                handlers,
                else_body,
                finally_body,
            });
        }

        Ok(Stmt::Try {
            body,
            handlers,
            else_body,
            finally_body,
        })
    }

    fn parse_except_handler(&mut self) -> Result<(ExceptHandler, bool), String> {
        self.expect_except()?;
        let is_star_handler = if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            true
        } else {
            false
        };

        let (type_expr, name) = if !is_star_handler && matches!(self.peek(), Some(Token::Colon)) {
            (None, None)
        } else {
            let is_parenthesized_type = matches!(self.peek(), Some(Token::LeftParen));
            let type_expr = self.parse_except_type_expr()?;
            let name = if matches!(self.peek(), Some(Token::As)) {
                if !is_parenthesized_type && except_type_expr_arity(&type_expr) > 1 {
                    return Err(
                        "multiple exception types must be parenthesized when using 'as'"
                            .to_string(),
                    );
                }
                self.advance();
                let name_start = self.current;
                let name = self.expect_identifier("exception name")?;
                if matches!(self.peek(), Some(Token::Dot)) {
                    self.current = name_start;
                    return Err("cannot use except statement with attribute".to_string());
                }
                Some(name)
            } else {
                None
            };
            (Some(type_expr), name)
        };

        self.expect_colon()?;
        let body = self.parse_block_after_colon()?;

        Ok((
            ExceptHandler {
                type_expr,
                name,
                body,
            },
            is_star_handler,
        ))
    }

    fn parse_except_type_expr(&mut self) -> Result<Expr, String> {
        let expr = self.parse_expression_list_until(|token| {
            matches!(token, Some(Token::As) | Some(Token::Colon))
        })?;
        if matches!(&expr, Expr::Tuple(elements) if elements.is_empty()) {
            return Err("empty exception type tuple".to_string());
        }
        Ok(expr)
    }

    fn parse_with_statement(&mut self) -> Result<Stmt, String> {
        self.expect_with()?;
        let (items, body, type_comment) = self.parse_with_items_and_body()?;

        Ok(Stmt::With {
            items,
            body,
            type_comment,
        })
    }

    fn parse_with_items_and_body(
        &mut self,
    ) -> Result<(Vec<WithItem>, Vec<Stmt>, Option<String>), String> {
        let parenthesized = self.left_paren_starts_parenthesized_with_items();
        if parenthesized {
            self.advance();
        }

        let items = self.parse_with_items(parenthesized)?;

        if parenthesized {
            self.expect_right_paren()?;
        }

        self.expect_colon()?;
        let type_comment = self.take_type_comment();
        let body = self.parse_block_after_colon()?;

        Ok((items, body, type_comment))
    }

    fn left_paren_starts_parenthesized_with_items(&self) -> bool {
        if !matches!(self.peek(), Some(Token::LeftParen)) {
            return false;
        }

        let mut depth = 0usize;
        for index in self.current..self.tokens.len() {
            match &self.tokens[index] {
                Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return !matches!(
                            self.tokens.get(index + 1),
                            Some(Token::As | Token::Comma)
                        );
                    }
                }
                Token::RightBracket | Token::RightBrace => {
                    depth = depth.saturating_sub(1);
                }
                Token::Eof => break,
                _ => {}
            }
        }

        true
    }

    fn parse_with_items(&mut self, parenthesized: bool) -> Result<Vec<WithItem>, String> {
        let mut items = Vec::new();

        loop {
            if parenthesized && matches!(self.peek(), Some(Token::RightParen)) {
                if items.is_empty() {
                    return Err("expected with item".to_string());
                }
                break;
            }

            items.push(self.parse_with_item()?);

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if parenthesized && matches!(self.peek(), Some(Token::RightParen)) {
                break;
            }
            if !parenthesized && matches!(self.peek(), Some(Token::Colon)) {
                return Err("expected with item after ','".to_string());
            }
        }

        Ok(items)
    }

    fn parse_with_item(&mut self) -> Result<WithItem, String> {
        let context_expr = self.parse_expression()?;
        let optional_vars = if matches!(self.peek(), Some(Token::As)) {
            self.advance();
            let target = self.parse_single_target()?;
            validate_store_target(&target)?;
            Some(target)
        } else {
            None
        };

        Ok(WithItem {
            context_expr,
            optional_vars,
        })
    }

    fn parse_match_statement(&mut self) -> Result<Stmt, String> {
        self.expect_soft_keyword("match")?;
        let subject = self.parse_match_subject_expression()?;
        self.expect_colon()?;
        self.expect_newline()?;
        self.skip_newlines();
        self.expect_indent()?;

        let mut cases = Vec::new();
        self.skip_newlines();
        while self.starts_case_block() {
            cases.push(self.parse_match_case()?);
            self.skip_newlines();
        }

        if cases.is_empty() {
            return Err("expected at least one case block".to_string());
        }
        if cases
            .iter()
            .take(cases.len().saturating_sub(1))
            .any(is_irrefutable_match_case)
        {
            return Err("irrefutable match case must be last".to_string());
        }

        self.expect_dedent()?;
        Ok(Stmt::Match { subject, cases })
    }

    fn parse_match_subject_expression(&mut self) -> Result<Expr, String> {
        let first = self.parse_star_named_expression()?;

        if !matches!(self.peek(), Some(Token::Comma)) {
            if matches!(first, Expr::Starred(_)) {
                return Err("cannot use starred expression here".to_string());
            }
            return Ok(first);
        }

        let mut elements = vec![first];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::Colon)) {
                break;
            }
            elements.push(self.parse_star_named_expression()?);
        }

        Ok(Expr::Tuple(elements))
    }

    fn parse_match_case(&mut self) -> Result<MatchCase, String> {
        self.expect_soft_keyword("case")?;
        let pattern = self.parse_match_patterns()?;
        ensure_unique_pattern_captures(&pattern)?;
        let guard = if matches!(self.peek(), Some(Token::If)) {
            self.advance();
            Some(self.parse_named_expression()?)
        } else {
            None
        };
        self.expect_colon()?;
        let body = self.parse_block_after_colon()?;

        Ok(MatchCase {
            pattern,
            guard,
            body,
        })
    }

    fn parse_match_patterns(&mut self) -> Result<Pattern, String> {
        let first = self.parse_sequence_match_pattern()?;
        if !matches!(self.peek(), Some(Token::Comma)) {
            if matches!(first, Pattern::Star(_)) {
                return Err("invalid syntax".to_string());
            }
            return Ok(first);
        }

        let mut patterns = vec![first];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::Colon) | Some(Token::If)) {
                break;
            }

            patterns.push(self.parse_sequence_match_pattern()?);
        }
        ensure_at_most_one_star_pattern(&patterns)?;
        Ok(Pattern::Sequence(patterns))
    }

    fn parse_match_pattern(&mut self) -> Result<Pattern, String> {
        let pattern = self.parse_or_match_pattern()?;
        if !matches!(self.peek(), Some(Token::As)) {
            return Ok(pattern);
        }

        self.advance();
        let name = self.parse_as_pattern_capture_target()?;
        Ok(Pattern::As {
            pattern: Box::new(pattern),
            name,
        })
    }

    fn parse_or_match_pattern(&mut self) -> Result<Pattern, String> {
        let first = self.parse_closed_match_pattern()?;
        if !matches!(self.peek(), Some(Token::Pipe)) {
            return Ok(first);
        }

        let mut alternatives = vec![first];
        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            alternatives.push(self.parse_closed_match_pattern()?);
        }

        if let Some(message) = alternatives
            .iter()
            .take(alternatives.len().saturating_sub(1))
            .find_map(irrefutable_or_pattern_unreachable_message)
        {
            return Err(message);
        }
        ensure_or_pattern_capture_compatibility(&alternatives)?;

        Ok(Pattern::Or(alternatives))
    }

    fn parse_closed_match_pattern(&mut self) -> Result<Pattern, String> {
        if matches!(self.peek(), Some(Token::LeftBracket)) {
            self.advance();
            let patterns = self.parse_sequence_match_patterns(Token::RightBracket)?;
            ensure_at_most_one_star_pattern(&patterns)?;
            let pattern = Pattern::Sequence(patterns);
            self.expect_right_bracket()?;
            return Ok(pattern);
        }

        if matches!(self.peek(), Some(Token::LeftBrace)) {
            self.advance();
            let pattern = self.parse_mapping_match_pattern()?;
            self.expect_right_brace()?;
            return Ok(pattern);
        }

        if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            if matches!(self.peek(), Some(Token::RightParen)) {
                self.advance();
                return Ok(Pattern::Sequence(Vec::new()));
            }

            let first = self.parse_sequence_match_pattern()?;
            if matches!(self.peek(), Some(Token::Comma)) {
                let mut patterns = vec![first];
                self.parse_sequence_match_pattern_tail(Token::RightParen, &mut patterns)?;
                ensure_at_most_one_star_pattern(&patterns)?;
                self.expect_right_paren()?;
                return Ok(Pattern::Sequence(patterns));
            }

            if matches!(first, Pattern::Star(_)) {
                return Err("invalid syntax".to_string());
            }

            self.expect_right_paren()?;
            return Ok(first);
        }

        if matches!(self.peek(), Some(Token::Identifier(name)) if name == "_") {
            self.advance();
            return Ok(Pattern::Wildcard);
        }

        if let Some(pattern) = self.try_parse_class_match_pattern()? {
            return Ok(pattern);
        }

        if matches!(
            (self.peek(), self.peek_next()),
            (Some(Token::Identifier(_)), Some(Token::Dot))
        ) {
            return Ok(Pattern::Value(self.parse_value_pattern_expr()?));
        }

        if let Some(Token::Identifier(name)) = self.peek() {
            if !matches!(
                self.peek_next(),
                Some(Token::Dot | Token::LeftParen | Token::Equal)
            ) {
                let name = name.clone();
                self.advance();
                return Ok(Pattern::Capture(name));
            }
        }

        if !is_literal_pattern_start(self.peek()) {
            return Err("unsupported match pattern".to_string());
        }

        let literal = self.parse_literal_pattern_expr()?;
        if !is_supported_literal_pattern(&literal) {
            return Err("patterns may only match literals and attribute lookups".to_string());
        }

        if is_singleton_literal_pattern(&literal) {
            return Ok(Pattern::Singleton(literal));
        }

        Ok(Pattern::Literal(literal))
    }

    fn parse_mapping_match_pattern(&mut self) -> Result<Pattern, String> {
        let mut entries = Vec::new();
        let mut rest = None;

        if matches!(self.peek(), Some(Token::RightBrace)) {
            return Ok(Pattern::Mapping { entries, rest });
        }

        loop {
            if matches!(self.peek(), Some(Token::DoubleStar)) {
                self.advance();
                rest = Some(self.parse_pattern_capture_target()?);
                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                }
                if !matches!(self.peek(), Some(Token::RightBrace)) {
                    return Err("invalid syntax".to_string());
                }
                break;
            }

            let key = self.parse_mapping_pattern_key()?;
            self.expect_colon()?;
            let pattern = self.parse_match_pattern()?;
            entries.push((key, pattern));

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if matches!(self.peek(), Some(Token::RightBrace)) {
                break;
            }
        }

        ensure_unique_mapping_literal_keys(&entries)?;
        Ok(Pattern::Mapping { entries, rest })
    }

    fn parse_mapping_pattern_key(&mut self) -> Result<Expr, String> {
        if matches!(
            (self.peek(), self.peek_next()),
            (Some(Token::Identifier(_)), Some(Token::Dot))
        ) {
            return self.parse_value_pattern_expr();
        }

        if !is_literal_pattern_start(self.peek()) {
            return Err("invalid syntax".to_string());
        }

        let key = self.parse_literal_pattern_expr()?;
        if !is_supported_literal_pattern(&key) {
            return Err(
                "mapping pattern keys may only match literals and attribute lookups".to_string(),
            );
        }

        Ok(key)
    }

    fn try_parse_class_match_pattern(&mut self) -> Result<Option<Pattern>, String> {
        if !matches!(self.peek(), Some(Token::Identifier(_))) {
            return Ok(None);
        }

        let start = self.current;
        let class = self.parse_name_or_attr_pattern_expr()?;
        if !matches!(self.peek(), Some(Token::LeftParen)) {
            self.current = start;
            return Ok(None);
        }

        self.advance();
        let (positional, keywords) = self.parse_class_match_pattern_arguments()?;
        self.expect_right_paren()?;

        Ok(Some(Pattern::Class {
            class,
            positional,
            keywords,
        }))
    }

    fn parse_class_match_pattern_arguments(
        &mut self,
    ) -> Result<(Vec<Pattern>, Vec<(String, Pattern)>), String> {
        let mut positional = Vec::new();
        let mut keywords = Vec::new();
        let mut seen_keyword = false;

        if matches!(self.peek(), Some(Token::RightParen)) {
            return Ok((positional, keywords));
        }

        loop {
            if matches!(self.peek(), Some(Token::Star | Token::DoubleStar)) {
                return Err("invalid syntax".to_string());
            }

            if matches!(
                (self.peek(), self.peek_next()),
                (Some(Token::Identifier(_)), Some(Token::Equal))
            ) {
                seen_keyword = true;
                let Some(Token::Identifier(name)) = self.advance().cloned() else {
                    unreachable!("class keyword pattern starts with an identifier");
                };
                self.expect_equal()?;
                if keywords.iter().any(|(existing, _)| existing == &name) {
                    return Err(format!("attribute name repeated in class pattern: {name}"));
                }
                keywords.push((name, self.parse_match_pattern()?));
            } else {
                if seen_keyword {
                    if !self.remaining_contains_right_paren() {
                        return Err(unclosed_delimiter_message('('));
                    }
                    return Err("positional patterns follow keyword patterns".to_string());
                }
                positional.push(self.parse_match_pattern()?);
            }

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if matches!(self.peek(), Some(Token::RightParen)) {
                break;
            }
        }

        Ok((positional, keywords))
    }

    fn parse_value_pattern_expr(&mut self) -> Result<Expr, String> {
        let expr = self.parse_name_or_attr_pattern_expr()?;

        if matches!(self.peek(), Some(Token::LeftParen | Token::Equal)) {
            return Err("invalid syntax".to_string());
        }

        Ok(expr)
    }

    fn parse_name_or_attr_pattern_expr(&mut self) -> Result<Expr, String> {
        let Some(Token::Identifier(name)) = self.advance().cloned() else {
            return Err("unsupported match pattern".to_string());
        };
        let mut expr = Expr::Name(name);

        while matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            let Some(Token::Identifier(name)) = self.advance().cloned() else {
                return Err("invalid syntax".to_string());
            };
            expr = Expr::Attribute {
                object: Box::new(expr),
                name,
            };
        }

        Ok(expr)
    }

    fn parse_pattern_capture_target(&mut self) -> Result<String, String> {
        let start = self.current;
        match self.advance().cloned() {
            Some(Token::Identifier(_)) if matches!(self.peek(), Some(Token::Dot)) => {
                self.current = start;
                Err("cannot use attribute as pattern target".to_string())
            }
            Some(Token::Identifier(_)) if matches!(self.peek(), Some(Token::LeftParen)) => {
                Err("cannot use function call as pattern target".to_string())
            }
            Some(token) if is_literal_pattern_target_token(&token) => {
                Err("cannot use literal as pattern target".to_string())
            }
            Some(Token::None) => Err("cannot use None as pattern target".to_string()),
            Some(Token::True) => Err("cannot use True as pattern target".to_string()),
            Some(Token::False) => Err("cannot use False as pattern target".to_string()),
            Some(Token::Ellipsis) => Err("cannot use ellipsis as pattern target".to_string()),
            Some(Token::FString(_)) => {
                Err("cannot use f-string expression as pattern target".to_string())
            }
            Some(Token::TString(_)) => {
                Err("cannot use t-string expression as pattern target".to_string())
            }
            Some(Token::Minus) => Err("cannot use expression as pattern target".to_string()),
            Some(Token::Identifier(name)) if name == "_" => {
                Err("cannot use '_' as a target".to_string())
            }
            Some(Token::Identifier(name))
                if !matches!(
                    self.peek(),
                    Some(Token::Dot | Token::LeftParen | Token::Equal)
                ) =>
            {
                Ok(name.clone())
            }
            _ => Err("unsupported match pattern".to_string()),
        }
    }

    fn parse_as_pattern_capture_target(&mut self) -> Result<String, String> {
        if matches!(self.peek(), Some(Token::Plus)) {
            self.advance();
            return Err("cannot use expression as pattern target".to_string());
        }

        self.parse_pattern_capture_target()
    }

    fn parse_star_pattern_capture_target(&mut self) -> Result<String, String> {
        if matches!(self.peek(), Some(Token::Plus)) {
            self.advance();
            return Err("invalid syntax".to_string());
        }

        self.parse_pattern_capture_target()
    }

    fn parse_sequence_match_patterns(&mut self, end: Token) -> Result<Vec<Pattern>, String> {
        if token_matches(self.peek(), &end) {
            return Ok(Vec::new());
        }

        let first = self.parse_sequence_match_pattern()?;
        let mut patterns = vec![first];
        self.parse_sequence_match_pattern_tail(end, &mut patterns)?;
        ensure_at_most_one_star_pattern(&patterns)?;
        Ok(patterns)
    }

    fn parse_sequence_match_pattern_tail(
        &mut self,
        end: Token,
        patterns: &mut Vec<Pattern>,
    ) -> Result<(), String> {
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if token_matches(self.peek(), &end) {
                break;
            }

            patterns.push(self.parse_sequence_match_pattern()?);
        }

        Ok(())
    }

    fn parse_sequence_match_pattern(&mut self) -> Result<Pattern, String> {
        if !matches!(self.peek(), Some(Token::Star)) {
            return self.parse_match_pattern();
        }

        self.advance();
        if matches!(self.peek(), Some(Token::Identifier(name)) if name == "_") {
            self.advance();
            Ok(Pattern::Star(None))
        } else {
            self.parse_star_pattern_capture_target()
                .map(|name| Pattern::Star(Some(name)))
        }
    }

    fn parse_literal_pattern_expr(&mut self) -> Result<Expr, String> {
        let expr = self.parse_signed_literal_pattern_expr()?;
        if !is_signed_real_literal_pattern_expr(&expr) {
            return Ok(expr);
        }

        let op = match self.peek() {
            Some(Token::Plus) => BinaryOp::Add,
            Some(Token::Minus) => BinaryOp::Subtract,
            _ => return Ok(expr),
        };
        let operator_index = self.current;
        self.advance();

        let Some(Token::Imaginary(imaginary)) = self.advance().cloned() else {
            self.current = operator_index;
            return Ok(expr);
        };

        Ok(Expr::Binary {
            left: Box::new(expr),
            op,
            right: Box::new(Expr::Imaginary(imaginary)),
        })
    }

    fn parse_signed_literal_pattern_expr(&mut self) -> Result<Expr, String> {
        match self.advance().cloned() {
            Some(Token::Number(value)) => Ok(Expr::Number(value)),
            Some(Token::BigInt(value)) => Ok(Expr::BigInt(value)),
            Some(Token::Float(value)) => Ok(Expr::Float(value)),
            Some(Token::Imaginary(value)) => Ok(Expr::Imaginary(value)),
            Some(Token::String(value)) => {
                let (parts, has_f_string) =
                    self.parse_adjacent_string_parts(vec![FStringPart::Literal(value)], false)?;
                Ok(if has_f_string {
                    joined_string_parts_to_joined_expr(parts)
                } else {
                    joined_string_parts_to_expr(parts)
                })
            }
            Some(Token::Bytes(value)) => {
                let value = self.parse_adjacent_bytes(value)?;
                Ok(Expr::Bytes(value))
            }
            Some(Token::FString(parts)) => {
                let parts = self.parse_f_string_token_parts(&parts)?;
                let (parts, _) = self.parse_adjacent_string_parts(parts, true)?;
                Ok(joined_string_parts_to_joined_expr(parts))
            }
            Some(Token::TString(parts)) => {
                let parts = self.parse_t_string_token_parts(&parts)?;
                let parts = self.parse_adjacent_t_string_parts(parts)?;
                Ok(Expr::TemplateString(parts))
            }
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::None) => Ok(Expr::None),
            Some(Token::Minus) => match self.advance() {
                Some(Token::Number(value)) => Ok(Expr::Unary {
                    op: UnaryOp::Negative,
                    operand: Box::new(Expr::Number(*value)),
                }),
                Some(Token::BigInt(value)) => Ok(Expr::Unary {
                    op: UnaryOp::Negative,
                    operand: Box::new(Expr::BigInt(value.clone())),
                }),
                Some(Token::Float(value)) => Ok(Expr::Unary {
                    op: UnaryOp::Negative,
                    operand: Box::new(Expr::Float(value.clone())),
                }),
                Some(Token::Imaginary(value)) => Ok(Expr::Unary {
                    op: UnaryOp::Negative,
                    operand: Box::new(Expr::Imaginary(value.clone())),
                }),
                Some(token) => Err(format!(
                    "expected number after '-' in match pattern, found {token:?}"
                )),
                None => Err(
                    "expected number after '-' in match pattern, found end of input".to_string(),
                ),
            },
            Some(token) => Err(format!("unsupported match pattern token: {token:?}")),
            None => Err("expected match pattern, found end of input".to_string()),
        }
    }

    fn starts_lazy_import_statement(&self) -> bool {
        matches!(
            (self.peek(), self.peek_next()),
            (Some(Token::Identifier(name)), Some(Token::Import | Token::From)) if name == "lazy"
        )
    }

    fn parse_lazy_import_statement(&mut self) -> Result<Stmt, String> {
        self.expect_soft_keyword("lazy")?;
        match self.peek() {
            Some(Token::Import) => self.parse_import_statement(true),
            Some(Token::From) => self.parse_import_from_statement(true),
            Some(token) => Err(format!("expected import after lazy, found {token:?}")),
            None => Err("expected import after lazy, found end of input".to_string()),
        }
    }

    fn parse_import_statement(&mut self, is_lazy: bool) -> Result<Stmt, String> {
        self.expect_import()?;
        if is_statement_boundary(self.peek()) {
            return Err("Expected one or more names after 'import'".to_string());
        }

        let aliases = self.parse_dotted_as_names()?;
        if matches!(self.peek(), Some(Token::From)) {
            return Err("Did you mean to use 'from ... import ...' instead?".to_string());
        }

        Ok(Stmt::Import { is_lazy, aliases })
    }

    fn parse_import_from_statement(&mut self, is_lazy: bool) -> Result<Stmt, String> {
        self.expect_from()?;

        let level = self.parse_relative_import_level();

        let module = if matches!(self.peek(), Some(Token::Identifier(_))) {
            Some(self.parse_dotted_name()?)
        } else {
            None
        };

        self.expect_import()?;
        let targets = self.parse_import_from_targets()?;

        Ok(Stmt::ImportFrom {
            is_lazy,
            module,
            level,
            targets,
        })
    }

    fn parse_relative_import_level(&mut self) -> usize {
        let mut level = 0;
        loop {
            match self.peek() {
                Some(Token::Dot) => {
                    self.advance();
                    level += 1;
                }
                Some(Token::Ellipsis) => {
                    self.advance();
                    level += 3;
                }
                _ => break,
            }
        }

        level
    }

    fn parse_dotted_as_names(&mut self) -> Result<Vec<ImportAlias>, String> {
        let mut aliases = Vec::new();

        loop {
            aliases.push(self.parse_dotted_as_name()?);

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
        }

        Ok(aliases)
    }

    fn parse_dotted_as_name(&mut self) -> Result<ImportAlias, String> {
        let name = self.parse_dotted_name()?;
        let asname = if matches!(self.peek(), Some(Token::As)) {
            self.advance();
            Some(self.parse_import_alias_target(false)?)
        } else {
            None
        };

        let alias = ImportAlias { name, asname };
        validate_import_alias_binding(&alias, false)?;
        Ok(alias)
    }

    fn parse_dotted_name(&mut self) -> Result<String, String> {
        let mut name = self.expect_identifier("module name")?;

        while matches!(self.peek(), Some(Token::Dot)) {
            self.advance();
            let part = self.expect_identifier("module name")?;
            name.push('.');
            name.push_str(&part);
        }

        Ok(name)
    }

    fn parse_import_from_targets(&mut self) -> Result<ImportFromTargets, String> {
        if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            return Ok(ImportFromTargets::Star);
        }

        if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            let aliases = self.parse_import_from_as_names(Token::RightParen, true)?;
            self.expect_right_paren()?;
            return Ok(ImportFromTargets::Aliases(aliases));
        }

        if self.import_from_list_at_end(&Token::Newline) {
            return Err("Expected one or more names after 'import'".to_string());
        }

        let aliases = self.parse_import_from_as_names(Token::Newline, false)?;
        Ok(ImportFromTargets::Aliases(aliases))
    }

    fn parse_import_from_as_names(
        &mut self,
        end: Token,
        allow_trailing_comma: bool,
    ) -> Result<Vec<ImportAlias>, String> {
        let mut aliases = Vec::new();

        loop {
            aliases.push(self.parse_import_from_as_name()?);

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if self.import_from_list_at_end(&end) {
                if allow_trailing_comma {
                    break;
                }
                return Err(
                    "trailing comma not allowed without surrounding parentheses".to_string()
                );
            }
        }

        Ok(aliases)
    }

    fn parse_import_from_as_name(&mut self) -> Result<ImportAlias, String> {
        let name = self.expect_identifier("import name")?;
        let asname = if matches!(self.peek(), Some(Token::As)) {
            self.advance();
            Some(self.parse_import_alias_target(true)?)
        } else {
            None
        };

        let alias = ImportAlias { name, asname };
        validate_import_alias_binding(&alias, true)?;
        Ok(alias)
    }

    fn parse_import_alias_target(&mut self, allow_right_paren: bool) -> Result<String, String> {
        let target_start = self.current;

        if matches!(self.peek(), Some(Token::Identifier(_)))
            && is_import_alias_boundary(self.peek_next(), allow_right_paren)
        {
            return self.expect_import_alias_identifier();
        }

        if is_import_alias_boundary(self.peek(), allow_right_paren) {
            return self.expect_import_alias_identifier();
        }

        match self.parse_expression() {
            Ok(expr) => Err(format!(
                "cannot use {} as import target",
                invalid_import_expression_target_name(&expr)
            )),
            Err(_) => {
                self.current = target_start;
                self.expect_import_alias_identifier()
            }
        }
    }

    fn expect_import_alias_identifier(&mut self) -> Result<String, String> {
        match self.advance().cloned() {
            Some(Token::Identifier(name)) => Ok(name),
            Some(token) => {
                if let Some(name) = invalid_import_target_name(&token) {
                    Err(format!("cannot use {name} as import target"))
                } else {
                    Err(format!("expected import alias, found {token:?}"))
                }
            }
            None => Err("expected import alias, found end of input".to_string()),
        }
    }

    fn import_from_list_at_end(&self, end: &Token) -> bool {
        match end {
            Token::RightParen => matches!(self.peek(), Some(Token::RightParen)),
            Token::Newline => matches!(
                self.peek(),
                Some(Token::Semicolon)
                    | Some(Token::TypeComment(_) | Token::TypeIgnore(_))
                    | Some(Token::Newline)
                    | Some(Token::Dedent)
                    | Some(Token::Eof)
                    | None
            ),
            _ => false,
        }
    }

    fn parse_decorated_statement(&mut self) -> Result<Stmt, String> {
        let decorators = self.parse_decorators()?;
        self.skip_newlines();

        match self.peek() {
            Some(Token::Def) => self.parse_function_def_statement(decorators),
            Some(Token::Async) => self.parse_async_statement(decorators),
            Some(Token::Class) => self.parse_class_def_statement(decorators),
            Some(token) => Err(format!(
                "expected function or class definition after decorator, found {token:?}"
            )),
            None => Err(
                "expected function or class definition after decorator, found end of input"
                    .to_string(),
            ),
        }
    }

    fn parse_decorators(&mut self) -> Result<Vec<Expr>, String> {
        let mut decorators = Vec::new();

        while matches!(self.peek(), Some(Token::At)) {
            self.advance();
            decorators.push(self.parse_named_expression()?);
            self.expect_newline()?;
        }

        Ok(decorators)
    }

    fn parse_function_def_statement(&mut self, decorators: Vec<Expr>) -> Result<Stmt, String> {
        self.expect_def()?;
        let name = self.expect_identifier("function name")?;
        validate_binding_name(&name)?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_left_paren()?;
        let params = self.parse_parameter_list(ParameterListEnd::RightParen)?;
        self.expect_right_paren()?;
        let returns = if matches!(self.peek(), Some(Token::Arrow)) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        if !type_params.is_empty() {
            validate_generic_function_annotations(&params, returns.as_ref())?;
        }
        self.expect_colon()?;
        let (type_comment, body) = self.parse_function_body_after_colon()?;

        Ok(Stmt::FunctionDef {
            name,
            type_params,
            params,
            body,
            decorators,
            returns,
            type_comment,
        })
    }

    fn parse_async_statement(&mut self, decorators: Vec<Expr>) -> Result<Stmt, String> {
        self.expect_async()?;

        match self.peek() {
            Some(Token::Def) => self.parse_async_function_def_statement(decorators),
            Some(Token::For) if decorators.is_empty() => self.parse_async_for_statement(),
            Some(Token::With) if decorators.is_empty() => self.parse_async_with_statement(),
            Some(token) => Err(format!(
                "expected 'def', 'for', or 'with' after 'async', found {token:?}"
            )),
            None => Err(
                "expected 'def', 'for', or 'with' after 'async', found end of input".to_string(),
            ),
        }
    }

    fn parse_async_for_statement(&mut self) -> Result<Stmt, String> {
        self.expect_for()?;
        let target = self.parse_assignment_target()?;
        validate_store_target(&target)?;
        self.expect_in()?;
        let iter = self.parse_expression_list_until_colon()?;
        self.expect_colon()?;
        let type_comment = self.take_type_comment();
        let body = self.parse_block_after_colon()?;
        let else_body = if matches!(self.peek(), Some(Token::Else)) {
            self.advance();
            self.expect_colon()?;
            self.parse_block_after_colon()?
        } else {
            Vec::new()
        };

        Ok(Stmt::AsyncFor {
            target,
            iter,
            body,
            else_body,
            type_comment,
        })
    }

    fn parse_async_with_statement(&mut self) -> Result<Stmt, String> {
        self.expect_with()?;
        let (items, body, type_comment) = self.parse_with_items_and_body()?;

        Ok(Stmt::AsyncWith {
            items,
            body,
            type_comment,
        })
    }

    fn parse_class_def_statement(&mut self, decorators: Vec<Expr>) -> Result<Stmt, String> {
        self.expect_class()?;
        let name = self.expect_identifier("class name")?;
        validate_binding_name(&name)?;
        let type_params = self.parse_optional_type_params()?;
        let mut bases = Vec::new();
        let mut keywords = Vec::new();
        if matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            let arguments = self.parse_arguments()?;
            if !type_params.is_empty() {
                validate_generic_definition_arguments(&arguments)?;
            }
            bases = arguments.args;
            keywords = arguments.keywords;
            self.expect_right_paren()?;
        }
        self.expect_colon()?;
        let body = self.parse_class_body_after_colon()?;

        Ok(Stmt::ClassDef {
            name,
            type_params,
            bases,
            keywords,
            body,
            decorators,
        })
    }

    fn parse_async_function_def_statement(
        &mut self,
        decorators: Vec<Expr>,
    ) -> Result<Stmt, String> {
        self.expect_def()?;
        let name = self.expect_identifier("function name")?;
        validate_binding_name(&name)?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_left_paren()?;
        let params = self.parse_parameter_list(ParameterListEnd::RightParen)?;
        self.expect_right_paren()?;
        let returns = if matches!(self.peek(), Some(Token::Arrow)) {
            self.advance();
            Some(self.parse_expression()?)
        } else {
            None
        };
        if !type_params.is_empty() {
            validate_generic_function_annotations(&params, returns.as_ref())?;
        }
        self.expect_colon()?;
        let (type_comment, body) = self.parse_function_body_after_colon()?;

        Ok(Stmt::AsyncFunctionDef {
            name,
            type_params,
            params,
            body,
            decorators,
            returns,
            type_comment,
        })
    }

    fn parse_type_alias_statement(&mut self) -> Result<Stmt, String> {
        self.expect_soft_keyword("type")?;
        let name = self.expect_identifier("type alias name")?;
        validate_binding_name(&name)?;
        let type_params = self.parse_optional_type_params()?;
        self.expect_equal()?;
        let value = self.parse_expression_list_until_statement_boundary()?;
        validate_type_scope_expression(&value, "a type alias")?;

        Ok(Stmt::TypeAlias {
            name,
            type_params,
            value,
        })
    }

    fn parse_optional_type_params(&mut self) -> Result<Vec<TypeParam>, String> {
        if !matches!(self.peek(), Some(Token::LeftBracket)) {
            return Ok(Vec::new());
        }

        self.expect_left_bracket()?;
        if matches!(self.peek(), Some(Token::RightBracket)) {
            return Err("Type parameter list cannot be empty".to_string());
        }

        let mut type_params = Vec::new();
        let mut seen_names = Vec::new();
        let mut saw_default = false;

        loop {
            let type_param = self.parse_type_param(&mut seen_names)?;
            if saw_default && type_param.default.is_none() {
                return Err(format!(
                    "non-default type parameter '{}' follows default type parameter",
                    type_param.name
                ));
            }
            saw_default |= type_param.default.is_some();
            type_params.push(type_param);

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if matches!(self.peek(), Some(Token::RightBracket)) {
                break;
            }
        }

        self.expect_right_bracket()?;
        Ok(type_params)
    }

    fn parse_type_param(&mut self, seen_names: &mut Vec<String>) -> Result<TypeParam, String> {
        let kind = match self.peek() {
            Some(Token::Star) => {
                self.advance();
                TypeParamKind::TypeVarTuple
            }
            Some(Token::DoubleStar) => {
                self.advance();
                TypeParamKind::ParamSpec
            }
            _ => TypeParamKind::TypeVar,
        };

        let name = self.expect_identifier("type parameter name")?;
        validate_type_parameter_name(&name)?;
        ensure_unique_type_parameter_name(&name, seen_names)?;

        let bound = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            let bound = self.parse_expression()?;
            let context = if matches!(bound, Expr::Tuple(_)) {
                "a TypeVar constraint"
            } else {
                "a TypeVar bound"
            };
            validate_type_scope_expression(&bound, context)?;
            if !matches!(kind, TypeParamKind::TypeVar) {
                return Err(invalid_type_param_bound_message(&kind, &bound).to_string());
            }
            Some(bound)
        } else {
            None
        };

        let default = if matches!(self.peek(), Some(Token::Equal)) {
            self.advance();
            let is_starred_typevartuple_default = matches!(kind, TypeParamKind::TypeVarTuple)
                && matches!(self.peek(), Some(Token::Star));
            if is_starred_typevartuple_default {
                self.advance();
            }
            let mut default = self.parse_expression()?;
            if is_starred_typevartuple_default {
                default = Expr::Starred(Box::new(default));
            }
            let context = match kind {
                TypeParamKind::TypeVar => "a TypeVar default",
                TypeParamKind::TypeVarTuple => "a TypeVarTuple default",
                TypeParamKind::ParamSpec => "a ParamSpec default",
            };
            validate_type_scope_expression(&default, context)?;
            Some(default)
        } else {
            None
        };

        Ok(TypeParam {
            kind,
            name,
            bound,
            default,
        })
    }

    fn parse_parameter_list(&mut self, end: ParameterListEnd) -> Result<FunctionParams, String> {
        let mut params = FunctionParams::default();
        let mut seen_names = Vec::new();
        let mut saw_default = false;
        let mut after_star = false;
        let mut bare_star_needs_keyword_only = false;
        let mut saw_slash = false;
        let mut vararg_name_for_unique_check = None;
        let mut keyword_only_names_for_unique_check = Vec::new();
        let mut kwarg_name_for_unique_check = None;
        let mut last_parameter: Option<ParameterTypeCommentTarget>;
        let allow_annotations = matches!(end, ParameterListEnd::RightParen);

        if self.at_parameter_list_end(end) {
            return Ok(params);
        }

        loop {
            if matches!(end, ParameterListEnd::RightParen)
                && matches!(self.peek(), Some(Token::Eof) | None)
            {
                return Err(unclosed_delimiter_message('('));
            }

            if matches!(self.peek(), Some(Token::LeftParen)) {
                return Err(match end {
                    ParameterListEnd::RightParen => {
                        "Function parameters cannot be parenthesized".to_string()
                    }
                    ParameterListEnd::Colon => {
                        "Lambda expression parameters cannot be parenthesized".to_string()
                    }
                });
            }

            if matches!(self.peek(), Some(Token::Slash)) {
                if after_star {
                    return Err("/ must be ahead of *".to_string());
                }
                if saw_slash {
                    return Err("/ may appear only once".to_string());
                }
                if params.positional.is_empty() {
                    return Err("at least one parameter must precede /".to_string());
                }

                self.advance();
                saw_slash = true;
                params.positional_only = std::mem::take(&mut params.positional);

                if matches!(self.peek(), Some(Token::Star)) {
                    return Err("expected comma between / and *".to_string());
                }

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                    if self.at_parameter_list_end(end) {
                        break;
                    }
                    continue;
                }

                if self.at_parameter_list_end(end) {
                    break;
                }

                return Err(format!(
                    "expected ',' or {} after positional-only marker",
                    parameter_list_end_label(end)
                ));
            }

            if matches!(self.peek(), Some(Token::DoubleStar)) {
                if bare_star_needs_keyword_only {
                    return Err("named parameters must follow bare *".to_string());
                }
                self.advance();
                let name = self.expect_identifier("** parameter name")?;
                validate_binding_name(&name)?;
                params.kwarg_annotation = self
                    .parse_optional_parameter_annotation(allow_annotations)?
                    .map(Box::new);
                if matches!(self.peek(), Some(Token::Equal)) {
                    return Err("var-keyword parameter cannot have default value".to_string());
                }
                kwarg_name_for_unique_check = Some(name.clone());
                params.kwarg = Some(name);
                if let Some(comment) = self.take_type_comment() {
                    params.kwarg_type_comment = Some(comment);
                }
                last_parameter = Some(ParameterTypeCommentTarget::Kwarg);

                if matches!(self.peek(), Some(Token::Comma)) {
                    self.advance();
                    if let Some(comment) = self.take_type_comment() {
                        assign_parameter_type_comment(&mut params, last_parameter, comment)?;
                    }
                    if !self.at_parameter_list_end(end) {
                        return Err("parameters cannot follow var-keyword parameter".to_string());
                    }
                }
                break;
            }

            if matches!(self.peek(), Some(Token::Star)) {
                self.advance();
                if after_star {
                    return Err("* may appear only once".to_string());
                }
                after_star = true;

                match self.peek() {
                    Some(Token::Identifier(_)) => {
                        let name = self.expect_identifier("* parameter name")?;
                        validate_binding_name(&name)?;
                        params.vararg_annotation = self
                            .parse_optional_parameter_annotation(allow_annotations)?
                            .map(Box::new);
                        if matches!(self.peek(), Some(Token::Equal)) {
                            return Err(
                                "var-positional parameter cannot have default value".to_string()
                            );
                        }
                        vararg_name_for_unique_check = Some(name.clone());
                        params.vararg = Some(name);
                        if let Some(comment) = self.take_type_comment() {
                            params.vararg_type_comment = Some(comment);
                        }
                        last_parameter = Some(ParameterTypeCommentTarget::Vararg);
                    }
                    Some(Token::Comma) => {
                        self.advance();
                        if self.at_parameter_list_end(end) {
                            return Err("named parameters must follow bare *".to_string());
                        }
                        bare_star_needs_keyword_only = true;
                        continue;
                    }
                    Some(_) if self.at_parameter_list_end(end) => {
                        return Err("named parameters must follow bare *".to_string());
                    }
                    Some(token) => {
                        return Err(format!(
                            "expected parameter name after '*', found {token:?}"
                        ));
                    }
                    None => {
                        return Err(
                            "expected parameter name after '*', found end of input".to_string()
                        );
                    }
                }
            } else {
                let mut param =
                    self.parse_parameter(&mut seen_names, allow_annotations, !after_star)?;
                if let Some(comment) = self.take_type_comment() {
                    param.type_comment = Some(comment);
                }

                if after_star {
                    keyword_only_names_for_unique_check.push(param.name.clone());
                    params.keyword_only.push(param);
                    last_parameter = Some(ParameterTypeCommentTarget::KeywordOnly(
                        params.keyword_only.len() - 1,
                    ));
                    bare_star_needs_keyword_only = false;
                } else {
                    if param.default.is_some() {
                        saw_default = true;
                    } else if saw_default {
                        return Err(
                            "parameter without a default follows parameter with a default"
                                .to_string(),
                        );
                    }
                    params.positional.push(param);
                    last_parameter = Some(ParameterTypeCommentTarget::Positional(
                        params.positional.len() - 1,
                    ));
                }
            }

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if let Some(comment) = self.take_type_comment() {
                assign_parameter_type_comment(&mut params, last_parameter, comment)?;
            }
            if self.at_parameter_list_end(end) {
                break;
            }
        }

        if let Some(name) = vararg_name_for_unique_check {
            ensure_unique_parameter_name(&name, &mut seen_names)?;
        }
        for name in keyword_only_names_for_unique_check {
            ensure_unique_parameter_name(&name, &mut seen_names)?;
        }
        if let Some(name) = kwarg_name_for_unique_check {
            ensure_unique_parameter_name(&name, &mut seen_names)?;
        }

        Ok(params)
    }

    fn at_parameter_list_end(&self, end: ParameterListEnd) -> bool {
        match end {
            ParameterListEnd::RightParen => matches!(self.peek(), Some(Token::RightParen)),
            ParameterListEnd::Colon => matches!(self.peek(), Some(Token::Colon)),
        }
    }

    fn parse_parameter(
        &mut self,
        seen_names: &mut Vec<String>,
        allow_annotations: bool,
        check_unique_name: bool,
    ) -> Result<Param, String> {
        let name = self.expect_identifier("parameter name")?;
        validate_binding_name(&name)?;
        if check_unique_name {
            ensure_unique_parameter_name(&name, seen_names)?;
        }

        let annotation = self.parse_optional_parameter_annotation(allow_annotations)?;
        let default = if matches!(self.peek(), Some(Token::Equal)) {
            self.advance();
            if is_parameter_default_end(self.peek()) {
                return Err("expected default value expression".to_string());
            }
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Param {
            name,
            annotation,
            default,
            type_comment: None,
        })
    }

    fn parse_optional_parameter_annotation(
        &mut self,
        allow_annotations: bool,
    ) -> Result<Option<Expr>, String> {
        if !allow_annotations || !matches!(self.peek(), Some(Token::Colon)) {
            return Ok(None);
        }

        self.advance();
        Ok(Some(self.parse_parameter_annotation_expression()?))
    }

    fn parse_parameter_annotation_expression(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Star)) {
            self.advance();
            if is_invalid_star_expression_end(self.peek()) {
                return Err("Invalid star expression".to_string());
            }

            let value = self.parse_bitwise_or()?;
            if matches!(self.peek(), Some(Token::If)) {
                return Err(
                    "invalid starred expression. Did you forget to wrap the conditional expression in parentheses?"
                        .to_string(),
                );
            }

            return Ok(Expr::Starred(Box::new(value)));
        }

        if matches!(self.peek(), Some(Token::DoubleStar)) {
            self.advance();
            self.parse_bitwise_or()?;
            return Err("cannot use dict unpacking here".to_string());
        }

        self.parse_expression()
    }

    fn parse_return_statement(&mut self) -> Result<Stmt, String> {
        self.expect_return()?;

        if matches!(
            self.peek(),
            Some(Token::Semicolon)
                | Some(Token::TypeComment(_) | Token::TypeIgnore(_))
                | Some(Token::Newline)
                | Some(Token::Dedent)
                | Some(Token::Eof)
                | None
        ) {
            return Ok(Stmt::Return(None));
        }

        let value = self.parse_expression_list_until_statement_boundary()?;
        Ok(Stmt::Return(Some(value)))
    }

    fn parse_yield_statement(&mut self) -> Result<Stmt, String> {
        Ok(Stmt::Expr(self.parse_yield_expression()?))
    }

    fn parse_raise_statement(&mut self) -> Result<Stmt, String> {
        self.expect_raise()?;

        if matches!(self.peek(), Some(Token::From)) {
            return Err("did you forget an expression between 'raise' and 'from'?".to_string());
        }

        if matches!(
            self.peek(),
            Some(Token::Semicolon)
                | Some(Token::TypeComment(_) | Token::TypeIgnore(_))
                | Some(Token::Newline)
                | Some(Token::Dedent)
                | Some(Token::Eof)
                | None
        ) {
            return Ok(Stmt::Raise {
                value: None,
                cause: None,
            });
        }

        let value = self.parse_expression()?;
        let cause = if matches!(self.peek(), Some(Token::From)) {
            self.advance();
            if is_statement_boundary(self.peek()) {
                return Err("did you forget an expression after 'from'?".to_string());
            }
            Some(self.parse_expression()?)
        } else {
            None
        };

        Ok(Stmt::Raise {
            value: Some(value),
            cause,
        })
    }

    fn parse_delete_statement(&mut self) -> Result<Stmt, String> {
        self.expect_del()?;
        let target_start = self.current;

        if matches!(self.peek(), Some(Token::LeftParen))
            && matches!(self.peek_next(), Some(Token::Comma))
        {
            return Err("invalid syntax".to_string());
        }

        match self.parse_assignment_target() {
            Ok(target) if is_statement_boundary(self.peek()) => {
                validate_delete_target(&target)?;
                Ok(Stmt::Delete { target })
            }
            Ok(_) if is_aug_assign_operator(self.peek()) => Err("invalid syntax".to_string()),
            Ok(_) => {
                self.current = target_start;
                let expr = self.parse_expression_list_until_statement_boundary()?;
                Err(invalid_delete_target_message(&expr))
            }
            Err(target_error) => {
                if target_error == "starred assignment target must be in a list or tuple" {
                    return Err("cannot delete starred target".to_string());
                }
                self.current = target_start;
                match self.parse_expression_list_until_statement_boundary() {
                    Ok(expr) => Err(invalid_delete_target_message(&expr)),
                    Err(_) => Err(target_error),
                }
            }
        }
    }

    fn parse_global_statement(&mut self) -> Result<Stmt, String> {
        self.expect_global()?;
        let mut names = vec![self.expect_identifier("global name")?];

        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            names.push(self.expect_identifier("global name")?);
        }

        Ok(Stmt::Global(names))
    }

    fn parse_nonlocal_statement(&mut self) -> Result<Stmt, String> {
        self.expect_nonlocal()?;
        let mut names = vec![self.expect_identifier("nonlocal name")?];

        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            names.push(self.expect_identifier("nonlocal name")?);
        }

        Ok(Stmt::Nonlocal(names))
    }

    fn parse_assert_statement(&mut self) -> Result<Stmt, String> {
        self.expect_assert()?;
        let condition = self.parse_expression()?;
        if matches!(self.peek(), Some(Token::Equal)) {
            return Err(invalid_assert_assignment_message(&condition));
        }
        if matches!(self.peek(), Some(Token::ColonEqual)) {
            return Err("cannot use named expression without parentheses here".to_string());
        }

        let message = if matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            let message = self.parse_expression_list_until_statement_boundary()?;
            if matches!(self.peek(), Some(Token::Equal)) {
                return Err(invalid_assert_assignment_message(&message));
            }
            if matches!(self.peek(), Some(Token::ColonEqual)) {
                return Err("cannot use named expression without parentheses here".to_string());
            }
            Some(message)
        } else {
            None
        };

        Ok(Stmt::Assert { condition, message })
    }

    fn parse_assignment_target(&mut self) -> Result<Target, String> {
        let start = self.current;
        let (targets, has_comma) = match self.parse_target_sequence() {
            Ok(parsed) => parsed,
            Err(error) if is_assignment_target_syntax_error(&error) => return Err(error),
            Err(_) => {
                self.current = start;
                return self.parse_t_primary_assignment_target();
            }
        };

        if !has_comma && matches!(self.peek(), Some(Token::LeftParen)) {
            self.current = start;
            return self.parse_t_primary_assignment_target();
        }

        if has_comma {
            Ok(Target::Tuple(targets))
        } else {
            let target = targets
                .into_iter()
                .next()
                .ok_or_else(|| "expected assignment target".to_string())?;
            if matches!(target, Target::Starred(_)) {
                return Err("starred assignment target must be in a list or tuple".to_string());
            }
            Ok(target)
        }
    }

    fn parse_t_primary_assignment_target(&mut self) -> Result<Target, String> {
        let expr = self.parse_trailer()?;
        expr_to_target(expr.clone()).ok_or_else(|| invalid_expression_assignment_message(&expr))
    }

    fn match_aug_assign_operator(&mut self) -> Option<BinaryOp> {
        let op = match self.peek()? {
            Token::PlusEqual => BinaryOp::Add,
            Token::MinusEqual => BinaryOp::Subtract,
            Token::StarEqual => BinaryOp::Multiply,
            Token::AtEqual => BinaryOp::MatrixMultiply,
            Token::SlashEqual => BinaryOp::TrueDivide,
            Token::DoubleSlashEqual => BinaryOp::FloorDivide,
            Token::PercentEqual => BinaryOp::Modulo,
            Token::DoubleStarEqual => BinaryOp::Power,
            Token::PipeEqual => BinaryOp::BitOr,
            Token::CaretEqual => BinaryOp::BitXor,
            Token::AmpersandEqual => BinaryOp::BitAnd,
            Token::LeftShiftEqual => BinaryOp::LeftShift,
            Token::RightShiftEqual => BinaryOp::RightShift,
            _ => return None,
        };

        self.advance();
        Some(op)
    }

    fn parse_target_sequence(&mut self) -> Result<(Vec<Target>, bool), String> {
        let first = self.parse_single_target()?;
        let mut starred_count = usize::from(matches!(first, Target::Starred(_)));
        let mut targets = vec![first];
        let mut has_comma = false;

        while matches!(self.peek(), Some(Token::Comma)) {
            has_comma = true;
            self.advance();

            if matches!(
                self.peek(),
                Some(Token::Equal)
                    | Some(Token::In)
                    | Some(Token::RightParen)
                    | Some(Token::RightBracket)
            ) || is_statement_boundary(self.peek())
            {
                break;
            }

            let target = self.parse_single_target()?;
            if matches!(target, Target::Starred(_)) {
                starred_count += 1;
                if starred_count > 1 {
                    return Err("multiple starred expressions in assignment".to_string());
                }
            }
            targets.push(target);
        }

        Ok((targets, has_comma))
    }

    fn parse_single_target(&mut self) -> Result<Target, String> {
        let mut target = match self.advance() {
            Some(Token::Identifier(name)) => Ok(Target::Name(name.clone())),
            Some(Token::Star) => {
                let target = self.parse_single_target()?;
                if matches!(target, Target::Starred(_)) {
                    Err("multiple starred expressions in assignment".to_string())
                } else {
                    Ok(Target::Starred(Box::new(target)))
                }
            }
            Some(Token::LeftParen) => {
                if matches!(self.peek(), Some(Token::RightParen)) {
                    self.advance();
                    return Ok(Target::Tuple(Vec::new()));
                }

                let (targets, has_comma) = self.parse_target_sequence()?;
                self.expect_right_paren()?;

                if has_comma {
                    Ok(Target::Tuple(targets))
                } else {
                    let target = targets
                        .into_iter()
                        .next()
                        .ok_or_else(|| "expected assignment target".to_string())?;
                    if matches!(target, Target::Starred(_)) {
                        Err("cannot use starred expression here".to_string())
                    } else {
                        Ok(target)
                    }
                }
            }
            Some(Token::LeftBracket) => {
                let targets = if matches!(self.peek(), Some(Token::RightBracket)) {
                    Vec::new()
                } else {
                    self.parse_target_sequence()?.0
                };
                self.expect_right_bracket()?;
                Ok(Target::List(targets))
            }
            Some(token) => Err(format!("expected assignment target, found {token:?}")),
            None => Err("expected assignment target, found end of input".to_string()),
        }?;

        while matches!(self.peek(), Some(Token::Dot) | Some(Token::LeftBracket)) {
            target = self.parse_target_trailer(target)?;
        }

        Ok(target)
    }

    fn parse_target_trailer(&mut self, target: Target) -> Result<Target, String> {
        match self.advance() {
            Some(Token::Dot) => {
                let name = self.expect_identifier("attribute name")?;
                let object = target_to_expr(target)
                    .ok_or_else(|| "invalid attribute assignment target".to_string())?;
                Ok(Target::Attribute {
                    object: Box::new(object),
                    name,
                })
            }
            Some(Token::LeftBracket) => {
                let object = target_to_expr(target)
                    .ok_or_else(|| "invalid subscript assignment target".to_string())?;
                let index = self.parse_target_subscript_index()?;
                self.expect_right_bracket()?;
                Ok(Target::Subscript {
                    object: Box::new(object),
                    index,
                })
            }
            Some(token) => Err(format!(
                "expected assignment target trailer, found {token:?}"
            )),
            None => Err("expected assignment target trailer, found end of input".to_string()),
        }
    }

    fn parse_target_subscript_index(&mut self) -> Result<Expr, String> {
        let first_item = self.parse_target_subscript_item()?;

        if !matches!(self.peek(), Some(Token::Comma)) {
            if matches!(first_item, Expr::Starred(_)) {
                return Ok(Expr::Tuple(vec![first_item]));
            }
            return Ok(first_item);
        }

        let mut items = vec![first_item];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::RightBracket)) {
                break;
            }
            items.push(self.parse_target_subscript_item()?);
        }

        Ok(Expr::Tuple(items))
    }

    fn parse_target_subscript_item(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            let (stop, step) = self.parse_subscript_slice_stop_and_step()?;
            return Ok(Expr::SliceLiteral {
                start: None,
                stop,
                step,
            });
        }

        if matches!(self.peek(), Some(Token::Star))
            && (is_invalid_star_expression_end(self.peek_next())
                || matches!(self.peek_next(), Some(Token::Colon | Token::Equal)))
        {
            return Err("Invalid star expression".to_string());
        }

        if matches!(self.peek(), Some(Token::Star)) {
            return self.parse_starred_bitwise_expression();
        }

        let start_or_index = self.parse_expression()?;
        if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            let (stop, step) = self.parse_subscript_slice_stop_and_step()?;
            return Ok(Expr::SliceLiteral {
                start: Some(Box::new(start_or_index)),
                stop,
                step,
            });
        }

        Ok(start_or_index)
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, String> {
        self.skip_newlines();
        self.expect_indent()?;
        let mut statements = Vec::new();

        self.skip_newlines();
        while !matches!(self.peek(), Some(Token::Dedent) | Some(Token::Eof) | None) {
            let statement = self.parse_statement()?;
            let has_own_boundary = statement_has_own_boundary(&statement);
            statements.push(statement);

            match self.peek() {
                Some(Token::Semicolon) => self.skip_statement_separators(),
                Some(Token::Newline) => self.skip_newlines(),
                Some(Token::TypeComment(_) | Token::TypeIgnore(_)) => {
                    self.skip_type_comment_tokens();
                    self.skip_newlines();
                }
                Some(Token::Dedent) | Some(Token::Eof) | None => {}
                Some(_) if has_own_boundary => {}
                Some(token) => {
                    let previous = statements.last().expect("statement was just pushed");
                    if let Some(error) = self.former_statement_boundary_error(previous) {
                        return Err(error);
                    }
                    return Err(format!(
                        "expected statement separator or dedent, found {token:?}"
                    ));
                }
            }
        }

        self.expect_dedent()?;
        Ok(statements)
    }

    fn parse_block_after_colon(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(
            self.peek(),
            Some(Token::Newline | Token::TypeComment(_) | Token::TypeIgnore(_))
        ) {
            self.expect_newline()?;
            return self.parse_block();
        }

        self.parse_inline_block()
    }

    fn parse_function_block_after_colon(&mut self) -> Result<(Option<String>, Vec<Stmt>), String> {
        if matches!(
            self.peek(),
            Some(Token::Newline | Token::TypeComment(_) | Token::TypeIgnore(_))
        ) {
            let type_comment = self.parse_func_type_comment()?;
            let body = self.parse_block()?;
            return Ok((type_comment, body));
        }

        self.parse_inline_block().map(|body| (None, body))
    }

    fn parse_function_body_after_colon(&mut self) -> Result<(Option<String>, Vec<Stmt>), String> {
        let saved_class_body_depth = self.class_body_depth;
        self.class_body_depth = 0;
        let body = self.parse_function_block_after_colon();
        self.class_body_depth = saved_class_body_depth;
        body
    }

    fn parse_class_body_after_colon(&mut self) -> Result<Vec<Stmt>, String> {
        self.class_body_depth += 1;
        let body = self.parse_block_after_colon();
        self.class_body_depth -= 1;
        body
    }

    fn parse_inline_block(&mut self) -> Result<Vec<Stmt>, String> {
        if matches!(
            self.peek(),
            Some(Token::Newline | Token::Dedent | Token::Eof) | None
        ) {
            return Err("expected statement after ':'".to_string());
        }

        let mut statements = Vec::new();
        loop {
            let statement = self.parse_statement()?;
            if statement_has_own_boundary(&statement) {
                return Err("compound statement cannot appear in inline suite".to_string());
            }
            statements.push(statement);

            match self.peek() {
                Some(Token::Semicolon) => {
                    while matches!(self.peek(), Some(Token::Semicolon)) {
                        self.advance();
                    }
                    if matches!(
                        self.peek(),
                        Some(
                            Token::Newline
                                | Token::TypeComment(_)
                                | Token::TypeIgnore(_)
                                | Token::Dedent
                                | Token::Eof
                        ) | None
                    ) {
                        break;
                    }
                }
                Some(Token::TypeComment(_) | Token::TypeIgnore(_)) => {
                    self.skip_type_comment_tokens();
                    break;
                }
                Some(Token::Newline | Token::Dedent | Token::Eof) | None => break,
                Some(token) => {
                    let previous = statements.last().expect("statement was just pushed");
                    if let Some(error) = self.former_statement_boundary_error(previous) {
                        return Err(error);
                    }
                    return Err(format!(
                        "expected statement separator or newline, found {token:?}"
                    ));
                }
            }
        }

        self.skip_newlines();
        Ok(statements)
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        if conditional_body_starts_statement(self.peek())
            && matches!(self.peek_next(), Some(Token::If))
        {
            return Err("expected expression before 'if', but statement is given".to_string());
        }

        if matches!(self.peek(), Some(Token::Yield)) {
            return self.parse_yield_expression();
        }

        if matches!(self.peek(), Some(Token::Lambda)) {
            return self.parse_lambda_expression();
        }

        let then_branch = self.parse_or()?;

        if !matches!(self.peek(), Some(Token::If)) {
            return Ok(then_branch);
        }

        self.advance();
        let condition = self.parse_or()?;

        if !matches!(self.peek(), Some(Token::Else)) {
            return match self.peek() {
                Some(token) => Err(format!(
                    "expected 'else' in conditional expression, found {token:?}"
                )),
                None => {
                    Err("expected 'else' in conditional expression, found end of input".to_string())
                }
            };
        }

        self.advance();
        if conditional_else_starts_statement(self.peek()) {
            return Err("expected expression after 'else', but statement is given".to_string());
        }
        let else_branch = self.parse_expression()?;

        Ok(Expr::IfExpression {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Box::new(else_branch),
        })
    }

    fn parse_named_expression(&mut self) -> Result<Expr, String> {
        self.parse_named_expression_with_assignment_error(None)
    }

    fn parse_call_argument_expression(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Yield)) {
            return Err("yield expression must be parenthesized".to_string());
        }
        self.parse_named_expression_with_assignment_error(Some(
            "expression cannot contain assignment, perhaps you meant \"==\"?",
        ))
    }

    fn parse_named_expression_with_assignment_error(
        &mut self,
        assignment_error: Option<&str>,
    ) -> Result<Expr, String> {
        if let (Some(Token::Identifier(name)), Some(Token::ColonEqual)) =
            (self.peek(), self.peek_next())
        {
            if self.comprehension_iter_depth > 0 {
                return Err(
                    "assignment expression cannot be used in a comprehension iterable expression"
                        .to_string(),
                );
            }
            let name = name.clone();
            self.advance();
            validate_binding_name(&name)?;
            self.expect_colon_equal()?;
            let value = self.parse_expression()?;

            return Ok(Expr::NamedExpr {
                name,
                value: Box::new(value),
            });
        }

        let expr = self.parse_expression()?;

        if matches!(self.peek(), Some(Token::ColonEqual)) {
            if self.comprehension_iter_depth > 0 {
                return Err(
                    "assignment expression cannot be used in a comprehension iterable expression"
                        .to_string(),
                );
            }
            return Err(format!(
                "cannot use assignment expressions with {}",
                invalid_named_expression_target_name(&expr)
            ));
        }

        if matches!(self.peek(), Some(Token::Equal)) {
            return Err(assignment_error
                .map(str::to_string)
                .unwrap_or_else(|| invalid_expression_assignment_message(&expr)));
        }

        Ok(expr)
    }

    fn parse_lambda_expression(&mut self) -> Result<Expr, String> {
        self.expect_lambda()?;
        let params = self.parse_parameter_list(ParameterListEnd::Colon)?;
        self.expect_colon()?;
        let saved_class_body_depth = self.class_body_depth;
        self.class_body_depth = 0;
        let body = self.parse_expression();
        self.class_body_depth = saved_class_body_depth;
        let body = body?;

        Ok(Expr::Lambda {
            params,
            body: Box::new(body),
        })
    }

    fn parse_func_type_input(&mut self) -> Result<FunctionType, String> {
        self.expect_left_paren()?;
        let arg_types = self.parse_type_expressions()?;
        self.expect_right_paren()?;
        self.expect_arrow()?;
        let returns = self.parse_expression()?;

        Ok(FunctionType { arg_types, returns })
    }

    fn parse_type_expressions(&mut self) -> Result<Vec<Expr>, String> {
        let mut arg_types = Vec::new();
        let mut seen_star = false;
        let mut seen_double_star = false;

        if matches!(self.peek(), Some(Token::RightParen)) {
            return Ok(arg_types);
        }

        loop {
            let marker = match self.peek() {
                Some(Token::Star) => {
                    if seen_star {
                        return Err("multiple '*' type expressions are not allowed".to_string());
                    }
                    if seen_double_star {
                        return Err("'*' type expression cannot follow '**'".to_string());
                    }
                    self.advance();
                    seen_star = true;
                    TypeExpressionMarker::Star
                }
                Some(Token::DoubleStar) => {
                    if seen_double_star {
                        return Err("multiple '**' type expressions are not allowed".to_string());
                    }
                    self.advance();
                    seen_double_star = true;
                    TypeExpressionMarker::DoubleStar
                }
                _ => {
                    if seen_star || seen_double_star {
                        return Err("plain type expression cannot follow '*' or '**'".to_string());
                    }
                    TypeExpressionMarker::Plain
                }
            };

            arg_types.push(self.parse_expression()?);

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }
            self.advance();

            if matches!(self.peek(), Some(Token::RightParen)) {
                return Err("trailing comma in type expressions is not allowed".to_string());
            }

            if matches!(marker, TypeExpressionMarker::DoubleStar) {
                return Err("'**' type expression must be last".to_string());
            }
        }

        Ok(arg_types)
    }

    fn parse_func_type_comment(&mut self) -> Result<Option<String>, String> {
        let mut type_comment = None;

        if let Some(Token::TypeComment(body)) = self.peek().cloned() {
            self.advance();
            type_comment = Some(body);
        } else if matches!(self.peek(), Some(Token::TypeIgnore(_))) {
            self.advance();
        }

        self.expect_newline()?;

        if let Some(Token::TypeComment(body)) = self.peek().cloned() {
            if type_comment.is_some() {
                return Err("Cannot have two type comments on def".to_string());
            }
            self.advance();
            type_comment = Some(body);

            match self.advance() {
                Some(Token::Newline) => {}
                Some(token) => return Err(format!("expected newline, found {token:?}")),
                None => return Err("expected newline, found end of input".to_string()),
            }
        }

        if type_comment.is_some() && matches!(self.peek(), Some(Token::TypeComment(_))) {
            return Err("Cannot have two type comments on def".to_string());
        }

        Ok(type_comment)
    }

    fn parse_yield_expression(&mut self) -> Result<Expr, String> {
        self.expect_yield()?;

        if matches!(self.peek(), Some(Token::Equal)) {
            return Err("assignment to yield expression not possible".to_string());
        }

        if matches!(self.peek(), Some(Token::From)) {
            self.advance();
            let value = self.parse_expression()?;
            if matches!(self.peek(), Some(Token::Comma)) {
                return Err("'yield from' does not accept an implicit tuple".to_string());
            }
            if matches!(self.peek(), Some(Token::Equal)) {
                return Err("assignment to yield expression not possible".to_string());
            }
            return Ok(Expr::YieldFrom(Box::new(value)));
        }

        if is_yield_value_end(self.peek()) {
            return Ok(Expr::Yield { value: None });
        }

        let value = self.parse_expression_list_until(is_yield_value_end)?;
        if matches!(self.peek(), Some(Token::Equal)) {
            return Err("assignment to yield expression not possible".to_string());
        }
        Ok(Expr::Yield {
            value: Some(Box::new(value)),
        })
    }

    fn parse_expression_list_until_statement_boundary(&mut self) -> Result<Expr, String> {
        self.parse_expression_list_until(|token| {
            matches!(
                token,
                Some(Token::Semicolon)
                    | Some(Token::Newline)
                    | Some(Token::Dedent)
                    | Some(Token::Eof)
                    | None
            )
        })
    }

    fn parse_expression_list_until_colon(&mut self) -> Result<Expr, String> {
        self.parse_expression_list_until(|token| matches!(token, Some(Token::Colon)))
    }

    fn parse_expression_list_until<F>(&mut self, is_terminator: F) -> Result<Expr, String>
    where
        F: Fn(Option<&Token>) -> bool,
    {
        let first = self.parse_star_expression()?;

        if !matches!(self.peek(), Some(Token::Comma)) {
            if matches!(first, Expr::Starred(_)) {
                return Err("cannot use starred expression here".to_string());
            }
            return Ok(first);
        }

        let mut elements = vec![first];
        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();

            if is_terminator(self.peek()) {
                break;
            }
            if matches!(self.peek(), Some(Token::Yield)) {
                return Err("yield expression must be parenthesized".to_string());
            }

            elements.push(self.parse_star_expression()?);
        }

        Ok(Expr::Tuple(elements))
    }

    fn parse_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_and()?;

        while matches!(self.peek(), Some(Token::Or)) {
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::Logical {
                left: Box::new(expr),
                op: LogicalOp::Or,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_not()?;

        while matches!(self.peek(), Some(Token::And)) {
            self.advance();
            let right = self.parse_not()?;
            expr = Expr::Logical {
                left: Box::new(expr),
                op: LogicalOp::And,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_not(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Not)) {
            self.advance();
            let operand = self.parse_not()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
            });
        }

        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let left = self.parse_bitwise_or()?;
        let mut comparisons = Vec::new();

        while let Some(op) = self.match_comparison_operator() {
            let right = self.parse_bitwise_or()?;
            comparisons.push((op, right));
        }

        match comparisons.len() {
            0 => Ok(left),
            1 => {
                let (op, right) = comparisons
                    .into_iter()
                    .next()
                    .expect("comparison length is one");
                Ok(Expr::Comparison {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                })
            }
            _ => Ok(Expr::ChainedComparison {
                left: Box::new(left),
                comparisons,
            }),
        }
    }

    fn match_comparison_operator(&mut self) -> Option<ComparisonOp> {
        let op = match self.peek()? {
            Token::EqualEqual => ComparisonOp::Equal,
            Token::BangEqual => ComparisonOp::NotEqual,
            Token::Less => ComparisonOp::Less,
            Token::LessEqual => ComparisonOp::LessEqual,
            Token::Greater => ComparisonOp::Greater,
            Token::GreaterEqual => ComparisonOp::GreaterEqual,
            Token::In => ComparisonOp::In,
            Token::Is if matches!(self.peek_next(), Some(Token::Not)) => {
                self.advance();
                self.advance();
                return Some(ComparisonOp::IsNot);
            }
            Token::Is => ComparisonOp::Is,
            Token::Not if matches!(self.peek_next(), Some(Token::In)) => {
                self.advance();
                self.advance();
                return Some(ComparisonOp::NotIn);
            }
            _ => return None,
        };

        self.advance();
        Some(op)
    }

    fn parse_bitwise_or(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_xor()?;

        while matches!(self.peek(), Some(Token::Pipe)) {
            self.advance();
            let right = self.parse_bitwise_xor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitOr,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_xor(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_bitwise_and()?;

        while matches!(self.peek(), Some(Token::Caret)) {
            self.advance();
            let right = self.parse_bitwise_and()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitXor,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_bitwise_and(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_shift()?;

        while matches!(self.peek(), Some(Token::Ampersand)) {
            self.advance();
            let right = self.parse_shift()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::BitAnd,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_shift(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_sum()?;

        while matches!(
            self.peek(),
            Some(Token::LeftShift) | Some(Token::RightShift)
        ) {
            let op = match self.advance() {
                Some(Token::LeftShift) => BinaryOp::LeftShift,
                Some(Token::RightShift) => BinaryOp::RightShift,
                _ => unreachable!("shift operator already matched"),
            };
            let right = self.parse_sum()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_sum(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_term()?;

        while matches!(self.peek(), Some(Token::Plus) | Some(Token::Minus)) {
            let op = match self.advance() {
                Some(Token::Plus) => BinaryOp::Add,
                Some(Token::Minus) => BinaryOp::Subtract,
                _ => unreachable!("sum operator already matched"),
            };
            self.reject_not_after_arithmetic_operator()?;
            let right = self.parse_term()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_term(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_factor()?;

        while matches!(
            self.peek(),
            Some(Token::Star)
                | Some(Token::At)
                | Some(Token::Slash)
                | Some(Token::DoubleSlash)
                | Some(Token::Percent)
        ) {
            let op = match self.advance() {
                Some(Token::Star) => BinaryOp::Multiply,
                Some(Token::At) => BinaryOp::MatrixMultiply,
                Some(Token::Slash) => BinaryOp::TrueDivide,
                Some(Token::DoubleSlash) => BinaryOp::FloorDivide,
                Some(Token::Percent) => BinaryOp::Modulo,
                _ => unreachable!("term operator already matched"),
            };
            self.reject_not_after_arithmetic_operator()?;
            let right = self.parse_factor()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_factor(&mut self) -> Result<Expr, String> {
        let mut operators = Vec::new();

        while matches!(
            self.peek(),
            Some(Token::Plus) | Some(Token::Minus) | Some(Token::Tilde)
        ) {
            if operators.len() >= MAX_UNARY_OPERATOR_DEPTH {
                return Err("too complex".to_string());
            }

            operators.push(match self.advance() {
                Some(Token::Plus) => UnaryOp::Positive,
                Some(Token::Minus) => UnaryOp::Negative,
                Some(Token::Tilde) => UnaryOp::Invert,
                _ => unreachable!("factor operator already matched"),
            });
            self.reject_not_after_arithmetic_operator()?;
        }

        let mut expr = self.parse_power()?;

        for op in operators.into_iter().rev() {
            expr = Expr::Unary {
                op,
                operand: Box::new(expr),
            };
        }

        Ok(expr)
    }

    fn reject_not_after_arithmetic_operator(&self) -> Result<(), String> {
        if matches!(self.peek(), Some(Token::Not)) {
            return Err("'not' after an operator must be parenthesized".to_string());
        }

        Ok(())
    }

    fn parse_power(&mut self) -> Result<Expr, String> {
        let left = self.parse_await_primary()?;

        if matches!(self.peek(), Some(Token::DoubleStar)) {
            self.advance();
            let right = self.parse_factor()?;
            return Ok(Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Power,
                right: Box::new(right),
            });
        }

        Ok(left)
    }

    fn parse_await_primary(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Await)) {
            self.advance();
            let operand = self.parse_trailer()?;
            return Ok(Expr::Await(Box::new(operand)));
        }

        self.parse_trailer()
    }

    fn parse_trailer(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek() {
                Some(Token::LeftParen) => {
                    self.advance();
                    let arguments = self.parse_arguments()?;
                    self.expect_right_paren()?;

                    expr = if arguments.has_unpack {
                        Expr::UnpackCall {
                            callee: Box::new(expr),
                            args: arguments.args,
                            keywords: arguments.keywords,
                        }
                    } else if arguments.keywords.is_empty() {
                        Expr::Call {
                            callee: Box::new(expr),
                            args: call_arg_exprs(arguments.args),
                        }
                    } else {
                        Expr::KeywordCall {
                            callee: Box::new(expr),
                            args: call_arg_exprs(arguments.args),
                            keywords: call_keyword_exprs(arguments.keywords),
                        }
                    };
                }
                Some(Token::LeftBracket) => {
                    self.advance();
                    expr = self.parse_subscript_trailer(expr)?;
                }
                Some(Token::Dot) => {
                    self.advance();
                    let name = self.expect_identifier("attribute name")?;
                    expr = Expr::Attribute {
                        object: Box::new(expr),
                        name,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_subscript_trailer(&mut self, object: Expr) -> Result<Expr, String> {
        let first_item = self.parse_subscript_item()?;

        let index = if matches!(self.peek(), Some(Token::Comma)) {
            let mut items = vec![first_item];
            while matches!(self.peek(), Some(Token::Comma)) {
                self.advance();
                if matches!(self.peek(), Some(Token::RightBracket)) {
                    break;
                }
                items.push(self.parse_subscript_item()?);
            }
            Expr::Tuple(items)
        } else if matches!(first_item, Expr::Starred(_)) {
            Expr::Tuple(vec![first_item])
        } else {
            first_item
        };

        self.expect_right_bracket()?;
        Ok(Expr::Subscript {
            object: Box::new(object),
            index: Box::new(index),
        })
    }

    fn parse_subscript_item(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            let (stop, step) = self.parse_subscript_slice_stop_and_step()?;
            return Ok(Expr::SliceLiteral {
                start: None,
                stop,
                step,
            });
        }

        if matches!(self.peek(), Some(Token::Star))
            && (is_invalid_star_expression_end(self.peek_next())
                || matches!(self.peek_next(), Some(Token::Colon | Token::Equal)))
        {
            return Err("Invalid star expression".to_string());
        }

        if matches!(self.peek(), Some(Token::Star)) {
            return self.parse_starred_bitwise_expression();
        }

        let starts_with_parenthesis = matches!(self.peek(), Some(Token::LeftParen));
        let start_or_index = self.parse_named_expression()?;

        if matches!(self.peek(), Some(Token::Colon)) {
            if matches!(start_or_index, Expr::NamedExpr { .. }) && !starts_with_parenthesis {
                return Err("cannot use named expression without parentheses here".to_string());
            }
            self.advance();
            let (stop, step) = self.parse_subscript_slice_stop_and_step()?;
            return Ok(Expr::SliceLiteral {
                start: Some(Box::new(start_or_index)),
                stop,
                step,
            });
        }

        Ok(start_or_index)
    }

    fn parse_subscript_slice_stop_and_step(
        &mut self,
    ) -> Result<(Option<Box<Expr>>, Option<Box<Expr>>), String> {
        self.parse_slice_stop_and_step_with_comma(true)
    }

    fn parse_slice_stop_and_step_with_comma(
        &mut self,
        allow_comma: bool,
    ) -> Result<(Option<Box<Expr>>, Option<Box<Expr>>), String> {
        let stop =
            if self.is_slice_part_end(allow_comma) || matches!(self.peek(), Some(Token::Colon)) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };

        let step = if matches!(self.peek(), Some(Token::Colon)) {
            self.advance();
            if self.is_slice_part_end(allow_comma) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            }
        } else {
            None
        };

        Ok((stop, step))
    }

    fn is_slice_part_end(&self, allow_comma: bool) -> bool {
        matches!(self.peek(), Some(Token::RightBracket))
            || (allow_comma && matches!(self.peek(), Some(Token::Comma)))
    }

    fn parse_arguments(&mut self) -> Result<CallArguments, String> {
        let mut args = Vec::new();
        let mut keywords = Vec::new();
        let mut saw_keyword = false;
        let mut has_unpack = false;
        let mut saw_keyword_unpack = false;

        if matches!(self.peek(), Some(Token::RightParen)) {
            return Ok(CallArguments {
                args,
                keywords,
                has_unpack,
            });
        }

        loop {
            match (self.peek(), self.peek_next()) {
                (Some(Token::True), Some(Token::Equal)) => {
                    return Err("cannot assign to True".to_string());
                }
                (Some(Token::False), Some(Token::Equal)) => {
                    return Err("cannot assign to False".to_string());
                }
                (Some(Token::None), Some(Token::Equal)) => {
                    return Err("cannot assign to None".to_string());
                }
                _ => {}
            }

            if matches!(self.peek(), Some(Token::DoubleStar)) {
                self.advance();
                let value = self.parse_expression()?;
                if matches!(self.peek(), Some(Token::Equal)) {
                    return Err("cannot assign to keyword argument unpacking".to_string());
                }
                keywords.push(CallKeyword::Unpack(value));
                saw_keyword = true;
                has_unpack = true;
                saw_keyword_unpack = true;
            } else if matches!(self.peek(), Some(Token::Star)) {
                self.advance();
                if is_invalid_star_expression_end(self.peek())
                    || matches!(self.peek(), Some(Token::Colon | Token::Equal))
                {
                    return Err("Invalid star expression".to_string());
                }
                if saw_keyword_unpack {
                    return Err(
                        "iterable argument unpacking follows keyword argument unpacking"
                            .to_string(),
                    );
                }

                let value = self.parse_expression()?;
                if matches!(self.peek(), Some(Token::Equal)) {
                    return Err("cannot assign to iterable argument unpacking".to_string());
                }
                args.push(CallArg::Unpack(value));
                has_unpack = true;
            } else if let (Some(Token::Identifier(name)), Some(Token::Equal)) =
                (self.peek(), self.peek_next())
            {
                let keyword_start = self.current;
                let name = name.clone();
                validate_binding_name(&name)?;
                self.advance();
                self.expect_equal()?;
                if matches!(self.peek(), Some(Token::Comma | Token::RightParen)) {
                    return Err("expected argument value expression".to_string());
                }
                let value = self.parse_expression()?;
                if self.starts_comprehension_clause() {
                    return Err(
                        "invalid syntax. Maybe you meant '==' or ':=' instead of '='?".to_string(),
                    );
                }
                if keywords.iter().any(|keyword| {
                    matches!(keyword, CallKeyword::Named(existing, _) if existing == &name)
                }) {
                    self.current = keyword_start;
                    return Err(format!("keyword argument repeated: {name}"));
                }
                keywords.push(CallKeyword::Named(name, value));
                saw_keyword = true;
            } else {
                if saw_keyword {
                    if saw_keyword_unpack {
                        return Err(
                            "positional argument follows keyword argument unpacking".to_string()
                        );
                    }
                    return Err("positional argument follows keyword argument".to_string());
                }
                let arg = self.parse_call_argument_expression()?;
                let missing_comma_hint_after_arg = matches!(
                    arg,
                    Expr::String(_) | Expr::JoinedString(_) | Expr::TemplateString(_)
                ) && matches!(
                    self.peek(),
                    Some(Token::Identifier(_) | Token::True | Token::False | Token::None)
                );
                if matches!(self.peek(), Some(Token::Equal)) {
                    return Err(
                        "expression cannot contain assignment, perhaps you meant \"==\"?"
                            .to_string(),
                    );
                }
                if self.starts_comprehension_clause() {
                    if !args.is_empty() {
                        return Err("Generator expression must be parenthesized".to_string());
                    }
                    let clauses = self.parse_comprehension_clauses()?;
                    validate_comprehension_named_expression_rebindings(
                        &[&arg],
                        &clauses,
                        self.class_body_depth > 0,
                    )?;
                    if matches!(self.peek(), Some(Token::Comma)) {
                        return Err("Generator expression must be parenthesized".to_string());
                    }
                    args.push(CallArg::Expr(Expr::GeneratorComp {
                        element: Box::new(arg),
                        clauses,
                    }));
                } else {
                    args.push(CallArg::Expr(arg));
                }

                if missing_comma_hint_after_arg {
                    return Err("Perhaps you forgot a comma".to_string());
                }
            }

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if matches!(self.peek(), Some(Token::Comma)) {
                return Err("invalid syntax".to_string());
            }
            if matches!(self.peek(), Some(Token::RightParen)) {
                break;
            }
        }

        Ok(CallArguments {
            args,
            keywords,
            has_unpack,
        })
    }

    fn parse_adjacent_string_parts(
        &mut self,
        mut parts: Vec<FStringPart>,
        mut has_f_string: bool,
    ) -> Result<(Vec<FStringPart>, bool), String> {
        loop {
            match self.peek() {
                Some(Token::String(value)) => {
                    parts.push(FStringPart::Literal(value.clone()));
                    self.advance();
                }
                Some(Token::FString(token_parts)) => {
                    has_f_string = true;
                    let token_parts = token_parts.clone();
                    self.advance();
                    parts.extend(self.parse_f_string_token_parts(&token_parts)?);
                }
                Some(Token::Bytes(_)) => {
                    return Err("cannot mix bytes and nonbytes literals".to_string());
                }
                Some(Token::TString(_)) => {
                    return Err(
                        "cannot mix t-string literals with string or bytes literals".to_string()
                    );
                }
                _ => break,
            }
        }

        Ok((parts, has_f_string))
    }

    fn parse_adjacent_bytes(&mut self, mut value: Vec<u8>) -> Result<Vec<u8>, String> {
        loop {
            match self.peek() {
                Some(Token::Bytes(next)) => {
                    value.extend(next.clone());
                    self.advance();
                }
                Some(Token::TString(_)) => {
                    return Err(
                        "cannot mix t-string literals with string or bytes literals".to_string()
                    );
                }
                Some(Token::String(_) | Token::FString(_)) => {
                    return Err("cannot mix bytes and nonbytes literals".to_string());
                }
                _ => break,
            }
        }

        Ok(value)
    }

    fn parse_adjacent_t_string_parts(
        &mut self,
        mut parts: Vec<TemplateStringPart>,
    ) -> Result<Vec<TemplateStringPart>, String> {
        while let Some(Token::TString(token_parts)) = self.peek() {
            let token_parts = token_parts.clone();
            self.advance();
            parts.extend(self.parse_t_string_token_parts(&token_parts)?);
        }

        if matches!(
            self.peek(),
            Some(Token::String(_) | Token::Bytes(_) | Token::FString(_))
        ) {
            return Err("cannot mix t-string literals with string or bytes literals".to_string());
        }

        Ok(parts)
    }

    fn parse_f_string_token_parts(
        &self,
        parts: &[TokenFStringPart],
    ) -> Result<Vec<FStringPart>, String> {
        let mut parsed = Vec::new();

        for part in parts {
            match part {
                TokenFStringPart::Literal(value) => {
                    parsed.push(FStringPart::Literal(value.clone()))
                }
                TokenFStringPart::Expression {
                    source,
                    conversion,
                    format_spec,
                    debug_label,
                } => {
                    if format_spec.is_some() && starts_unparenthesized_lambda_expression(source) {
                        return Err(
                            "f-string: lambda expressions are not allowed without parentheses"
                                .to_string(),
                        );
                    }
                    if let Some(debug_label) = debug_label {
                        parsed.push(FStringPart::Literal(debug_label.clone()));
                    }
                    let tokens = lex_interpolated_expression_source(source, "f-string")?;
                    let value = parse_eval(&tokens)
                        .map_err(|error| format!("f-string expression parse error: {error}"))?;
                    let format_spec = format_spec
                        .as_deref()
                        .map(|parts| self.parse_f_string_token_parts(parts))
                        .transpose()?;
                    parsed.push(FStringPart::Formatted {
                        value: Box::new(value),
                        conversion: conversion.map(token_f_string_conversion_to_ast),
                        format_spec,
                    });
                }
            }
        }

        Ok(parsed)
    }

    fn parse_t_string_token_parts(
        &self,
        parts: &[TokenFStringPart],
    ) -> Result<Vec<TemplateStringPart>, String> {
        let mut parsed = Vec::new();

        for part in parts {
            match part {
                TokenFStringPart::Literal(value) => {
                    parsed.push(TemplateStringPart::Literal(value.clone()))
                }
                TokenFStringPart::Expression {
                    source,
                    conversion,
                    format_spec,
                    debug_label,
                } => {
                    if format_spec.is_some() && starts_unparenthesized_lambda_expression(source) {
                        return Err(
                            "t-string: lambda expressions are not allowed without parentheses"
                                .to_string(),
                        );
                    }
                    if let Some(debug_label) = debug_label {
                        parsed.push(TemplateStringPart::Literal(debug_label.clone()));
                    }
                    let tokens = lex_interpolated_expression_source(source, "t-string")?;
                    let value = parse_eval(&tokens)
                        .map_err(|error| format!("t-string expression parse error: {error}"))?;
                    let format_spec = format_spec
                        .as_deref()
                        .map(|parts| self.parse_f_string_token_parts(parts))
                        .transpose()?;
                    let expression = if debug_label.is_some() {
                        source.trim().to_string()
                    } else {
                        source.clone()
                    };
                    parsed.push(TemplateStringPart::Interpolation {
                        value: Box::new(value),
                        expression,
                        conversion: conversion.map(token_f_string_conversion_to_ast),
                        format_spec,
                    });
                }
            }
        }

        Ok(parsed)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance().cloned() {
            Some(Token::Number(value)) => Ok(Expr::Number(value)),
            Some(Token::BigInt(value)) => Ok(Expr::BigInt(value)),
            Some(Token::Float(value)) => Ok(Expr::Float(value)),
            Some(Token::Imaginary(value)) => Ok(Expr::Imaginary(value)),
            Some(Token::String(value)) => {
                let (parts, has_f_string) =
                    self.parse_adjacent_string_parts(vec![FStringPart::Literal(value)], false)?;
                Ok(if has_f_string {
                    joined_string_parts_to_joined_expr(parts)
                } else {
                    joined_string_parts_to_expr(parts)
                })
            }
            Some(Token::Bytes(value)) => {
                let value = self.parse_adjacent_bytes(value)?;
                Ok(Expr::Bytes(value))
            }
            Some(Token::FString(parts)) => {
                let parts = self.parse_f_string_token_parts(&parts)?;
                let (parts, _) = self.parse_adjacent_string_parts(parts, true)?;
                Ok(joined_string_parts_to_joined_expr(parts))
            }
            Some(Token::TString(parts)) => {
                let parts = self.parse_t_string_token_parts(&parts)?;
                let parts = self.parse_adjacent_t_string_parts(parts)?;
                Ok(Expr::TemplateString(parts))
            }
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::None) => Ok(Expr::None),
            Some(Token::Ellipsis) => Ok(Expr::Ellipsis),
            Some(Token::Identifier(name)) => Ok(Expr::Name(name)),
            Some(Token::LeftParen) => {
                if matches!(self.peek(), Some(Token::RightParen)) {
                    self.advance();
                    return Ok(Expr::Tuple(Vec::new()));
                }

                if matches!(self.peek(), Some(Token::DoubleStar)) {
                    self.advance();
                    self.parse_bitwise_or()?;
                    if self.starts_comprehension_clause() {
                        return Err("cannot use dict unpacking in generator expression".to_string());
                    }
                    if matches!(self.peek(), Some(Token::Comma)) {
                        return Err("cannot use dict unpacking here".to_string());
                    }
                    self.expect_right_paren()?;
                    return Err("cannot use double starred expression here".to_string());
                }

                let expr = self.parse_star_named_expression()?;
                if self.starts_comprehension_clause() {
                    let clauses = self.parse_comprehension_clauses()?;
                    self.expect_right_paren()?;
                    validate_comprehension_named_expression_rebindings(
                        &[&expr],
                        &clauses,
                        self.class_body_depth > 0,
                    )?;
                    return Ok(Expr::GeneratorComp {
                        element: Box::new(expr),
                        clauses,
                    });
                }

                if matches!(self.peek(), Some(Token::Comma)) {
                    let elements = self.parse_tuple_tail(expr)?;
                    self.expect_right_paren()?;
                    return Ok(Expr::Tuple(elements));
                }

                self.expect_right_paren()?;
                if matches!(expr, Expr::Starred(_)) {
                    return Err("cannot use starred expression here".to_string());
                }
                Ok(expr)
            }
            Some(Token::LeftBracket) => {
                let expr = self.parse_list_expression()?;
                self.expect_right_bracket()?;
                Ok(expr)
            }
            Some(Token::LeftBrace) => {
                let expr = self.parse_brace_expression()?;
                self.expect_right_brace()?;
                Ok(expr)
            }
            Some(token) => Err(format!("expected expression, found {token:?}")),
            None => Err("expected expression, found end of input".to_string()),
        }
    }

    fn parse_star_named_expression(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Star)) {
            return self.parse_starred_bitwise_expression();
        }

        if matches!(self.peek(), Some(Token::DoubleStar)) {
            self.advance();
            self.parse_bitwise_or()?;
            return Err("cannot use dict unpacking here".to_string());
        }

        self.parse_named_expression()
    }

    fn parse_star_expression(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::Star)) {
            return self.parse_starred_bitwise_expression();
        }

        if matches!(self.peek(), Some(Token::DoubleStar)) {
            self.advance();
            self.parse_bitwise_or()?;
            return Err("cannot use dict unpacking here".to_string());
        }

        self.parse_expression()
    }

    fn parse_starred_bitwise_expression(&mut self) -> Result<Expr, String> {
        self.advance();
        if is_invalid_star_expression_end(self.peek()) {
            return Err("Invalid star expression".to_string());
        }

        let value = self.parse_bitwise_or()?;
        if matches!(self.peek(), Some(Token::If)) {
            return Err(
                "invalid starred expression. Did you forget to wrap the conditional expression in parentheses?"
                    .to_string(),
            );
        }
        if matches!(self.peek(), Some(Token::Equal)) {
            return Err("cannot assign to iterable argument unpacking".to_string());
        }

        Ok(Expr::Starred(Box::new(value)))
    }

    fn parse_tuple_tail(&mut self, first: Expr) -> Result<Vec<Expr>, String> {
        let mut elements = vec![first];

        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::RightParen)) {
                break;
            }

            elements.push(self.parse_star_named_expression()?);
        }

        Ok(elements)
    }

    fn parse_brace_expression(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::RightBrace)) {
            return Ok(Expr::Dict(Vec::new()));
        }

        if matches!(self.peek(), Some(Token::DoubleStar)) {
            let first = self.parse_dict_unpack_item()?;
            if self.starts_comprehension_clause() {
                let DictItem::Unpack(value) = first else {
                    unreachable!("dict unpack item is always an unpack entry");
                };
                let clauses = self.parse_comprehension_clauses()?;
                validate_comprehension_named_expression_rebindings(
                    &[&value],
                    &clauses,
                    self.class_body_depth > 0,
                )?;
                return Ok(Expr::DictUnpackComp {
                    value: Box::new(value),
                    clauses,
                });
            }
            let entries = self.parse_dict_tail(first)?;
            return Ok(Expr::Dict(entries));
        }

        let first = self.parse_star_named_expression()?;

        if !matches!(self.peek(), Some(Token::Colon)) {
            if self.starts_comprehension_clause() {
                let clauses = self.parse_comprehension_clauses()?;
                validate_comprehension_named_expression_rebindings(
                    &[&first],
                    &clauses,
                    self.class_body_depth > 0,
                )?;
                return Ok(Expr::SetComp {
                    element: Box::new(first),
                    clauses,
                });
            }

            let elements = self.parse_set_tail(first)?;
            return Ok(Expr::Set(elements));
        }

        self.expect_dict_key_colon()?;
        self.reject_invalid_dict_key(&first)?;
        let value = self.parse_dict_value_expression()?;
        let first = DictItem::Entry { key: first, value };

        if self.starts_comprehension_clause() {
            let clauses = self.parse_comprehension_clauses()?;
            let DictItem::Entry { key, value } = first else {
                unreachable!("dict comprehension first item is always a key/value entry");
            };
            validate_comprehension_named_expression_rebindings(
                &[&key, &value],
                &clauses,
                self.class_body_depth > 0,
            )?;
            return Ok(Expr::DictComp {
                key: Box::new(key),
                value: Box::new(value),
                clauses,
            });
        }

        let entries = self.parse_dict_tail(first)?;
        Ok(Expr::Dict(entries))
    }

    fn parse_dict_unpack_item(&mut self) -> Result<DictItem, String> {
        self.expect_double_star()?;
        let value = self.parse_bitwise_or()?;
        if matches!(self.peek(), Some(Token::If)) {
            return Err(
                "invalid double starred expression. Did you forget to wrap the conditional expression in parentheses?"
                    .to_string(),
            );
        }
        if matches!(self.peek(), Some(Token::Colon)) {
            return Err("cannot use dict unpacking in a dictionary key".to_string());
        }
        Ok(DictItem::Unpack(value))
    }

    fn reject_invalid_dict_key(&self, key: &Expr) -> Result<(), String> {
        if matches!(key, Expr::Starred(_)) {
            return Err("cannot use a starred expression in a dictionary key".to_string());
        }
        Ok(())
    }

    fn expect_dict_key_colon(&mut self) -> Result<(), String> {
        if !matches!(self.peek(), Some(Token::Colon)) {
            return Err("':' expected after dictionary key".to_string());
        }
        self.expect_colon()
    }

    fn parse_dict_value_expression(&mut self) -> Result<Expr, String> {
        match self.peek() {
            Some(Token::RightBrace) | Some(Token::Comma) => {
                Err("expression expected after dictionary key and ':'".to_string())
            }
            Some(Token::Star) => {
                Err("cannot use a starred expression in a dictionary value".to_string())
            }
            Some(Token::DoubleStar) => {
                Err("cannot use dict unpacking in a dictionary value".to_string())
            }
            _ => self.parse_expression(),
        }
    }

    fn parse_set_tail(&mut self, first: Expr) -> Result<Vec<Expr>, String> {
        let mut elements = vec![first];

        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::RightBrace)) {
                break;
            }
            if self.starts_comprehension_clause() {
                return Err(
                    "did you forget parentheses around the comprehension target?".to_string(),
                );
            }

            elements.push(self.parse_star_named_expression()?);
            if self.starts_comprehension_clause() {
                return Err(
                    "did you forget parentheses around the comprehension target?".to_string(),
                );
            }
        }

        Ok(elements)
    }

    fn parse_dict_tail(&mut self, first: DictItem) -> Result<Vec<DictItem>, String> {
        let mut entries = vec![first];

        loop {
            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
            if matches!(self.peek(), Some(Token::RightBrace)) {
                break;
            }

            if matches!(self.peek(), Some(Token::DoubleStar)) {
                entries.push(self.parse_dict_unpack_item()?);
                continue;
            }

            if matches!(self.peek(), Some(Token::Star)) {
                return Err("cannot use a starred expression in a dictionary key".to_string());
            }

            let key = self.parse_expression()?;
            self.expect_dict_key_colon()?;
            let value = self.parse_dict_value_expression()?;
            entries.push(DictItem::Entry { key, value });
        }

        Ok(entries)
    }

    fn parse_list_expression(&mut self) -> Result<Expr, String> {
        if matches!(self.peek(), Some(Token::RightBracket)) {
            return Ok(Expr::List(Vec::new()));
        }

        if matches!(self.peek(), Some(Token::DoubleStar)) {
            self.advance();
            self.parse_bitwise_or()?;
            if self.starts_comprehension_clause() {
                return Err("cannot use dict unpacking in list comprehension".to_string());
            }
            return Err("cannot use dict unpacking here".to_string());
        }

        let first = self.parse_star_named_expression()?;
        if self.starts_comprehension_clause() {
            let clauses = self.parse_comprehension_clauses()?;
            validate_comprehension_named_expression_rebindings(
                &[&first],
                &clauses,
                self.class_body_depth > 0,
            )?;
            return Ok(Expr::ListComp {
                element: Box::new(first),
                clauses,
            });
        }

        let elements = self.parse_list_tail(first)?;
        Ok(Expr::List(elements))
    }

    fn parse_list_tail(&mut self, first: Expr) -> Result<Vec<Expr>, String> {
        let mut elements = vec![first];

        while matches!(self.peek(), Some(Token::Comma)) {
            self.advance();
            if matches!(self.peek(), Some(Token::RightBracket)) {
                break;
            }
            if self.starts_comprehension_clause() {
                return Err(
                    "did you forget parentheses around the comprehension target?".to_string(),
                );
            }

            elements.push(self.parse_star_named_expression()?);
            if self.starts_comprehension_clause() {
                return Err(
                    "did you forget parentheses around the comprehension target?".to_string(),
                );
            }
        }

        Ok(elements)
    }

    fn parse_comprehension_clauses(&mut self) -> Result<Vec<ComprehensionClause>, String> {
        let mut clauses = Vec::new();

        while self.starts_comprehension_clause() {
            let is_async = if matches!(self.peek(), Some(Token::Async)) {
                self.advance();
                true
            } else {
                false
            };
            self.advance();
            if !self.has_top_level_in_before_comprehension_clause_end(self.current) {
                return Err("'in' expected after for-loop variables".to_string());
            }
            let target = self.parse_assignment_target()?;
            validate_store_target(&target)?;
            self.expect_in()?;
            let iter = self.parse_comprehension_iter_expression()?;
            let mut ifs = Vec::new();

            while matches!(self.peek(), Some(Token::If)) {
                self.advance();
                ifs.push(self.parse_or()?);
            }

            clauses.push(ComprehensionClause {
                is_async,
                target,
                iter,
                ifs,
            });
        }

        if clauses.is_empty() {
            Err("expected comprehension 'for' clause".to_string())
        } else {
            Ok(clauses)
        }
    }

    fn parse_comprehension_iter_expression(&mut self) -> Result<Expr, String> {
        self.comprehension_iter_depth += 1;
        let iter = self.parse_or();
        self.comprehension_iter_depth -= 1;
        let iter = iter?;

        if expr_contains_named_expression(&iter) {
            return Err(
                "assignment expression cannot be used in a comprehension iterable expression"
                    .to_string(),
            );
        }

        Ok(iter)
    }

    fn starts_comprehension_clause(&self) -> bool {
        matches!(
            (self.peek(), self.peek_next()),
            (Some(Token::For), _) | (Some(Token::Async), Some(Token::For))
        )
    }

    fn starts_match_statement(&self) -> bool {
        if !matches!(self.peek(), Some(Token::Identifier(name)) if name == "match") {
            return false;
        }

        self.has_top_level_colon_before_boundary(self.current + 1)
    }

    fn starts_case_block(&self) -> bool {
        matches!(self.peek(), Some(Token::Identifier(name)) if name == "case")
    }

    fn starts_type_alias_statement(&self) -> bool {
        matches!(self.peek(), Some(Token::Identifier(name)) if name == "type")
            && matches!(self.peek_next(), Some(Token::Identifier(_)))
    }

    fn has_top_level_colon_before_boundary(&self, start: usize) -> bool {
        let mut depth = 0usize;

        for token in self.tokens.iter().skip(start) {
            match token {
                Token::Newline | Token::Semicolon | Token::Dedent | Token::Eof if depth == 0 => {
                    return false;
                }
                Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen | Token::RightBracket | Token::RightBrace if depth > 0 => {
                    depth -= 1;
                }
                Token::Colon if depth == 0 => return true,
                _ => {}
            }
        }

        false
    }

    fn has_top_level_in_before_comprehension_clause_end(&self, start: usize) -> bool {
        let mut depth = 0usize;

        for token in self.tokens.iter().skip(start) {
            match token {
                Token::In if depth == 0 => return true,
                Token::If
                | Token::For
                | Token::RightParen
                | Token::RightBracket
                | Token::RightBrace
                | Token::Semicolon
                | Token::Newline
                | Token::Dedent
                | Token::Eof
                    if depth == 0 =>
                {
                    return false;
                }
                Token::LeftParen | Token::LeftBracket | Token::LeftBrace => depth += 1,
                Token::RightParen | Token::RightBracket | Token::RightBrace if depth > 0 => {
                    depth -= 1;
                }
                _ => {}
            }
        }

        false
    }

    fn expect_if(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::If) => Ok(()),
            Some(token) => Err(format!("expected 'if', found {token:?}")),
            None => Err("expected 'if', found end of input".to_string()),
        }
    }

    fn expect_elif(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Elif) => Ok(()),
            Some(token) => Err(format!("expected 'elif', found {token:?}")),
            None => Err("expected 'elif', found end of input".to_string()),
        }
    }

    fn expect_while(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::While) => Ok(()),
            Some(token) => Err(format!("expected 'while', found {token:?}")),
            None => Err("expected 'while', found end of input".to_string()),
        }
    }

    fn expect_for(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::For) => Ok(()),
            Some(token) => Err(format!("expected 'for', found {token:?}")),
            None => Err("expected 'for', found end of input".to_string()),
        }
    }

    fn expect_try(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Try) => Ok(()),
            Some(token) => Err(format!("expected 'try', found {token:?}")),
            None => Err("expected 'try', found end of input".to_string()),
        }
    }

    fn expect_except(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Except) => Ok(()),
            Some(token) => Err(format!("expected 'except', found {token:?}")),
            None => Err("expected 'except', found end of input".to_string()),
        }
    }

    fn expect_with(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::With) => Ok(()),
            Some(token) => Err(format!("expected 'with', found {token:?}")),
            None => Err("expected 'with', found end of input".to_string()),
        }
    }

    fn expect_import(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Import) => Ok(()),
            Some(token) => Err(format!("expected 'import', found {token:?}")),
            None => Err("expected 'import', found end of input".to_string()),
        }
    }

    fn expect_yield(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Yield) => Ok(()),
            Some(token) => Err(format!("expected 'yield', found {token:?}")),
            None => Err("expected 'yield', found end of input".to_string()),
        }
    }

    fn expect_async(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Async) => Ok(()),
            Some(token) => Err(format!("expected 'async', found {token:?}")),
            None => Err("expected 'async', found end of input".to_string()),
        }
    }

    fn expect_from(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::From) => Ok(()),
            Some(token) => Err(format!("expected 'from', found {token:?}")),
            None => Err("expected 'from', found end of input".to_string()),
        }
    }

    fn expect_def(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Def) => Ok(()),
            Some(token) => Err(format!("expected 'def', found {token:?}")),
            None => Err("expected 'def', found end of input".to_string()),
        }
    }

    fn expect_class(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Class) => Ok(()),
            Some(token) => Err(format!("expected 'class', found {token:?}")),
            None => Err("expected 'class', found end of input".to_string()),
        }
    }

    fn expect_lambda(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Lambda) => Ok(()),
            Some(token) => Err(format!("expected 'lambda', found {token:?}")),
            None => Err("expected 'lambda', found end of input".to_string()),
        }
    }

    fn expect_return(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Return) => Ok(()),
            Some(token) => Err(format!("expected 'return', found {token:?}")),
            None => Err("expected 'return', found end of input".to_string()),
        }
    }

    fn expect_raise(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Raise) => Ok(()),
            Some(token) => Err(format!("expected 'raise', found {token:?}")),
            None => Err("expected 'raise', found end of input".to_string()),
        }
    }

    fn expect_del(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Del) => Ok(()),
            Some(token) => Err(format!("expected 'del', found {token:?}")),
            None => Err("expected 'del', found end of input".to_string()),
        }
    }

    fn expect_global(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Global) => Ok(()),
            Some(token) => Err(format!("expected 'global', found {token:?}")),
            None => Err("expected 'global', found end of input".to_string()),
        }
    }

    fn expect_nonlocal(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Nonlocal) => Ok(()),
            Some(token) => Err(format!("expected 'nonlocal', found {token:?}")),
            None => Err("expected 'nonlocal', found end of input".to_string()),
        }
    }

    fn expect_assert(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Assert) => Ok(()),
            Some(token) => Err(format!("expected 'assert', found {token:?}")),
            None => Err("expected 'assert', found end of input".to_string()),
        }
    }

    fn expect_identifier(&mut self, expected: &str) -> Result<String, String> {
        match self.advance() {
            Some(Token::Identifier(name)) => Ok(name.clone()),
            Some(token) => Err(format!("expected {expected}, found {token:?}")),
            None => Err(format!("expected {expected}, found end of input")),
        }
    }

    fn expect_soft_keyword(&mut self, keyword: &str) -> Result<(), String> {
        match self.advance() {
            Some(Token::Identifier(name)) if name == keyword => Ok(()),
            Some(token) => Err(format!("expected '{keyword}', found {token:?}")),
            None => Err(format!("expected '{keyword}', found end of input")),
        }
    }

    fn expect_in(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::In) => Ok(()),
            Some(token) => Err(format!("expected 'in', found {token:?}")),
            None => Err("expected 'in', found end of input".to_string()),
        }
    }

    fn expect_colon(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Colon) => Ok(()),
            Some(token) => Err(format!("expected ':', found {token:?}")),
            None => Err("expected ':', found end of input".to_string()),
        }
    }

    fn expect_colon_equal(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::ColonEqual) => Ok(()),
            Some(token) => Err(format!("expected ':=', found {token:?}")),
            None => Err("expected ':=', found end of input".to_string()),
        }
    }

    fn expect_equal(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Equal) => Ok(()),
            Some(token) => Err(format!("expected '=', found {token:?}")),
            None => Err("expected '=', found end of input".to_string()),
        }
    }

    fn expect_double_star(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::DoubleStar) => Ok(()),
            Some(token) => Err(format!("expected '**', found {token:?}")),
            None => Err("expected '**', found end of input".to_string()),
        }
    }

    fn expect_newline(&mut self) -> Result<(), String> {
        self.skip_type_comment_tokens();

        match self.advance() {
            Some(Token::Newline) => Ok(()),
            Some(token) => Err(format!("expected newline, found {token:?}")),
            None => Err("expected newline, found end of input".to_string()),
        }
    }

    fn expect_left_paren(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::LeftParen) => Ok(()),
            Some(token) => Err(format!("expected '(', found {token:?}")),
            None => Err("expected '(', found end of input".to_string()),
        }
    }

    fn expect_left_bracket(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::LeftBracket) => Ok(()),
            Some(token) => Err(format!("expected '[', found {token:?}")),
            None => Err("expected '[', found end of input".to_string()),
        }
    }

    fn expect_indent(&mut self) -> Result<(), String> {
        match self.peek() {
            Some(Token::Indent) => {
                self.advance();
                Ok(())
            }
            Some(_) | None => Err("expected an indented block".to_string()),
        }
    }

    fn expect_dedent(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Dedent) => Ok(()),
            Some(token) => Err(format!("expected dedent, found {token:?}")),
            None => Err("expected dedent, found end of input".to_string()),
        }
    }

    fn expect_right_paren(&mut self) -> Result<(), String> {
        if self.starts_suppressed_assignment_statement() || !self.remaining_contains_right_paren() {
            return Err(unclosed_delimiter_message('('));
        }

        match self.advance() {
            Some(Token::RightParen) => Ok(()),
            Some(Token::Eof) | None => Err(unclosed_delimiter_message('(')),
            Some(token) => Err(format!("expected ')', found {token:?}")),
        }
    }

    fn expect_arrow(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Arrow) => Ok(()),
            Some(token) => Err(format!("expected '->', found {token:?}")),
            None => Err("expected '->', found end of input".to_string()),
        }
    }

    fn expect_right_bracket(&mut self) -> Result<(), String> {
        if self.starts_suppressed_assignment_statement() || !self.remaining_contains_right_bracket()
        {
            return Err(unclosed_delimiter_message('['));
        }

        match self.advance() {
            Some(Token::RightBracket) => Ok(()),
            Some(Token::Eof) | None => Err(unclosed_delimiter_message('[')),
            Some(token) => Err(format!("expected ']', found {token:?}")),
        }
    }

    fn expect_right_brace(&mut self) -> Result<(), String> {
        if self.starts_suppressed_assignment_statement() || !self.remaining_contains_right_brace() {
            return Err(unclosed_delimiter_message('{'));
        }

        match self.advance() {
            Some(Token::RightBrace) => Ok(()),
            Some(Token::Eof) | None => Err(unclosed_delimiter_message('{')),
            Some(token) => Err(format!("expected '}}', found {token:?}")),
        }
    }

    fn expect_eof(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Eof) => Ok(()),
            Some(Token::LeftBrace) => Err("invalid syntax".to_string()),
            Some(token) => Err(format!("expected end of input, found {token:?}")),
            None => Err("expected end of input".to_string()),
        }
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.current);
        self.current += 1;
        token
    }

    fn previous(&self) -> Option<&Token> {
        self.current
            .checked_sub(1)
            .and_then(|index| self.tokens.get(index))
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.current + 1)
    }

    fn starts_suppressed_assignment_statement(&self) -> bool {
        matches!(
            (self.peek(), self.peek_next()),
            (Some(Token::Identifier(_)), Some(Token::Equal))
        )
    }

    fn remaining_contains_right_paren(&self) -> bool {
        self.tokens
            .get(self.current..)
            .unwrap_or(&[])
            .iter()
            .any(|token| matches!(token, Token::RightParen))
    }

    fn remaining_contains_right_bracket(&self) -> bool {
        self.tokens
            .get(self.current..)
            .unwrap_or(&[])
            .iter()
            .any(|token| matches!(token, Token::RightBracket))
    }

    fn remaining_contains_right_brace(&self) -> bool {
        self.tokens
            .get(self.current..)
            .unwrap_or(&[])
            .iter()
            .any(|token| matches!(token, Token::RightBrace))
    }

    fn former_statement_boundary_error(&self, stmt: &Stmt) -> Option<String> {
        let Stmt::Expr(Expr::Name(name)) = stmt else {
            return None;
        };
        if !matches!(name.as_str(), "print" | "exec") {
            return None;
        }
        if !former_statement_argument_starts(self.peek()) {
            return None;
        }
        if !self.remaining_expression_is_valid_statement_argument() {
            return Some("invalid syntax".to_string());
        }

        Some(format!(
            "Missing parentheses in call to '{name}'. Did you mean {name}(...)?"
        ))
    }

    fn remaining_expression_is_valid_statement_argument(&self) -> bool {
        let mut parser = Parser {
            tokens: self.tokens,
            current: self.current,
            comprehension_iter_depth: self.comprehension_iter_depth,
            class_body_depth: self.class_body_depth,
        };

        parser
            .parse_expression_list_until_statement_boundary()
            .is_ok()
            && former_statement_argument_boundary(parser.peek())
    }

    fn skip_newlines(&mut self) {
        loop {
            self.skip_type_comment_tokens();
            if matches!(self.peek(), Some(Token::Newline)) {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_statement_separators(&mut self) {
        while matches!(self.peek(), Some(Token::Semicolon)) {
            self.advance();
        }

        self.skip_newlines();
    }

    fn interactive_compound_statement_has_terminator(&self) -> bool {
        matches!(self.previous(), Some(Token::Newline | Token::Dedent))
    }

    fn skip_type_comment_tokens(&mut self) {
        while is_type_comment_token(self.peek()) {
            self.advance();
        }
    }

    fn take_type_comment(&mut self) -> Option<String> {
        let Some(Token::TypeComment(comment)) = self.peek().cloned() else {
            return None;
        };
        self.advance();
        Some(comment)
    }
}

fn token_f_string_conversion_to_ast(conversion: TokenFStringConversion) -> FStringConversion {
    match conversion {
        TokenFStringConversion::Str => FStringConversion::Str,
        TokenFStringConversion::Repr => FStringConversion::Repr,
        TokenFStringConversion::Ascii => FStringConversion::Ascii,
    }
}

fn parser_token_matches_found(token: &Token, found: &str) -> bool {
    format!("{token:?}") == found
}

fn unclosed_delimiter_message(open: char) -> String {
    format!("'{open}' was never closed")
}

fn lex_interpolated_expression_source(
    source: &str,
    kind_label: &str,
) -> Result<Vec<Token>, String> {
    let mut tokens = lex(source).map_err(|error| format!("{kind_label}: {error}"))?;
    tokens.retain(|token| {
        !matches!(
            token,
            Token::Newline
                | Token::Indent
                | Token::Dedent
                | Token::TypeComment(_)
                | Token::TypeIgnore(_)
        )
    });
    Ok(tokens)
}

fn starts_unparenthesized_lambda_expression(source: &str) -> bool {
    let Ok(tokens) = lex(source) else {
        return false;
    };
    let mut depth = 0usize;
    let mut can_start_item = true;

    for token in tokens {
        match token {
            Token::LeftParen | Token::LeftBracket | Token::LeftBrace => {
                if depth == 0 {
                    can_start_item = false;
                }
                depth += 1;
            }
            Token::RightParen | Token::RightBracket | Token::RightBrace => {
                depth = depth.saturating_sub(1);
            }
            Token::Comma if depth == 0 => {
                can_start_item = true;
            }
            Token::Lambda if depth == 0 && can_start_item => {
                return true;
            }
            Token::Newline | Token::TypeComment(_) | Token::TypeIgnore(_) => {}
            Token::Eof => break,
            _ if depth == 0 => {
                can_start_item = false;
            }
            _ => {}
        }
    }

    false
}

fn joined_string_parts_to_expr(parts: Vec<FStringPart>) -> Expr {
    let normalized = normalize_f_string_parts(parts);

    if normalized
        .iter()
        .all(|part| matches!(part, FStringPart::Literal(_)))
    {
        let value = normalized
            .into_iter()
            .filter_map(|part| match part {
                FStringPart::Literal(value) => Some(value),
                FStringPart::Formatted { .. } => None,
            })
            .collect::<String>();
        Expr::String(value)
    } else {
        Expr::JoinedString(normalized)
    }
}

fn joined_string_parts_to_joined_expr(parts: Vec<FStringPart>) -> Expr {
    Expr::JoinedString(normalize_f_string_parts(parts))
}

fn normalize_f_string_parts(parts: Vec<FStringPart>) -> Vec<FStringPart> {
    let mut normalized = Vec::new();

    for part in parts {
        match part {
            FStringPart::Literal(value) if value.is_empty() => {}
            FStringPart::Literal(value) => {
                if let Some(FStringPart::Literal(previous)) = normalized.last_mut() {
                    previous.push_str(&value);
                } else {
                    normalized.push(FStringPart::Literal(value));
                }
            }
            FStringPart::Formatted { .. } => normalized.push(part),
        }
    }

    normalized
}

fn statement_has_own_boundary(statement: &Stmt) -> bool {
    matches!(
        statement,
        Stmt::If { .. }
            | Stmt::While { .. }
            | Stmt::For { .. }
            | Stmt::AsyncFor { .. }
            | Stmt::AsyncWith { .. }
            | Stmt::TryStar { .. }
            | Stmt::Try { .. }
            | Stmt::With { .. }
            | Stmt::Match { .. }
            | Stmt::FunctionDef { .. }
            | Stmt::AsyncFunctionDef { .. }
            | Stmt::ClassDef { .. }
    )
}

fn is_literal_pattern_start(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Number(_)
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
                | Token::Minus
        )
    )
}

fn is_literal_pattern_target_token(token: &Token) -> bool {
    matches!(
        token,
        Token::Number(_)
            | Token::BigInt(_)
            | Token::Float(_)
            | Token::Imaginary(_)
            | Token::String(_)
            | Token::Bytes(_)
    )
}

fn is_supported_literal_pattern(expr: &Expr) -> bool {
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
                && is_signed_real_literal_pattern_expr(left)
                && matches!(right.as_ref(), Expr::Imaginary(_))
        }
        _ => false,
    }
}

fn is_singleton_literal_pattern(expr: &Expr) -> bool {
    matches!(expr, Expr::Bool(_) | Expr::None)
}

fn is_signed_real_literal_pattern_expr(expr: &Expr) -> bool {
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

fn is_irrefutable_match_case(case: &MatchCase) -> bool {
    case.guard.is_none() && pattern_is_irrefutable(&case.pattern)
}

fn except_type_expr_arity(expr: &Expr) -> usize {
    match expr {
        Expr::Tuple(elements) => elements.len(),
        _ => 1,
    }
}

fn pattern_is_irrefutable(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Capture(_) | Pattern::Wildcard => true,
        Pattern::Or(alternatives) => alternatives.iter().any(pattern_is_irrefutable),
        Pattern::As { pattern, .. } => pattern_is_irrefutable(pattern),
        Pattern::Literal(_)
        | Pattern::Singleton(_)
        | Pattern::Value(_)
        | Pattern::Sequence(_)
        | Pattern::Class { .. }
        | Pattern::Mapping { .. }
        | Pattern::Star(_) => false,
    }
}

fn irrefutable_or_pattern_unreachable_message(pattern: &Pattern) -> Option<String> {
    match pattern {
        Pattern::Capture(name) => Some(format!(
            "name capture '{name}' makes remaining patterns unreachable"
        )),
        Pattern::Wildcard => Some("wildcard makes remaining patterns unreachable".to_string()),
        Pattern::As { pattern, .. } => irrefutable_or_pattern_unreachable_message(pattern),
        Pattern::Or(alternatives) => alternatives
            .iter()
            .find_map(irrefutable_or_pattern_unreachable_message),
        _ => None,
    }
}

fn ensure_at_most_one_star_pattern(patterns: &[Pattern]) -> Result<(), String> {
    if patterns
        .iter()
        .filter(|pattern| matches!(pattern, Pattern::Star(_)))
        .count()
        > 1
    {
        return Err("multiple starred names in sequence pattern".to_string());
    }

    Ok(())
}

fn ensure_unique_pattern_captures(pattern: &Pattern) -> Result<(), String> {
    let mut names = HashSet::new();
    collect_pattern_captures(pattern, &mut names)?;
    Ok(())
}

fn collect_pattern_captures(pattern: &Pattern, names: &mut HashSet<String>) -> Result<(), String> {
    match pattern {
        Pattern::Capture(name) | Pattern::Star(Some(name)) => insert_pattern_capture(name, names),
        Pattern::As { pattern, name } => {
            collect_pattern_captures(pattern, names)?;
            insert_pattern_capture(name, names)
        }
        Pattern::Or(alternatives) => {
            let captures = ensure_or_pattern_capture_compatibility(alternatives)?;
            for name in captures {
                insert_pattern_capture(&name, names)?;
            }
            Ok(())
        }
        Pattern::Sequence(alternatives) => {
            for pattern in alternatives {
                collect_pattern_captures(pattern, names)?;
            }
            Ok(())
        }
        Pattern::Mapping { entries, rest } => {
            for (_, pattern) in entries {
                collect_pattern_captures(pattern, names)?;
            }
            if let Some(name) = rest {
                insert_pattern_capture(name, names)?;
            }
            Ok(())
        }
        Pattern::Class {
            positional,
            keywords,
            ..
        } => {
            for pattern in positional {
                collect_pattern_captures(pattern, names)?;
            }
            for (_, pattern) in keywords {
                collect_pattern_captures(pattern, names)?;
            }
            Ok(())
        }
        Pattern::Literal(_)
        | Pattern::Singleton(_)
        | Pattern::Value(_)
        | Pattern::Wildcard
        | Pattern::Star(None) => Ok(()),
    }
}

fn ensure_or_pattern_capture_compatibility(
    patterns: &[Pattern],
) -> Result<HashSet<String>, String> {
    let mut expected = None;

    for pattern in patterns {
        let mut names = HashSet::new();
        collect_pattern_captures(pattern, &mut names)?;
        match &expected {
            Some(expected) if *expected != names => {
                return Err("alternative patterns bind different names".to_string());
            }
            Some(_) => {}
            None => expected = Some(names),
        }
    }

    Ok(expected.unwrap_or_default())
}

fn insert_pattern_capture(name: &str, names: &mut HashSet<String>) -> Result<(), String> {
    if !names.insert(name.to_string()) {
        return Err(format!("multiple assignments to name '{name}' in pattern"));
    }

    Ok(())
}

fn ensure_unique_mapping_literal_keys(entries: &[(Expr, Pattern)]) -> Result<(), String> {
    for (index, (key, _)) in entries.iter().enumerate() {
        let Some(key) = static_mapping_literal_key(key)? else {
            continue;
        };

        if let Some(duplicate) = entries[index + 1..]
            .iter()
            .map(|(other, _)| static_mapping_literal_key(other))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .find(|other| other == &key)
        {
            return Err(format!(
                "mapping pattern checks duplicate key ({})",
                duplicate.display
            ));
        }
    }

    Ok(())
}

#[derive(Debug, Clone)]
struct StaticMappingLiteralKey {
    value: StaticMappingLiteralKeyValue,
    display: String,
}

impl StaticMappingLiteralKey {
    fn new(value: StaticMappingLiteralKeyValue, display: String) -> Self {
        Self { value, display }
    }
}

impl PartialEq for StaticMappingLiteralKey {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

#[derive(Debug, Clone, PartialEq)]
enum StaticMappingLiteralKeyValue {
    None,
    String(String),
    Bytes(Vec<u8>),
    Number { real: f64, imag: f64 },
}

fn static_mapping_literal_key(expr: &Expr) -> Result<Option<StaticMappingLiteralKey>, String> {
    match expr {
        Expr::String(value) => Ok(Some(StaticMappingLiteralKey::new(
            StaticMappingLiteralKeyValue::String(value.clone()),
            repr_pattern_string(value),
        ))),
        Expr::Bytes(value) => Ok(Some(StaticMappingLiteralKey::new(
            StaticMappingLiteralKeyValue::Bytes(value.clone()),
            repr_pattern_bytes(value),
        ))),
        Expr::None => Ok(Some(StaticMappingLiteralKey::new(
            StaticMappingLiteralKeyValue::None,
            "None".to_string(),
        ))),
        Expr::Bool(value) => Ok(Some(StaticMappingLiteralKey::new(
            StaticMappingLiteralKeyValue::Number {
                real: if *value { 1.0 } else { 0.0 },
                imag: 0.0,
            },
            (if *value { "True" } else { "False" }).to_string(),
        ))),
        Expr::Number(value) => Ok(Some(StaticMappingLiteralKey::new(
            StaticMappingLiteralKeyValue::Number {
                real: *value as f64,
                imag: 0.0,
            },
            value.to_string(),
        ))),
        Expr::BigInt(value) => Ok(Some(StaticMappingLiteralKey::new(
            StaticMappingLiteralKeyValue::Number {
                real: parse_pattern_big_int_literal(value)?,
                imag: 0.0,
            },
            value.replace('_', ""),
        ))),
        Expr::Float(value) => {
            let real = parse_pattern_float_literal(value)?;
            Ok(Some(StaticMappingLiteralKey::new(
                StaticMappingLiteralKeyValue::Number { real, imag: 0.0 },
                repr_pattern_float(real),
            )))
        }
        Expr::Imaginary(value) => {
            let imag = parse_pattern_float_literal(value)?;
            Ok(Some(StaticMappingLiteralKey::new(
                StaticMappingLiteralKeyValue::Number { real: 0.0, imag },
                repr_pattern_imaginary(imag, value),
            )))
        }
        Expr::Unary {
            op: UnaryOp::Negative,
            operand,
        } => match operand.as_ref() {
            Expr::Number(value) => {
                let real = -(*value as f64);
                Ok(Some(StaticMappingLiteralKey::new(
                    StaticMappingLiteralKeyValue::Number { real, imag: 0.0 },
                    (-*value).to_string(),
                )))
            }
            Expr::BigInt(value) => {
                let real = -parse_pattern_big_int_literal(value)?;
                Ok(Some(StaticMappingLiteralKey::new(
                    StaticMappingLiteralKeyValue::Number { real, imag: 0.0 },
                    format!("-{}", value.replace('_', "")).replace("-0", "0"),
                )))
            }
            Expr::Float(value) => {
                let real = -parse_pattern_float_literal(value)?;
                Ok(Some(StaticMappingLiteralKey::new(
                    StaticMappingLiteralKeyValue::Number { real, imag: 0.0 },
                    repr_pattern_float(real),
                )))
            }
            Expr::Imaginary(value) => {
                let imag = -parse_pattern_float_literal(value)?;
                Ok(Some(StaticMappingLiteralKey::new(
                    StaticMappingLiteralKeyValue::Number { real: 0.0, imag },
                    repr_pattern_imaginary(imag, value),
                )))
            }
            _ => Ok(None),
        },
        Expr::Binary { left, op, right } if matches!(op, BinaryOp::Add | BinaryOp::Subtract) => {
            let Some(real) = static_mapping_real_literal_key(left)? else {
                return Ok(None);
            };
            let Expr::Imaginary(imaginary) = right.as_ref() else {
                return Ok(None);
            };
            let imaginary = parse_pattern_float_literal(imaginary)?;
            let imag = if matches!(op, BinaryOp::Subtract) {
                -imaginary
            } else {
                imaginary
            };
            Ok(Some(StaticMappingLiteralKey::new(
                StaticMappingLiteralKeyValue::Number { real, imag },
                repr_pattern_complex(real, imag),
            )))
        }
        Expr::Attribute { .. } => Ok(None),
        _ => Ok(None),
    }
}

fn static_mapping_real_literal_key(expr: &Expr) -> Result<Option<f64>, String> {
    match expr {
        Expr::Number(value) => Ok(Some(*value as f64)),
        Expr::BigInt(value) => Ok(Some(parse_pattern_big_int_literal(value)?)),
        Expr::Float(value) => Ok(Some(parse_pattern_float_literal(value)?)),
        Expr::Unary {
            op: UnaryOp::Negative,
            operand,
        } => match operand.as_ref() {
            Expr::Number(value) => Ok(Some(-(*value as f64))),
            Expr::BigInt(value) => Ok(Some(-parse_pattern_big_int_literal(value)?)),
            Expr::Float(value) => Ok(Some(-parse_pattern_float_literal(value)?)),
            _ => Ok(None),
        },
        _ => Ok(None),
    }
}

fn parse_pattern_float_literal(value: &str) -> Result<f64, String> {
    value
        .replace('_', "")
        .parse::<f64>()
        .map_err(|_| format!("invalid float literal: {value}"))
}

fn parse_pattern_big_int_literal(value: &str) -> Result<f64, String> {
    value
        .parse::<f64>()
        .map_err(|_| format!("invalid int literal: {value}"))
}

fn repr_pattern_string(value: &str) -> String {
    format!("'{}'", value.replace('\\', "\\\\").replace('\'', "\\'"))
}

fn repr_pattern_bytes(value: &[u8]) -> String {
    let mut result = String::from("b'");
    for byte in value {
        match byte {
            b'\\' => result.push_str("\\\\"),
            b'\'' => result.push_str("\\'"),
            32..=126 => result.push(*byte as char),
            _ => result.push_str(&format!("\\x{byte:02x}")),
        }
    }
    result.push('\'');
    result
}

fn repr_pattern_float(value: f64) -> String {
    format!("{value:?}")
}

fn repr_pattern_imaginary(value: f64, source: &str) -> String {
    if value.is_sign_negative() {
        format!("-{}j", source.replace('_', ""))
    } else {
        format!("{}j", source.replace('_', ""))
    }
}

fn repr_pattern_complex(real: f64, imag: f64) -> String {
    let real = repr_pattern_float(real);
    let imag_abs = repr_pattern_float(imag.abs());
    if imag.is_sign_negative() {
        format!("({real}-{imag_abs}j)")
    } else {
        format!("({real}+{imag_abs}j)")
    }
}

fn token_matches(token: Option<&Token>, expected: &Token) -> bool {
    matches!(
        (token, expected),
        (Some(Token::RightBracket), Token::RightBracket)
            | (Some(Token::RightParen), Token::RightParen)
    )
}

fn is_yield_value_end(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Semicolon
                | Token::TypeComment(_)
                | Token::TypeIgnore(_)
                | Token::Newline
                | Token::Dedent
                | Token::Eof
                | Token::RightParen
                | Token::RightBracket
                | Token::RightBrace
        ) | None
    )
}

fn is_statement_boundary(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Semicolon
                | Token::TypeComment(_)
                | Token::TypeIgnore(_)
                | Token::Newline
                | Token::Dedent
                | Token::Eof
        ) | None
    )
}

fn is_import_alias_boundary(token: Option<&Token>, allow_right_paren: bool) -> bool {
    matches!(
        token,
        Some(
            Token::Comma
                | Token::Semicolon
                | Token::TypeComment(_)
                | Token::TypeIgnore(_)
                | Token::Newline
                | Token::Dedent
                | Token::Eof
        ) | None
    ) || (allow_right_paren && matches!(token, Some(Token::RightParen)))
}

fn is_parameter_default_end(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(Token::Comma | Token::RightParen | Token::Colon) | None
    )
}

fn is_assignment_target_syntax_error(error: &str) -> bool {
    matches!(
        error,
        "multiple starred expressions in assignment"
            | "starred assignment target must be in a list or tuple"
            | "cannot use starred expression here"
    )
}

fn former_statement_argument_starts(token: Option<&Token>) -> bool {
    matches!(
        token,
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
                | Token::Ellipsis
                | Token::LeftBracket
                | Token::LeftBrace
                | Token::Lambda
                | Token::Yield
                | Token::Await
                | Token::Plus
                | Token::Minus
                | Token::Tilde
                | Token::Not
                | Token::Star
        )
    )
}

fn former_statement_argument_boundary(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Semicolon
                | Token::TypeComment(_)
                | Token::TypeIgnore(_)
                | Token::Newline
                | Token::Dedent
                | Token::Eof
        ) | None
    )
}

fn is_aug_assign_operator(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
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
    )
}

fn invalid_import_target_name(token: &Token) -> Option<&'static str> {
    match token {
        Token::Number(_)
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
        | Token::Ellipsis => Some("literal"),
        _ => None,
    }
}

fn invalid_import_expression_target_name(expr: &Expr) -> &'static str {
    match expr {
        Expr::Bool(_)
        | Expr::None
        | Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::JoinedString(_)
        | Expr::TemplateString(_)
        | Expr::TemplateInterpolation { .. }
        | Expr::Ellipsis => "literal",
        _ => invalid_named_expression_target_name(expr),
    }
}

fn is_type_comment_token(token: Option<&Token>) -> bool {
    matches!(token, Some(Token::TypeComment(_) | Token::TypeIgnore(_)))
}

fn target_to_expr(target: Target) -> Option<Expr> {
    match target {
        Target::Name(name) => Some(Expr::Name(name)),
        Target::Attribute { object, name } => Some(Expr::Attribute { object, name }),
        Target::Subscript { object, index } => Some(Expr::Subscript {
            object,
            index: Box::new(index),
        }),
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => Some(Expr::Slice {
            object,
            start: start.map(Box::new),
            stop: stop.map(Box::new),
            step: step.map(Box::new),
        }),
        Target::Starred(_) | Target::Tuple(_) | Target::List(_) => None,
    }
}

fn expr_to_target(expr: Expr) -> Option<Target> {
    match expr {
        Expr::Name(name) => Some(Target::Name(name)),
        Expr::Attribute { object, name } => Some(Target::Attribute { object, name }),
        Expr::Subscript { object, index } => Some(Target::Subscript {
            object,
            index: *index,
        }),
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => Some(Target::Slice {
            object,
            start: start.map(|expr| *expr),
            stop: stop.map(|expr| *expr),
            step: step.map(|expr| *expr),
        }),
        _ => None,
    }
}

fn validate_binding_name(name: &str) -> Result<(), String> {
    if name == "__debug__" {
        Err("cannot assign to __debug__".to_string())
    } else {
        Ok(())
    }
}

fn validate_import_alias_binding(alias: &ImportAlias, is_from_import: bool) -> Result<(), String> {
    if let Some(asname) = &alias.asname {
        return validate_binding_name(asname);
    }

    let binding = if is_from_import {
        alias.name.as_str()
    } else {
        alias.name.split('.').next().unwrap_or(alias.name.as_str())
    };
    validate_binding_name(binding)
}

fn validate_store_target(target: &Target) -> Result<(), String> {
    if target_assigns_to_debug(target) {
        Err("cannot assign to __debug__".to_string())
    } else {
        Ok(())
    }
}

fn validate_annotation_target(target: &Target) -> Result<(), String> {
    match target {
        Target::Tuple(_) => Err("only single target (not tuple) can be annotated".to_string()),
        Target::List(_) => Err("only single target (not list) can be annotated".to_string()),
        Target::Starred(_) => Err("illegal target for annotation".to_string()),
        Target::Name(_)
        | Target::Attribute { .. }
        | Target::Subscript { .. }
        | Target::Slice { .. } => Ok(()),
    }
}

fn validate_delete_target(target: &Target) -> Result<(), String> {
    if target_deletes_debug_name(target) {
        Err("cannot delete __debug__".to_string())
    } else if target_deletes_starred(target) {
        Err("cannot delete starred target".to_string())
    } else {
        Ok(())
    }
}

fn validate_aug_assign_target(target: &Target) -> Result<(), String> {
    validate_store_target(target)?;

    match target {
        Target::Tuple(_) => {
            Err("'tuple' is an illegal expression for augmented assignment".to_string())
        }
        Target::List(_) => {
            Err("'list' is an illegal expression for augmented assignment".to_string())
        }
        Target::Starred(_) => Err("starred assignment target cannot be augmented".to_string()),
        Target::Name(_)
        | Target::Attribute { .. }
        | Target::Subscript { .. }
        | Target::Slice { .. } => Ok(()),
    }
}

fn invalid_named_expression_target_name(expr: &Expr) -> &'static str {
    match expr {
        Expr::Bool(true) => "True",
        Expr::Bool(false) => "False",
        Expr::None => "None",
        Expr::Name(_) => "name",
        Expr::Attribute { .. } => "attribute",
        Expr::Subscript { .. } | Expr::Slice { .. } => "subscript",
        Expr::Tuple(_) => "tuple",
        Expr::List(_) | Expr::ListComp { .. } => "list",
        Expr::Dict(_) | Expr::DictComp { .. } | Expr::DictUnpackComp { .. } => "dict display",
        Expr::Set(_) | Expr::FrozenSet(_) | Expr::SetComp { .. } => "set display",
        Expr::GeneratorComp { .. } => "generator expression",
        Expr::Lambda { .. } => "lambda",
        Expr::Call { .. } | Expr::KeywordCall { .. } | Expr::UnpackCall { .. } => "function call",
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::JoinedString(_)
        | Expr::TemplateString(_)
        | Expr::TemplateInterpolation { .. }
        | Expr::Ellipsis => "literal",
        Expr::Binary { .. }
        | Expr::Comparison { .. }
        | Expr::ChainedComparison { .. }
        | Expr::Unary { .. }
        | Expr::Logical { .. }
        | Expr::IfExpression { .. } => "operator",
        Expr::NamedExpr { .. } => "named expression",
        Expr::Yield { .. } | Expr::YieldFrom(_) => "yield expression",
        Expr::Await(_) => "await expression",
        Expr::Starred(_) => "starred expression",
        Expr::SliceLiteral { .. } => "slice",
    }
}

fn invalid_expression_assignment_message(expr: &Expr) -> String {
    if matches!(expr, Expr::Name(_)) {
        "invalid syntax. Maybe you meant '==' or ':=' instead of '='?".to_string()
    } else {
        format!(
            "cannot assign to {} here. Maybe you meant '==' instead of '='?",
            invalid_named_expression_target_name(expr)
        )
    }
}

fn validate_type_scope_expression(expr: &Expr, context: &str) -> Result<(), String> {
    if expr_contains_named_expression(expr) {
        return Err(format!("named expression cannot be used within {context}"));
    }
    if expr_contains_yield_expression(expr) {
        return Err(format!("yield expression cannot be used within {context}"));
    }
    if expr_contains_await_expression(expr) {
        return Err(format!("await expression cannot be used within {context}"));
    }

    Ok(())
}

fn validate_generic_definition_arguments(arguments: &CallArguments) -> Result<(), String> {
    for arg in &arguments.args {
        match arg {
            CallArg::Expr(expr) | CallArg::Unpack(expr) => {
                validate_type_scope_expression(expr, "the definition of a generic")?;
            }
        }
    }

    for keyword in &arguments.keywords {
        match keyword {
            CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
                validate_type_scope_expression(expr, "the definition of a generic")?;
            }
        }
    }

    Ok(())
}

fn validate_generic_function_annotations(
    params: &FunctionParams,
    returns: Option<&Expr>,
) -> Result<(), String> {
    for param in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
    {
        if let Some(annotation) = &param.annotation {
            validate_type_scope_expression(annotation, "the definition of a generic")?;
        }
    }

    if let Some(annotation) = params.vararg_annotation.as_deref() {
        validate_type_scope_expression(annotation, "the definition of a generic")?;
    }
    if let Some(annotation) = params.kwarg_annotation.as_deref() {
        validate_type_scope_expression(annotation, "the definition of a generic")?;
    }
    if let Some(returns) = returns {
        validate_type_scope_expression(returns, "the definition of a generic")?;
    }

    Ok(())
}

fn expr_contains_named_expression(expr: &Expr) -> bool {
    expr_has_kind(expr, ExprKind::NamedExpression)
}

fn expr_contains_yield_expression(expr: &Expr) -> bool {
    expr_has_kind(expr, ExprKind::YieldExpression)
}

fn expr_contains_await_expression(expr: &Expr) -> bool {
    expr_has_kind(expr, ExprKind::AwaitExpression)
}

#[derive(Clone, Copy)]
enum ExprKind {
    NamedExpression,
    YieldExpression,
    AwaitExpression,
}

fn expr_has_kind(expr: &Expr, kind: ExprKind) -> bool {
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
        Expr::NamedExpr { value, .. } => {
            matches!(kind, ExprKind::NamedExpression) || expr_has_kind(value, kind)
        }
        Expr::Yield { value } => {
            matches!(kind, ExprKind::YieldExpression)
                || value
                    .as_deref()
                    .is_some_and(|value| expr_has_kind(value, kind))
        }
        Expr::YieldFrom(value) => {
            matches!(kind, ExprKind::YieldExpression) || expr_has_kind(value, kind)
        }
        Expr::Await(value) => {
            matches!(kind, ExprKind::AwaitExpression) || expr_has_kind(value, kind)
        }
        Expr::Attribute { object, .. } | Expr::Starred(object) => expr_has_kind(object, kind),
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            expr_has_kind(left, kind) || expr_has_kind(right, kind)
        }
        Expr::Comparison { left, right, .. } => {
            expr_has_kind(left, kind) || expr_has_kind(right, kind)
        }
        Expr::ChainedComparison { left, comparisons } => {
            expr_has_kind(left, kind)
                || comparisons
                    .iter()
                    .any(|(_, right)| expr_has_kind(right, kind))
        }
        Expr::Unary { operand, .. } => expr_has_kind(operand, kind),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            expr_has_kind(condition, kind)
                || expr_has_kind(then_branch, kind)
                || expr_has_kind(else_branch, kind)
        }
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => elements.iter().any(|element| expr_has_kind(element, kind)),
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            expr_has_kind(element, kind) || comprehension_clauses_have_kind(clauses, kind)
        }
        Expr::Dict(items) => items.iter().any(|item| match item {
            DictItem::Entry { key, value } => {
                expr_has_kind(key, kind) || expr_has_kind(value, kind)
            }
            DictItem::Unpack(value) => expr_has_kind(value, kind),
        }),
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            expr_has_kind(key, kind)
                || expr_has_kind(value, kind)
                || comprehension_clauses_have_kind(clauses, kind)
        }
        Expr::DictUnpackComp { value, clauses } => {
            expr_has_kind(value, kind) || comprehension_clauses_have_kind(clauses, kind)
        }
        Expr::Subscript { object, index } => {
            expr_has_kind(object, kind) || expr_has_kind(index, kind)
        }
        Expr::SliceLiteral { start, stop, step } => {
            optional_expr_has_kind(start.as_deref(), kind)
                || optional_expr_has_kind(stop.as_deref(), kind)
                || optional_expr_has_kind(step.as_deref(), kind)
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_has_kind(object, kind)
                || optional_expr_has_kind(start.as_deref(), kind)
                || optional_expr_has_kind(stop.as_deref(), kind)
                || optional_expr_has_kind(step.as_deref(), kind)
        }
        Expr::Call { callee, args } => {
            expr_has_kind(callee, kind) || args.iter().any(|arg| expr_has_kind(arg, kind))
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            expr_has_kind(callee, kind)
                || args.iter().any(|arg| expr_has_kind(arg, kind))
                || keywords.iter().any(|(_, value)| expr_has_kind(value, kind))
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            expr_has_kind(callee, kind)
                || args.iter().any(|arg| call_arg_has_kind(arg, kind))
                || keywords
                    .iter()
                    .any(|keyword| call_keyword_has_kind(keyword, kind))
        }
        Expr::Lambda { params, body } => {
            function_params_have_kind(params, kind) || expr_has_kind(body, kind)
        }
        Expr::JoinedString(parts) => f_string_parts_have_kind(parts, kind),
        Expr::TemplateString(parts) => template_string_parts_have_kind(parts, kind),
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            expr_has_kind(value, kind)
                || format_spec
                    .as_deref()
                    .is_some_and(|parts| f_string_parts_have_kind(parts, kind))
        }
    }
}

fn optional_expr_has_kind(expr: Option<&Expr>, kind: ExprKind) -> bool {
    expr.is_some_and(|expr| expr_has_kind(expr, kind))
}

fn call_arg_has_kind(arg: &CallArg, kind: ExprKind) -> bool {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => expr_has_kind(expr, kind),
    }
}

fn call_keyword_has_kind(keyword: &CallKeyword, kind: ExprKind) -> bool {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => expr_has_kind(expr, kind),
    }
}

fn comprehension_clauses_have_kind(clauses: &[ComprehensionClause], kind: ExprKind) -> bool {
    clauses.iter().any(|clause| {
        target_has_kind(&clause.target, kind)
            || expr_has_kind(&clause.iter, kind)
            || clause
                .ifs
                .iter()
                .any(|condition| expr_has_kind(condition, kind))
    })
}

fn validate_comprehension_named_expression_rebindings(
    head_exprs: &[&Expr],
    clauses: &[ComprehensionClause],
    in_class_body: bool,
) -> Result<(), String> {
    let mut all_target_names = Vec::new();
    for clause in clauses {
        collect_target_binding_names(&clause.target, &mut all_target_names);
    }

    for expr in head_exprs {
        let mut names = Vec::new();
        collect_named_expression_names(expr, &mut names);
        if let Some(name) = first_name_in(&names, &all_target_names) {
            return Err(format!(
                "assignment expression cannot rebind comprehension iteration variable '{name}'"
            ));
        }
    }

    let mut seen_target_names = Vec::new();
    let mut prior_filter_named_expression_names = Vec::new();

    for clause in clauses {
        if let Err(error) = validate_comprehension_target_rebindings(
            &clause.target,
            &mut seen_target_names,
            &prior_filter_named_expression_names,
        ) {
            return match error {
                ComprehensionTargetRebinding::IterationVariable(name) => Err(format!(
                    "assignment expression cannot rebind comprehension iteration variable '{name}'"
                )),
                ComprehensionTargetRebinding::AssignmentExpressionTarget(name) => {
                    if in_class_body {
                        Err(
                            "assignment expression within a comprehension cannot be used in a class body"
                                .to_string(),
                        )
                    } else {
                        Err(format!(
                            "comprehension inner loop cannot rebind assignment expression target '{name}'"
                        ))
                    }
                }
            };
        }

        for condition in &clause.ifs {
            let mut names = Vec::new();
            collect_named_expression_names(condition, &mut names);

            if let Some(name) = first_name_in(&names, &seen_target_names) {
                return Err(format!(
                    "assignment expression cannot rebind comprehension iteration variable '{name}'"
                ));
            }

            for name in names {
                push_unique_name(&mut prior_filter_named_expression_names, &name);
            }
        }
    }

    if in_class_body && comprehension_has_named_expression(head_exprs, clauses) {
        return Err(
            "assignment expression within a comprehension cannot be used in a class body"
                .to_string(),
        );
    }

    Ok(())
}

fn comprehension_has_named_expression(
    head_exprs: &[&Expr],
    clauses: &[ComprehensionClause],
) -> bool {
    head_exprs
        .iter()
        .any(|expr| comprehension_class_body_expr_has_named_expression(expr))
        || clauses.iter().any(|clause| {
            comprehension_class_body_target_has_named_expression(&clause.target)
                || clause
                    .ifs
                    .iter()
                    .any(|condition| comprehension_class_body_expr_has_named_expression(condition))
        })
}

fn comprehension_class_body_expr_has_named_expression(expr: &Expr) -> bool {
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
        Expr::NamedExpr { .. } => true,
        Expr::Yield { value } => value
            .as_deref()
            .is_some_and(comprehension_class_body_expr_has_named_expression),
        Expr::YieldFrom(value) | Expr::Await(value) => {
            comprehension_class_body_expr_has_named_expression(value)
        }
        Expr::Attribute { object, .. } | Expr::Starred(object) => {
            comprehension_class_body_expr_has_named_expression(object)
        }
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            comprehension_class_body_expr_has_named_expression(left)
                || comprehension_class_body_expr_has_named_expression(right)
        }
        Expr::Comparison { left, right, .. } => {
            comprehension_class_body_expr_has_named_expression(left)
                || comprehension_class_body_expr_has_named_expression(right)
        }
        Expr::ChainedComparison { left, comparisons } => {
            comprehension_class_body_expr_has_named_expression(left)
                || comparisons
                    .iter()
                    .any(|(_, right)| comprehension_class_body_expr_has_named_expression(right))
        }
        Expr::Unary { operand, .. } => comprehension_class_body_expr_has_named_expression(operand),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            comprehension_class_body_expr_has_named_expression(condition)
                || comprehension_class_body_expr_has_named_expression(then_branch)
                || comprehension_class_body_expr_has_named_expression(else_branch)
        }
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => elements
            .iter()
            .any(comprehension_class_body_expr_has_named_expression),
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            comprehension_class_body_expr_has_named_expression(element)
                || comprehension_class_body_clauses_have_named_expression(clauses)
        }
        Expr::Dict(items) => items
            .iter()
            .any(comprehension_class_body_dict_item_has_named_expression),
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            comprehension_class_body_expr_has_named_expression(key)
                || comprehension_class_body_expr_has_named_expression(value)
                || comprehension_class_body_clauses_have_named_expression(clauses)
        }
        Expr::DictUnpackComp { value, clauses } => {
            comprehension_class_body_expr_has_named_expression(value)
                || comprehension_class_body_clauses_have_named_expression(clauses)
        }
        Expr::Subscript { object, index } => {
            comprehension_class_body_expr_has_named_expression(object)
                || comprehension_class_body_expr_has_named_expression(index)
        }
        Expr::SliceLiteral { start, stop, step } => {
            optional_comprehension_class_body_expr_has_named_expression(start.as_deref())
                || optional_comprehension_class_body_expr_has_named_expression(stop.as_deref())
                || optional_comprehension_class_body_expr_has_named_expression(step.as_deref())
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            comprehension_class_body_expr_has_named_expression(object)
                || optional_comprehension_class_body_expr_has_named_expression(start.as_deref())
                || optional_comprehension_class_body_expr_has_named_expression(stop.as_deref())
                || optional_comprehension_class_body_expr_has_named_expression(step.as_deref())
        }
        Expr::Call { callee, args } => {
            comprehension_class_body_expr_has_named_expression(callee)
                || args
                    .iter()
                    .any(comprehension_class_body_expr_has_named_expression)
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            comprehension_class_body_expr_has_named_expression(callee)
                || args
                    .iter()
                    .any(comprehension_class_body_expr_has_named_expression)
                || keywords
                    .iter()
                    .any(|(_, value)| comprehension_class_body_expr_has_named_expression(value))
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            comprehension_class_body_expr_has_named_expression(callee)
                || args
                    .iter()
                    .any(comprehension_class_body_call_arg_has_named_expression)
                || keywords
                    .iter()
                    .any(comprehension_class_body_call_keyword_has_named_expression)
        }
        Expr::Lambda { params, .. } => {
            comprehension_class_body_function_params_have_named_expression(params)
        }
        Expr::JoinedString(parts) => {
            comprehension_class_body_f_string_parts_have_named_expression(parts)
        }
        Expr::TemplateString(parts) => {
            comprehension_class_body_template_string_parts_have_named_expression(parts)
        }
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            comprehension_class_body_expr_has_named_expression(value)
                || format_spec
                    .as_deref()
                    .is_some_and(comprehension_class_body_f_string_parts_have_named_expression)
        }
    }
}

fn optional_comprehension_class_body_expr_has_named_expression(expr: Option<&Expr>) -> bool {
    expr.is_some_and(comprehension_class_body_expr_has_named_expression)
}

fn comprehension_class_body_clauses_have_named_expression(clauses: &[ComprehensionClause]) -> bool {
    clauses.iter().any(|clause| {
        comprehension_class_body_target_has_named_expression(&clause.target)
            || comprehension_class_body_expr_has_named_expression(&clause.iter)
            || clause
                .ifs
                .iter()
                .any(comprehension_class_body_expr_has_named_expression)
    })
}

fn comprehension_class_body_target_has_named_expression(target: &Target) -> bool {
    match target {
        Target::Name(_) => false,
        Target::Attribute { object, .. } => {
            comprehension_class_body_expr_has_named_expression(object)
        }
        Target::Subscript { object, index } => {
            comprehension_class_body_expr_has_named_expression(object)
                || comprehension_class_body_expr_has_named_expression(index)
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            comprehension_class_body_expr_has_named_expression(object)
                || optional_comprehension_class_body_expr_has_named_expression(start.as_ref())
                || optional_comprehension_class_body_expr_has_named_expression(stop.as_ref())
                || optional_comprehension_class_body_expr_has_named_expression(step.as_ref())
        }
        Target::Starred(target) => comprehension_class_body_target_has_named_expression(target),
        Target::Tuple(targets) | Target::List(targets) => targets
            .iter()
            .any(comprehension_class_body_target_has_named_expression),
    }
}

fn comprehension_class_body_dict_item_has_named_expression(item: &DictItem) -> bool {
    match item {
        DictItem::Entry { key, value } => {
            comprehension_class_body_expr_has_named_expression(key)
                || comprehension_class_body_expr_has_named_expression(value)
        }
        DictItem::Unpack(value) => comprehension_class_body_expr_has_named_expression(value),
    }
}

fn comprehension_class_body_call_arg_has_named_expression(arg: &CallArg) -> bool {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => {
            comprehension_class_body_expr_has_named_expression(expr)
        }
    }
}

fn comprehension_class_body_call_keyword_has_named_expression(keyword: &CallKeyword) -> bool {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
            comprehension_class_body_expr_has_named_expression(expr)
        }
    }
}

fn comprehension_class_body_function_params_have_named_expression(params: &FunctionParams) -> bool {
    params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
        .any(comprehension_class_body_param_has_named_expression)
        || optional_comprehension_class_body_expr_has_named_expression(
            params.vararg_annotation.as_deref(),
        )
        || optional_comprehension_class_body_expr_has_named_expression(
            params.kwarg_annotation.as_deref(),
        )
}

fn comprehension_class_body_param_has_named_expression(param: &Param) -> bool {
    optional_comprehension_class_body_expr_has_named_expression(param.annotation.as_ref())
        || optional_comprehension_class_body_expr_has_named_expression(param.default.as_ref())
}

fn comprehension_class_body_f_string_parts_have_named_expression(parts: &[FStringPart]) -> bool {
    parts.iter().any(|part| match part {
        FStringPart::Literal(_) => false,
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            comprehension_class_body_expr_has_named_expression(value)
                || format_spec.as_ref().is_some_and(|parts| {
                    comprehension_class_body_f_string_parts_have_named_expression(parts)
                })
        }
    })
}

fn comprehension_class_body_template_string_parts_have_named_expression(
    parts: &[TemplateStringPart],
) -> bool {
    parts.iter().any(|part| match part {
        TemplateStringPart::Literal(_) => false,
        TemplateStringPart::Interpolation {
            value, format_spec, ..
        } => {
            comprehension_class_body_expr_has_named_expression(value)
                || format_spec.as_ref().is_some_and(|parts| {
                    comprehension_class_body_f_string_parts_have_named_expression(parts)
                })
        }
    })
}

fn collect_target_binding_names(target: &Target, names: &mut Vec<String>) {
    match target {
        Target::Name(name) => push_unique_name(names, name),
        Target::Starred(target) => collect_target_binding_names(target, names),
        Target::Tuple(targets) | Target::List(targets) => {
            for target in targets {
                collect_target_binding_names(target, names);
            }
        }
        Target::Attribute { .. } | Target::Subscript { .. } | Target::Slice { .. } => {}
    }
}

enum ComprehensionTargetRebinding {
    IterationVariable(String),
    AssignmentExpressionTarget(String),
}

fn validate_comprehension_target_rebindings(
    target: &Target,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    match target {
        Target::Name(name) => {
            if contains_name(prior_filter_named_expression_names, name) {
                return Err(ComprehensionTargetRebinding::AssignmentExpressionTarget(
                    name.clone(),
                ));
            }
            push_unique_name(seen_target_names, name);
        }
        Target::Attribute { object, .. } => {
            validate_comprehension_target_expr_rebindings(
                object,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Target::Subscript { object, index } => {
            validate_comprehension_target_expr_rebindings(
                object,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                index,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            validate_comprehension_target_expr_rebindings(
                object,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                start.as_ref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                stop.as_ref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                step.as_ref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Target::Starred(target) => validate_comprehension_target_rebindings(
            target,
            seen_target_names,
            prior_filter_named_expression_names,
        )?,
        Target::Tuple(targets) | Target::List(targets) => {
            for target in targets {
                validate_comprehension_target_rebindings(
                    target,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
    }

    Ok(())
}

fn validate_optional_comprehension_target_expr_rebindings(
    expr: Option<&Expr>,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    if let Some(expr) = expr {
        validate_comprehension_target_expr_rebindings(
            expr,
            seen_target_names,
            prior_filter_named_expression_names,
        )?;
    }

    Ok(())
}

fn validate_comprehension_target_expr_rebindings(
    expr: &Expr,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    match expr {
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::Bool(_)
        | Expr::None
        | Expr::Ellipsis => {}
        Expr::Name(name) => {
            if contains_name(prior_filter_named_expression_names, name) {
                return Err(ComprehensionTargetRebinding::AssignmentExpressionTarget(
                    name.clone(),
                ));
            }
        }
        Expr::NamedExpr { name, .. } => {
            return if contains_name(seen_target_names, name) {
                Err(ComprehensionTargetRebinding::IterationVariable(
                    name.clone(),
                ))
            } else {
                Err(ComprehensionTargetRebinding::AssignmentExpressionTarget(
                    name.clone(),
                ))
            };
        }
        Expr::Yield { value } => {
            if let Some(value) = value {
                validate_comprehension_target_expr_rebindings(
                    value,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::YieldFrom(value) | Expr::Await(value) => {
            validate_comprehension_target_expr_rebindings(
                value,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::Attribute { object, .. } | Expr::Starred(object) => {
            validate_comprehension_target_expr_rebindings(
                object,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            validate_comprehension_target_expr_rebindings(
                left,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                right,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::Comparison { left, right, .. } => {
            validate_comprehension_target_expr_rebindings(
                left,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                right,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::ChainedComparison { left, comparisons } => {
            validate_comprehension_target_expr_rebindings(
                left,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            for (_, right) in comparisons {
                validate_comprehension_target_expr_rebindings(
                    right,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::Unary { operand, .. } => {
            validate_comprehension_target_expr_rebindings(
                operand,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            validate_comprehension_target_expr_rebindings(
                condition,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                then_branch,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                else_branch,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => {
            for element in elements {
                validate_comprehension_target_expr_rebindings(
                    element,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::ListComp { .. }
        | Expr::SetComp { .. }
        | Expr::GeneratorComp { .. }
        | Expr::DictComp { .. }
        | Expr::DictUnpackComp { .. } => {}
        Expr::Dict(items) => {
            for item in items {
                validate_comprehension_target_dict_item_rebindings(
                    item,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::Subscript { object, index } => {
            validate_comprehension_target_expr_rebindings(
                object,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                index,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::SliceLiteral { start, stop, step } => {
            validate_optional_comprehension_target_expr_rebindings(
                start.as_deref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                stop.as_deref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                step.as_deref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            validate_comprehension_target_expr_rebindings(
                object,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                start.as_deref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                stop.as_deref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_optional_comprehension_target_expr_rebindings(
                step.as_deref(),
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::Call { callee, args } => {
            validate_comprehension_target_expr_rebindings(
                callee,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            for arg in args {
                validate_comprehension_target_expr_rebindings(
                    arg,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            validate_comprehension_target_expr_rebindings(
                callee,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            for arg in args {
                validate_comprehension_target_expr_rebindings(
                    arg,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
            for (_, value) in keywords {
                validate_comprehension_target_expr_rebindings(
                    value,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            validate_comprehension_target_expr_rebindings(
                callee,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            for arg in args {
                validate_comprehension_target_call_arg_rebindings(
                    arg,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
            for keyword in keywords {
                validate_comprehension_target_call_keyword_rebindings(
                    keyword,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
        Expr::Lambda { params, .. } => {
            validate_comprehension_target_function_params_rebindings(
                params,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::JoinedString(parts) => {
            validate_comprehension_target_f_string_part_rebindings(
                parts,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::TemplateString(parts) => {
            validate_comprehension_target_template_string_part_rebindings(
                parts,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            validate_comprehension_target_expr_rebindings(
                value,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            if let Some(format_spec) = format_spec {
                validate_comprehension_target_f_string_part_rebindings(
                    format_spec,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
            }
        }
    }

    Ok(())
}

fn validate_comprehension_target_dict_item_rebindings(
    item: &DictItem,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    match item {
        DictItem::Entry { key, value } => {
            validate_comprehension_target_expr_rebindings(
                key,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
            validate_comprehension_target_expr_rebindings(
                value,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
        DictItem::Unpack(value) => {
            validate_comprehension_target_expr_rebindings(
                value,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
    }

    Ok(())
}

fn validate_comprehension_target_call_arg_rebindings(
    arg: &CallArg,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => {
            validate_comprehension_target_expr_rebindings(
                expr,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
    }

    Ok(())
}

fn validate_comprehension_target_call_keyword_rebindings(
    keyword: &CallKeyword,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
            validate_comprehension_target_expr_rebindings(
                expr,
                seen_target_names,
                prior_filter_named_expression_names,
            )?;
        }
    }

    Ok(())
}

fn validate_comprehension_target_function_params_rebindings(
    params: &FunctionParams,
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    for param in params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
    {
        validate_optional_comprehension_target_expr_rebindings(
            param.annotation.as_ref(),
            seen_target_names,
            prior_filter_named_expression_names,
        )?;
        validate_optional_comprehension_target_expr_rebindings(
            param.default.as_ref(),
            seen_target_names,
            prior_filter_named_expression_names,
        )?;
    }

    validate_optional_comprehension_target_expr_rebindings(
        params.vararg_annotation.as_deref(),
        seen_target_names,
        prior_filter_named_expression_names,
    )?;
    validate_optional_comprehension_target_expr_rebindings(
        params.kwarg_annotation.as_deref(),
        seen_target_names,
        prior_filter_named_expression_names,
    )?;

    Ok(())
}

fn validate_comprehension_target_f_string_part_rebindings(
    parts: &[FStringPart],
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    for part in parts {
        match part {
            FStringPart::Literal(_) => {}
            FStringPart::Formatted {
                value, format_spec, ..
            } => {
                validate_comprehension_target_expr_rebindings(
                    value,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
                if let Some(format_spec) = format_spec {
                    validate_comprehension_target_f_string_part_rebindings(
                        format_spec,
                        seen_target_names,
                        prior_filter_named_expression_names,
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn validate_comprehension_target_template_string_part_rebindings(
    parts: &[TemplateStringPart],
    seen_target_names: &mut Vec<String>,
    prior_filter_named_expression_names: &[String],
) -> Result<(), ComprehensionTargetRebinding> {
    for part in parts {
        match part {
            TemplateStringPart::Literal(_) => {}
            TemplateStringPart::Interpolation {
                value, format_spec, ..
            } => {
                validate_comprehension_target_expr_rebindings(
                    value,
                    seen_target_names,
                    prior_filter_named_expression_names,
                )?;
                if let Some(format_spec) = format_spec {
                    validate_comprehension_target_f_string_part_rebindings(
                        format_spec,
                        seen_target_names,
                        prior_filter_named_expression_names,
                    )?;
                }
            }
        }
    }

    Ok(())
}

fn collect_named_expression_names(expr: &Expr, names: &mut Vec<String>) {
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
        Expr::NamedExpr { name, value } => {
            push_unique_name(names, name);
            collect_named_expression_names(value, names);
        }
        Expr::Yield { value } => {
            if let Some(value) = value {
                collect_named_expression_names(value, names);
            }
        }
        Expr::YieldFrom(value) | Expr::Await(value) => collect_named_expression_names(value, names),
        Expr::Attribute { object, .. } | Expr::Starred(object) => {
            collect_named_expression_names(object, names);
        }
        Expr::Binary { left, right, .. } | Expr::Logical { left, right, .. } => {
            collect_named_expression_names(left, names);
            collect_named_expression_names(right, names);
        }
        Expr::Comparison { left, right, .. } => {
            collect_named_expression_names(left, names);
            collect_named_expression_names(right, names);
        }
        Expr::ChainedComparison { left, comparisons } => {
            collect_named_expression_names(left, names);
            for (_, right) in comparisons {
                collect_named_expression_names(right, names);
            }
        }
        Expr::Unary { operand, .. } => collect_named_expression_names(operand, names),
        Expr::IfExpression {
            condition,
            then_branch,
            else_branch,
        } => {
            collect_named_expression_names(condition, names);
            collect_named_expression_names(then_branch, names);
            collect_named_expression_names(else_branch, names);
        }
        Expr::List(elements)
        | Expr::Set(elements)
        | Expr::FrozenSet(elements)
        | Expr::Tuple(elements) => {
            for element in elements {
                collect_named_expression_names(element, names);
            }
        }
        Expr::ListComp { element, clauses }
        | Expr::SetComp { element, clauses }
        | Expr::GeneratorComp { element, clauses } => {
            collect_named_expression_names(element, names);
            collect_named_expression_names_from_clauses(clauses, names);
        }
        Expr::Dict(items) => {
            for item in items {
                collect_named_expression_names_from_dict_item(item, names);
            }
        }
        Expr::DictComp {
            key,
            value,
            clauses,
        } => {
            collect_named_expression_names(key, names);
            collect_named_expression_names(value, names);
            collect_named_expression_names_from_clauses(clauses, names);
        }
        Expr::DictUnpackComp { value, clauses } => {
            collect_named_expression_names(value, names);
            collect_named_expression_names_from_clauses(clauses, names);
        }
        Expr::Subscript { object, index } => {
            collect_named_expression_names(object, names);
            collect_named_expression_names(index, names);
        }
        Expr::SliceLiteral { start, stop, step } => {
            collect_optional_named_expression_names(start.as_deref(), names);
            collect_optional_named_expression_names(stop.as_deref(), names);
            collect_optional_named_expression_names(step.as_deref(), names);
        }
        Expr::Slice {
            object,
            start,
            stop,
            step,
        } => {
            collect_named_expression_names(object, names);
            collect_optional_named_expression_names(start.as_deref(), names);
            collect_optional_named_expression_names(stop.as_deref(), names);
            collect_optional_named_expression_names(step.as_deref(), names);
        }
        Expr::Call { callee, args } => {
            collect_named_expression_names(callee, names);
            for arg in args {
                collect_named_expression_names(arg, names);
            }
        }
        Expr::KeywordCall {
            callee,
            args,
            keywords,
        } => {
            collect_named_expression_names(callee, names);
            for arg in args {
                collect_named_expression_names(arg, names);
            }
            for (_, value) in keywords {
                collect_named_expression_names(value, names);
            }
        }
        Expr::UnpackCall {
            callee,
            args,
            keywords,
        } => {
            collect_named_expression_names(callee, names);
            for arg in args {
                collect_named_expression_names_from_call_arg(arg, names);
            }
            for keyword in keywords {
                collect_named_expression_names_from_call_keyword(keyword, names);
            }
        }
        Expr::JoinedString(parts) => {
            collect_named_expression_names_from_f_string_parts(parts, names)
        }
        Expr::TemplateString(parts) => {
            collect_named_expression_names_from_template_string_parts(parts, names)
        }
        Expr::TemplateInterpolation {
            value, format_spec, ..
        } => {
            collect_named_expression_names(value, names);
            if let Some(format_spec) = format_spec {
                collect_named_expression_names_from_f_string_parts(format_spec, names);
            }
        }
        Expr::Lambda { .. } => {}
    }
}

fn collect_optional_named_expression_names(expr: Option<&Expr>, names: &mut Vec<String>) {
    if let Some(expr) = expr {
        collect_named_expression_names(expr, names);
    }
}

fn collect_named_expression_names_from_clauses(
    clauses: &[ComprehensionClause],
    names: &mut Vec<String>,
) {
    for clause in clauses {
        collect_named_expression_names(&clause.iter, names);
        for condition in &clause.ifs {
            collect_named_expression_names(condition, names);
        }
    }
}

fn collect_named_expression_names_from_dict_item(item: &DictItem, names: &mut Vec<String>) {
    match item {
        DictItem::Entry { key, value } => {
            collect_named_expression_names(key, names);
            collect_named_expression_names(value, names);
        }
        DictItem::Unpack(value) => collect_named_expression_names(value, names),
    }
}

fn collect_named_expression_names_from_call_arg(arg: &CallArg, names: &mut Vec<String>) {
    match arg {
        CallArg::Expr(expr) | CallArg::Unpack(expr) => collect_named_expression_names(expr, names),
    }
}

fn collect_named_expression_names_from_call_keyword(
    keyword: &CallKeyword,
    names: &mut Vec<String>,
) {
    match keyword {
        CallKeyword::Named(_, expr) | CallKeyword::Unpack(expr) => {
            collect_named_expression_names(expr, names);
        }
    }
}

fn collect_named_expression_names_from_f_string_parts(
    parts: &[FStringPart],
    names: &mut Vec<String>,
) {
    for part in parts {
        match part {
            FStringPart::Literal(_) => {}
            FStringPart::Formatted {
                value, format_spec, ..
            } => {
                collect_named_expression_names(value, names);
                if let Some(format_spec) = format_spec {
                    collect_named_expression_names_from_f_string_parts(format_spec, names);
                }
            }
        }
    }
}

fn collect_named_expression_names_from_template_string_parts(
    parts: &[TemplateStringPart],
    names: &mut Vec<String>,
) {
    for part in parts {
        match part {
            TemplateStringPart::Literal(_) => {}
            TemplateStringPart::Interpolation {
                value, format_spec, ..
            } => {
                collect_named_expression_names(value, names);
                if let Some(format_spec) = format_spec {
                    collect_named_expression_names_from_f_string_parts(format_spec, names);
                }
            }
        }
    }
}

fn push_unique_name(names: &mut Vec<String>, name: &str) {
    if !names.iter().any(|existing| existing == name) {
        names.push(name.to_string());
    }
}

fn first_name_in<'a>(names: &'a [String], others: &[String]) -> Option<&'a str> {
    names
        .iter()
        .find(|name| others.iter().any(|other| other == *name))
        .map(String::as_str)
}

fn contains_name(names: &[String], name: &str) -> bool {
    names.iter().any(|existing| existing == name)
}

fn target_has_kind(target: &Target, kind: ExprKind) -> bool {
    match target {
        Target::Name(_) => false,
        Target::Attribute { object, .. } => expr_has_kind(object, kind),
        Target::Subscript { object, index } => {
            expr_has_kind(object, kind) || expr_has_kind(index, kind)
        }
        Target::Slice {
            object,
            start,
            stop,
            step,
        } => {
            expr_has_kind(object, kind)
                || optional_expr_has_kind(start.as_ref(), kind)
                || optional_expr_has_kind(stop.as_ref(), kind)
                || optional_expr_has_kind(step.as_ref(), kind)
        }
        Target::Starred(target) => target_has_kind(target, kind),
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(|target| target_has_kind(target, kind))
        }
    }
}

fn function_params_have_kind(params: &FunctionParams, kind: ExprKind) -> bool {
    params
        .positional_only
        .iter()
        .chain(params.positional.iter())
        .chain(params.keyword_only.iter())
        .any(|param| param_has_kind(param, kind))
        || optional_expr_has_kind(params.vararg_annotation.as_deref(), kind)
        || optional_expr_has_kind(params.kwarg_annotation.as_deref(), kind)
}

fn param_has_kind(param: &Param, kind: ExprKind) -> bool {
    optional_expr_has_kind(param.annotation.as_ref(), kind)
        || optional_expr_has_kind(param.default.as_ref(), kind)
}

fn f_string_parts_have_kind(parts: &[FStringPart], kind: ExprKind) -> bool {
    parts.iter().any(|part| match part {
        FStringPart::Literal(_) => false,
        FStringPart::Formatted {
            value, format_spec, ..
        } => {
            expr_has_kind(value, kind)
                || format_spec
                    .as_ref()
                    .is_some_and(|parts| f_string_parts_have_kind(parts, kind))
        }
    })
}

fn template_string_parts_have_kind(parts: &[TemplateStringPart], kind: ExprKind) -> bool {
    parts.iter().any(|part| match part {
        TemplateStringPart::Literal(_) => false,
        TemplateStringPart::Interpolation {
            value, format_spec, ..
        } => {
            expr_has_kind(value, kind)
                || format_spec
                    .as_ref()
                    .is_some_and(|parts| f_string_parts_have_kind(parts, kind))
        }
    })
}

fn conditional_else_starts_statement(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Pass
                | Token::Return
                | Token::Raise
                | Token::Del
                | Token::Yield
                | Token::Assert
                | Token::Break
                | Token::Continue
                | Token::Import
                | Token::From
        )
    )
}

fn conditional_body_starts_statement(token: Option<&Token>) -> bool {
    matches!(token, Some(Token::Pass | Token::Break | Token::Continue))
}

fn invalid_annotation_assignment_message(expr: &Expr) -> String {
    match expr {
        Expr::Tuple(_) => "only single target (not tuple) can be annotated".to_string(),
        Expr::List(_) => "only single target (not list) can be annotated".to_string(),
        _ => "illegal target for annotation".to_string(),
    }
}

fn invalid_assert_assignment_message(expr: &Expr) -> String {
    format!(
        "cannot assign to {} here. Maybe you meant '==' instead of '='?",
        invalid_named_expression_target_name(expr)
    )
}

fn invalid_delete_target_message(expr: &Expr) -> String {
    if delete_expr_contains_starred(expr) {
        return "cannot delete starred target".to_string();
    }

    if let Expr::Tuple(elements) | Expr::List(elements) = expr {
        if let Some(invalid) = first_invalid_delete_expr(elements) {
            return invalid_delete_target_message(invalid);
        }
    }

    format!("cannot delete {}", invalid_delete_target_name(expr))
}

fn first_invalid_delete_expr(elements: &[Expr]) -> Option<&Expr> {
    for element in elements {
        if is_valid_delete_expr(element) {
            continue;
        }

        if let Expr::Tuple(nested) | Expr::List(nested) = element {
            if let Some(invalid) = first_invalid_delete_expr(nested) {
                return Some(invalid);
            }
        }

        return Some(element);
    }

    None
}

fn is_valid_delete_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Name(_) | Expr::Attribute { .. } | Expr::Subscript { .. } | Expr::Slice { .. } => {
            true
        }
        Expr::Tuple(elements) | Expr::List(elements) => elements.iter().all(is_valid_delete_expr),
        _ => false,
    }
}

fn delete_expr_contains_starred(expr: &Expr) -> bool {
    match expr {
        Expr::Starred(_) => true,
        Expr::Tuple(elements) | Expr::List(elements) => {
            elements.iter().any(delete_expr_contains_starred)
        }
        _ => false,
    }
}

fn invalid_delete_target_name(expr: &Expr) -> &'static str {
    match expr {
        Expr::Bool(true) => "True",
        Expr::Bool(false) => "False",
        Expr::None => "None",
        Expr::Ellipsis => "Ellipsis",
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::Float(_)
        | Expr::Imaginary(_)
        | Expr::String(_)
        | Expr::Bytes(_)
        | Expr::JoinedString(_)
        | Expr::TemplateString(_)
        | Expr::TemplateInterpolation { .. } => "literal",
        Expr::Call { .. } | Expr::KeywordCall { .. } | Expr::UnpackCall { .. } => "function call",
        Expr::IfExpression { .. } => "conditional expression",
        Expr::NamedExpr { .. } => "named expression",
        Expr::Starred(_) => "starred target",
        Expr::Tuple(_) => "tuple",
        Expr::List(_) | Expr::ListComp { .. } => "list",
        Expr::Dict(_) | Expr::DictComp { .. } | Expr::DictUnpackComp { .. } => "dict display",
        Expr::Set(_) | Expr::FrozenSet(_) | Expr::SetComp { .. } => "set display",
        Expr::GeneratorComp { .. } => "generator expression",
        Expr::Lambda { .. } => "lambda",
        Expr::Binary { .. }
        | Expr::Comparison { .. }
        | Expr::ChainedComparison { .. }
        | Expr::Unary { .. }
        | Expr::Logical { .. } => "expression",
        Expr::Yield { .. } | Expr::YieldFrom(_) => "yield expression",
        Expr::Await(_) => "await expression",
        Expr::SliceLiteral { .. } => "slice",
        Expr::Name(_) => "name",
        Expr::Attribute { .. } => "attribute",
        Expr::Subscript { .. } | Expr::Slice { .. } => "subscript",
    }
}

fn target_assigns_to_debug(target: &Target) -> bool {
    match target {
        Target::Name(name) => name == "__debug__",
        Target::Attribute { name, .. } => name == "__debug__",
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(target_assigns_to_debug)
        }
        Target::Starred(target) => target_assigns_to_debug(target),
        Target::Subscript { .. } | Target::Slice { .. } => false,
    }
}

fn target_deletes_debug_name(target: &Target) -> bool {
    match target {
        Target::Name(name) => name == "__debug__",
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(target_deletes_debug_name)
        }
        Target::Starred(target) => target_deletes_debug_name(target),
        Target::Attribute { .. } | Target::Subscript { .. } | Target::Slice { .. } => false,
    }
}

fn target_deletes_starred(target: &Target) -> bool {
    match target {
        Target::Starred(_) => true,
        Target::Tuple(targets) | Target::List(targets) => {
            targets.iter().any(target_deletes_starred)
        }
        Target::Name(_)
        | Target::Attribute { .. }
        | Target::Subscript { .. }
        | Target::Slice { .. } => false,
    }
}

fn ensure_unique_parameter_name(name: &str, seen_names: &mut Vec<String>) -> Result<(), String> {
    if seen_names.iter().any(|seen| seen == name) {
        Err(format!("duplicate parameter name: {name}"))
    } else {
        seen_names.push(name.to_string());
        Ok(())
    }
}

fn ensure_unique_type_parameter_name(
    name: &str,
    seen_names: &mut Vec<String>,
) -> Result<(), String> {
    if seen_names.iter().any(|seen| seen == name) {
        Err(format!("duplicate type parameter name: {name}"))
    } else {
        seen_names.push(name.to_string());
        Ok(())
    }
}

fn validate_type_parameter_name(name: &str) -> Result<(), String> {
    if name == "__classdict__" {
        Err("reserved name '__classdict__' cannot be used for type parameter".to_string())
    } else {
        Ok(())
    }
}

fn invalid_type_param_bound_message(kind: &TypeParamKind, bound: &Expr) -> &'static str {
    match (kind, bound) {
        (TypeParamKind::TypeVarTuple, Expr::Tuple(_)) => "cannot use constraints with TypeVarTuple",
        (TypeParamKind::TypeVarTuple, _) => "cannot use bound with TypeVarTuple",
        (TypeParamKind::ParamSpec, Expr::Tuple(_)) => "cannot use constraints with ParamSpec",
        (TypeParamKind::ParamSpec, _) => "cannot use bound with ParamSpec",
        (TypeParamKind::TypeVar, _) => unreachable!("plain type parameters can have bounds"),
    }
}

fn is_invalid_star_expression_end(token: Option<&Token>) -> bool {
    matches!(
        token,
        Some(
            Token::Comma
                | Token::RightParen
                | Token::RightBracket
                | Token::RightBrace
                | Token::Semicolon
                | Token::Newline
                | Token::Dedent
                | Token::Eof
                | Token::Colon
                | Token::Equal
        ) | None
    )
}

fn parameter_list_end_label(end: ParameterListEnd) -> &'static str {
    match end {
        ParameterListEnd::RightParen => "')'",
        ParameterListEnd::Colon => "':'",
    }
}

fn assign_parameter_type_comment(
    params: &mut FunctionParams,
    target: Option<ParameterTypeCommentTarget>,
    comment: String,
) -> Result<(), String> {
    let Some(target) = target else {
        return Err("type comment does not belong to a parameter".to_string());
    };

    let slot = match target {
        ParameterTypeCommentTarget::Positional(index) => &mut params.positional[index].type_comment,
        ParameterTypeCommentTarget::KeywordOnly(index) => {
            &mut params.keyword_only[index].type_comment
        }
        ParameterTypeCommentTarget::Vararg => &mut params.vararg_type_comment,
        ParameterTypeCommentTarget::Kwarg => &mut params.kwarg_type_comment,
    };
    if slot.is_some() {
        return Err("Cannot have two type comments on parameter".to_string());
    }
    *slot = Some(comment);
    Ok(())
}

fn call_arg_exprs(args: Vec<CallArg>) -> Vec<Expr> {
    args.into_iter()
        .map(|arg| match arg {
            CallArg::Expr(expr) => expr,
            CallArg::Unpack(_) => unreachable!("unpacked call arg is only used by UnpackCall"),
        })
        .collect()
}

fn call_keyword_exprs(keywords: Vec<CallKeyword>) -> Vec<(String, Expr)> {
    keywords
        .into_iter()
        .map(|keyword| match keyword {
            CallKeyword::Named(name, expr) => (name, expr),
            CallKeyword::Unpack(_) => {
                unreachable!("unpacked call keyword is only used by UnpackCall")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        parse, parse_eval, parse_func_type, parse_interactive,
        starts_unparenthesized_lambda_expression,
    };
    use crate::ast::{
        BinaryOp, ComparisonOp, ComprehensionClause, DictItem, ExceptHandler, Expr,
        FStringConversion, FStringPart, FunctionParams, FunctionType, ImportAlias,
        ImportFromTargets, LogicalOp, MatchCase, Param, Pattern, Program, Stmt, Target, UnaryOp,
        WithItem,
    };
    use crate::lexer::{Token, TokenFStringConversion, TokenFStringPart, lex};

    #[test]
    fn parses_print_number_statement() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::Number(123),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Number(123)],
                })],
            })
        );
    }

    #[test]
    fn parses_float_expression() {
        let tokens = lex("1.5 + .25").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Float("1.5".to_string())),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Float(".25".to_string())),
                })],
            })
        );
    }

    #[test]
    fn parses_imaginary_expression() {
        let tokens = lex("1 + 2j").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Imaginary("2".to_string())),
                })],
            })
        );
    }

    #[test]
    fn parses_eval_input_expression() {
        let tokens = lex("1 + 2\n").unwrap();

        assert_eq!(
            parse_eval(&tokens),
            Ok(Expr::Binary {
                left: Box::new(Expr::Number(1)),
                op: BinaryOp::Add,
                right: Box::new(Expr::Number(2)),
            })
        );
    }

    #[test]
    fn parses_eval_input_tuple_expression() {
        let tokens = lex("1, 2,\n").unwrap();

        assert_eq!(
            parse_eval(&tokens),
            Ok(Expr::Tuple(vec![Expr::Number(1), Expr::Number(2)]))
        );
    }

    #[test]
    fn parses_interactive_simple_statement() {
        let tokens = lex("1 + 2\n").unwrap();

        assert_eq!(
            parse_interactive(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                })],
            })
        );
    }

    #[test]
    fn parses_interactive_semicolon_separated_simple_statements() {
        let tokens = lex("1; 2").unwrap();

        assert_eq!(
            parse_interactive(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Number(1)), Stmt::Expr(Expr::Number(2))],
            })
        );
    }

    #[test]
    fn rejects_interactive_inline_compound_without_terminal_newline() {
        let tokens = lex("def f(): pass").unwrap();

        assert_eq!(
            parse_interactive(&tokens),
            Err("unexpected EOF while parsing".to_string())
        );
    }

    #[test]
    fn parses_interactive_inline_compound_with_terminal_newline() {
        let tokens = lex("def f(): pass\n").unwrap();

        assert!(parse_interactive(&tokens).is_ok());
    }

    #[test]
    fn parses_interactive_indented_compound_without_terminal_newline() {
        let tokens = lex("def f():\n    pass").unwrap();

        assert!(parse_interactive(&tokens).is_ok());
    }

    #[test]
    fn rejects_interactive_multiple_physical_statements() {
        let tokens = lex("1\n2").unwrap();

        assert_eq!(
            parse_interactive(&tokens),
            Err("expected end of input, found Number(2)".to_string())
        );
    }

    #[test]
    fn parses_func_type_input() {
        let tokens = lex("(int, *str, **Any) -> float\n").unwrap();

        assert_eq!(
            parse_func_type(&tokens),
            Ok(FunctionType {
                arg_types: vec![
                    Expr::Name("int".to_string()),
                    Expr::Name("str".to_string()),
                    Expr::Name("Any".to_string()),
                ],
                returns: Expr::Name("float".to_string()),
            })
        );
    }

    #[test]
    fn rejects_invalid_func_type_order() {
        let tokens = lex("(int, *str, value) -> float").unwrap();

        assert_eq!(
            parse_func_type(&tokens),
            Err("plain type expression cannot follow '*' or '**'".to_string())
        );
    }

    #[test]
    fn parses_addition_expression() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Binary {
                        left: Box::new(Expr::Number(1)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(2)),
                    }],
                })],
            })
        );
    }

    #[test]
    fn parses_arithmetic_precedence() {
        let tokens = lex("1 + 2 * 3").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::Multiply,
                        right: Box::new(Expr::Number(3)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_matrix_multiply_at_term_precedence() {
        let tokens = lex("1 + 2 @ 3 * 4").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Binary {
                            left: Box::new(Expr::Number(2)),
                            op: BinaryOp::MatrixMultiply,
                            right: Box::new(Expr::Number(3)),
                        }),
                        op: BinaryOp::Multiply,
                        right: Box::new(Expr::Number(4)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_ellipsis_expression() {
        let tokens = lex("...").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Ellipsis)],
            })
        );
    }

    #[test]
    fn parses_bitwise_precedence() {
        let tokens = lex("1 | 2 ^ 3 & 4 << 5 + 6").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::BitOr,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::BitXor,
                        right: Box::new(Expr::Binary {
                            left: Box::new(Expr::Number(3)),
                            op: BinaryOp::BitAnd,
                            right: Box::new(Expr::Binary {
                                left: Box::new(Expr::Number(4)),
                                op: BinaryOp::LeftShift,
                                right: Box::new(Expr::Binary {
                                    left: Box::new(Expr::Number(5)),
                                    op: BinaryOp::Add,
                                    right: Box::new(Expr::Number(6)),
                                }),
                            }),
                        }),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_bitwise_invert_expression() {
        let tokens = lex("~1").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Unary {
                    op: UnaryOp::Invert,
                    operand: Box::new(Expr::Number(1)),
                })],
            })
        );
    }

    #[test]
    fn parses_conditional_expression() {
        let tokens = lex("1 if True else 2").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::IfExpression {
                    condition: Box::new(Expr::Bool(true)),
                    then_branch: Box::new(Expr::Number(1)),
                    else_branch: Box::new(Expr::Number(2)),
                })],
            })
        );
    }

    #[test]
    fn parses_conditional_expression_as_right_associative() {
        let tokens = lex("1 if False else 2 if True else 3").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::IfExpression {
                    condition: Box::new(Expr::Bool(false)),
                    then_branch: Box::new(Expr::Number(1)),
                    else_branch: Box::new(Expr::IfExpression {
                        condition: Box::new(Expr::Bool(true)),
                        then_branch: Box::new(Expr::Number(2)),
                        else_branch: Box::new(Expr::Number(3)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_assert_statement() {
        let tokens = lex("assert x, \"message\"").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Assert {
                    condition: Expr::Name("x".to_string()),
                    message: Some(Expr::String("message".to_string())),
                }],
            })
        );
    }

    #[test]
    fn parses_raise_statement() {
        let tokens = lex("raise Exception(\"boom\")").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Raise {
                    value: Some(Expr::Call {
                        callee: Box::new(Expr::Name("Exception".to_string())),
                        args: vec![Expr::String("boom".to_string())],
                    }),
                    cause: None,
                }],
            })
        );
    }

    #[test]
    fn parses_raise_from_statement() {
        let tokens = lex("raise ValueError(\"bad\") from Exception(\"root\")").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_yield_statement() {
        let tokens = lex("yield 1, 2").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Yield {
                    value: Some(Box::new(Expr::Tuple(vec![
                        Expr::Number(1),
                        Expr::Number(2),
                    ]))),
                })],
            })
        );
    }

    #[test]
    fn parses_try_except_statement() {
        let tokens = lex(
            "try:\n    raise Exception(\"boom\")\nexcept Exception as error:\n    print(error)",
        )
        .unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Try {
                    body: vec![Stmt::Raise {
                        value: Some(Expr::Call {
                            callee: Box::new(Expr::Name("Exception".to_string())),
                            args: vec![Expr::String("boom".to_string())],
                        }),
                        cause: None,
                    }],
                    handlers: vec![ExceptHandler {
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
            })
        );
    }

    #[test]
    fn parses_tuple_except_handler_type() {
        let tokens = lex(
            "try:\n    raise TypeError(\"bad\")\nexcept (ValueError, TypeError) as error:\n    print(error)",
        )
        .unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::Try { handlers, .. }] => {
                assert_eq!(
                    handlers[0].type_expr,
                    Some(Expr::Tuple(vec![
                        Expr::Name("ValueError".to_string()),
                        Expr::Name("TypeError".to_string())
                    ]))
                );
                assert_eq!(handlers[0].name, Some("error".to_string()));
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_except_star_handlers() {
        let tokens = lex(
            "try:\n    raise TypeError(\"bad\")\nexcept* ValueError:\n    pass\nexcept* TypeError as error:\n    print(error)",
        )
        .unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::TryStar { handlers, .. }] => {
                assert_eq!(
                    handlers[0].type_expr,
                    Some(Expr::Name("ValueError".to_string()))
                );
                assert_eq!(handlers[0].name, None);
                assert_eq!(
                    handlers[1].type_expr,
                    Some(Expr::Name("TypeError".to_string()))
                );
                assert_eq!(handlers[1].name, Some("error".to_string()));
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_tuple_except_star_handler_type() {
        let tokens =
            lex("try:\n    raise ValueError(\"bad\")\nexcept* (TypeError, ValueError):\n    pass")
                .unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::TryStar { handlers, .. }] => assert_eq!(
                handlers[0].type_expr,
                Some(Expr::Tuple(vec![
                    Expr::Name("TypeError".to_string()),
                    Expr::Name("ValueError".to_string())
                ]))
            ),
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_unparenthesized_except_tuple_types_without_as() {
        let tokens = lex("try:\n    pass\nexcept ValueError, TypeError:\n    pass").unwrap();
        let program = parse(&tokens).unwrap();
        match &program.statements[..] {
            [Stmt::Try { handlers, .. }] => assert_eq!(
                handlers[0].type_expr,
                Some(Expr::Tuple(vec![
                    Expr::Name("ValueError".to_string()),
                    Expr::Name("TypeError".to_string())
                ]))
            ),
            statements => panic!("unexpected statements: {statements:?}"),
        }

        let tokens = lex("try:\n    pass\nexcept* ValueError, TypeError:\n    pass").unwrap();
        let program = parse(&tokens).unwrap();
        match &program.statements[..] {
            [Stmt::TryStar { handlers, .. }] => assert_eq!(
                handlers[0].type_expr,
                Some(Expr::Tuple(vec![
                    Expr::Name("ValueError".to_string()),
                    Expr::Name("TypeError".to_string())
                ]))
            ),
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_dotted_except_handler_type() {
        let tokens = lex("try:\n    pass\nexcept tty.error:\n    pass").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::Try { handlers, .. }] => assert_eq!(
                handlers[0].type_expr,
                Some(Expr::Attribute {
                    object: Box::new(Expr::Name("tty".to_string())),
                    name: "error".to_string()
                })
            ),
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn rejects_unparenthesized_except_tuple_types_with_as() {
        let tokens =
            lex("try:\n    pass\nexcept ValueError, TypeError as error:\n    pass").unwrap();
        assert!(parse(&tokens).is_err());

        let tokens =
            lex("try:\n    pass\nexcept* ValueError, TypeError as error:\n    pass").unwrap();
        assert!(parse(&tokens).is_err());
    }

    #[test]
    fn rejects_mixed_except_and_except_star_handlers() {
        let tokens =
            lex("try:\n    pass\nexcept ValueError:\n    pass\nexcept* TypeError:\n    pass")
                .unwrap();

        assert_eq!(
            parse(&tokens),
            Err("cannot have both 'except' and 'except*' on the same 'try'".to_string())
        );
    }

    #[test]
    fn rejects_default_except_before_typed_handler() {
        let tokens =
            lex("try:\n    pass\nexcept:\n    pass\nexcept ValueError:\n    pass").unwrap();

        assert_eq!(
            parse(&tokens),
            Err("default 'except:' must be last".to_string())
        );
    }

    #[test]
    fn parses_with_statement() {
        let tokens = lex("with manager() as value:\n    print(value)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::With {
                    items: vec![WithItem {
                        context_expr: Expr::Call {
                            callee: Box::new(Expr::Name("manager".to_string())),
                            args: Vec::new(),
                        },
                        optional_vars: Some(Target::Name("value".to_string())),
                    }],
                    body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::Name("value".to_string())],
                    })],
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_parenthesized_with_items() {
        let tokens =
            lex("with (\n    manager(\"a\") as a,\n    manager(\"b\") as b,\n):\n    print(a, b)")
                .unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::With { items, body, .. }] => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0].context_expr, Expr::Call { .. }));
                assert_eq!(items[0].optional_vars, Some(Target::Name("a".to_string())));
                assert!(matches!(items[1].context_expr, Expr::Call { .. }));
                assert_eq!(items[1].optional_vars, Some(Target::Name("b".to_string())));
                assert_eq!(body.len(), 1);
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_grouped_with_item_as_target() {
        let tokens = lex("with (manager()) as value:\n    print(value)").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::With { items, body, .. }] => {
                assert_eq!(items.len(), 1);
                assert!(matches!(items[0].context_expr, Expr::Call { .. }));
                assert_eq!(
                    items[0].optional_vars,
                    Some(Target::Name("value".to_string()))
                );
                assert_eq!(body.len(), 1);
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_match_statement() {
        let tokens =
            lex("match x:\n    case 1:\n        print(\"one\")\n    case _:\n        pass")
                .unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Match {
                    subject: Expr::Name("x".to_string()),
                    cases: vec![
                        MatchCase {
                            pattern: Pattern::Literal(Expr::Number(1)),
                            guard: None,
                            body: vec![Stmt::Expr(Expr::Call {
                                callee: Box::new(Expr::Name("print".to_string())),
                                args: vec![Expr::String("one".to_string())],
                            })],
                        },
                        MatchCase {
                            pattern: Pattern::Wildcard,
                            guard: None,
                            body: vec![Stmt::Pass],
                        },
                    ],
                }],
            })
        );
    }

    #[test]
    fn parses_match_complex_literal_pattern() {
        let tokens = lex("match x:\n    case -0.25 + 1.75j:\n        pass").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Match {
                    subject: Expr::Name("x".to_string()),
                    cases: vec![MatchCase {
                        pattern: Pattern::Literal(Expr::Binary {
                            left: Box::new(Expr::Unary {
                                op: UnaryOp::Negative,
                                operand: Box::new(Expr::Float("0.25".to_string())),
                            }),
                            op: BinaryOp::Add,
                            right: Box::new(Expr::Imaginary("1.75".to_string())),
                        }),
                        guard: None,
                        body: vec![Stmt::Pass],
                    }],
                }],
            })
        );
    }

    #[test]
    fn parses_match_class_pattern() {
        let tokens = lex("match point:\n    case Point(0, y=value):\n        pass").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Match {
                    subject: Expr::Name("point".to_string()),
                    cases: vec![MatchCase {
                        pattern: Pattern::Class {
                            class: Expr::Name("Point".to_string()),
                            positional: vec![Pattern::Literal(Expr::Number(0))],
                            keywords: vec![(
                                "y".to_string(),
                                Pattern::Capture("value".to_string())
                            )],
                        },
                        guard: None,
                        body: vec![Stmt::Pass],
                    }],
                }],
            })
        );
    }

    #[test]
    fn parses_multiple_with_items() {
        let tokens = lex("with first() as a, second() as b:\n    pass").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::With {
                    items: vec![
                        WithItem {
                            context_expr: Expr::Call {
                                callee: Box::new(Expr::Name("first".to_string())),
                                args: Vec::new(),
                            },
                            optional_vars: Some(Target::Name("a".to_string())),
                        },
                        WithItem {
                            context_expr: Expr::Call {
                                callee: Box::new(Expr::Name("second".to_string())),
                                args: Vec::new(),
                            },
                            optional_vars: Some(Target::Name("b".to_string())),
                        },
                    ],
                    body: vec![Stmt::Pass],
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_import_statement() {
        let tokens = lex("import sys, math as m").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Import {
                    is_lazy: false,
                    aliases: vec![
                        ImportAlias {
                            name: "sys".to_string(),
                            asname: None,
                        },
                        ImportAlias {
                            name: "math".to_string(),
                            asname: Some("m".to_string()),
                        },
                    ],
                }],
            })
        );
    }

    #[test]
    fn parses_import_from_statement() {
        let tokens = lex("from sys import (path, argv,)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::ImportFrom {
                    is_lazy: false,
                    module: Some("sys".to_string()),
                    level: 0,
                    targets: ImportFromTargets::Aliases(vec![
                        ImportAlias {
                            name: "path".to_string(),
                            asname: None,
                        },
                        ImportAlias {
                            name: "argv".to_string(),
                            asname: None,
                        },
                    ]),
                }],
            })
        );
    }

    #[test]
    fn parses_import_from_star_statement() {
        let tokens = lex("from math import *").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::ImportFrom {
                    is_lazy: false,
                    module: Some("math".to_string()),
                    level: 0,
                    targets: ImportFromTargets::Star,
                }],
            })
        );
    }

    #[test]
    fn parses_relative_import_ellipsis_levels() {
        let tokens = lex("from ...pkg import name\nfrom .... import value").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![
                    Stmt::ImportFrom {
                        is_lazy: false,
                        module: Some("pkg".to_string()),
                        level: 3,
                        targets: ImportFromTargets::Aliases(vec![ImportAlias {
                            name: "name".to_string(),
                            asname: None,
                        }]),
                    },
                    Stmt::ImportFrom {
                        is_lazy: false,
                        module: None,
                        level: 4,
                        targets: ImportFromTargets::Aliases(vec![ImportAlias {
                            name: "value".to_string(),
                            asname: None,
                        }]),
                    },
                ],
            })
        );
    }

    #[test]
    fn rejects_invalid_import_forms_with_cpython_messages() {
        let tokens = lex("import").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("Expected one or more names after 'import'".to_string())
        );

        let tokens = lex("from sys import").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("Expected one or more names after 'import'".to_string())
        );

        let tokens = lex("from sys import path,").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("trailing comma not allowed without surrounding parentheses".to_string())
        );

        let tokens = lex("import sys from time").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("Did you mean to use 'from ... import ...' instead?".to_string())
        );

        let tokens = lex("import sys as 1").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use literal as import target".to_string())
        );

        let tokens = lex("import __debug__").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot assign to __debug__".to_string())
        );

        let tokens = lex("import sys as __debug__").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot assign to __debug__".to_string())
        );

        let tokens = lex("import sys as alias.name").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use attribute as import target".to_string())
        );

        let tokens = lex("import sys as alias()").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use function call as import target".to_string())
        );

        let tokens = lex("import sys as (alias, other)").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use tuple as import target".to_string())
        );

        let tokens = lex("from sys import path as None").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use literal as import target".to_string())
        );

        let tokens = lex("from sys import __debug__").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot assign to __debug__".to_string())
        );

        let tokens = lex("from sys import path as __debug__").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot assign to __debug__".to_string())
        );

        let tokens = lex("from sys import (path as alias.name)").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use attribute as import target".to_string())
        );

        let tokens = lex("from sys import (path as func())").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use function call as import target".to_string())
        );

        let tokens = lex("from sys import (path as [])").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use list as import target".to_string())
        );

        let tokens = lex("from sys import (path as ())").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use tuple as import target".to_string())
        );

        let tokens = lex("from sys import path as alias[0]").unwrap();
        assert_eq!(
            parse(&tokens),
            Err("cannot use subscript as import target".to_string())
        );
    }

    #[test]
    fn parses_underscore_relative_import_module_after_dot() {
        let tokens = lex("from ._threading_handler import install_threading_hook").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::ImportFrom {
                    is_lazy: false,
                    module: Some("_threading_handler".to_string()),
                    level: 1,
                    targets: ImportFromTargets::Aliases(vec![ImportAlias {
                        name: "install_threading_hook".to_string(),
                        asname: None,
                    }]),
                }],
            })
        );
    }

    #[test]
    fn parses_lazy_import_statements() {
        let tokens = lex("lazy import sys as system\nlazy from math import sqrt").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![
                    Stmt::Import {
                        is_lazy: true,
                        aliases: vec![ImportAlias {
                            name: "sys".to_string(),
                            asname: Some("system".to_string()),
                        }],
                    },
                    Stmt::ImportFrom {
                        is_lazy: true,
                        module: Some("math".to_string()),
                        level: 0,
                        targets: ImportFromTargets::Aliases(vec![ImportAlias {
                            name: "sqrt".to_string(),
                            asname: None,
                        }]),
                    },
                ],
            })
        );
    }

    #[test]
    fn parses_power_as_right_associative() {
        let tokens = lex("2 ** 3 ** 2").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Binary {
                    left: Box::new(Expr::Number(2)),
                    op: BinaryOp::Power,
                    right: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(3)),
                        op: BinaryOp::Power,
                        right: Box::new(Expr::Number(2)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_unary_minus_below_power() {
        let tokens = lex("-2 ** 2").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Unary {
                    op: UnaryOp::Negative,
                    operand: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::Power,
                        right: Box::new(Expr::Number(2)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_multiple_call_arguments() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::Number(1),
            Token::Comma,
            Token::Number(2),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Number(1), Expr::Number(2)],
                })],
            })
        );
    }

    #[test]
    fn parses_assignment_statement() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Equal,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Assign {
                    targets: vec![Target::Name("x".to_string())],
                    value: Expr::Binary {
                        left: Box::new(Expr::Number(1)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(2)),
                    },
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_chained_assignment_statement() {
        let tokens = lex("a = b = 3").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Assign {
                    targets: vec![Target::Name("a".to_string()), Target::Name("b".to_string()),],
                    value: Expr::Number(3),
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_annotated_assignment_statement() {
        let tokens = lex("x: int = 1").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::AnnAssign {
                    target: Target::Name("x".to_string()),
                    annotation: Expr::Name("int".to_string()),
                    value: Some(Expr::Number(1)),
                    simple: true,
                }],
            })
        );
    }

    #[test]
    fn parses_parenthesized_annotated_assignment_as_non_simple() {
        let tokens = lex("(x): int").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::AnnAssign {
                    target: Target::Name("x".to_string()),
                    annotation: Expr::Name("int".to_string()),
                    value: None,
                    simple: false,
                }],
            })
        );
    }

    #[test]
    fn parses_augmented_assignment_statement() {
        let tokens = lex("x += 2").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::AugAssign {
                    target: Target::Name("x".to_string()),
                    op: BinaryOp::Add,
                    value: Expr::Number(2),
                }],
            })
        );
    }

    #[test]
    fn parses_matrix_augmented_assignment_statement() {
        let tokens = lex("x @= y").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::AugAssign {
                    target: Target::Name("x".to_string()),
                    op: BinaryOp::MatrixMultiply,
                    value: Expr::Name("y".to_string()),
                }],
            })
        );
    }

    #[test]
    fn parses_slice_subscript_assignment_target() {
        let tokens = lex("items[1:3, ::-1] = value").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Assign {
                    targets: vec![Target::Subscript {
                        object: Box::new(Expr::Name("items".to_string())),
                        index: Expr::Tuple(vec![
                            Expr::SliceLiteral {
                                start: Some(Box::new(Expr::Number(1))),
                                stop: Some(Box::new(Expr::Number(3))),
                                step: None,
                            },
                            Expr::SliceLiteral {
                                start: None,
                                stop: None,
                                step: Some(Box::new(Expr::Unary {
                                    op: UnaryOp::Negative,
                                    operand: Box::new(Expr::Number(1)),
                                })),
                            },
                        ]),
                    }],
                    value: Expr::Name("value".to_string()),
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_tuple_assignment_statement() {
        let tokens = lex("a, b = (1, 2)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Assign {
                    targets: vec![Target::Tuple(vec![
                        Target::Name("a".to_string()),
                        Target::Name("b".to_string()),
                    ])],
                    value: Expr::Tuple(vec![Expr::Number(1), Expr::Number(2)]),
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_starred_tuple_assignment_statement() {
        let tokens = lex("a, *rest, b = values").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Assign {
                    targets: vec![Target::Tuple(vec![
                        Target::Name("a".to_string()),
                        Target::Starred(Box::new(Target::Name("rest".to_string()))),
                        Target::Name("b".to_string()),
                    ])],
                    value: Expr::Name("values".to_string()),
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_naked_tuple_expression() {
        let tokens = lex("1, 2, 3").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Tuple(vec![
                    Expr::Number(1),
                    Expr::Number(2),
                    Expr::Number(3),
                ]))],
            })
        );
    }

    #[test]
    fn parses_string_expression() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::String("hello".to_string()),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::String("hello".to_string())],
                })],
            })
        );
    }

    #[test]
    fn parses_adjacent_string_literals_as_one_expression() {
        let tokens = lex("\"mini\" 'python'").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::String("minipython".to_string()))],
            })
        );
    }

    #[test]
    fn parses_f_string_expression() {
        let tokens = lex("f\"hello {name!r} {1 + 2}\"").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::JoinedString(vec![
                    FStringPart::Literal("hello ".to_string()),
                    FStringPart::Formatted {
                        value: Box::new(Expr::Name("name".to_string())),
                        conversion: Some(FStringConversion::Repr),
                        format_spec: None,
                    },
                    FStringPart::Literal(" ".to_string()),
                    FStringPart::Formatted {
                        value: Box::new(Expr::Binary {
                            left: Box::new(Expr::Number(1)),
                            op: BinaryOp::Add,
                            right: Box::new(Expr::Number(2)),
                        }),
                        conversion: None,
                        format_spec: None,
                    },
                ]))],
            })
        );
    }

    #[test]
    fn parses_literal_only_f_string_as_joined_string() {
        assert_eq!(
            parse(&lex("f\"hello\"").unwrap()),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::JoinedString(vec![FStringPart::Literal(
                    "hello".to_string()
                )]))],
            })
        );
        assert_eq!(
            parse(&lex("\"mini\" f\"python\"").unwrap()),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::JoinedString(vec![FStringPart::Literal(
                    "minipython".to_string()
                )]))],
            })
        );
    }

    #[test]
    fn parses_adjacent_f_strings_and_plain_strings() {
        let tokens = vec![
            Token::FString(vec![
                TokenFStringPart::Literal("hello ".to_string()),
                TokenFStringPart::Expression {
                    source: "name".to_string(),
                    conversion: Some(TokenFStringConversion::Str),
                    format_spec: None,
                    debug_label: None,
                },
            ]),
            Token::String("!".to_string()),
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::JoinedString(vec![
                    FStringPart::Literal("hello ".to_string()),
                    FStringPart::Formatted {
                        value: Box::new(Expr::Name("name".to_string())),
                        conversion: Some(FStringConversion::Str),
                        format_spec: None,
                    },
                    FStringPart::Literal("!".to_string()),
                ]))],
            })
        );
    }

    #[test]
    fn parses_f_string_format_spec() {
        let tokens = lex("f\"{value:{width}.2f}\"").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::JoinedString(vec![
                    FStringPart::Formatted {
                        value: Box::new(Expr::Name("value".to_string())),
                        conversion: None,
                        format_spec: Some(vec![
                            FStringPart::Formatted {
                                value: Box::new(Expr::Name("width".to_string())),
                                conversion: None,
                                format_spec: None,
                            },
                            FStringPart::Literal(".2f".to_string()),
                        ]),
                    }
                ]))],
            })
        );
    }

    #[test]
    fn detects_unparenthesized_f_string_lambda_format_sources() {
        assert!(starts_unparenthesized_lambda_expression("lambda x"));
        assert!(starts_unparenthesized_lambda_expression("lambda "));
        assert!(starts_unparenthesized_lambda_expression("1, lambda"));
        assert!(starts_unparenthesized_lambda_expression("lambda *arg, "));
        assert!(!starts_unparenthesized_lambda_expression("+ lambda"));
        assert!(!starts_unparenthesized_lambda_expression("\"a, lambda\""));
        assert!(!starts_unparenthesized_lambda_expression("[lambda]"));
    }

    #[test]
    fn parses_f_string_debug_expression() {
        let tokens = lex("f\"{name=} {name=!s}\"").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::JoinedString(vec![
                    FStringPart::Literal("name=".to_string()),
                    FStringPart::Formatted {
                        value: Box::new(Expr::Name("name".to_string())),
                        conversion: Some(FStringConversion::Repr),
                        format_spec: None,
                    },
                    FStringPart::Literal(" name=".to_string()),
                    FStringPart::Formatted {
                        value: Box::new(Expr::Name("name".to_string())),
                        conversion: Some(FStringConversion::Str),
                        format_spec: None,
                    },
                ]))],
            })
        );
    }

    #[test]
    fn parses_list_expression() {
        let tokens = lex("[1, 2 + 3,]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::List(vec![
                    Expr::Number(1),
                    Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(3)),
                    },
                ]))],
            })
        );
    }

    #[test]
    fn parses_empty_list_expression() {
        let tokens = lex("[]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::List(Vec::new()))],
            })
        );
    }

    #[test]
    fn parses_list_comprehension_expression() {
        let tokens = lex("[x * 2 for x in items if x > 1]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_async_list_comprehension_clause() {
        let tokens = lex("[x async for x in items if x > 1]").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::Expr(Expr::ListComp { element, clauses })] => {
                assert_eq!(element.as_ref(), &Expr::Name("x".to_string()));
                assert_eq!(clauses.len(), 1);
                assert!(clauses[0].is_async);
                assert_eq!(clauses[0].target, Target::Name("x".to_string()));
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_nested_list_comprehension_clauses() {
        let tokens = lex("[(x, y) for x in xs for y in ys]").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::Expr(Expr::ListComp { clauses, .. })] => {
                assert_eq!(clauses.len(), 2);
                assert_eq!(clauses[0].target, Target::Name("x".to_string()));
                assert_eq!(clauses[1].target, Target::Name("y".to_string()));
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_decorated_function_statement() {
        let tokens = lex("@decorator\n@lambda f: f\ndef f():\n    return 1").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [
                Stmt::FunctionDef {
                    name,
                    params,
                    body,
                    decorators,
                    returns,
                    ..
                },
            ] => {
                assert_eq!(name, "f");
                assert_eq!(params, &FunctionParams::default());
                assert_eq!(body.len(), 1);
                assert_eq!(decorators.len(), 2);
                assert_eq!(returns, &None);
                assert_eq!(decorators[0], Expr::Name("decorator".to_string()));
                assert!(matches!(decorators[1], Expr::Lambda { .. }));
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_decorated_function_after_blank_line() {
        let tokens = lex("@classmethod\n\ndef f(cls):\n    return 1").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [
                Stmt::FunctionDef {
                    name, decorators, ..
                },
            ] => {
                assert_eq!(name, "f");
                assert_eq!(decorators, &vec![Expr::Name("classmethod".to_string())]);
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_async_function_statement() {
        let tokens = lex("async def f():\n    return await g()").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [
                Stmt::AsyncFunctionDef {
                    name, params, body, ..
                },
            ] => {
                assert_eq!(name, "f");
                assert_eq!(params, &FunctionParams::default());
                match &body[..] {
                    [Stmt::Return(Some(Expr::Await(expr)))] => {
                        assert!(matches!(expr.as_ref(), Expr::Call { .. }));
                    }
                    statements => panic!("unexpected async function body: {statements:?}"),
                }
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_async_with_statement() {
        let tokens = lex("async with manager() as value:\n    print(value)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::AsyncWith {
                    items: vec![WithItem {
                        context_expr: Expr::Call {
                            callee: Box::new(Expr::Name("manager".to_string())),
                            args: Vec::new(),
                        },
                        optional_vars: Some(Target::Name("value".to_string())),
                    }],
                    body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::Name("value".to_string())],
                    })],
                    type_comment: None,
                }],
            })
        );
    }

    #[test]
    fn parses_function_annotations() {
        let tokens = lex(
            "def f(x: int, *args: str, y: bool = True, **kwargs: list) -> float:\n    return x",
        )
        .unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [
                Stmt::FunctionDef {
                    name,
                    params,
                    returns,
                    ..
                },
            ] => {
                assert_eq!(name, "f");
                assert_eq!(returns, &Some(Expr::Name("float".to_string())));
                assert_eq!(
                    params.positional,
                    vec![Param {
                        name: "x".to_string(),
                        annotation: Some(Expr::Name("int".to_string())),
                        default: None,
                        type_comment: None,
                    }]
                );
                assert_eq!(params.vararg, Some("args".to_string()));
                assert_eq!(
                    params.vararg_annotation.as_deref(),
                    Some(&Expr::Name("str".to_string()))
                );
                assert_eq!(
                    params.keyword_only,
                    vec![Param {
                        name: "y".to_string(),
                        annotation: Some(Expr::Name("bool".to_string())),
                        default: Some(Expr::Bool(true)),
                        type_comment: None,
                    }]
                );
                assert_eq!(params.kwarg, Some("kwargs".to_string()));
                assert_eq!(
                    params.kwarg_annotation.as_deref(),
                    Some(&Expr::Name("list".to_string()))
                );
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_positional_only_parameters() {
        let tokens = lex("def f(a, b: int = 2, /, c=3, *, d):\n    pass").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::FunctionDef { params, .. }] => {
                assert_eq!(
                    params.positional_only,
                    vec![
                        Param {
                            name: "a".to_string(),
                            annotation: None,
                            default: None,
                            type_comment: None,
                        },
                        Param {
                            name: "b".to_string(),
                            annotation: Some(Expr::Name("int".to_string())),
                            default: Some(Expr::Number(2)),
                            type_comment: None,
                        },
                    ]
                );
                assert_eq!(
                    params.positional,
                    vec![Param {
                        name: "c".to_string(),
                        annotation: None,
                        default: Some(Expr::Number(3)),
                        type_comment: None,
                    }]
                );
                assert_eq!(
                    params.keyword_only,
                    vec![Param {
                        name: "d".to_string(),
                        annotation: None,
                        default: None,
                        type_comment: None,
                    }]
                );
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_decorated_class_statement() {
        let tokens = lex("@identity\nclass C:\n    pass").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [
                Stmt::ClassDef {
                    name,
                    body,
                    decorators,
                    ..
                },
            ] => {
                assert_eq!(name, "C");
                assert_eq!(body, &vec![Stmt::Pass]);
                assert_eq!(decorators, &vec![Expr::Name("identity".to_string())]);
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_generator_expression() {
        let tokens = lex("(x * 2 for x in items if x > 1)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_call_with_unparenthesized_generator_expression() {
        let tokens = lex("next(x for x in range(3))").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::Expr(Expr::Call { callee, args })] => {
                assert_eq!(callee.as_ref(), &Expr::Name("next".to_string()));
                match &args[..] {
                    [Expr::GeneratorComp { element, clauses }] => {
                        assert_eq!(element.as_ref(), &Expr::Name("x".to_string()));
                        assert_eq!(clauses.len(), 1);
                        assert_eq!(clauses[0].target, Target::Name("x".to_string()));
                    }
                    args => panic!("expected one generator expression arg, found {args:?}"),
                }
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_dict_expression() {
        let tokens = lex("{\"a\": 1, \"b\": 2 + 3,}").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_dict_unpack_expression() {
        let tokens = lex("{**base, \"x\": 1, **override}").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Dict(vec![
                    DictItem::Unpack(Expr::Name("base".to_string())),
                    DictItem::Entry {
                        key: Expr::String("x".to_string()),
                        value: Expr::Number(1),
                    },
                    DictItem::Unpack(Expr::Name("override".to_string())),
                ]))],
            })
        );
    }

    #[test]
    fn parses_dict_comprehension_expression() {
        let tokens = lex("{x: x * 2 for x in items if x > 1}").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_set_expression() {
        let tokens = lex("{1, 2 + 3,}").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Set(vec![
                    Expr::Number(1),
                    Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(3)),
                    },
                ]))],
            })
        );
    }

    #[test]
    fn parses_set_comprehension_expression() {
        let tokens = lex("{x * 2 for x in items if x > 1}").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_tuple_expression() {
        let tokens = lex("(1, 2 + 3,)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Tuple(vec![
                    Expr::Number(1),
                    Expr::Binary {
                        left: Box::new(Expr::Number(2)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(3)),
                    },
                ]))],
            })
        );
    }

    #[test]
    fn parses_empty_tuple_expression() {
        let tokens = lex("()").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Tuple(Vec::new()))],
            })
        );
    }

    #[test]
    fn parses_singleton_tuple_expression() {
        let tokens = lex("(1,)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Tuple(vec![Expr::Number(1)]))],
            })
        );
    }

    #[test]
    fn parses_subscript_expression() {
        let tokens = lex("items[0]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Subscript {
                    object: Box::new(Expr::Name("items".to_string())),
                    index: Box::new(Expr::Number(0)),
                })],
            })
        );
    }

    #[test]
    fn parses_chained_subscript_expression() {
        let tokens = lex("[[1]][0][0]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Subscript {
                    object: Box::new(Expr::Subscript {
                        object: Box::new(Expr::List(vec![Expr::List(vec![Expr::Number(1)])])),
                        index: Box::new(Expr::Number(0)),
                    }),
                    index: Box::new(Expr::Number(0)),
                })],
            })
        );
    }

    #[test]
    fn parses_slice_expression() {
        let tokens = lex("items[1:3]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Subscript {
                    object: Box::new(Expr::Name("items".to_string())),
                    index: Box::new(Expr::SliceLiteral {
                        start: Some(Box::new(Expr::Number(1))),
                        stop: Some(Box::new(Expr::Number(3))),
                        step: None,
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_slice_expression_with_omitted_parts() {
        let tokens = lex("items[::2]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Subscript {
                    object: Box::new(Expr::Name("items".to_string())),
                    index: Box::new(Expr::SliceLiteral {
                        start: None,
                        stop: None,
                        step: Some(Box::new(Expr::Number(2))),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_multi_item_slice_subscript_expression() {
        let tokens = lex("items[1:3, ::-1, 2]").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Subscript {
                    object: Box::new(Expr::Name("items".to_string())),
                    index: Box::new(Expr::Tuple(vec![
                        Expr::SliceLiteral {
                            start: Some(Box::new(Expr::Number(1))),
                            stop: Some(Box::new(Expr::Number(3))),
                            step: None,
                        },
                        Expr::SliceLiteral {
                            start: None,
                            stop: None,
                            step: Some(Box::new(Expr::Unary {
                                op: UnaryOp::Negative,
                                operand: Box::new(Expr::Number(1)),
                            })),
                        },
                        Expr::Number(2),
                    ])),
                })],
            })
        );
    }

    #[test]
    fn parses_boolean_expression() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::True,
            Token::Comma,
            Token::False,
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Bool(true), Expr::Bool(false)],
                })],
            })
        );
    }

    #[test]
    fn parses_equality_comparison_after_addition() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::EqualEqual,
            Token::Number(3),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Comparison {
                        left: Box::new(Expr::Binary {
                            left: Box::new(Expr::Number(1)),
                            op: BinaryOp::Add,
                            right: Box::new(Expr::Number(2)),
                        }),
                        op: ComparisonOp::Equal,
                        right: Box::new(Expr::Number(3)),
                    }],
                })],
            })
        );
    }

    #[test]
    fn parses_ordering_comparison() {
        let tokens = vec![Token::Number(1), Token::Less, Token::Number(2), Token::Eof];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Comparison {
                    left: Box::new(Expr::Number(1)),
                    op: ComparisonOp::Less,
                    right: Box::new(Expr::Number(2)),
                })],
            })
        );
    }

    #[test]
    fn parses_none_expression() {
        let tokens = vec![Token::None, Token::Eof];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::None)],
            })
        );
    }

    #[test]
    fn parses_membership_comparisons() {
        let tokens = vec![
            Token::Number(1),
            Token::In,
            Token::LeftBracket,
            Token::Number(1),
            Token::RightBracket,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Comparison {
                    left: Box::new(Expr::Number(1)),
                    op: ComparisonOp::In,
                    right: Box::new(Expr::List(vec![Expr::Number(1)])),
                })],
            })
        );

        let tokens = vec![
            Token::Number(1),
            Token::Not,
            Token::In,
            Token::LeftBracket,
            Token::Number(2),
            Token::RightBracket,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Comparison {
                    left: Box::new(Expr::Number(1)),
                    op: ComparisonOp::NotIn,
                    right: Box::new(Expr::List(vec![Expr::Number(2)])),
                })],
            })
        );
    }

    #[test]
    fn parses_identity_comparisons() {
        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Is,
            Token::None,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Comparison {
                    left: Box::new(Expr::Name("x".to_string())),
                    op: ComparisonOp::Is,
                    right: Box::new(Expr::None),
                })],
            })
        );

        let tokens = vec![
            Token::Identifier("x".to_string()),
            Token::Is,
            Token::Not,
            Token::None,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Comparison {
                    left: Box::new(Expr::Name("x".to_string())),
                    op: ComparisonOp::IsNot,
                    right: Box::new(Expr::None),
                })],
            })
        );
    }

    #[test]
    fn parses_chained_comparison() {
        let tokens = vec![
            Token::Number(1),
            Token::Less,
            Token::Number(2),
            Token::Less,
            Token::Number(3),
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::ChainedComparison {
                    left: Box::new(Expr::Number(1)),
                    comparisons: vec![
                        (ComparisonOp::Less, Expr::Number(2)),
                        (ComparisonOp::Less, Expr::Number(3)),
                    ],
                })],
            })
        );
    }

    #[test]
    fn parses_not_expression() {
        let tokens = vec![Token::Not, Token::True, Token::Eof];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Unary {
                    op: UnaryOp::Not,
                    operand: Box::new(Expr::Bool(true)),
                })],
            })
        );
    }

    #[test]
    fn parses_and_before_or_expression() {
        let tokens = vec![
            Token::True,
            Token::Or,
            Token::False,
            Token::And,
            Token::True,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Logical {
                    left: Box::new(Expr::Bool(true)),
                    op: LogicalOp::Or,
                    right: Box::new(Expr::Logical {
                        left: Box::new(Expr::Bool(false)),
                        op: LogicalOp::And,
                        right: Box::new(Expr::Bool(true)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_if_statement() {
        let tokens = vec![
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
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::If {
                    condition: Expr::Bool(true),
                    then_body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::String("yes".to_string())],
                    })],
                    else_body: Vec::new(),
                }],
            })
        );
    }

    #[test]
    fn parses_if_else_statement() {
        let tokens = vec![
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
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::If {
                    condition: Expr::Bool(false),
                    then_body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::String("no".to_string())],
                    })],
                    else_body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::String("yes".to_string())],
                    })],
                }],
            })
        );
    }

    #[test]
    fn parses_elif_as_nested_if_in_else_body() {
        let tokens =
            lex("if False:\n    pass\nelif True:\n    print(\"elif\")\nelse:\n    print(\"else\")")
                .unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [
                Stmt::If {
                    condition,
                    then_body,
                    else_body,
                },
            ] => {
                assert_eq!(condition, &Expr::Bool(false));
                assert_eq!(then_body, &vec![Stmt::Pass]);

                match &else_body[..] {
                    [
                        Stmt::If {
                            condition,
                            then_body,
                            else_body,
                        },
                    ] => {
                        assert_eq!(condition, &Expr::Bool(true));
                        assert_eq!(then_body.len(), 1);
                        assert_eq!(else_body.len(), 1);
                    }
                    statements => panic!("expected nested elif if, found {statements:?}"),
                }
            }
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_while_statement() {
        let tokens = lex("while True:\n    print(\"loop\")").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::While {
                    condition: Expr::Bool(true),
                    body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::String("loop".to_string())],
                    })],
                    else_body: Vec::new(),
                }],
            })
        );
    }

    #[test]
    fn parses_while_else_statement() {
        let tokens = lex("while False:\n    pass\nelse:\n    print(\"done\")").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::While {
                    condition: Expr::Bool(false),
                    body: vec![Stmt::Pass],
                    else_body: vec![Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::String("done".to_string())],
                    })],
                }],
            })
        );
    }

    #[test]
    fn parses_for_statement() {
        let tokens = lex("for x in range(3):\n    print(x)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_for_else_statement() {
        let tokens = lex("for x in range(0):\n    pass\nelse:\n    print(\"done\")").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_break_and_continue_statements() {
        let tokens = lex("while True:\n    continue\n    break").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::While {
                    condition: Expr::Bool(true),
                    body: vec![Stmt::Continue, Stmt::Break],
                    else_body: Vec::new(),
                }],
            })
        );
    }

    #[test]
    fn parses_pass_statement() {
        let tokens = vec![Token::Pass, Token::Eof];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Pass],
            })
        );
    }

    #[test]
    fn parses_statement_after_dedent() {
        let tokens = lex("if True:\n    print(\"inside\")\nprint(\"after\")").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::If { then_body, .. }, Stmt::Expr(_)] => assert_eq!(then_body.len(), 1),
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn parses_nested_if_followed_by_statement_in_same_block() {
        let tokens =
            lex("if True:\n    if False:\n        print(\"skip\")\n    print(\"after\")").unwrap();
        let program = parse(&tokens).unwrap();

        match &program.statements[..] {
            [Stmt::If { then_body, .. }] => match &then_body[..] {
                [Stmt::If { .. }, Stmt::Expr(_)] => {}
                statements => panic!("unexpected then body: {statements:?}"),
            },
            statements => panic!("unexpected statements: {statements:?}"),
        }
    }

    #[test]
    fn rejects_if_missing_colon() {
        let tokens = vec![Token::If, Token::True, Token::Newline, Token::Eof];

        assert_eq!(
            parse(&tokens),
            Err("expected ':', found Newline".to_string())
        );
    }

    #[test]
    fn rejects_if_missing_indent() {
        let tokens = vec![
            Token::If,
            Token::True,
            Token::Colon,
            Token::Newline,
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::String("yes".to_string()),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Err("expected an indented block".to_string())
        );
    }

    #[test]
    fn parses_grouped_expression() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::LeftParen,
            Token::Number(2),
            Token::Plus,
            Token::Number(3),
            Token::RightParen,
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Call {
                    callee: Box::new(Expr::Name("print".to_string())),
                    args: vec![Expr::Binary {
                        left: Box::new(Expr::Number(1)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Binary {
                            left: Box::new(Expr::Number(2)),
                            op: BinaryOp::Add,
                            right: Box::new(Expr::Number(3)),
                        }),
                    }],
                })],
            })
        );
    }

    #[test]
    fn parses_nested_grouped_expression_without_ast_wrapper() {
        let tokens = vec![
            Token::LeftParen,
            Token::LeftParen,
            Token::Number(1),
            Token::RightParen,
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::Number(1))],
            })
        );
    }

    #[test]
    fn parses_named_expression_in_grouped_expression() {
        let tokens = lex("(x := 1 + 2)").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::Expr(Expr::NamedExpr {
                    name: "x".to_string(),
                    value: Box::new(Expr::Binary {
                        left: Box::new(Expr::Number(1)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(2)),
                    }),
                })],
            })
        );
    }

    #[test]
    fn parses_named_expression_in_if_condition() {
        let tokens = lex("if x := 1:\n    pass").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![Stmt::If {
                    condition: Expr::NamedExpr {
                        name: "x".to_string(),
                        value: Box::new(Expr::Number(1)),
                    },
                    then_body: vec![Stmt::Pass],
                    else_body: Vec::new(),
                }],
            })
        );
    }

    #[test]
    fn rejects_bare_named_expression_statement() {
        let tokens = lex("x := 1").unwrap();

        assert_eq!(
            parse(&tokens),
            Err("expected statement separator or end of input, found ColonEqual".to_string())
        );
    }

    #[test]
    fn rejects_missing_expression_before_tuple_comma() {
        let tokens = vec![
            Token::LeftParen,
            Token::Comma,
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Err("expected expression, found Comma".to_string())
        );
    }

    #[test]
    fn rejects_unclosed_grouped_expression() {
        let tokens = vec![
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::Eof,
        ];

        assert_eq!(parse(&tokens), Err("'(' was never closed".to_string()));
    }

    #[test]
    fn parses_multiple_statements() {
        let tokens = vec![
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
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn parses_semicolon_separated_simple_statements() {
        let tokens = lex("x = 1; print(x);").unwrap();

        assert_eq!(
            parse(&tokens),
            Ok(Program {
                statements: vec![
                    Stmt::Assign {
                        targets: vec![Target::Name("x".to_string())],
                        value: Expr::Number(1),
                        type_comment: None,
                    },
                    Stmt::Expr(Expr::Call {
                        callee: Box::new(Expr::Name("print".to_string())),
                        args: vec![Expr::Name("x".to_string())],
                    }),
                ],
            })
        );
    }

    #[test]
    fn skips_blank_lines_between_statements() {
        let tokens = vec![
            Token::Newline,
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
            Token::Newline,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Program {
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
            })
        );
    }

    #[test]
    fn rejects_missing_right_paren() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::LeftParen,
            Token::Number(123),
            Token::Eof,
        ];

        assert_eq!(parse(&tokens), Err("'(' was never closed".to_string()));
    }

    #[test]
    fn rejects_extra_tokens_after_name() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::Number(123),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(parse(&tokens), Err("invalid syntax".to_string()));
    }
}
