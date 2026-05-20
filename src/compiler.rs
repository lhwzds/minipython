use crate::ast::{BinaryOp, Expr, Program, Stmt};
use crate::bytecode::{Instruction, Register};
use crate::value::Value;

pub fn compile(program: &Program) -> Result<Vec<Instruction>, String> {
    let mut compiler = Compiler {
        instructions: Vec::new(),
        next_register: 0,
    };

    for stmt in &program.statements {
        compiler.compile_stmt(stmt)?;
    }
    compiler.instructions.push(Instruction::Halt);

    Ok(compiler.instructions)
}

struct Compiler {
    instructions: Vec<Instruction>,
    next_register: Register,
}

impl Compiler {
    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), String> {
        match stmt {
            Stmt::Expr(expr) => {
                let src = self.compile_expr(expr)?;
                self.instructions.push(Instruction::Pop { src });
                Ok(())
            }
        }
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
            Expr::Binary { left, op, right } => {
                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;
                let dst = self.alloc_register();

                match op {
                    BinaryOp::Add => {
                        self.instructions
                            .push(Instruction::Add { dst, left, right });
                    }
                }

                Ok(dst)
            }
            Expr::Name(name) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadName {
                    dst,
                    name: name.clone(),
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
        }
    }

    fn alloc_register(&mut self) -> Register {
        let register = self.next_register;
        self.next_register += 1;
        register
    }
}

#[cfg(test)]
mod tests {
    use super::compile;
    use crate::ast::{BinaryOp, Expr, Program, Stmt};
    use crate::bytecode::Instruction;
    use crate::value::Value;

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
