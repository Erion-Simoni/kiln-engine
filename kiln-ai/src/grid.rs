//! Grid-based A* pathfinding

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Reverse;

/// Node in a grid for pathfinding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridNode {
    pub x: i32,
    pub y: i32,
}

impl GridNode {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &GridNode) -> f32 {
        let dx = (self.x - other.x) as f32;
        let dy = (self.y - other.y) as f32;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn manhattan_distance_to(&self, other: &GridNode) -> i32 {
        (self.x - other.x).abs() + (self.y - other.y).abs()
    }

    pub fn neighbors(&self) -> [GridNode; 8] {
        [
            GridNode::new(self.x - 1, self.y - 1),
            GridNode::new(self.x, self.y - 1),
            GridNode::new(self.x + 1, self.y - 1),
            GridNode::new(self.x - 1, self.y),
            GridNode::new(self.x + 1, self.y),
            GridNode::new(self.x - 1, self.y + 1),
            GridNode::new(self.x, self.y + 1),
            GridNode::new(self.x + 1, self.y + 1),
        ]
    }
}

/// Grid for pathfinding with passable/obstacle cells
#[derive(Debug, Clone)]
pub struct Grid {
    width: i32,
    height: i32,
    obstacles: HashMap<(i32, i32), bool>,
}

impl Grid {
    pub fn new(width: i32, height: i32) -> Self {
        Self {
            width,
            height,
            obstacles: HashMap::new(),
        }
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn is_valid(&self, node: &GridNode) -> bool {
        node.x >= 0 && node.x < self.width && node.y >= 0 && node.y < self.height
    }

    pub fn is_obstacle(&self, node: &GridNode) -> bool {
        self.obstacles.get(&(node.x, node.y)).copied().unwrap_or(false)
    }

    pub fn set_obstacle(&mut self, node: &GridNode, obstacle: bool) {
        if self.is_valid(node) {
            self.obstacles.insert((node.x, node.y), obstacle);
        }
    }

    pub fn clear_obstacle(&mut self, node: &GridNode) {
        self.obstacles.remove(&(node.x, node.y));
    }

    pub fn is_passable(&self, node: &GridNode) -> bool {
        self.is_valid(node) && !self.is_obstacle(node)
    }

    /// Find path from start to goal using A* algorithm
    pub fn find_path(&self, start: GridNode, goal: GridNode) -> Result<Vec<GridNode>, PathfindError> {
        if !self.is_passable(&start) {
            return Err(PathfindError::InvalidStart);
        }
        if !self.is_passable(&goal) {
            return Err(PathfindError::InvalidGoal);
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<GridNode, GridNode> = HashMap::new();
        let mut g_score: HashMap<GridNode, i32> = HashMap::new();
        let mut f_score: HashMap<GridNode, i32> = HashMap::new();

        g_score.insert(start, 0);
        f_score.insert(start, start.manhattan_distance_to(&goal));
        open_set.push(Reverse((f_score[&start], start)));

        while let Some(Reverse((_, current))) = open_set.pop() {
            if current == goal {
                return Ok(self.reconstruct_path(&came_from, current));
            }

            for neighbor in current.neighbors() {
                if !self.is_passable(&neighbor) {
                    continue;
                }

                let tentative_g = g_score[&current] + 1;

                if tentative_g < *g_score.get(&neighbor).unwrap_or(&i32::MAX) {
                    came_from.insert(neighbor, current);
                    g_score.insert(neighbor, tentative_g);
                    let f = tentative_g + neighbor.manhattan_distance_to(&goal);
                    f_score.insert(neighbor, f);
                    open_set.push(Reverse((f, neighbor)));
                }
            }
        }

        Err(PathfindError::NoPath)
    }

    fn reconstruct_path(&self, came_from: &HashMap<GridNode, GridNode>, current: GridNode) -> Vec<GridNode> {
        let mut path = vec![current];
        let mut node = current;

        while let Some(&parent) = came_from.get(&node) {
            path.push(parent);
            node = parent;
        }

        path.reverse();
        path
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathfindError {
    InvalidStart,
    InvalidGoal,
    NoPath,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_creation() {
        let grid = Grid::new(10, 10);
        assert_eq!(grid.width(), 10);
        assert_eq!(grid.height(), 10);
    }

    #[test]
    fn test_grid_valid() {
        let grid = Grid::new(10, 10);
        assert!(grid.is_valid(&GridNode::new(0, 0)));
        assert!(grid.is_valid(&GridNode::new(9, 9)));
        assert!(!grid.is_valid(&GridNode::new(-1, 0)));
        assert!(!grid.is_valid(&GridNode::new(10, 0)));
    }

    #[test]
    fn test_obstacle() {
        let mut grid = Grid::new(10, 10);
        let node = GridNode::new(5, 5);
        assert!(!grid.is_obstacle(&node));

        grid.set_obstacle(&node, true);
        assert!(grid.is_obstacle(&node));

        grid.clear_obstacle(&node);
        assert!(!grid.is_obstacle(&node));
    }

    #[test]
    fn test_passable() {
        let mut grid = Grid::new(10, 10);
        let node = GridNode::new(5, 5);
        assert!(grid.is_passable(&node));

        grid.set_obstacle(&node, true);
        assert!(!grid.is_passable(&node));
    }

    #[test]
    fn test_manhattan_distance() {
        let a = GridNode::new(0, 0);
        let b = GridNode::new(3, 4);
        assert_eq!(a.manhattan_distance_to(&b), 7);
    }

    #[test]
    fn test_neighbors() {
        let node = GridNode::new(5, 5);
        let neighbors = node.neighbors();
        assert_eq!(neighbors.len(), 8);
        assert!(neighbors.contains(&GridNode::new(4, 4)));
        assert!(neighbors.contains(&GridNode::new(5, 4)));
        assert!(neighbors.contains(&GridNode::new(6, 4)));
    }

    #[test]
    fn test_find_path_straight() {
        let grid = Grid::new(10, 10);
        let start = GridNode::new(0, 0);
        let goal = GridNode::new(5, 0);

        let path = grid.find_path(start, goal).unwrap();
        assert!(!path.is_empty());
        assert_eq!(path[0], start);
        assert_eq!(path.last().unwrap(), &goal);
    }

    #[test]
    fn test_find_path_with_obstacle() {
        let mut grid = Grid::new(10, 10);
        let start = GridNode::new(0, 0);
        let goal = GridNode::new(5, 0);

        // Block direct path
        grid.set_obstacle(&GridNode::new(2, 0), true);
        grid.set_obstacle(&GridNode::new(3, 0), true);

        let path = grid.find_path(start, goal).unwrap();
        assert!(!path.is_empty());
        assert_eq!(path[0], start);
        assert_eq!(path.last().unwrap(), &goal);
    }

    #[test]
    fn test_find_path_no_path() {
        let mut grid = Grid::new(10, 10);
        let start = GridNode::new(0, 0);
        let goal = GridNode::new(5, 5);

        // Create a wall blocking the path
        for y in 0..10 {
            grid.set_obstacle(&GridNode::new(3, y), true);
        }

        let result = grid.find_path(start, goal);
        assert_eq!(result, Err(PathfindError::NoPath));
    }

    #[test]
    fn test_find_path_invalid_start() {
        let grid = Grid::new(10, 10);
        let start = GridNode::new(-1, 0);
        let goal = GridNode::new(5, 0);

        let result = grid.find_path(start, goal);
        assert_eq!(result, Err(PathfindError::InvalidStart));
    }

    #[test]
    fn test_find_path_invalid_goal() {
        let grid = Grid::new(10, 10);
        let start = GridNode::new(0, 0);
        let goal = GridNode::new(15, 0);

        let result = grid.find_path(start, goal);
        assert_eq!(result, Err(PathfindError::InvalidGoal));
    }
}
