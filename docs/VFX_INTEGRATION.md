# VFX / Particle Integration — Design Doc

**Problem:** `kiln-vfx` (CPU `ParticleSystem` + GPU `GpuParticleSystem`, `Emitter`, `Particle`) exists as a library but is not connected to:
- the ECS world (no `ParticleEmitter` component),
- the sprite renderer (no per-frame particle → sprite instance conversion),
- the slice runtime (no `spawn_particles` host call for KilnScript),
- the player (no VFX update/render in the frame loop).

This doc specifies the integration path. `cargo` is not available here, so this is a design doc to implement when `cargo` is present.

---

## Current state (verified from source)

| Item | Status |
|------|--------|
| `kiln-vfx/src/system.rs` | `ParticleSystem { config, emitters, particles }`, `update(dt)` (spawns + integrates gravity + removes dead), `add_emitter`, `particle_count`, `particles()`. |
| `kiln-vfx/src/emitter.rs` | `EmitterConfig { shape, rate, speed_range, angle_range, lifetime_range, size_range, color, gravity, fade_out }`, `Emitter { config, position, time_since_last_emit }`, `update(dt) -> u32` (count to emit), `spawn_position/velocity/lifetime/size/color`. Shapes: `Point`, `Circle`, `Rectangle`. |
| `kiln-vfx/src/particle.rs` | `Particle { position, velocity, color, size, lifetime, age }`, `is_alive()`, `normalized_age()`. `ParticleData` (repr C, Pod) matches shader layout for GPU. |
| `kiln-vfx/src/gpu.rs` | `GpuParticleSystem { particle_buffer, uniform_buffer, compute_pipeline, bind_group }`, `new(device, config)`, `update(device, queue, dt, gravity, time)`, `particle_buffer()`, `set_particle_count()`. Uses `particle_compute.wgsl` compute shader. `MAX_GPU_PARTICLES = 10_000`. |
| `kiln-vfx/src/lib.rs` | `pub use emitter::{EmissionShape, Emitter, EmitterConfig}; pub use gpu::{GpuParticleSystem, GpuParticleSystemConfig, MAX_GPU_PARTICLES}; pub use particle::{Particle, ParticleData}; pub use system::{ParticleSystem, ParticleSystemConfig};` |
| `kiln-render/src/sprite_renderer.rs` | `SpriteInstance { position, size, color, uv_rect }`, `push_textured(TextureId, instance)`, batch-by-texture. Particles can be drawn as textured quads or solid-color quads. |

---

## Design

### ParticleEmitter component

Add to `kiln-core::Component` or `kiln-runtime`:

```rust
pub struct ParticleEmitter {
    system: vfx::ParticleSystem,   // CPU simulation
    gpu_system: Option<vfx::GpuParticleSystem>,  // GPU render path (when device available)
    texture: Option<TextureId>,    // particle sprite texture
    active: bool,
}
impl ParticleEmitter {
    pub fn new(config: vfx::EmitterConfig) -> Self;
    pub fn update(&mut self, dt: f32);  // spawn + integrate particles
    pub fn particles(&self) -> &[vfx::Particle];
    pub fn set_texture(&mut self, id: TextureId);
    pub fn set_position(&mut self, pos: [f32; 2]);
}
```

When `kiln-vfx` is not wired, this is a transparent stub.

### ECS wiring

Each VFX entity gets `Transform + ParticleEmitter`. In the render loop:

```rust
for (transform, emitter) in query::<(&Transform, &mut ParticleEmitter)>() {
    emitter.update(dt);
    emitter.set_position([transform.position.x, transform.position.y]);
    
    for particle in emitter.particles() {
        sprite_renderer.push(SpriteInstance {
            position: particle.position,
            size: [particle.size, particle.size],
            color: particle.color,
            uv_rect: FULL_UV,  // or a particle UV rect
        });
    }
}
```

CPU particles → `SpriteRenderer` instances (one draw call per texture via batching). GPU particles → `GpuParticleSystem.particle_buffer()` rendered as a compute-driven vertex buffer (separate render pass).

### Texture

Particles typically use a small soft circle/square texture (e.g. `vfx/particle.png`). Load via `TextureLibrary` like sprites (`kiln-player/src/textures.rs`).

### KilnScript host calls

Add VFX builtins to `kiln-runtime/src/slice.rs` `BUILTINS`:

| Function | Args | Returns | Description |
|----------|------|---------|-------------|
| `spawn_particles(entity_id, effect_name)` | 2 (str, str) | — | Spawn a particle effect at an entity |
| `spawn_particles_at(x, y, effect_name)` | 3 (f64, f64, str) | — | Spawn at world position |

The `SliceSession` looks up the entity's `ParticleEmitter` and triggers a burst (temporarily raise `EmitterConfig.rate` or directly push particles).

### CPU vs GPU path

- **CPU** (`ParticleSystem`): simpler, works on all platforms, drawn via `SpriteRenderer`. Good for modest counts (< 1000 particles).
- **GPU** (`GpuParticleSystem`): compute shader, 10k particles, drawn via dedicated vertex buffer. Requires a wgpu device; better for explosions, magic, rain.

Games choose based on tier: Low uses CPU or no VFX; High uses GPU.

### Acceptance criteria (testable)

1. **Command:** `cargo build -p kiln-vfx` compiles (already true).
2. **Command:** `cargo test -p kiln-vfx` passes (emitter, particle, system tests already exist).
3. **Unit:** `ParticleEmitter::update` spawns particles and they age/die.
4. **Unit:** Particles are removed when `age >= lifetime`.
5. **Runtime:** An entity with `ParticleEmitter` renders particles as sprite quads in the player.

### Out of scope

- GPU particle rendering pass (compute → vertex buffer draw) — requires a dedicated render pipeline; document separately
- Particle trails, ribbons, decals
- Shaders beyond `particle_compute.wgsl`

## See Also

- `kiln-vfx/src/system.rs` — `ParticleSystem`, `update`
- `kiln-vfx/src/emitter.rs` — `EmitterConfig`, `Emitter`, shapes
- `kiln-vfx/src/particle.rs` — `Particle`, `ParticleData`
- `kiln-vfx/src/gpu.rs` — `GpuParticleSystem`, compute pipeline
- `kiln-render/src/sprite_renderer.rs` — `SpriteInstance`, `push_textured` (CPU particle draw target)
- `kiln-player/src/textures.rs` — `TextureLibrary` (particle texture lookup)
- `kiln-runtime/src/slice.rs` — `BUILTINS` list to add host calls to