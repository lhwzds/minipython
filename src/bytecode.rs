use crate::value::Value;

pub type Register = usize;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    LoadConst {
        dst: Register,
        value: Value,
    },
    LoadName {
        dst: Register,
        name: String,
    },
    StoreName {
        name: String,
        src: Register,
    },
    Add {
        dst: Register,
        left: Register,
        right: Register,
    },
    Call {
        dst: Register,
        callee: Register,
        args: Vec<Register>,
    },
    Pop {
        src: Register,
    },
    Halt,
}
