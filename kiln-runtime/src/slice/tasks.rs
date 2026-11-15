//! Bounded cooperative script tasks; time and persistence reuse the actual session.
use super::{game_save, SliceHost, Value, VmError};
const PREFIX: &str = "__kiln_task:";
impl SliceHost {
    pub(super) fn call_task(&mut self, id: u32, args: &[Value]) -> Result<Value, VmError> {
        if id == 122 {
            return self.call_task(116, &[args[0].clone(), args[1].clone(), Value::I64(0)]);
        }
        if id == 123 {
            let ready = self.call_task(117, args)?;
            if ready == Value::Bool(true) {
                let _ = self.call_task(118, &[args[0].clone(), Value::I64(0)])?;
            }
            return Ok(ready);
        }
        let name = args[0]
            .as_str()
            .filter(|name| !name.is_empty() && name.len() <= 240)
            .ok_or_else(|| VmError::HostCallError("Task names require 1–240 UTF-8 bytes".into()))?;
        let key = format!("{PREFIX}{name}");
        if id == 116 {
            let delay = args[1]
                .to_f64()
                .filter(|delay| delay.is_finite() && *delay >= 0.0)
                .ok_or_else(|| {
                    VmError::HostCallError("Task delay must be finite and nonnegative".into())
                })?;
            let deadline = self.facts.elapsed + delay;
            if !deadline.is_finite() {
                return Err(VmError::HostCallError("Task deadline overflow".into()));
            }
            if !self.script_state.contains_key(&key)
                && (self.script_state.len() >= 4096
                    || self
                        .script_state
                        .keys()
                        .filter(|key| key.starts_with(PREFIX))
                        .count()
                        >= 256)
            {
                return Err(VmError::HostCallError(
                    "Task/state count limit reached".into(),
                ));
            }
            let task = Value::Map(std::collections::BTreeMap::from([
                ("deadline".into(), Value::F64(deadline)),
                ("payload".into(), args[2].clone()),
            ]));
            game_save::SaveValue::capture(&task).map_err(VmError::HostCallError)?;
            self.script_state.insert(key, task);
            return Ok(Value::Str(name.into()));
        }
        if id == 119 {
            return Ok(Value::Bool(self.script_state.remove(&key).is_some()));
        }
        let Some(task) = self.script_state.get(&key) else {
            return Ok(match id {
                118 => args[1].clone(),
                121 => Value::F64(0.0),
                _ => Value::Bool(false),
            });
        };
        let Value::Map(record) = task else {
            return Err(VmError::HostCallError("Invalid saved task state".into()));
        };
        let deadline = record
            .get("deadline")
            .and_then(Value::to_f64)
            .filter(|v| v.is_finite() && *v >= 0.0)
            .ok_or_else(|| VmError::HostCallError("Invalid saved task deadline".into()))?;
        let payload = record
            .get("payload")
            .ok_or_else(|| VmError::HostCallError("Invalid saved task payload".into()))?;
        let ready = self.facts.elapsed >= deadline;
        match id {
            117 => Ok(Value::Bool(ready)),
            118 if ready => {
                let result = payload.clone();
                self.script_state.remove(&key);
                Ok(result)
            }
            118 => Ok(args[1].clone()),
            120 => Ok(Value::Bool(true)),
            121 => Ok(Value::F64((deadline - self.facts.elapsed).max(0.0))),
            _ => Err(VmError::HostCallError("Unknown task operation".into())),
        }
    }
}
