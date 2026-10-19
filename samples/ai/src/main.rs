//! AI pathfinding demo

use kiln_ai::{
    format_grid_path, AIDebugState, DynamicObstacleManager, Grid, GridNode, NavMesh, Point,
};

fn main() {
    println!("=== Kiln AI Pathfinding Demo ===\n");

    demo_grid_pathfinding();
    demo_navmesh_pathfinding();
    demo_dynamic_obstacles();
    demo_ai_debug_state();
}

fn demo_grid_pathfinding() {
    println!("--- Grid-based A* Pathfinding ---");

    let mut grid = Grid::new(10, 10);

    // Add some obstacles
    grid.set_obstacle(&GridNode::new(3, 3), true);
    grid.set_obstacle(&GridNode::new(3, 4), true);
    grid.set_obstacle(&GridNode::new(3, 5), true);

    let start = GridNode::new(0, 0);
    let goal = GridNode::new(9, 9);

    println!("Grid size: {}x{}", grid.width(), grid.height());
    println!("Start: ({}, {})", start.x, start.y);
    println!("Goal: ({}, {})", goal.x, goal.y);
    println!("Obstacles at: (3,3), (3,4), (3,5)");

    match grid.find_path(start, goal) {
        Ok(path) => {
            println!("Path found with {} nodes:", path.len());
            for (i, node) in path.iter().enumerate() {
                println!("  {}: ({}, {})", i, node.x, node.y);
            }
            println!("\nVisual representation:");
            println!("* = path, # = obstacle, . = empty");
            println!("{}", format_grid_path(&grid, &path));
        }
        Err(e) => println!("No path found: {:?}", e),
    }

    println!();
}

fn demo_navmesh_pathfinding() {
    println!("--- Navmesh Pathfinding ---");

    let mut navmesh = NavMesh::new();

    // Create a simple L-shaped navmesh
    let poly0 = vec![
        Point::new(0.0, 0.0),
        Point::new(4.0, 0.0),
        Point::new(4.0, 4.0),
        Point::new(0.0, 4.0),
    ];
    let poly1 = vec![
        Point::new(4.0, 0.0),
        Point::new(8.0, 0.0),
        Point::new(8.0, 4.0),
        Point::new(4.0, 4.0),
    ];
    let poly2 = vec![
        Point::new(4.0, 4.0),
        Point::new(8.0, 4.0),
        Point::new(8.0, 8.0),
        Point::new(4.0, 8.0),
    ];

    let id0 = navmesh.add_polygon(poly0);
    let id1 = navmesh.add_polygon(poly1);
    let id2 = navmesh.add_polygon(poly2);

    // Connect polygons
    navmesh.add_connection(id0, id1);
    navmesh.add_connection(id1, id2);

    let start = Point::new(0.5, 0.5);
    let goal = Point::new(7.5, 7.5);

    println!("Navmesh with 3 polygons");
    println!("Start: ({}, {})", start.x, start.y);
    println!("Goal: ({}, {})", goal.x, goal.y);

    match navmesh.find_path(start, goal) {
        Ok(path) => {
            println!("Path found with {} points:", path.len());
            for (i, point) in path.iter().enumerate() {
                println!("  {}: ({}, {})", i, point.x, point.y);
            }
        }
        Err(e) => println!("No path found: {:?}", e),
    }

    println!();
}

fn demo_dynamic_obstacles() {
    println!("--- Dynamic Obstacles ---");

    let mut grid = Grid::new(10, 10);
    let mut manager = DynamicObstacleManager::new();

    let start = GridNode::new(0, 0);
    let goal = GridNode::new(9, 9);

    println!("Grid size: {}x{}", grid.width(), grid.height());
    println!("Start: ({}, {})", start.x, start.y);
    println!("Goal: ({}, {})", goal.x, goal.y);

    // Find initial path
    match grid.find_path(start, goal) {
        Ok(path) => println!("Initial path has {} nodes", path.len()),
        Err(e) => println!("No path found: {:?}", e),
    }

    // Add dynamic obstacle
    let obstacle_node = GridNode::new(5, 5);
    let obstacle_id = manager.add_obstacle(obstacle_node);
    println!(
        "\nAdded dynamic obstacle at ({}, {}) with ID {}",
        obstacle_node.x, obstacle_node.y, obstacle_id
    );

    // Apply obstacles to grid
    manager.apply_to_grid(&mut grid);

    // Find path with obstacle
    match grid.find_path(start, goal) {
        Ok(path) => println!("Path with obstacle has {} nodes", path.len()),
        Err(e) => println!("No path found with obstacle: {:?}", e),
    }

    // Remove obstacle
    manager.remove_obstacle(obstacle_id);
    manager.remove_from_grid(&mut grid);
    println!("\nRemoved dynamic obstacle");

    // Find path again
    match grid.find_path(start, goal) {
        Ok(path) => println!("Path after removal has {} nodes", path.len()),
        Err(e) => println!("No path found: {:?}", e),
    }

    println!();
}

fn demo_ai_debug_state() {
    println!("--- AI Debug State ---");

    let state = AIDebugState {
        current_state: "patrol".to_string(),
        current_behavior: "chase".to_string(),
        path_length: 15,
        has_path: true,
    };

    println!("{}", state);

    let idle_state = AIDebugState::default();
    println!("\nDefault state: {}", idle_state);

    println!();
}
