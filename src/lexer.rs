#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Identifier(String),
    If,
    Else,
    Pass,
    LeftParen,
    RightParen,
    Comma,
    Colon,
    Newline,
    Indent,
    Dedent,
    Plus,
    Equal,
    EqualEqual,
    Number(i64),
    String(String),
    True,
    False,
    Eof,
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut current = 0;
    let mut at_line_start = true;
    let mut indent_stack = vec![0];

    while current < chars.len() {
        if at_line_start {
            let mut indent = 0;

            while current < chars.len() && chars[current] == ' ' {
                indent += 1;
                current += 1;
            }

            if current < chars.len() && chars[current] == '\t' {
                return Err("tabs in indentation are not supported".to_string());
            }

            if current >= chars.len() {
                break;
            }

            if chars[current] != '\n' && chars[current] != '\r' && chars[current] != '#' {
                update_indentation(indent, &mut indent_stack, &mut tokens)?;
                at_line_start = false;
            }
        }

        let ch = chars[current];

        match ch {
            ' ' | '\t' => {
                current += 1;
            }
            '\n' => {
                tokens.push(Token::Newline);
                current += 1;
                at_line_start = true;
            }
            '\r' => {
                tokens.push(Token::Newline);
                current += 1;
                if current < chars.len() && chars[current] == '\n' {
                    current += 1;
                }
                at_line_start = true;
            }
            '(' => {
                tokens.push(Token::LeftParen);
                current += 1;
            }
            ')' => {
                tokens.push(Token::RightParen);
                current += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                current += 1;
            }
            ':' => {
                tokens.push(Token::Colon);
                current += 1;
            }
            '+' => {
                tokens.push(Token::Plus);
                current += 1;
            }
            '=' => {
                current += 1;
                if current < chars.len() && chars[current] == '=' {
                    tokens.push(Token::EqualEqual);
                    current += 1;
                } else {
                    tokens.push(Token::Equal);
                }
            }
            '#' => {
                current += 1;
                while current < chars.len() && chars[current] != '\n' && chars[current] != '\r' {
                    current += 1;
                }
            }
            '"' => {
                current += 1;
                let start = current;

                while current < chars.len() && chars[current] != '"' {
                    if chars[current] == '\n' || chars[current] == '\r' {
                        return Err("unterminated string".to_string());
                    }
                    current += 1;
                }

                if current >= chars.len() {
                    return Err("unterminated string".to_string());
                }

                let value: String = chars[start..current].iter().collect();
                tokens.push(Token::String(value));
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
                    "if" => tokens.push(Token::If),
                    "else" => tokens.push(Token::Else),
                    "pass" => tokens.push(Token::Pass),
                    "True" => tokens.push(Token::True),
                    "False" => tokens.push(Token::False),
                    _ => tokens.push(Token::Identifier(word)),
                }
            }
            _ => return Err(format!("unexpected character: {ch}")),
        }
    }

    while indent_stack.len() > 1 {
        indent_stack.pop();
        tokens.push(Token::Dedent);
    }

    tokens.push(Token::Eof);
    Ok(tokens)
}

fn update_indentation(
    indent: usize,
    indent_stack: &mut Vec<usize>,
    tokens: &mut Vec<Token>,
) -> Result<(), String> {
    let current_indent = *indent_stack
        .last()
        .expect("indent stack always contains the base indent");

    if indent > current_indent {
        indent_stack.push(indent);
        tokens.push(Token::Indent);
        return Ok(());
    }

    while indent
        < *indent_stack
            .last()
            .expect("indent stack always contains the base indent")
    {
        indent_stack.pop();
        tokens.push(Token::Dedent);
    }

    if indent
        != *indent_stack
            .last()
            .expect("indent stack always contains the base indent")
    {
        return Err("unmatched indentation".to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Token, lex};

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
    fn lexes_boolean_literals() {
        assert_eq!(
            lex("True False"),
            Ok(vec![Token::True, Token::False, Token::Eof])
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
    fn rejects_unterminated_string() {
        assert_eq!(lex("\"hello"), Err("unterminated string".to_string()));
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
    fn lexes_pass_keyword() {
        assert_eq!(lex("pass"), Ok(vec![Token::Pass, Token::Eof]));
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
            Err("unmatched indentation".to_string())
        );
    }

    #[test]
    fn rejects_tabs_in_indentation() {
        assert_eq!(
            lex("\tprint(1)"),
            Err("tabs in indentation are not supported".to_string())
        );
    }

    #[test]
    fn rejects_unknown_character() {
        assert_eq!(lex("print(@)"), Err("unexpected character: @".to_string()));
    }
}
