use crate::value::Value;

pub type Register = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    LoadConst {
        dst: Register,
        value: Value,
    },
    Add {
        dst: Register,
        left: Register,
        right: Register,
    },
    Print {
        src: Register,
    },
    Halt,
}
