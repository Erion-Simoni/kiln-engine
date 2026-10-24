# Audio Integration — Design Doc

**Problem:** `kiln-runtime/src/audio.rs` is a feature-gated stub. It opens the default audio device via rodio and can `play_file(path)`, but it is not connected to:
- the slice runtime (no `play_sound`/`stop_sound` host calls for KilnScript),
- the player (no audio device initialization in `kiln-player`),
- asset loading (no decoding of OGG/Opus from project assets),
- volume / music / SFX separation.

This doc specifies the integration path. `cargo` is not available here, so this is a design doc to implement when `cargo` is present.

---

## Current state (verified from source)

| Item | Status |
|------|--------|
| `kiln-runtime/src/audio.rs` | `AudioEngine` struct, `new()`, `play_file(path)`. Feature-gated behind `audio`. No-op stub when disabled. |
| `kiln-runtime/src/lib.rs` | `pub mod audio` behind `#[cfg(feature = "audio")]`, re-exports `audio::*`. |
| `Cargo.toml` workspace | No `rodio` / `symphonia` dependency declared at workspace level. |
| `kiln-player/src/main.rs` | No audio initialization; no `AudioEngine` usage. |
| `kiln-runtime/src/slice.rs` | Builtins list has no audio functions (`play_sound`, `stop_sound`, etc.). |
| `BUDGETS.md` | Audio section (R0) — stub using rodio's high-level API; device management deferred to M49 per ADR-0014. |

---

## Design

### Dependencies

Add to workspace `Cargo.toml` `[workspace.dependencies]`:
- `rodio = "0.19"` — playback backend
- `symphonia = "0.7"` — OGG/Opus decoding (or `cpal` + a decoder of choice)

Any new dependency requires an ADR entry (`docs/DECISIONS.md`) per rule 5 (size, license, maintenance).

### AudioEngine (kiln-runtime/src/audio.rs)

Extend the existing `AudioEngine` to support multiple simultaneous sources and basic control:

```rust
#[cfg(feature = "audio")]
pub struct AudioEngine {
    sink: MixerDeviceSink,
    sources: HashMap<String, rodio::Sink>,  // named sounds (music / SFX)
}

#[cfg(feature = "audio")]
impl AudioEngine {
    pub fn new() -> Result<Self, ...> { ... }
    pub fn play_sound(&mut self, key: &str, source: rodio::Source) { ... }
    pub fn play_music(&mut self, source: rodio::Source) { ... }  // loops, lower volume
    pub fn stop_sound(&mut self, key: &str) { ... }
    pub fn stop_all(&mut self) { ... }
    pub fn set_master_volume(&mut self, volume: f32) { ... }  // 0.0–1.0
}
```

Keep the no-op stub when `audio` is off so non-audio builds compile unchanged.

### KilnScript host calls

Add audio builtins to `kiln-runtime/src/slice.rs` `BUILTINS`:

| Function | Args | Returns | Description |
|----------|------|---------|-------------|
| `play_sound(key)` | 1 (str) | — | Play a named SFX |
| `stop_sound(key)` | 1 (str) | — | Stop a named sound |
| `play_music(path)` | 1 (str) | — | Stream/loop background music |
| `stop_music()` | 0 | — | Stop background music |
| `set_volume(volume)` | 1 (f64) | — | Master volume 0–1 |

The `SliceSession` holds an `Option<AudioEngine>` and routes host calls to it.

### Asset loading

Audio assets live in the project directory (e.g. `audio/jump.ogg`). The player loads the project manifest via `kiln-player/src/loader.rs` which already collects all files into `assets: HashMap<String, Vec<u8>>`. Decode bytes in memory with `symphonia` instead of `File::open`, so the same code path works for `.kar` archives and web.

### Player wiring

In `kiln-player/src/main.rs`:
1. After GPU init, create `AudioEngine::new()` (feature-gated; ignore failure on headless/web).
2. Pass it into the `PlayerApp` and `SliceSession`.
3. On window resize / focus, pause/resume music appropriately.

### Acceptance criteria (testable)

1. **Command:** `cargo build --features audio -p kiln-runtime` compiles.
2. **Command:** `cargo test -p kiln-runtime --features audio` passes (audio engine creation test).
3. **Runtime:** A KilnScript calling `play_sound("jump")` produces audio on desktop.
4. **Runtime:** `set_volume(0.5)` halves output volume.
5. **No-audio build:** `cargo build -p kiln-runtime` (no `audio` feature) compiles and `AudioEngine` is a no-op.

### Out of scope

- Spatial / 3D audio (M29+)
- Device selection / hot-plug (deferred to M49 per ADR-0014)
- Web Audio API backend (web uses rodio's default or falls back to no-op)

## See Also

- `kiln-runtime/src/audio.rs` — current stub to extend
- `kiln-runtime/src/slice.rs` — `BUILTINS` list to add host calls to
- `kiln-player/src/loader.rs` — asset collection (`assets: HashMap<String, Vec<u8>>`)
- `docs/BUDGETS.md` — Audio section (R0)
- `docs/DECISIONS.md` — ADR-0014 (audio device management deferred)