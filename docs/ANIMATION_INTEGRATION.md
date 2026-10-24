# Animation Integration — Design Doc

**Problem:** `kiln-anim` (`Clip`, `Tween`, `Ease`, `FrameEvent`) exists as a library but is not connected to:
- the sprite renderer (no animation component drives sprite UV rects per frame),
- the ECS world (no `AnimationPlayer` / `SpriteAnimation` components),
- the slice runtime (no `play_clip`, `set_clip`, `stop_animation` host calls for KilnScript),
- the player (no animation update in the frame loop).

This doc specifies the integration path. `cargo` is not available here, so this is a design doc to implement when `cargo` is present.

---

## Current state (verified from source)

| Item | Status |
|------|--------|
| `kiln-anim/src/clip.rs` | `Clip { name, texture, frames: Vec<Frame>, loops }`, `Frame { rect, pivot, duration, events }`, `frame_at_time(t) -> (index, local_t)`, `events_at_time(t)`, `total_duration()`. |
| `kiln-anim/src/tween.rs` | `Tween { from, to, duration, ease, loops }`, `value_at(t) -> f32`, `is_complete(t)`. `Ease` enum with 27 curves (Linear, QuadIn/Out, Cubic, Quart, Quint, Sine, Expo, Circ, Back, Elastic, Bounce). |
| `kiln-anim/src/lib.rs` | `pub use clip::{Clip, Frame, FrameEvent}; pub use tween::{Ease, Tween};` plus `AnimError`. |
| `kiln-runtime/src/sprite.rs` | `Sprite { color, size, uv_rect, texture }` — `uv_rect` is the per-frame UV that animation would drive. |
| `kiln-runtime/src/slice.rs` | Builtins list has no animation functions. |
| `kiln-render/src/sprite_renderer.rs` | `SpriteInstance { position, size, color, uv_rect }` — takes `uv_rect` per sprite, so animation can update `uv_rect` each frame. |

---

## Design

### AnimationPlayer component

Add to `kiln-core::Component` or `kiln-runtime`:

```rust
pub struct AnimationPlayer {
    clips: HashMap<String, anim::Clip>,
    current_clip: Option<String>,
    elapsed: f32,
}
impl AnimationPlayer {
    pub fn new() -> Self;
    pub fn add_clip(&mut self, clip: anim::Clip);
    pub fn play(&mut self, clip_name: &str);
    pub fn stop(&mut self);
    pub fn update(&mut self, dt: f32);  // advances elapsed, fires frame events
    pub fn current_uv_rect(&self) -> Option<[f32; 4]>;
    pub fn current_frame(&self) -> Option<&anim::Frame>;
}
```

When `kiln-anim` is not a dependency, this is a transparent stub.

### ECS wiring

Each animated entity gets `Transform + Sprite + AnimationPlayer`. In the render loop, before pushing to `SpriteRenderer`:

```rust
for (transform, sprite, anim) in query::<(&Transform, &Sprite, &mut AnimationPlayer)>() {
    anim.update(dt);
    let mut instance = SpriteInstance {
        position: [transform.position.x, transform.position.y],
        size: sprite.size,
        color: sprite.color,
        uv_rect: anim.current_uv_rect().unwrap_or(sprite.uv_rect),
    };
    // Override texture with the clip's texture if set
    let texture = anim.current_frame().map(|f| &f.texture)...;
    sprite_renderer.push_textured(texture_id, instance);
}
```

### Texture handling

A `Clip.texture` is a project-relative path (e.g. `sprites/hero_walk.png`). The animation system looks up the `TextureId` from the player's `TextureLibrary` (same as `kiln-player/src/textures.rs`). If the clip texture differs from the sprite's base texture, push with the clip's texture ID.

### Frame events

`FrameEvent { name, payload }` (e.g. `"footstep"`, `"hit"`) fire via `events_at_time(t)`. The `AnimationPlayer` can emit these as KilnScript host events or Rust callbacks. For KilnScript integration, expose a `poll_events()` that returns `Vec<(name, payload)>`, which the slice runtime dispatches as script events.

### Tween support

`Tween` + `Ease` are for property interpolation (alpha fade, scale pulse, position lerp). A game can:
- Create a `Tween` per animated property
- Call `tween.value_at(elapsed)` each frame
- Apply the result to `Sprite.color`, `Transform.scale`, or `Transform.position`

Expose `create_tween(from, to, duration, ease)` as a KilnScript host call if needed.

### KilnScript host calls

Add animation builtins to `kiln-runtime/src/slice.rs` `BUILTINS`:

| Function | Args | Returns | Description |
|----------|------|---------|-------------|
| `play_clip(entity_id, clip_name)` | 2 (str, str) | — | Play an animation clip on an entity |
| `stop_animation(entity_id)` | 1 (str) | — | Stop animation on an entity |
| `set_clip(entity_id, clip_name)` | 2 (str, str) | — | Queue/switch to a clip |

The `SliceSession` needs access to the ECS world's `AnimationPlayer` components (via entity source id → hecs entity mapping).

### Acceptance criteria (testable)

1. **Command:** `cargo build -p kiln-anim` compiles (already true; add tests if missing).
2. **Command:** `cargo test -p kiln-anim` passes (clip frame_at_time, tween value_at, ease tests already exist).
3. **Unit:** `AnimationPlayer::play` advances frame; `current_uv_rect()` changes over time.
4. **Unit:** `Clip.frame_at_time` loops correctly when `loops = true`, clamps when `false`.
5. **Runtime:** An entity with `AnimationPlayer` shows different sprite frames over time in the player.

### Out of scope

- Animation state machine (the `NodeType::AnimationState`/`AnimationTransition` in `kiln-graph` already exist as graph nodes; wiring them to the runtime player is a separate step)
- Skeletal / bone animation (2D spritesheet only)
- Blend trees / animation blending

## See Also

- `kiln-anim/src/clip.rs` — `Clip`, `Frame`, `FrameEvent`, `frame_at_time`
- `kiln-anim/src/tween.rs` — `Tween`, `Ease` (27 curves)
- `kiln-runtime/src/sprite.rs` — `Sprite` with `uv_rect` (driven by animation)
- `kiln-render/src/sprite_renderer.rs` — `SpriteInstance`, `push_textured`
- `kiln-player/src/textures.rs` — `TextureLibrary` (texture path → TextureId lookup)
- `kiln-runtime/src/slice.rs` — `BUILTINS` list to add host calls to