//! Pure value reads shared by gameplay and isolated debugger evaluation.
use kiln_ir::{Value, VmError};
pub(super) fn call(id: u32, args: &[Value]) -> Result<Value, VmError> {
    let error = |message: &str| VmError::HostCallError(message.into());
    match id {
        61 => match &args[0] {
            Value::Array(values) => Ok(Value::I64(values.len() as i64)),
            _ => Err(error("array_length expects an array")),
        },
        62 => match (&args[0], args[1].as_i64()) {
            (Value::Array(values), Some(index)) if index >= 0 => values
                .get(index as usize)
                .cloned()
                .ok_or_else(|| error("array index is out of range")),
            _ => Err(error("array_get expects an array and nonnegative integer")),
        },
        111 | 113 => {
            let Value::Map(values) = &args[0] else {
                return Err(error("Expected dictionary"));
            };
            let key = args[1]
                .as_str()
                .filter(|key| key.len() <= 256)
                .ok_or_else(|| error("Dictionary key must be a string of at most 256 bytes"))?;
            if id == 111 {
                Ok(values.get(key).cloned().unwrap_or_else(|| args[2].clone()))
            } else {
                Ok(Value::Bool(values.contains_key(key)))
            }
        }
        _ => Err(error("Unknown read-only value function")),
    }
}
