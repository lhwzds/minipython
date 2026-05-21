use crate::bytecode::{Instruction, Register};
use crate::value::Value;
use std::collections::HashMap;

pub struct Vm {
    instructions: Vec<Instruction>,
    ip: usize,
    registers: Vec<Option<Value>>,
    globals: HashMap<String, Value>,
    output: Vec<String>,
}

impl Vm {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self {
            instructions,
            ip: 0,
            registers: Vec::new(),
            globals: HashMap::new(),
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
                Instruction::LoadName { dst, name } => {
                    let value = self.load_name(&name)?;
                    self.write_register(dst, value);
                }
                Instruction::StoreName { name, src } => {
                    let value = self.read_register(src)?.clone();
                    self.globals.insert(name, value);
                }
                Instruction::Add { dst, left, right } => {
                    let left = self.read_register(left)?.clone();
                    let right = self.read_register(right)?.clone();
                    let value = add_values(left, right)?;
                    self.write_register(dst, value);
                }
                Instruction::Equal { dst, left, right } => {
                    let left = self.read_register(left)?.clone();
                    let right = self.read_register(right)?.clone();
                    self.write_register(dst, Value::Bool(left == right));
                }
                Instruction::Call { dst, callee, args } => {
                    let callee = self.read_register(callee)?.clone();
                    let args = args
                        .iter()
                        .map(|arg| self.read_register(*arg).cloned())
                        .collect::<Result<Vec<_>, _>>()?;
                    let value = self.call_value(callee, args)?;
                    self.write_register(dst, value);
                }
                Instruction::Pop { src } => {
                    self.read_register(src)?;
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

    fn load_name(&self, name: &str) -> Result<Value, String> {
        if let Some(value) = self.globals.get(name).cloned() {
            Ok(value)
        } else {
            match name {
                "print" => Ok(Value::Builtin(name.to_string())),
                _ => Err(format!("unknown name: {name}")),
            }
        }
    }

    fn call_value(&mut self, callee: Value, args: Vec<Value>) -> Result<Value, String> {
        match callee {
            Value::Builtin(name) if name == "print" => {
                let line = args
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ");
                self.output.push(line);
                Ok(Value::None)
            }
            Value::Builtin(name) => Err(format!("unknown builtin: {name}")),
            value => Err(format!("{value} is not callable")),
        }
    }
}

fn add_values(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => Ok(Value::Number(left + right)),
        (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
        (left, right) => Err(format!("cannot add {left} and {right}")),
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
            Instruction::LoadName {
                dst: 0,
                name: "print".to_string(),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::Number(123),
            },
            Instruction::Call {
                dst: 2,
                callee: 0,
                args: vec![1],
            },
            Instruction::Pop { src: 2 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["123".to_string()]));
    }

    #[test]
    fn runs_addition_program() {
        let instructions = vec![
            Instruction::LoadName {
                dst: 0,
                name: "print".to_string(),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::Number(1),
            },
            Instruction::LoadConst {
                dst: 2,
                value: Value::Number(2),
            },
            Instruction::Add {
                dst: 3,
                left: 1,
                right: 2,
            },
            Instruction::Call {
                dst: 4,
                callee: 0,
                args: vec![3],
            },
            Instruction::Pop { src: 4 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["3".to_string()]));
    }

    #[test]
    fn runs_print_with_multiple_arguments() {
        let instructions = vec![
            Instruction::LoadName {
                dst: 0,
                name: "print".to_string(),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::Number(1),
            },
            Instruction::LoadConst {
                dst: 2,
                value: Value::Number(2),
            },
            Instruction::Call {
                dst: 3,
                callee: 0,
                args: vec![1, 2],
            },
            Instruction::Pop { src: 3 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["1 2".to_string()]));
    }

    #[test]
    fn runs_equality_program() {
        let instructions = vec![
            Instruction::LoadName {
                dst: 0,
                name: "print".to_string(),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::Number(1),
            },
            Instruction::LoadConst {
                dst: 2,
                value: Value::Number(1),
            },
            Instruction::Equal {
                dst: 3,
                left: 1,
                right: 2,
            },
            Instruction::Call {
                dst: 4,
                callee: 0,
                args: vec![3],
            },
            Instruction::Pop { src: 4 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["True".to_string()]));
    }

    #[test]
    fn stores_and_loads_global_name() {
        let instructions = vec![
            Instruction::LoadConst {
                dst: 0,
                value: Value::Number(3),
            },
            Instruction::StoreName {
                name: "x".to_string(),
                src: 0,
            },
            Instruction::LoadName {
                dst: 1,
                name: "print".to_string(),
            },
            Instruction::LoadName {
                dst: 2,
                name: "x".to_string(),
            },
            Instruction::Call {
                dst: 3,
                callee: 1,
                args: vec![2],
            },
            Instruction::Pop { src: 3 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["3".to_string()]));
    }

    #[test]
    fn prints_string_value() {
        let instructions = vec![
            Instruction::LoadName {
                dst: 0,
                name: "print".to_string(),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::String("hello".to_string()),
            },
            Instruction::Call {
                dst: 2,
                callee: 0,
                args: vec![1],
            },
            Instruction::Pop { src: 2 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["hello".to_string()]));
    }

    #[test]
    fn adds_string_values() {
        let instructions = vec![
            Instruction::LoadName {
                dst: 0,
                name: "print".to_string(),
            },
            Instruction::LoadConst {
                dst: 1,
                value: Value::String("hello ".to_string()),
            },
            Instruction::LoadConst {
                dst: 2,
                value: Value::String("mini".to_string()),
            },
            Instruction::Add {
                dst: 3,
                left: 1,
                right: 2,
            },
            Instruction::Call {
                dst: 4,
                callee: 0,
                args: vec![3],
            },
            Instruction::Pop { src: 4 },
            Instruction::Halt,
        ];

        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Ok(vec!["hello mini".to_string()]));
    }

    #[test]
    fn rejects_uninitialized_register() {
        let instructions = vec![Instruction::Pop { src: 0 }, Instruction::Halt];
        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Err("register r0 is not initialized".to_string()));
    }

    #[test]
    fn rejects_unknown_name() {
        let instructions = vec![
            Instruction::LoadName {
                dst: 0,
                name: "unknown".to_string(),
            },
            Instruction::Halt,
        ];
        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Err("unknown name: unknown".to_string()));
    }

    #[test]
    fn rejects_non_callable_value() {
        let instructions = vec![
            Instruction::LoadConst {
                dst: 0,
                value: Value::Number(1),
            },
            Instruction::Call {
                dst: 1,
                callee: 0,
                args: Vec::new(),
            },
            Instruction::Halt,
        ];
        let mut vm = Vm::new(instructions);

        assert_eq!(vm.run(), Err("1 is not callable".to_string()));
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
