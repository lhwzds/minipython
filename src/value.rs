use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Number(i64),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(value) => write!(f, "{value}"),
        }
    }
}
