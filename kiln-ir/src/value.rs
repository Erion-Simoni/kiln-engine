//! Runtime values for the Kiln VM.

/// Runtime values for the Kiln VM.
///
/// Numeric operations promote `I64` to `F64` when mixed with floats
/// (standard arithmetic promotion), so `1 + 2.5` yields `F64(3.5)`.
///
/// `Str` is heap-allocated, so [`Value`] is `Clone` but not `Copy`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 64-bit signed integer.
    I64(i64),
    /// 64-bit float.
    F64(f64),
    /// Boolean.
    Bool(bool),
    /// UTF-8 string (heap-allocated).
    Str(String),
}

impl Value {
    /// Returns the value as `i64` if it is an integer.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the value as `f64` if it is a float.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::F64(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the value as `bool` if it is a boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(v) => Some(*v),
            _ => None,
        }
    }

    /// Returns the value as `&str` if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(v) => Some(v),
            _ => None,
        }
    }

    /// Returns `true` for either numeric variant.
    pub fn is_number(self) -> bool {
        matches!(self, Value::I64(_) | Value::F64(_))
    }

    /// Returns the value as `f64`, promoting integers.
    ///
    /// Used for arithmetic promotion: `1` becomes `1.0` so mixed integer and
    /// float operations can be evaluated as floats.
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Value::I64(v) => Some(*v as f64),
            Value::F64(v) => Some(*v),
            Value::Bool(_) | Value::Str(_) => None,
        }
    }

    /// Numeric-aware equality.
    ///
    /// Integers and floats are compared numerically (`1` equals `1.0`); strings
    /// compare by content; all other mismatched types are unequal.
    ///
    /// When comparing I64 with F64, the F64 is checked if it represents an exact
    /// integer value. If so, comparison is done as integers to preserve precision
    /// above 2^53.
    pub fn values_equal(a: Value, b: Value) -> bool {
        match (a, b) {
            (Value::I64(x), Value::I64(y)) => x == y,
            (Value::F64(x), Value::F64(y)) => x == y,
            (Value::I64(x), Value::F64(y)) => {
                // Check if the float is an exact integer
                if y.fract() == 0.0 && y >= i64::MIN as f64 && y <= i64::MAX as f64 {
                    x == y as i64
                } else {
                    (x as f64) == y
                }
            }
            (Value::F64(x), Value::I64(y)) => {
                // Check if the float is an exact integer
                if x.fract() == 0.0 && x >= i64::MIN as f64 && x <= i64::MAX as f64 {
                    x == (y as f64)
                } else {
                    x == (y as f64)
                }
            }
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Str(x), Value::Str(y)) => x == y,
            _ => false,
        }
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::I64(v)
    }
}

impl From<f64> for Value {
    fn from(v: f64) -> Self {
        Value::F64(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Str(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Str(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors() {
        assert_eq!(Value::I64(3).as_i64(), Some(3));
        assert_eq!(Value::I64(3).as_f64(), None);
        assert_eq!(Value::F64(2.5).as_f64(), Some(2.5));
        assert_eq!(Value::F64(2.5).as_i64(), None);
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
    }

    #[test]
    fn is_number() {
        assert!(Value::I64(1).is_number());
        assert!(Value::F64(1.0).is_number());
        assert!(!Value::Bool(true).is_number());
    }

    #[test]
    fn conversions() {
        assert_eq!(Value::from(7_i64), Value::I64(7));
        assert_eq!(Value::from(1.5_f64), Value::F64(1.5));
        assert_eq!(Value::from(true), Value::Bool(true));
    }

    #[test]
    fn to_f64_promotes_integers() {
        assert_eq!(Value::I64(3).to_f64(), Some(3.0));
        assert_eq!(Value::F64(2.5).to_f64(), Some(2.5));
        assert_eq!(Value::Bool(true).to_f64(), None);
    }

    #[test]
    fn str_accessor_and_conversions() {
        let s = Value::Str("hello".to_string());
        assert_eq!(s.as_str(), Some("hello"));
        assert_eq!(Value::I64(1).as_str(), None);
        assert_eq!(Value::from("world").as_str(), Some("world"));
        assert_eq!(Value::from(String::from("world")).as_str(), Some("world"));
    }

    #[test]
    fn values_equal_is_numeric_aware() {
        assert!(Value::values_equal(Value::I64(1), Value::F64(1.0)));
        assert!(Value::values_equal(Value::F64(1.0), Value::I64(1)));
        assert!(!Value::values_equal(Value::I64(1), Value::F64(1.5)));
        assert!(Value::values_equal(Value::Bool(true), Value::Bool(true)));
        assert!(!Value::values_equal(Value::Bool(true), Value::I64(1)));
    }

    #[test]
    fn values_equal_strings() {
        assert!(Value::values_equal(
            Value::Str("abc".into()),
            Value::Str("abc".into()),
        ));
        assert!(!Value::values_equal(
            Value::Str("abc".into()),
            Value::Str("abd".into()),
        ));
        assert!(!Value::values_equal(
            Value::Str("abc".into()),
            Value::I64(1)
        ));
    }
}
