//! Dynamic obstacle support for pathfinding

use crate::grid::{Grid, GridNode};

/// Dynamic obstacle in a grid
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DynamicObstacle {
    pub node: GridNode,
    pub id: u64,
}

impl DynamicObstacle {
    pub fn new(node: GridNode, id: u64) -> Self {
        Self { node, id }
    }
}

/// Manager for dynamic obstacles
#[derive(Debug, Clone)]
pub struct DynamicObstacleManager {
    obstacles: HashMap<GridNode, u64>,
    next_id: u64,
}

use std::collections::HashMap;

impl DynamicObstacleManager {
    pub fn new() -> Self {
        Self {
            obstacles: HashMap::new(),
            next_id: 0,
        }
    }

    /// Add a dynamic obstacle at the given node
    pub fn add_obstacle(&mut self, node: GridNode) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.obstacles.insert(node, id);
        id
    }

    /// Remove a dynamic obstacle by ID
    pub fn remove_obstacle(&mut self, id: u64) -> bool {
        let node_to_remove = self
            .obstacles
            .iter()
            .find(|(_, &obstacle_id)| obstacle_id == id)
            .map(|(node, _)| *node);

        if let Some(node) = node_to_remove {
            self.obstacles.remove(&node);
            return true;
        }
        false
    }

    /// Remove a dynamic obstacle at the given node
    pub fn remove_at(&mut self, node: &GridNode) -> bool {
        self.obstacles.remove(node).is_some()
    }

    /// Check if a node has a dynamic obstacle
    pub fn has_obstacle(&self, node: &GridNode) -> bool {
        self.obstacles.contains_key(node)
    }

    /// Get all dynamic obstacles
    pub fn obstacles(&self) -> impl Iterator<Item = (GridNode, u64)> + '_ {
        self.obstacles.iter().map(|(node, &id)| (*node, id))
    }

    /// Clear all dynamic obstacles
    pub fn clear(&mut self) {
        self.obstacles.clear();
    }

    /// Apply dynamic obstacles to a grid (for pathfinding)
    pub fn apply_to_grid(&self, grid: &mut Grid) {
        for (node, _) in self.obstacles() {
            grid.set_obstacle(&node, true);
        }
    }

    /// Remove dynamic obstacles from a grid
    pub fn remove_from_grid(&self, grid: &mut Grid) {
        for (node, _) in self.obstacles() {
            grid.clear_obstacle(&node);
        }
    }
}

impl Default for DynamicObstacleManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_obstacle_creation() {
        let node = GridNode::new(5, 5);
        let obstacle = DynamicObstacle::new(node, 1);
        assert_eq!(obstacle.node, node);
        assert_eq!(obstacle.id, 1);
    }

    #[test]
    fn test_manager_add_obstacle() {
        let mut manager = DynamicObstacleManager::new();
        let node = GridNode::new(5, 5);
        let id = manager.add_obstacle(node);
        assert_eq!(id, 0);
        assert!(manager.has_obstacle(&node));
    }

    #[test]
    fn test_manager_remove_obstacle() {
        let mut manager = DynamicObstacleManager::new();
        let node = GridNode::new(5, 5);
        let id = manager.add_obstacle(node);
        assert!(manager.remove_obstacle(id));
        assert!(!manager.has_obstacle(&node));
    }

    #[test]
    fn test_manager_remove_at() {
        let mut manager = DynamicObstacleManager::new();
        let node = GridNode::new(5, 5);
        manager.add_obstacle(node);
        assert!(manager.remove_at(&node));
        assert!(!manager.has_obstacle(&node));
    }

    #[test]
    fn test_manager_clear() {
        let mut manager = DynamicObstacleManager::new();
        manager.add_obstacle(GridNode::new(1, 1));
        manager.add_obstacle(GridNode::new(2, 2));
        manager.clear();
        assert!(!manager.has_obstacle(&GridNode::new(1, 1)));
        assert!(!manager.has_obstacle(&GridNode::new(2, 2)));
    }

    #[test]
    fn test_apply_to_grid() {
        let mut manager = DynamicObstacleManager::new();
        let mut grid = Grid::new(10, 10);
        let node = GridNode::new(5, 5);

        manager.add_obstacle(node);
        manager.apply_to_grid(&mut grid);

        assert!(grid.is_obstacle(&node));
    }

    #[test]
    fn test_remove_from_grid() {
        let mut manager = DynamicObstacleManager::new();
        let mut grid = Grid::new(10, 10);
        let node = GridNode::new(5, 5);

        manager.add_obstacle(node);
        manager.apply_to_grid(&mut grid);
        manager.remove_from_grid(&mut grid);

        assert!(!grid.is_obstacle(&node));
    }
}
