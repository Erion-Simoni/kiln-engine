//! Threads sample: parallel schedule, task pool, async asset load, speedup.
//!
//! Run with: cargo run -p threads-sample --release

use hecs::World;
use kiln_assets::Archive;
use kiln_core::{AccessSet, Transform};
use kiln_runtime::{build_stages, load_file_async, Schedule, SystemEntry, TaskPool};
use std::path::PathBuf;
use std::time::Instant;

struct Velocity;
struct Health;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Kiln Threads Sample (M27)");
    println!("=========================");
    println!();

    demo_access_stages();
    demo_schedule();
    let speedup = demo_task_pool_speedup()?;
    demo_async_asset_load()?;

    println!();
    println!("Measured parallel speedup (chunked CPU work): {speedup:.2}x");
    println!("Record this in docs/BUDGETS.md when re-benchmarking.");
    Ok(())
}

fn demo_access_stages() {
    println!("1) Access-set stage packing");
    let accesses = vec![
        AccessSet::new().write::<Transform>(),
        AccessSet::new().write::<Velocity>(),
        AccessSet::new().read::<Health>(),
        AccessSet::new().write::<Transform>(), // conflicts with first
    ];
    let plan = build_stages(&accesses);
    println!("   systems: 4 → stages: {}", plan.stages.len());
    for (i, stage) in plan.stages.iter().enumerate() {
        println!("   stage {i}: {stage:?}");
    }
}

fn demo_schedule() {
    println!("2) Schedule run (serial World systems)");
    let mut schedule = Schedule::new();
    schedule.add(SystemEntry::new(
        "spawn_marker",
        AccessSet::new().write::<Transform>(),
        |world| {
            world.spawn((Transform::default(),));
        },
    ));
    schedule.add(SystemEntry::new(
        "read_health",
        AccessSet::new().read::<Health>(),
        |_world| {},
    ));
    let mut world = World::new();
    schedule.run(&mut world);
    let entities = world.query::<&Transform>().iter().count();
    println!(
        "   plan stages: {}, entities: {}",
        schedule.plan().stages.len(),
        entities
    );
}

fn demo_task_pool_speedup() -> Result<f64, Box<dyn std::error::Error>> {
    println!("3) TaskPool chunked speedup");
    let n = 2_000_000usize;
    let chunk = 64_000usize;

    // Serial baseline
    let mut data: Vec<f32> = (0..n).map(|i| i as f32).collect();
    let t0 = Instant::now();
    for v in data.iter_mut() {
        *v = v.sin().cos().sin();
    }
    let serial_ms = t0.elapsed().as_secs_f64() * 1000.0;

    // Parallel via TaskPool
    let pool = TaskPool::with_default_size();
    let mut data: Vec<f32> = (0..n).map(|i| i as f32).collect();
    let t1 = Instant::now();
    pool.scope_chunks(&mut data, chunk, |chunk| {
        for v in chunk.iter_mut() {
            *v = v.sin().cos().sin();
        }
    });
    let parallel_ms = t1.elapsed().as_secs_f64() * 1000.0;

    let speedup = if parallel_ms > 0.0 {
        serial_ms / parallel_ms
    } else {
        1.0
    };

    println!(
        "   threads: {} (parallel={})",
        pool.thread_count(),
        pool.is_parallel()
    );
    println!("   serial:   {serial_ms:.2} ms");
    println!("   parallel: {parallel_ms:.2} ms");
    println!(
        "   speedup:  {speedup:.2}x on {} cores",
        pool.thread_count()
    );
    Ok(speedup)
}

fn demo_async_asset_load() -> Result<(), Box<dyn std::error::Error>> {
    println!("4) Async asset load via TaskPool");
    let pool = TaskPool::new(2);

    // Pack a tiny archive in a temp dir, then load a member on a background job.
    let dir = std::env::temp_dir().join("kiln_m27_threads_sample");
    std::fs::create_dir_all(&dir)?;
    let payload = dir.join("note.txt");
    std::fs::write(&payload, b"hello from M27")?;
    let archive_path = dir.join("demo.kilnar");
    let mut archive = Archive::new(archive_path.clone());
    archive.pack(std::slice::from_ref(&payload))?;

    let archive_path_buf = archive_path.clone();
    let handle = pool.spawn(move || -> Result<Vec<u8>, String> {
        let archive = Archive::load(&archive_path_buf).map_err(|e| e.to_string())?;
        archive.extract("note.txt").map_err(|e| e.to_string())
    });
    let bytes = handle.join()?;
    println!("   loaded {} bytes from archive on job thread", bytes.len());

    // Also exercise the file helper
    let file_handle = load_file_async(&pool, PathBuf::from(&payload));
    let file_bytes = file_handle.join()?;
    println!("   load_file_async: {} bytes", file_bytes.len());
    Ok(())
}
