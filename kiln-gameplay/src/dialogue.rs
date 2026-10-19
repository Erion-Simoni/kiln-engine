//! Dialogue system with conditions, variables, and localization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A dialogue tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialogue {
    pub id: String,
    pub nodes: HashMap<String, DialogueNode>,
    pub start_node: String,
}

/// A single dialogue node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueNode {
    /// Localization key for the speaker name
    pub speaker_key: String,
    /// Localization key for the dialogue text
    pub text_key: String,
    /// Choices the player can make (leads to other nodes)
    pub choices: Vec<DialogueChoice>,
    /// Condition that must be met to show this node
    pub condition: Option<DialogueCondition>,
    /// Variables to set when this node is visited
    pub set_variables: Vec<VariableSet>,
}

/// A choice in dialogue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueChoice {
    /// Localization key for the choice text
    pub text_key: String,
    /// ID of the node this choice leads to
    pub target_node: String,
    /// Condition that must be met to show this choice
    pub condition: Option<DialogueCondition>,
    /// Variables to set when this choice is made
    pub set_variables: Vec<VariableSet>,
}

/// Condition for showing dialogue or choices
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum DialogueCondition {
    /// Check if a variable equals a value
    VariableEquals { variable: String, value: i64 },
    /// Check if a variable is greater than a value
    VariableGreaterThan { variable: String, value: i64 },
    /// Check if a variable is less than a value
    VariableLessThan { variable: String, value: i64 },
    /// Check if a quest has a specific status
    QuestStatus {
        quest_id: String,
        status: QuestStatusCondition,
    },
    /// Check if a quest objective is complete
    QuestObjectiveComplete {
        quest_id: String,
        objective_index: usize,
    },
    /// Multiple conditions must all be true (AND)
    AllOf(Vec<DialogueCondition>),
    /// At least one condition must be true (OR)
    AnyOf(Vec<DialogueCondition>),
    /// Condition must be false (NOT)
    Not(Box<DialogueCondition>),
}

/// Quest status for dialogue conditions
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuestStatusCondition {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

/// Variable set operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableSet {
    pub variable: String,
    pub operation: VariableOperation,
}

/// Variable operation type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum VariableOperation {
    Set(i64),
    Add(i64),
    Subtract(i64),
    Multiply(i64),
}

/// Dialogue variable state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueVariable {
    pub name: String,
    pub value: i64,
}

impl Dialogue {
    /// Create a new dialogue
    pub fn new(id: String, start_node: String) -> Self {
        Self {
            id,
            nodes: HashMap::new(),
            start_node,
        }
    }

    /// Add a node to the dialogue
    pub fn add_node(&mut self, node: DialogueNode) {
        self.nodes.insert(node.speaker_key.clone(), node);
    }

    /// Get the start node
    pub fn get_start_node(&self) -> Option<&DialogueNode> {
        self.nodes.get(&self.start_node)
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&DialogueNode> {
        self.nodes.get(node_id)
    }

    /// Check if a condition is met given variable and quest states
    pub fn check_condition(
        &self,
        condition: &Option<DialogueCondition>,
        variables: &HashMap<String, i64>,
        quest_states: &HashMap<String, QuestStatusCondition>,
    ) -> bool {
        match condition {
            None => true,
            Some(cond) => self.evaluate_condition(cond, variables, quest_states),
        }
    }

    fn evaluate_condition(
        &self,
        condition: &DialogueCondition,
        variables: &HashMap<String, i64>,
        quest_states: &HashMap<String, QuestStatusCondition>,
    ) -> bool {
        match condition {
            DialogueCondition::VariableEquals { variable, value } => {
                variables.get(variable).is_some_and(|v| *v == *value)
            }
            DialogueCondition::VariableGreaterThan { variable, value } => {
                variables.get(variable).is_some_and(|v| *v > *value)
            }
            DialogueCondition::VariableLessThan { variable, value } => {
                variables.get(variable).is_some_and(|v| *v < *value)
            }
            DialogueCondition::QuestStatus { quest_id, status } => {
                quest_states.get(quest_id).is_some_and(|s| s == status)
            }
            DialogueCondition::QuestObjectiveComplete { .. } => {
                // Quest objective tracking deferred - assume true for now
                true
            }
            DialogueCondition::AllOf(conditions) => conditions
                .iter()
                .all(|c| self.evaluate_condition(c, variables, quest_states)),
            DialogueCondition::AnyOf(conditions) => conditions
                .iter()
                .any(|c| self.evaluate_condition(c, variables, quest_states)),
            DialogueCondition::Not(cond) => !self.evaluate_condition(cond, variables, quest_states),
        }
    }

    /// Apply variable operations
    pub fn apply_variables(
        &self,
        operations: &[VariableSet],
        variables: &mut HashMap<String, i64>,
    ) {
        for op in operations {
            let current = variables.get(&op.variable).copied().unwrap_or(0);
            let new_value = match &op.operation {
                VariableOperation::Set(v) => *v,
                VariableOperation::Add(v) => current + v,
                VariableOperation::Subtract(v) => current - v,
                VariableOperation::Multiply(v) => current * v,
            };
            variables.insert(op.variable.clone(), new_value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialogue_creation() {
        let mut dialogue = Dialogue::new("test_dialogue".to_string(), "node1".to_string());
        let node = DialogueNode {
            speaker_key: "npc".to_string(),
            text_key: "hello".to_string(),
            choices: vec![],
            condition: None,
            set_variables: vec![],
        };
        dialogue.add_node(node);
        assert_eq!(dialogue.nodes.len(), 1);
    }

    #[test]
    fn test_variable_equals_condition() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut variables = HashMap::new();
        variables.insert("friendship".to_string(), 10);

        let condition = DialogueCondition::VariableEquals {
            variable: "friendship".to_string(),
            value: 10,
        };
        assert!(dialogue.evaluate_condition(&condition, &variables, &HashMap::new()));

        let condition2 = DialogueCondition::VariableEquals {
            variable: "friendship".to_string(),
            value: 5,
        };
        assert!(!dialogue.evaluate_condition(&condition2, &variables, &HashMap::new()));
    }

    #[test]
    fn test_variable_comparison_conditions() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut variables = HashMap::new();
        variables.insert("level".to_string(), 15);

        let greater = DialogueCondition::VariableGreaterThan {
            variable: "level".to_string(),
            value: 10,
        };
        assert!(dialogue.evaluate_condition(&greater, &variables, &HashMap::new()));

        let less = DialogueCondition::VariableLessThan {
            variable: "level".to_string(),
            value: 20,
        };
        assert!(dialogue.evaluate_condition(&less, &variables, &HashMap::new()));
    }

    #[test]
    fn test_quest_status_condition() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut quest_states = HashMap::new();
        quest_states.insert("main_quest".to_string(), QuestStatusCondition::InProgress);

        let condition = DialogueCondition::QuestStatus {
            quest_id: "main_quest".to_string(),
            status: QuestStatusCondition::InProgress,
        };
        assert!(dialogue.evaluate_condition(&condition, &HashMap::new(), &quest_states));
    }

    #[test]
    fn test_all_of_condition() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), 1);
        variables.insert("b".to_string(), 2);

        let condition = DialogueCondition::AllOf(vec![
            DialogueCondition::VariableEquals {
                variable: "a".to_string(),
                value: 1,
            },
            DialogueCondition::VariableEquals {
                variable: "b".to_string(),
                value: 2,
            },
        ]);
        assert!(dialogue.evaluate_condition(&condition, &variables, &HashMap::new()));
    }

    #[test]
    fn test_any_of_condition() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), 1);

        let condition = DialogueCondition::AnyOf(vec![
            DialogueCondition::VariableEquals {
                variable: "a".to_string(),
                value: 1,
            },
            DialogueCondition::VariableEquals {
                variable: "b".to_string(),
                value: 2,
            },
        ]);
        assert!(dialogue.evaluate_condition(&condition, &variables, &HashMap::new()));
    }

    #[test]
    fn test_not_condition() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut variables = HashMap::new();
        variables.insert("a".to_string(), 1);

        let condition = DialogueCondition::Not(Box::new(DialogueCondition::VariableEquals {
            variable: "a".to_string(),
            value: 2,
        }));
        assert!(dialogue.evaluate_condition(&condition, &variables, &HashMap::new()));
    }

    #[test]
    fn test_apply_variables() {
        let dialogue = Dialogue::new("test".to_string(), "start".to_string());
        let mut variables = HashMap::new();
        variables.insert("score".to_string(), 10);

        let operations = vec![
            VariableSet {
                variable: "score".to_string(),
                operation: VariableOperation::Add(5),
            },
            VariableSet {
                variable: "new_var".to_string(),
                operation: VariableOperation::Set(100),
            },
        ];
        dialogue.apply_variables(&operations, &mut variables);

        assert_eq!(variables.get("score"), Some(&15));
        assert_eq!(variables.get("new_var"), Some(&100));
    }
}
