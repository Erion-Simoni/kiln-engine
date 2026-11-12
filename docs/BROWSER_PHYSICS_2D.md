# Browser 2D physics — Task 8

Default browser builds now use the same Rapier 2D implementation as the native
editor/player. Existing shared scene spawning, tilemap colliders, fixed-step body
and joint synchronization, script queries/events and logical save reconstruction
execute in WASM. No browser-specific physics solver or new dependency is added.

## Prepare/export

```sh
cargo build -p kiln-editor -p kiln-cli -p kiln-player --bins
just web-runtime
target/debug/kiln-cli export --platform web --project samples/browser_physics2d/kiln.toml --output target/physics-game
```

Serve that complete output over HTTP(S), including under a nested path. The normal
loading/consent gate prepares resources before gameplay starts. The playground is
also included in portable editor bundles at `examples/browser_physics2d/kiln.toml`.
Its README lists controls: J jump, C checkpoint, L load, S inspection snapshot,
N next scene. The inspector/export/project format is shared with native authoring.

## Runtime capability and compatibility

`RuntimeInfo.physics` advertises compiled simulation support. Missing fields in
legacy JSON deserialize as false. Existing three-line `kiln.runtime.v1` WASM
metadata likewise defaults false; new metadata appends exactly `physics=0` or
`physics=1`. Invalid/extra/duplicate capability lines are rejected, and prepared
JS/WASM hashes plus descriptor/runtime equality remain checked.

Projects declaring the physics feature or containing authored bodies/joints,
physics prefab definitions or referenced solid tilemaps require a capable selected
runtime. Detection includes unused prefab definitions and potentially solid
layers conservatively, so later spawning/editing cannot silently lose simulation.
The check applies to both Browser and Native exports; current native players
always compile shared physics. Legacy prebuilt runtimes still work for projects
that do not require physics. Rebuild matching editor/exporter/player/web runtime
together when upgrading; older tools may reject extended runtime metadata.

Explicit opt-out remains available:

```sh
cargo build -p kiln-web --target wasm32-unknown-unknown --no-default-features --features fonts
```

That build retains fonts but advertises physics=false. Exporting a physics project
fails without publishing incomplete output. Direct `run`/`prepare_game` callers
also receive an actionable error when trying to load authored physics with an
opted-out browser host, even if they bypass export validation. Complete feature
opt-out (`--no-default-features`) is checked separately by WASM Clippy.

## Executed behavior

`tests/web/physics.cjs` copies the authored playground, exports it with empty tools
PATH through the real CLI, relocates the complete output and hides the project
copy. It serves only that artifact under a nested HTTP path and runs real WASM in
Chromium/SwiftShader. It reads ordinary gameplay slots, rather than adding engine
inspection APIs, to verify:

- Gravity and floor contacts, a deliberately filtered overlapping collider, sensor
  enter/exit callbacks and layer-aware source-ID raycasts.
- A separate green body landing on actual solid tile geometry and a moving
  kinematic body.
- Distance, revolute and prismatic constraints; slider motor movement and limits.
- A checkpoint containing upward velocity, full page reload, load with restored
  transform/velocity and attached Start count 1, then contact reconstruction and
  landing after restore.
- Fresh scene colliders replacing old ones, disappearance of old bodies/joints,
  and loading the saved prior scene with reconstructed constraints.
- Pixel positions of the rendered blue hero at the two actual simulated floor
  heights. A corrected stationary camera keeps the playground visible.
- Missing-physics rejection in both the exporter and real browser loading shell.

Run after preparing default and font-only/physics-disabled bundles:

```sh
WEB_PHYSICS_MINIMAL_RUNTIME=target/web-no-physics-runtime node tests/web/physics.cjs target/debug/web-runtime
```

The optional environment path enables the real opt-out checks. CI prepares both
variants and runs them; local selected captures/results are in
[evidence/task8-physics/README.md](evidence/task8-physics/README.md). Existing
movement, saved UI/input/audio/storage, editor Browser export and relocated-page
acceptance also pass with physics enabled.

## Remaining boundaries

Sprite materials now have a shared validated host; see [SPRITE_MATERIALS_2D.md](SPRITE_MATERIALS_2D.md). This checkpoint does not certify every possible script/platform
integration, other browser engines/mobile or physical GPU/audio/controller
behavior. Reference-hardware startup/frame-time/memory and release provenance
remain separate Task 8 checks. Package-size goals are removed (ADR-0062).
