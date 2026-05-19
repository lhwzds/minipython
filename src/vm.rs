use crate::bytecode::{Instruction, Register};
use crate::value::Value;

pub struct Vm {
    instructions: Vec<Instruction>,
    ip: usize,
    registers: Vec<Option<Value>>,
    output: Vec<String>,
}

impl Vm {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            ip: 0,
            registers: Vec::new(),
            output: Vec::new(),
        }
    }

    pub fn run(&mut self) -> Result<Vec<String>, String> {
        loop {
            let instruction = self
                .instructions
                .get(self.ip)
                .cloned()
                .ok_or_else(|| "instruction pointer moved past end of bytecode".to_string())?;
            self.ip += 1;

            match instruction {
                Instruction::LoadConst { dst, value } => {
                    self.write_register(dst, value);
                }
                Instruction::Add { dst, left, right } => {
                    let left = self.read_register(left)?.clone();
                    let right = self.read_register(right)?.clone();
                    let value = add_values(left, right);
                    self.write_register(dst, value);
                }
                Instruction::Print { src } => {
                    let value = self.read_register(src)?;
                    self.output.push(value.to_string());
                }
                Instruction::Halt => return Ok(std::mem::take(&mut self.output)),
            }
        }
    }

    fn write_register(&mut self, register: Register, value: Value) {
        if register >= self.registers.len() {
            self.registers.resize(register + 1, None);
        }

        self.registers[register] = Some(value);
    }

    fn read_register(&self, register: Register) -> Result<&Value, String> {
        self.registers
            .get(register)
            .and_then(Option::as_ref)
            .ok_or_else(|| format!("register r{register} is not initialized"))
    }
}

fn add_values(left: Value, right: Value) -> Value {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => Value::Number(left + right),
    }
}

#[cfg(test)]
mod tests {
    use super::Vm;
    use crate::bytecode::Instruction;
    use crate::value::Value;

    #[test]
    fn runs_print_number_program() {
        let instructions = vec![
            Instruction::LoadConst {
                dst: 0,
                value: Value::Number(123),
            },
            Instruction::Print { src: 0 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["123".to_string()]));
    }

    #[test]
    fn runs_addition_program() {
        let instructions = vec![
            Instruction::LoadConst {
                dst: 0,
                value: Value::Number(1),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::Number(2),
            },
            Instruction::Add {
                dst: 2,
                left: 0,
                right: 1,
            },
            Instruction::Print { src: 2 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["3".to_string()]));
    }

    #[test]
    fn rejects_uninitialized_register() {
        let instructions = vec![Instruction::Print { src: 0 }, Instruction::Halt];
        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Err("register r0 is not initialized".to_string()));
    }

    #[test]
    fn rejects_missing_halt() {
        let instructions = vec![Instruction::LoadConst {
            dst: 0,
            value: Value::Number(123),
        }];
        let mut vm = Vm::new(instructions);

        assert_eq!(
            vm.run(),
            Err("instruction pointer moved past end of bytecode".to_string())
        );
    }
}
