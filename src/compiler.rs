use crate::ast::{BinaryOp, ComparisonOp, Expr, Program, Stmt};
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
            Stmt::Assign { name, value } => {
                let src = self.compile_expr(value)?;
                self.instructions.push(Instruction::StoreName {
                    name: name.clone(),
                    src,
                });
                Ok(())
            }
            Stmt::If {
                condition,
                then_body,
                else_body,
            } => self.compile_if_stmt(condition, then_body, else_body),
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
            Expr::String(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::String(value.clone()),
                });
                Ok(dst)
            }
            Expr::Bool(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::Bool(*value),
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
            Expr::Comparison { left, op, right } => {
                let left = self.compile_expr(left)?;
                let right = self.compile_expr(right)?;
                let dst = self.alloc_register();

                match op {
                    ComparisonOp::Equal => {
                        self.instructions
                            .push(Instruction::Equal { dst, left, right });
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
            Some(instruction) => Err(format!("cannot patch jump target on {instruction:?}")),
            None => Err(format!(
                "cannot patch missing instruction at index {instruction_index}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compile;
    use crate::ast::{BinaryOp, ComparisonOp, Expr, Program, Stmt};
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
    fn compiles_assignment_to_bytecode() {
        let program = Program {
            statements: vec![Stmt::Assign {
                name: "x".to_string(),
                value: Expr::Binary {
                    left: Box::new(Expr::Number(1)),
                    op: BinaryOp::Add,
                    right: Box::new(Expr::Number(2)),
                },
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
