use kiln_ir::Value;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Tagged values retain integer precision and distinct script value types.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SaveValue {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Text(String),
    Array(Vec<SaveValue>),
}

impl SaveValue {
    pub fn capture(value: &Value) -> Result<Self, String> {
        fn convert(value: &Value, depth: usize, count: &mut usize) -> Result<SaveValue, String> {
            *count += 1;
            if depth > 16 || *count > 16384 {
                return Err("Saved value depth/item limit reached".into());
            }
            Ok(match value {
                Value::I64(v) => SaveValue::Integer(*v),
                Value::F64(v) if v.is_finite() => SaveValue::Float(*v),
                Value::Bool(v) => SaveValue::Boolean(*v),
                Value::Str(v) if v.len() <= 65536 => SaveValue::Text(v.clone()),
                Value::Array(values) if values.len() <= 4096 => SaveValue::Array(
                    values
                        .iter()
                        .map(|v| convert(v, depth + 1, count))
                        .collect::<Result<_, _>>()?,
                ),
                _ => return Err("Saved value is non-finite or exceeds its limit".into()),
            })
        }
        convert(value, 0, &mut 0)
    }
    pub fn value(&self) -> Result<Value, String> {
        let value = match self {
            Self::Integer(v) => Value::I64(*v),
            Self::Float(v) => Value::F64(*v),
            Self::Boolean(v) => Value::Bool(*v),
            Self::Text(v) => Value::Str(v.clone()),
            Self::Array(v) => Value::Array(v.iter().map(Self::value).collect::<Result<_, _>>()?),
        };
        Self::capture(&value)?;
        Ok(value)
    }
}

pub(super) fn capture_map<'a>(
    values: impl IntoIterator<Item = (&'a String, &'a Value)>,
) -> Result<BTreeMap<String, SaveValue>, String> {
    let mut output = BTreeMap::new();
    for (key, value) in values {
        if key.is_empty() || key.len() > 256 || output.len() >= 4096 {
            return Err("Saved state key limit reached".into());
        }
        output.insert(key.clone(), SaveValue::capture(value)?);
    }
    Ok(output)
}

pub(super) fn restore_map(
    values: &BTreeMap<String, SaveValue>,
) -> Result<std::collections::HashMap<String, Value>, String> {
    if values.len() > 4096 {
        return Err("Saved state key limit reached".into());
    }
    values
        .iter()
        .map(|(k, v)| {
            if k.is_empty() || k.len() > 256 {
                return Err("Invalid saved state key".into());
            }
            Ok((k.clone(), v.value()?))
        })
        .collect()
}
