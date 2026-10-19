//! 2D navmesh pathfinding

use std::collections::{BinaryHeap, HashMap};
use std::cmp::{Reverse, Ordering};

/// Wrapper for f32 with total ordering for use in BinaryHeap
#[derive(Debug, Clone, Copy, PartialEq)]
struct OrderedFloat(f32);

impl Eq for OrderedFloat {}

impl PartialOrd for OrderedFloat {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedFloat {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

/// 2D point
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance_to(&self, other: &Point) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }
}

/// Navigation polygon
#[derive(Debug, Clone)]
pub struct NavPoly {
    pub id: usize,
    pub vertices: Vec<Point>,
    pub center: Point,
}

impl NavPoly {
    pub fn new(id: usize, vertices: Vec<Point>) -> Self {
        let center = Self::compute_center(&vertices);
        Self { id, vertices, center }
    }

    fn compute_center(vertices: &[Point]) -> Point {
        if vertices.is_empty() {
            return Point::new(0.0, 0.0);
        }

        let mut sum_x = 0.0;
        let mut sum_y = 0.0;
        for v in vertices {
            sum_x += v.x;
            sum_y += v.y;
        }

        Point::new(sum_x / vertices.len() as f32, sum_y / vertices.len() as f32)
    }

    pub fn contains(&self, point: &Point) -> bool {
        // Ray casting algorithm for point-in-polygon
        let mut inside = false;
        let n = self.vertices.len();

        for i in 0..n {
            let j = (i + n - 1) % n;
            let vi = &self.vertices[i];
            let vj = &self.vertices[j];

            if ((vi.y > point.y) != (vj.y > point.y))
                && (point.x < (vj.x - vi.x) * (point.y - vi.y) / (vj.y - vi.y) + vi.x)
            {
                inside = !inside;
            }
        }

        inside
    }
}

/// Node in navmesh for pathfinding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NavMeshNode {
    pub poly_id: usize,
}

impl NavMeshNode {
    pub fn new(poly_id: usize) -> Self {
        Self { poly_id }
    }
}

/// Navigation mesh for pathfinding
#[derive(Debug, Clone)]
pub struct NavMesh {
    polygons: Vec<NavPoly>,
    connections: HashMap<usize, Vec<usize>>,
}

impl NavMesh {
    pub fn new() -> Self {
        Self {
            polygons: Vec::new(),
            connections: HashMap::new(),
        }
    }

    pub fn add_polygon(&mut self, vertices: Vec<Point>) -> usize {
        let id = self.polygons.len();
        let poly = NavPoly::new(id, vertices);
        self.polygons.push(poly);
        id
    }

    pub fn add_connection(&mut self, from: usize, to: usize) {
        self.connections.entry(from).or_insert_with(Vec::new).push(to);
        self.connections.entry(to).or_insert_with(Vec::new).push(from);
    }

    pub fn get_polygon(&self, id: usize) -> Option<&NavPoly> {
        self.polygons.get(id)
    }

    pub fn find_poly_containing(&self, point: &Point) -> Option<usize> {
        for (i, poly) in self.polygons.iter().enumerate() {
            if poly.contains(point) {
                return Some(i);
            }
        }
        None
    }

    /// Find path from start point to goal point using A* on navmesh
    pub fn find_path(&self, start: Point, goal: Point) -> Result<Vec<Point>, PathfindError> {
        let start_poly = self.find_poly_containing(&start)
            .ok_or(PathfindError::InvalidStart)?;
        let goal_poly = self.find_poly_containing(&goal)
            .ok_or(PathfindError::InvalidGoal)?;

        if start_poly == goal_poly {
            return Ok(vec![start, goal]);
        }

        let mut open_set = BinaryHeap::new();
        let mut came_from: HashMap<usize, usize> = HashMap::new();
        let mut g_score: HashMap<usize, f32> = HashMap::new();
        let mut f_score: HashMap<usize, f32> = HashMap::new();

        let _start_node = NavMeshNode::new(start_poly);
        let _goal_node = NavMeshNode::new(goal_poly);

        g_score.insert(start_poly, 0.0);
        let start_poly_center = self.polygons[start_poly].center;
        let goal_poly_center = self.polygons[goal_poly].center;
        f_score.insert(start_poly, start_poly_center.distance_to(&goal_poly_center));
        open_set.push(Reverse((OrderedFloat(f_score[&start_poly]), start_poly)));

        while let Some(Reverse((_, current))) = open_set.pop() {
            if current == goal_poly {
                return Ok(self.reconstruct_path(&came_from, current, start, goal));
            }

            let current_poly_center = self.polygons[current].center;

            if let Some(neighbors) = self.connections.get(&current) {
                for &neighbor in neighbors {
                    let neighbor_poly_center = self.polygons[neighbor].center;
                    let distance = current_poly_center.distance_to(&neighbor_poly_center);
                    let tentative_g = g_score[&current] + distance;

                    if tentative_g < *g_score.get(&neighbor).unwrap_or(&f32::MAX) {
                        came_from.insert(neighbor, current);
                        g_score.insert(neighbor, tentative_g);
                        let f = tentative_g + neighbor_poly_center.distance_to(&goal_poly_center);
                        f_score.insert(neighbor, f);
                        open_set.push(Reverse((OrderedFloat(f), neighbor)));
                    }
                }
            }
        }

        Err(PathfindError::NoPath)
    }

    fn reconstruct_path(&self, came_from: &HashMap<usize, usize>, current: usize, start: Point, goal: Point) -> Vec<Point> {
        let mut poly_path = vec![current];
        let mut node = current;

        while let Some(&parent) = came_from.get(&node) {
            poly_path.push(parent);
            node = parent;
        }

        poly_path.reverse();

        // Convert polygon path to point path using polygon centers
        let mut point_path = vec![start];
        for poly_id in &poly_path {
            if let Some(poly) = self.polygons.get(*poly_id) {
                point_path.push(poly.center);
            }
        }
        point_path.push(goal);

        point_path
    }
}

impl Default for NavMesh {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PathfindError {
    InvalidStart,
    InvalidGoal,
    NoPath,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_distance() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!((a.distance_to(&b) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_navpoly_center() {
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        let poly = NavPoly::new(0, vertices);
        assert!((poly.center.x - 1.0).abs() < 0.001);
        assert!((poly.center.y - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_navpoly_contains() {
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        let poly = NavPoly::new(0, vertices);

        assert!(poly.contains(&Point::new(1.0, 1.0)));
        assert!(poly.contains(&Point::new(0.5, 0.5)));
        assert!(!poly.contains(&Point::new(3.0, 3.0)));
    }

    #[test]
    fn test_navmesh_creation() {
        let navmesh = NavMesh::new();
        assert_eq!(navmesh.polygons.len(), 0);
    }

    #[test]
    fn test_navmesh_add_polygon() {
        let mut navmesh = NavMesh::new();
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        let id = navmesh.add_polygon(vertices);
        assert_eq!(id, 0);
        assert_eq!(navmesh.polygons.len(), 1);
    }

    #[test]
    fn test_navmesh_add_connection() {
        let mut navmesh = NavMesh::new();
        navmesh.add_polygon(vec![Point::new(0.0, 0.0), Point::new(2.0, 0.0), Point::new(2.0, 2.0)]);
        navmesh.add_polygon(vec![Point::new(2.0, 0.0), Point::new(4.0, 0.0), Point::new(4.0, 2.0)]);
        navmesh.add_connection(0, 1);

        assert!(navmesh.connections.get(&0).unwrap().contains(&1));
        assert!(navmesh.connections.get(&1).unwrap().contains(&0));
    }

    #[test]
    fn test_navmesh_find_poly_containing() {
        let mut navmesh = NavMesh::new();
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        navmesh.add_polygon(vertices);

        let result = navmesh.find_poly_containing(&Point::new(1.0, 1.0));
        assert_eq!(result, Some(0));

        let result = navmesh.find_poly_containing(&Point::new(3.0, 3.0));
        assert_eq!(result, None);
    }

    #[test]
    fn test_navmesh_find_path_same_poly() {
        let mut navmesh = NavMesh::new();
        let vertices = vec![
            Point::new(0.0, 0.0),
            Point::new(2.0, 0.0),
            Point::new(2.0, 2.0),
            Point::new(0.0, 2.0),
        ];
        navmesh.add_polygon(vertices);

        let start = Point::new(0.5, 0.5);
        let goal = Point::new(1.5, 1.5);

        let path = navmesh.find_path(start, goal).unwrap();
        assert_eq!(path.len(), 2);
        assert_eq!(path[0], start);
        assert_eq!(path[1], goal);
    }

    #[test]
    fn test_navmesh_find_path_multiple_polys() {
        let mut navmesh = NavMesh::new();
        let poly0 = vec![Point::new(0.0, 0.0), Point::new(2.0, 0.0), Point::new(2.0, 2.0), Point::new(0.0, 2.0)];
        let poly1 = vec![Point::new(2.0, 0.0), Point::new(4.0, 0.0), Point::new(4.0, 2.0), Point::new(2.0, 2.0)];
        let poly2 = vec![Point::new(4.0, 0.0), Point::new(6.0, 0.0), Point::new(6.0, 2.0), Point::new(4.0, 2.0)];

        navmesh.add_polygon(poly0);
        navmesh.add_polygon(poly1);
        navmesh.add_polygon(poly2);
        navmesh.add_connection(0, 1);
        navmesh.add_connection(1, 2);

        let start = Point::new(0.5, 1.0);
        let goal = Point::new(5.5, 1.0);

        let path = navmesh.find_path(start, goal).unwrap();
        assert!(!path.is_empty());
        assert_eq!(path[0], start);
        assert_eq!(path.last().unwrap(), &goal);
    }

    #[test]
    fn test_navmesh_find_path_invalid_start() {
        let navmesh = NavMesh::new();
        let start = Point::new(0.0, 0.0);
        let goal = Point::new(1.0, 1.0);

        let result = navmesh.find_path(start, goal);
        assert_eq!(result, Err(PathfindError::InvalidStart));
    }

    #[test]
    fn test_navmesh_find_path_invalid_goal() {
        let mut navmesh = NavMesh::new();
        navmesh.add_polygon(vec![Point::new(0.0, 0.0), Point::new(2.0, 0.0), Point::new(2.0, 2.0)]);
        let start = Point::new(0.5, 0.5);
        let goal = Point::new(5.0, 5.0);

        let result = navmesh.find_path(start, goal);
        assert_eq!(result, Err(PathfindError::InvalidGoal));
    }
}
