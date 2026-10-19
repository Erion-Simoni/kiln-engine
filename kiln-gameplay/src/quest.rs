//! Quest system with state persistence

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A quest definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    pub id: String,
    pub name_key: String,
    pub description_key: String,
    pub objectives: Vec<QuestObjective>,
    pub prerequisites: Vec<String>, // Quest IDs that must be completed first
    pub auto_start: bool,
}

/// A quest objective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestObjective {
    pub id: String,
    pub description_key: String,
    pub target_type: ObjectiveType,
    pub target_value: i64,
    pub is_optional: bool,
}

/// Type of objective
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum ObjectiveType {
    /// Kill N enemies
    KillEnemies { enemy_id: String },
    /// Collect N items
    CollectItems { item_id: String },
    /// Talk to NPC
    TalkToNpc { npc_id: String },
    /// Reach a location
    ReachLocation { location_id: String },
    /// Custom objective (script-defined)
    Custom { condition_key: String },
}

/// Quest runtime state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestState {
    pub quest_id: String,
    pub status: QuestStatus,
    pub objective_progress: HashMap<String, i64>, // objective_id -> progress
    pub started_at: Option<i64>, // timestamp
    pub completed_at: Option<i64>, // timestamp
}

/// Quest status
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QuestStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
}

impl Quest {
    /// Create a new quest
    pub fn new(id: String, name_key: String, description_key: String) -> Self {
        Self {
            id,
            name_key,
            description_key,
            objectives: vec![],
            prerequisites: vec![],
            auto_start: false,
        }
    }

    /// Add an objective to the quest
    pub fn add_objective(&mut self, objective: QuestObjective) {
        self.objectives.push(objective);
    }

    /// Check if all prerequisites are met
    pub fn prerequisites_met(&self, quest_states: &HashMap<String, QuestStatus>) -> bool {
        self.prerequisites
            .iter()
            .all(|quest_id| quest_states.get(quest_id) == Some(&QuestStatus::Completed))
    }

    /// Check if all prerequisites are met (using QuestState)
    pub fn prerequisites_met_from_states(&self, quest_states: &HashMap<String, QuestState>) -> bool {
        self.prerequisites
            .iter()
            .all(|quest_id| {
                quest_states
                    .get(quest_id)
                    .map(|s| s.status == QuestStatus::Completed)
                    .unwrap_or(false)
            })
    }
}

impl QuestState {
    /// Create a new quest state from a quest definition
    pub fn new(quest: &Quest) -> Self {
        let mut objective_progress = HashMap::new();
        for objective in &quest.objectives {
            objective_progress.insert(objective.id.clone(), 0);
        }

        Self {
            quest_id: quest.id.clone(),
            status: QuestStatus::NotStarted,
            objective_progress,
            started_at: None,
            completed_at: None,
        }
    }

    /// Start the quest
    pub fn start(&mut self, timestamp: i64) {
        self.status = QuestStatus::InProgress;
        self.started_at = Some(timestamp);
    }

    /// Complete the quest
    pub fn complete(&mut self, timestamp: i64) {
        self.status = QuestStatus::Completed;
        self.completed_at = Some(timestamp);
    }

    /// Fail the quest
    pub fn fail(&mut self) {
        self.status = QuestStatus::Failed;
    }

    /// Update objective progress
    pub fn update_objective(&mut self, objective_id: &str, progress: i64) {
        self.objective_progress
            .insert(objective_id.to_string(), progress);
    }

    /// Get progress for an objective
    pub fn get_objective_progress(&self, objective_id: &str) -> Option<i64> {
        self.objective_progress.get(objective_id).copied()
    }

    /// Check if an objective is complete
    pub fn is_objective_complete(&self, objective_id: &str, target_value: i64) -> bool {
        self.objective_progress
            .get(objective_id)
            .map_or(false, |p| *p >= target_value)
    }

    /// Check if all objectives are complete
    pub fn all_objectives_complete(&self, quest: &Quest) -> bool {
        quest.objectives.iter().all(|obj| {
            if obj.is_optional {
                return true;
            }
            self.is_objective_complete(&obj.id, obj.target_value)
        })
    }

    /// Check if quest can be started
    pub fn can_start(&self, quest: &Quest, quest_states: &HashMap<String, QuestState>) -> bool {
        self.status == QuestStatus::NotStarted && quest.prerequisites_met_from_states(quest_states)
    }
}

/// Quest manager for tracking multiple quests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestManager {
    pub quests: HashMap<String, Quest>, // quest definitions
    pub quest_states: HashMap<String, QuestState>, // runtime states
}

impl QuestManager {
    /// Create a new quest manager
    pub fn new() -> Self {
        Self {
            quests: HashMap::new(),
            quest_states: HashMap::new(),
        }
    }

    /// Add a quest definition
    pub fn add_quest(&mut self, quest: Quest) {
        self.quests.insert(quest.id.clone(), quest);
    }

    /// Initialize state for a quest
    pub fn initialize_quest(&mut self, quest_id: &str) {
        if let Some(quest) = self.quests.get(quest_id) {
            if !self.quest_states.contains_key(quest_id) {
                let state = QuestState::new(quest);
                self.quest_states.insert(quest_id.to_string(), state);
            }
        }
    }

    /// Start a quest
    pub fn start_quest(&mut self, quest_id: &str, timestamp: i64) -> Result<(), QuestError> {
        if let Some(quest) = self.quests.get(quest_id) {
            // Check prerequisites first before mutable borrow
            if !quest.prerequisites_met_from_states(&self.quest_states) {
                return Err(QuestError::CannotStart);
            }

            if let Some(state) = self.quest_states.get_mut(quest_id) {
                if state.status == QuestStatus::NotStarted {
                    state.start(timestamp);
                    Ok(())
                } else {
                    Err(QuestError::CannotStart)
                }
            } else {
                Err(QuestError::NotInitialized)
            }
        } else {
            Err(QuestError::NotFound)
        }
    }

    /// Update objective progress
    pub fn update_objective(
        &mut self,
        quest_id: &str,
        objective_id: &str,
        progress: i64,
    ) -> Result<(), QuestError> {
        if let Some(state) = self.quest_states.get_mut(quest_id) {
            state.update_objective(objective_id, progress);
            Ok(())
        } else {
            Err(QuestError::NotFound)
        }
    }

    /// Check if a quest is complete and complete it if so
    pub fn check_quest_completion(&mut self, quest_id: &str, timestamp: i64) -> bool {
        if let Some(quest) = self.quests.get(quest_id) {
            if let Some(state) = self.quest_states.get_mut(quest_id) {
                if state.status == QuestStatus::InProgress && state.all_objectives_complete(quest) {
                    state.complete(timestamp);
                    return true;
                }
            }
        }
        false
    }

    /// Get quest state
    pub fn get_quest_state(&self, quest_id: &str) -> Option<&QuestState> {
        self.quest_states.get(quest_id)
    }

    /// Get all quests with a specific status
    pub fn get_quests_by_status(&self, status: QuestStatus) -> Vec<&QuestState> {
        self.quest_states
            .values()
            .filter(|s| s.status == status)
            .collect()
    }
}

impl Default for QuestManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Quest error types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QuestError {
    NotFound,
    NotInitialized,
    CannotStart,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quest_creation() {
        let quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        assert_eq!(quest.id, "test_quest");
        assert_eq!(quest.objectives.len(), 0);
    }

    #[test]
    fn test_quest_state_creation() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        quest.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "obj_desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 5,
            is_optional: false,
        });

        let state = QuestState::new(&quest);
        assert_eq!(state.status, QuestStatus::NotStarted);
        assert_eq!(state.objective_progress.len(), 1);
        assert_eq!(state.get_objective_progress("obj1"), Some(0));
    }

    #[test]
    fn test_quest_start() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        quest.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "obj_desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 5,
            is_optional: false,
        });

        let mut state = QuestState::new(&quest);
        state.start(1000);
        assert_eq!(state.status, QuestStatus::InProgress);
        assert_eq!(state.started_at, Some(1000));
    }

    #[test]
    fn test_objective_progress() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        quest.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "obj_desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 5,
            is_optional: false,
        });

        let mut state = QuestState::new(&quest);
        state.update_objective("obj1", 3);
        assert_eq!(state.get_objective_progress("obj1"), Some(3));
        assert!(!state.is_objective_complete("obj1", 5));

        state.update_objective("obj1", 5);
        assert!(state.is_objective_complete("obj1", 5));
    }

    #[test]
    fn test_quest_completion() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        quest.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "obj_desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 5,
            is_optional: false,
        });

        let mut state = QuestState::new(&quest);
        state.start(1000);
        state.update_objective("obj1", 5);
        state.complete(2000);

        assert_eq!(state.status, QuestStatus::Completed);
        assert_eq!(state.completed_at, Some(2000));
    }

    #[test]
    fn test_all_objectives_complete() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        quest.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "obj_desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 5,
            is_optional: false,
        });
        quest.add_objective(QuestObjective {
            id: "obj2".to_string(),
            description_key: "obj_desc2".to_string(),
            target_type: ObjectiveType::CollectItems {
                item_id: "coin".to_string(),
            },
            target_value: 10,
            is_optional: false,
        });

        let mut state = QuestState::new(&quest);
        state.update_objective("obj1", 5);
        assert!(!state.all_objectives_complete(&quest));

        state.update_objective("obj2", 10);
        assert!(state.all_objectives_complete(&quest));
    }

    #[test]
    fn test_prerequisites() {
        let quest1 = Quest::new("quest1".to_string(), "name1".to_string(), "desc1".to_string());
        let mut quest2 = Quest::new("quest2".to_string(), "name2".to_string(), "desc2".to_string());
        quest2.prerequisites = vec!["quest1".to_string()];

        let mut manager = QuestManager::new();
        manager.add_quest(quest1);
        manager.add_quest(quest2);

        manager.initialize_quest("quest1");
        manager.initialize_quest("quest2");

        let mut states = HashMap::new();
        states.insert("quest1".to_string(), QuestState::new(&manager.quests["quest1"]));

        assert!(!manager.quests["quest2"].prerequisites_met_from_states(&states));

        states.get_mut("quest1").unwrap().status = QuestStatus::Completed;
        assert!(manager.quests["quest2"].prerequisites_met_from_states(&states));
    }

    #[test]
    fn test_quest_manager() {
        let mut quest = Quest::new(
            "test_quest".to_string(),
            "quest_name".to_string(),
            "quest_desc".to_string(),
        );
        quest.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "obj_desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 5,
            is_optional: false,
        });

        let mut manager = QuestManager::new();
        manager.add_quest(quest);
        manager.initialize_quest("test_quest");

        manager.start_quest("test_quest", 1000).unwrap();
        manager.update_objective("test_quest", "obj1", 5).unwrap();
        manager.check_quest_completion("test_quest", 2000);

        let state = manager.get_quest_state("test_quest").unwrap();
        assert_eq!(state.status, QuestStatus::Completed);
    }

    #[test]
    fn test_get_quests_by_status() {
        let mut manager = QuestManager::new();

        let mut quest1 = Quest::new("quest1".to_string(), "name1".to_string(), "desc1".to_string());
        quest1.add_objective(QuestObjective {
            id: "obj1".to_string(),
            description_key: "desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 1,
            is_optional: false,
        });

        let mut quest2 = Quest::new("quest2".to_string(), "name2".to_string(), "desc2".to_string());
        quest2.add_objective(QuestObjective {
            id: "obj2".to_string(),
            description_key: "desc".to_string(),
            target_type: ObjectiveType::KillEnemies {
                enemy_id: "goblin".to_string(),
            },
            target_value: 1,
            is_optional: false,
        });

        manager.add_quest(quest1);
        manager.add_quest(quest2);
        manager.initialize_quest("quest1");
        manager.initialize_quest("quest2");

        manager.start_quest("quest1", 1000).unwrap();
        manager.update_objective("quest1", "obj1", 1).unwrap();
        manager.check_quest_completion("quest1", 2000);

        let in_progress = manager.get_quests_by_status(QuestStatus::InProgress);
        let completed = manager.get_quests_by_status(QuestStatus::Completed);

        assert_eq!(in_progress.len(), 0);
        assert_eq!(completed.len(), 1);
    }
}
