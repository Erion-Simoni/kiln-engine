# Physics Integration — Design Doc

**Problem:** `kiln-runtime/src/physics.rs` is a feature-gated `rapier2d` wrapper (`PhysicsWorld`) that is not connected to:
- the ECS world (no `RigidBody` / `Collider` components),
- the slice runtime (no `apply_force`, `raycast`, `overlap` host calls for KilnScript),
- the tilemap collision system (currently uses `kiln-runtime::tilemap_collision::move_with_collision` against solid tiles, not rapier),
- the player (no `PhysicsWorld` initialization or step in the frame loop).

This doc specifies the integration path. `cargo` is not available here, so this is a design doc to implement when `cargo` is present.

---

## Current state (verified from source)

| Item | Status |
|------|--------|
| `kiln-runtime/src/physics.rs` | `PhysicsWorld` wrapper over rapier2d (`RigidBodySet`, `ColliderSet`, `PhysicsPipeline`, etc.). `new()`, `set_gravity`, `create_rigid_body`, `create_collider`, `step`, `get_rigid_body_position`, `set_rigid_body_velocity`. Feature-gated behind `physics`. No-op stub when disabled. |
| `kiln-runtime/src/lib.rs` | `pub mod audio` behind `#[cfg(feature = "audio")]` — wait, actually `pub mod physics` behind `#[cfg(feature = "physics")]`, re-exports `physics::*`. |
| `kiln-runtime/src/tilemap_collision.rs` | `entity_box`, `first_solid_hit`, `move_with_collision` — axis-split movement against solid tiles (the current 2D collision path). |
| `kiln-runtime/src/slice.rs` | Builtins list has no physics functions. |
| `kiln-runtime/src/world.rs` | `GameWorld` wraps `hecs::World`; entities have `Transform`, `Sprite`, `Role`, `SourceId`. |
| `Cargo.toml` workspace | No `rapier2d` dependency declared at workspace level (though `BUDGETS.md` lists `rapier2d = "0.36.1"` as feature-gated). |

---

## Design

### Dependencies

Add to workspace `Cargo.toml` `[workspace.dependencies]`:
- `rapier2d = "0.36"` — 2D rigid body physics

Any new dependency requires an ADR entry (`docs/DECISIONS.md`) per rule 5.

### ECS components

Add physics components to `kiln-core` or `kiln-runtime`:

```rust
// In kiln-core::Component or a new kiln-runtime physics module
pub struct RigidBody {
    pub handle: rapier2d::dynamics::RigidBodyHandle,
    pub body_type: BodyType,  // Dynamic, Static, Kinematic
}
pub struct Collider {
    pub handle: rapier2d::geometry::ColliderHandle,
    pub shape: ColliderShape,  // Ball, Cuboid, Capsule, Segment
}
```

When `physics` is off, these are transparent stubs (handle = 0) so the ECS compiles unchanged.

### PhysicsWorld in GameWorld

`kiln-runtime::GameWorld` gains an `Option<PhysicsWorld>`:

```rust
pub struct GameWorld {
    world: World,
    physics: Option<PhysicsWorld>,
}
impl GameWorld {
    pub fn new() -> Self { ... }
    pub fn physics(&mut self) -> Option<&mut PhysicsWorld> { ... }
}
```

### Frame loop integration

In the player's `about_to_wait` / `update`, after the slice step and before render:

```rust
if let Some(physics) = &mut self.world.physics {
    physics.step();
}
// Sync physics transforms back to ECS Transform components
for (handle, transform, rigid_body) in query (...) {
    if let Some(pos) = physics.get_rigid_body_position(rigid_body.handle) {
        transform.position = pos.into();
    }
}
```

### Tilemap collision vs rapier

Keep both, but clarify roles:
- **`move_with_collision`** (tilemap_collision) — for 2D platformer-style movement against solid tiles (the primary path for tilemap-based games).
- **rapier2d** — for free-form rigid body physics (stacking crates, bouncing balls, ragdolls).

Games choose one or the other (or both for different entity types). The `PhysicsWorld` can query solid tiles via `tilemap_collision::first_solid_hit` for terrain collision if desired.

### KilnScript host calls

Add physics builtins to `kiln-runtime/src/slice.rs` `BUILTINS`:

| Function | Args | Returns | Description |
|----------|------|---------|-------------|
| `apply_force(entity_id, fx, fy)` | 3 (str, f64, f64) | — | Apply force to a physics body |
| `set_velocity(entity_id, vx, vy)` | 3 (str, f64, f64) | — | Set linear velocity |
| `raycast(x1, y1, x2, y2)` | 4 (f64, f64, f64, f64) | bool | Raycast hit test |
| `set_gravity(gx, gy)` | 2 (f64, f64) | — | Set world gravity |

The `SliceSession` holds a reference to the `PhysicsWorld` and routes host calls.

### Acceptance criteria (testable)

1. **Command:** `cargo build --features physics -p kiln-runtime` compiles.
2. **Command:** `cargo test -p kiln-runtime --features physics` passes (physics world creation / rigid body / step tests).
3. **Runtime:** A dynamic rigid body falls under gravity when `physics.step()` runs each frame.
4. **Runtime:** `set_velocity("player", 5.0, 0.0)` moves the player entity.
5. **No-physics build:** `cargo build -p kiln-runtime` (no `physics` feature) compiles and `PhysicsWorld` is a no-op.

### Out of scope

- Joints, constraints, soft bodies (rapier has them; expose later if needed)
- Continuous collision detection tuning
- 3D physics (M29+)

## See Also

- `kiln-runtime/src/physics.rs` — current stub to wire up
- `kiln-runtime/src/world.rs` — `GameWorld` to add `physics` field
- `kiln-runtime/src/tilemap_collision.rs` — existing 2D collision path (keep alongside)
- `kiln-runtime/src/slice.rs` — `BUILTINS` list to add host calls to
- `kiln-core/src/project.rs` — `Component` enum (add `RigidBody`/`Collider` if desired)
- `docs/BUDGETS.md` — Dependency size tracking
- `docs/DECISIONS.md` — ADR entry needed for `rapier2d`