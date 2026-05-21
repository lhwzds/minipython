use crate::ast::{BinaryOp, ComparisonOp, Expr, Program, Stmt};
use crate::lexer::Token;

pub fn parse(tokens: &[Token]) -> Result<Program, String> {
    let mut parser = Parser { tokens, current: 0 };
    let program = parser.parse_program()?;
    parser.expect_eof()?;
    Ok(program)
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl Parser<'_> {
    fn parse_program(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();

        self.skip_newlines();
        while !matches!(self.peek(), Some(Token::Eof) | None) {
            statements.push(self.parse_statement()?);

            match self.peek() {
                Some(Token::Newline) => self.skip_newlines(),
                Some(Token::Eof) | None => {}
                Some(token) => {
                    return Err(format!("expected newline or end of input, found {token:?}"));
                }
            }
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Stmt, String> {
        if let (Some(Token::Identifier(name)), Some(Token::Equal)) = (self.peek(), self.peek_next())
        {
            let name = name.clone();
            self.advance();
            self.advance();
            let value = self.parse_expression()?;

            return Ok(Stmt::Assign { name, value });
        }

        let expr = self.parse_expression()?;
        Ok(Stmt::Expr(expr))
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_comparison()
    }

    fn parse_comparison(&mut self) -> Result<Expr, String> {
        let left = self.parse_addition()?;

        if matches!(self.peek(), Some(Token::EqualEqual)) {
            self.advance();
            let right = self.parse_addition()?;

            Ok(Expr::Comparison {
                left: Box::new(left),
                op: ComparisonOp::Equal,
                right: Box::new(right),
            })
        } else {
            Ok(left)
        }
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_call()?;

        while matches!(self.peek(), Some(Token::Plus)) {
            self.advance();
            let right = self.parse_call()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_call(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        while matches!(self.peek(), Some(Token::LeftParen)) {
            self.advance();
            let args = self.parse_arguments()?;
            self.expect_right_paren()?;

            expr = Expr::Call {
                callee: Box::new(expr),
                args,
            };
        }

        Ok(expr)
    }

    fn parse_arguments(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();

        if matches!(self.peek(), Some(Token::RightParen)) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);

            if !matches!(self.peek(), Some(Token::Comma)) {
                break;
            }

            self.advance();
        }

        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Some(Token::Number(value)) => Ok(Expr::Number(*value)),
            Some(Token::String(value)) => Ok(Expr::String(value.clone())),
            Some(Token::True) => Ok(Expr::Bool(true)),
            Some(Token::False) => Ok(Expr::Bool(false)),
            Some(Token::Identifier(name)) => Ok(Expr::Name(name.clone())),
            Some(Token::LeftParen) => {
                let expr = self.parse_expression()?;
                self.expect_right_paren()?;
                Ok(expr)
            }
            Some(token) => Err(format!("expected expression, found {token:?}")),
            None => Err("expected expression, found end of input".to_string()),
        }
    }

    fn expect_right_paren(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::RightParen) => Ok(()),
            Some(token) => Err(format!("expected ')', found {token:?}")),
            None => Err("expected ')', found end of input".to_string()),
        }
    }

    fn expect_eof(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Eof) => Ok(()),
            Some(token) => Err(format!("expected end of input, found {token:?}")),
            None => Err("expected end of input".to_string()),
        }
    }

    fn advance(&mut self) -> Option<&Token> {
        let token = self.tokens.get(self.current);
        self.current += 1;
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.current)
    }

    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.current + 1)
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token::Newline)) {
            self.advance();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::ast::{BinaryOp, ComparisonOp, Expr, Program, Stmt};
    use crate::lexer::Token;

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
                    name: "x".to_string(),
                    value: Expr::Binary {
                        left: Box::new(Expr::Number(1)),
                        op: BinaryOp::Add,
                        right: Box::new(Expr::Number(2)),
                    },
                }],
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
    fn rejects_empty_grouped_expression() {
        let tokens = vec![Token::LeftParen, Token::RightParen, Token::Eof];

        assert_eq!(
            parse(&tokens),
            Err("expected expression, found RightParen".to_string())
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

        assert_eq!(parse(&tokens), Err("expected ')', found Eof".to_string()));
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

        assert_eq!(parse(&tokens), Err("expected ')', found Eof".to_string()));
    }

    #[test]
    fn rejects_extra_tokens_after_name() {
        let tokens = vec![
            Token::Identifier("print".to_string()),
            Token::Number(123),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Err("expected newline or end of input, found Number(123)".to_string())
        );
    }
}
