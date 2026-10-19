//! Gameplay content tools for Kiln
//!
//! This crate provides systems for:
//! - Dialogue trees with conditions and localization
//! - Quest tracking with state persistence
//! - Data tables for items, enemies, loot, etc.

pub mod datatable;
pub mod dialogue;
pub mod quest;

pub use datatable::{DataTable, DataTableColumn, DataTableRow, DataType, DataValue};
pub use dialogue::{
    Dialogue, DialogueChoice, DialogueCondition, DialogueNode, DialogueVariable,
    QuestStatusCondition, VariableOperation, VariableSet,
};
pub use quest::{
    ObjectiveType, Quest, QuestError, QuestManager, QuestObjective, QuestState, QuestStatus,
};
