//! Pin types for node graph.

use serde::{Deserialize, Serialize};

/// Direction of a pin (input or output).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinDirection {
    Input,
    Output,
}

/// Data type of a pin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PinType {
    /// Integer value (i64 in IR).
    Integer,
    /// Float value (f64 in IR).
    Float,
    /// Boolean value.
    Boolean,
    /// Event/trigger (no data, just signal).
    Event,
    /// String value (for localization keys, data table lookups).
    String,
}

/// A pin on a node (connection point).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: uuid::Uuid,
    pub direction: PinDirection,
    pub pin_type: PinType,
    pub name: String,
}

impl Pin {
    pub fn new(direction: PinDirection, pin_type: PinType, name: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            direction,
            pin_type,
            name: name.into(),
        }
    }
}
