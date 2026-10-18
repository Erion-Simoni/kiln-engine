//! Runtime values for the Kiln VM.

/// Runtime values for the Kiln VM.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    I64(i64),
    Bool(bool),
}

impl Value {
    pub fn as_i64(self) -> Option<i64> {
        match self {
            Value::I64(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_bool(self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(v),
            _ => None,
        }
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}
