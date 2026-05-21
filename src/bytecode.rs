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
    Equal {
        dst: Register,
        left: Register,
        right: Register,
    },
    NotEqual {
        dst: Register,
        left: Register,
        right: Register,
    },
    Less {
        dst: Register,
        left: Register,
        right: Register,
    },
    LessEqual {
        dst: Register,
        left: Register,
        right: Register,
    },
    Greater {
        dst: Register,
        left: Register,
        right: Register,
    },
    GreaterEqual {
        dst: Register,
        left: Register,
        right: Register,
    },
    Not {
        dst: Register,
        src: Register,
    },
    ToBool {
        dst: Register,
        src: Register,
    },
    JumpIfFalse {
        condition: Register,
        target: usize,
    },
    Jump {
        target: usize,
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
