//! Gameplay content tools for Kiln
//!
//! This crate provides systems for:
//! - Dialogue trees with conditions and localization
//! - Quest tracking with state persistence
//! - Data tables for items, enemies, loot, etc.

pub mod dialogue;
pub mod quest;
pub mod datatable;

pub use dialogue::{Dialogue, DialogueNode, DialogueCondition, DialogueVariable, DialogueChoice, VariableSet, VariableOperation, QuestStatusCondition};
pub use quest::{Quest, QuestState, QuestObjective, QuestStatus, QuestManager, QuestError, ObjectiveType};
pub use datatable::{DataTable, DataTableRow, DataTableColumn, DataType, DataValue};
