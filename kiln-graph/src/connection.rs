//! Connection between two pins.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A connection from an output pin to an input pin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: Uuid,
    pub source_pin: Uuid,
    pub target_pin: Uuid,
}

impl Connection {
    pub fn new(source_pin: Uuid, target_pin: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_pin,
            target_pin,
        }
    }
}
