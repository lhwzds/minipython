use crate::ast::{BinaryOp, Expr, Stmt};
use crate::bytecode::{Instruction, Register};
use crate::value::Value;

pub fn compile(stmt: &Stmt) -> Vec<Instruction> {
    let mut compiler = Compiler {
        instructions: Vec::new(),
        next_register: 0,
    };

    compiler.compile_stmt(stmt);
    compiler.instructions.push(Instruction::Halt);

    compiler.instructions
}

struct Compiler {
    instructions: Vec<Instruction>,
    next_register: Register,
}

impl Compiler {
    fn compile_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Print(expr) => {
                let src = self.compile_expr(expr);
                self.instructions.push(Instruction::Print { src });
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Register {
        match expr {
            Expr::Number(value) => {
                let dst = self.alloc_register();
                self.instructions.push(Instruction::LoadConst {
                    dst,
                    value: Value::Number(*value),
                });
                dst
            }
            Expr::Binary { left, op, right } => {
                let left = self.compile_expr(left);
                let right = self.compile_expr(right);
                let dst = self.alloc_register();

                match op {
                    BinaryOp::Add => {
                        self.instructions
                            .push(Instruction::Add { dst, left, right });
                    }
                }

                dst
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
    use crate::ast::{Expr, Stmt};
    use crate::bytecode::Instruction;
    use crate::value::Value;

    #[test]
    fn compiles_print_number_to_bytecode() {
        let stmt = Stmt::Print(Expr::Number(123));

        assert_eq!(
            compile(&stmt),
            vec![
                Instruction::LoadConst {
                    dst: 0,
                    value: Value::Number(123)
                },
                Instruction::Print { src: 0 },
                Instruction::Halt,
            ]
        );
    }

    #[test]
    fn compiles_addition_to_bytecode() {
        let stmt = Stmt::Print(Expr::Binary {
            left: Box::new(Expr::Number(1)),
            op: crate::ast::BinaryOp::Add,
            right: Box::new(Expr::Number(2)),
        });

        assert_eq!(
            compile(&stmt),
            vec![
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
                Instruction::Print { src: 2 },
                Instruction::Halt,
            ]
        );
    }
}
