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

pub fn run_source(source: &str) -> Result<Vec<String>, String> {
    let tokens = lex(source).map_err(|message| format!("lex error: {message}"))?;
    let stmt = parse(&tokens).map_err(|message| format!("parse error: {message}"))?;
    let instructions = compile(&stmt).map_err(|message| format!("compile error: {message}"))?;
    let mut vm = Vm::new(instructions);

    vm.run()
        .map_err(|message| format!("runtime error: {message}"))
}
