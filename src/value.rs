use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Number(i64),
    Builtin(String),
    None,
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(value) => write!(f, "{value}"),
            Value::Builtin(name) => write!(f, "<builtin {name}>"),
            Value::None => write!(f, "None"),
        }
    }
}
