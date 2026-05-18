#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Print,
    LeftParen,
    RightParen,
    Number(i64),
    Eof,
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut current = 0;

    while current < chars.len() {
        let ch = chars[current];

        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                current += 1;
            }
            '(' => {
                tokens.push(Token::LeftParen);
                current += 1;
            }
            ')' => {
                tokens.push(Token::RightParen);
                current += 1;
            }
            '0'..='9' => {
                let start = current;

                while current < chars.len() && chars[current].is_ascii_digit() {
                    current += 1;
                }

                let text: String = chars[start..current].iter().collect();
                let value = text
                    .parse::<i64>()
                    .map_err(|_| format!("invalid number: {text}"))?;
                tokens.push(Token::Number(value));
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                let start = current;

                while current < chars.len()
                    && (chars[current].is_ascii_alphanumeric() || chars[current] == '_')
                {
                    current += 1;
                }

                let word: String = chars[start..current].iter().collect();
                match word.as_str() {
                    "print" => tokens.push(Token::Print),
                    _ => return Err(format!("unexpected word: {word}")),
                }
            }
            _ => return Err(format!("unexpected character: {ch}")),
        }
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::{Token, lex};

    #[test]
    fn lexes_print_number() {
        assert_eq!(
            lex("print(123)"),
            Ok(vec![
                Token::Print,
                Token::LeftParen,
                Token::Number(123),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn skips_whitespace() {
        assert_eq!(
            lex("print( 123 )"),
            Ok(vec![
                Token::Print,
                Token::LeftParen,
                Token::Number(123),
                Token::RightParen,
                Token::Eof,
            ])
        );
    }

    #[test]
    fn rejects_unknown_character() {
        assert_eq!(lex("print(@)"), Err("unexpected character: @".to_string()));
    }
}
