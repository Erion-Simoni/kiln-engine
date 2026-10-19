//! Debug visualization helpers for AI paths and state

use crate::{Grid, GridNode, NavMesh, Point};
use std::fmt;

/// Format a grid path as a string for debugging
pub fn format_grid_path(grid: &Grid, path: &[GridNode]) -> String {
    let mut output = String::new();
    let width = grid.width();
    let height = grid.height();

    let path_set: std::collections::HashSet<(i32, i32)> =
        path.iter().map(|n| (n.x, n.y)).collect();

    for y in (0..height).rev() {
        for x in 0..width {
            let node = GridNode::new(x, y);
            if path_set.contains(&(x, y)) {
                output.push('*');
            } else if grid.is_obstacle(&node) {
                output.push('#');
            } else {
                output.push('.');
            }
        }
        output.push('\n');
    }

    output
}

/// Format a navmesh path as a string for debugging
pub fn format_navmesh_path(_navmesh: &NavMesh, path: &[Point]) -> String {
    let mut output = String::new();

    output.push_str(&format!("Path with {} points:\n", path.len()));
    for (i, point) in path.iter().enumerate() {
        output.push_str(&format!("  {}: ({:.1}, {:.1})\n", i, point.x, point.y));
    }

    output
}

/// Debug representation of AI state
#[derive(Debug, Clone)]
pub struct AIDebugState {
    pub current_state: String,
    pub current_behavior: String,
    pub path_length: usize,
    pub has_path: bool,
}

impl fmt::Display for AIDebugState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AI State: {} | Behavior: {} | Path: {} nodes ({}found)",
            self.current_state,
            self.current_behavior,
            self.path_length,
            if self.has_path { "" } else { "not " }
        )
    }
}

impl Default for AIDebugState {
    fn default() -> Self {
        Self {
            current_state: "idle".to_string(),
            current_behavior: "none".to_string(),
            path_length: 0,
            has_path: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_grid_path() {
        let mut grid = Grid::new(5, 5);
        grid.set_obstacle(&GridNode::new(2, 2), true);

        let path = vec![
            GridNode::new(0, 0),
            GridNode::new(1, 1),
            GridNode::new(2, 3),
            GridNode::new(3, 3),
        ];

        let output = format_grid_path(&grid, &path);
        assert!(output.contains('*'));
        assert!(output.contains('#'));
        assert!(output.contains('.'));
    }

    #[test]
    fn test_format_navmesh_path() {
        let navmesh = NavMesh::new();
        let path = vec![
            Point::new(0.0, 0.0),
            Point::new(1.5, 2.5),
            Point::new(3.0, 5.0),
        ];

        let output = format_navmesh_path(&navmesh, &path);
        assert!(output.contains("Path with 3 points"));
        assert!(output.contains("(0.0, 0.0)"));
    }

    #[test]
    fn test_ai_debug_state_display() {
        let state = AIDebugState {
            current_state: "patrol".to_string(),
            current_behavior: "chase".to_string(),
            path_length: 10,
            has_path: true,
        };

        let output = format!("{}", state);
        assert!(output.contains("patrol"));
        assert!(output.contains("chase"));
        assert!(output.contains("10 nodes"));
    }
}
