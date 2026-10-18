//! Unique IDs for entities, assets, etc.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(u64);

impl EntityId {
    pub const INVALID: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn is_valid(self) -> bool {
        self.0 != 0
    }
}

impl fmt::Display for EntityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "EntityId({})", self.0)
    }
}
