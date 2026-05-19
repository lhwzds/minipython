mod ast;
mod bytecode;
mod compiler;
mod lexer;
mod parser;
mod value;
mod vm;

use compiler::compile;
use lexer::lex;
use parser::parse;
use vm::Vm;

fn main() {
    let source = "print(1 + 2)";

    match lex(source) {
        Ok(tokens) => match parse(&tokens) {
            Ok(stmt) => {
                let instructions = compile(&stmt);
                let mut vm = Vm::new(instructions);

                match vm.run() {
                    Ok(output) => {
                        for line in output {
                            println!("{line}");
                        }
                    }
                    Err(message) => eprintln!("runtime error: {message}"),
                }
            }
            Err(message) => eprintln!("parse error: {message}"),
        },
        Err(message) => eprintln!("{message}"),
    }
}
