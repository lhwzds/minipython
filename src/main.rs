mod lexer;

use lexer::lex;

fn main() {
    let source = "print(123)";

    match lex(source) {
        Ok(tokens) => println!("{tokens:#?}"),
        Err(message) => eprintln!("{message}"),
    }
}
