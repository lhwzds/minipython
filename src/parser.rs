use crate::ast::{BinaryOp, Expr, Stmt};
use crate::lexer::Token;

pub fn parse(tokens: &[Token]) -> Result<Stmt, String> {
    let mut parser = Parser { tokens, current: 0 };
    let stmt = parser.parse_statement()?;
    parser.expect_eof()?;
    Ok(stmt)
}

struct Parser<'a> {
    tokens: &'a [Token],
    current: usize,
}

impl Parser<'_> {
    fn parse_statement(&mut self) -> Result<Stmt, String> {
        self.expect_print()?;
        self.expect_left_paren()?;
        let expr = self.parse_expression()?;
        self.expect_right_paren()?;

        Ok(Stmt::Print(expr))
    }

    fn parse_expression(&mut self) -> Result<Expr, String> {
        self.parse_addition()
    }

    fn parse_addition(&mut self) -> Result<Expr, String> {
        let mut expr = self.parse_primary()?;

        while matches!(self.peek(), Some(Token::Plus)) {
            self.advance();
            let right = self.parse_primary()?;
            expr = Expr::Binary {
                left: Box::new(expr),
                op: BinaryOp::Add,
                right: Box::new(right),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        match self.advance() {
            Some(Token::Number(value)) => Ok(Expr::Number(*value)),
            Some(token) => Err(format!("expected expression, found {token:?}")),
            None => Err("expected expression, found end of input".to_string()),
        }
    }

    fn expect_print(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::Print) => Ok(()),
            Some(token) => Err(format!("expected print, found {token:?}")),
            None => Err("expected print, found end of input".to_string()),
        }
    }

    fn expect_left_paren(&mut self) -> Result<(), String> {
        match self.advance() {
            Some(Token::LeftParen) => Ok(()),
            Some(token) => Err(format!("expected '(', found {token:?}")),
            None => Err("expected '(', found end of input".to_string()),
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
}

#[cfg(test)]
mod tests {
    use super::parse;
    use crate::ast::{BinaryOp, Expr, Stmt};
    use crate::lexer::Token;

    #[test]
    fn parses_print_number_statement() {
        let tokens = vec![
            Token::Print,
            Token::LeftParen,
            Token::Number(123),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(parse(&tokens), Ok(Stmt::Print(Expr::Number(123))));
    }

    #[test]
    fn parses_addition_expression() {
        let tokens = vec![
            Token::Print,
            Token::LeftParen,
            Token::Number(1),
            Token::Plus,
            Token::Number(2),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Ok(Stmt::Print(Expr::Binary {
                left: Box::new(Expr::Number(1)),
                op: BinaryOp::Add,
                right: Box::new(Expr::Number(2)),
            }))
        );
    }

    #[test]
    fn rejects_missing_right_paren() {
        let tokens = vec![
            Token::Print,
            Token::LeftParen,
            Token::Number(123),
            Token::Eof,
        ];

        assert_eq!(parse(&tokens), Err("expected ')', found Eof".to_string()));
    }

    #[test]
    fn rejects_missing_left_paren() {
        let tokens = vec![
            Token::Print,
            Token::Number(123),
            Token::RightParen,
            Token::Eof,
        ];

        assert_eq!(
            parse(&tokens),
            Err("expected '(', found Number(123)".to_string())
        );
    }
}
