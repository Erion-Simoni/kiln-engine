//! Physics sample: 2D collision demo
//! Run with: cargo run --bin physics-sample

use kiln_runtime::PhysicsWorld;
use rapier2d::prelude::*;

fn main() {
    println!("Kiln Physics Sample");
    println!("====================");
    println!();

    let mut world = PhysicsWorld::new();
    println!("Physics world created with gravity: ({}, {})", 0.0, -9.81);

    // Create a dynamic ball falling from height
    let ball = RigidBodyBuilder::dynamic().translation(Vec2::new(0.0, 10.0));
    let ball_handle = world.create_rigid_body(ball);
    let ball_collider = ColliderBuilder::ball(1.0);
    world.create_collider(ball_collider, ball_handle);
    println!("Created dynamic ball at (0.0, 10.0)");

    // Create a static floor
    let floor = RigidBodyBuilder::fixed().translation(Vec2::new(0.0, -2.0));
    let floor_handle = world.create_rigid_body(floor);
    let floor_collider = ColliderBuilder::cuboid(10.0, 0.5);
    world.create_collider(floor_collider, floor_handle);
    println!("Created static floor at (0.0, -2.0)");

    println!();
    println!("Simulating physics steps...");
    for i in 0..10 {
        world.step();
        if let Some(pos) = world.get_rigid_body_position(ball_handle) {
            println!("Step {}: ball at ({:.2}, {:.2})", i + 1, pos.x, pos.y);
        }
    }

    println!();
    println!("Physics simulation complete");
}
