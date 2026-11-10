# Task 8 browser physics evidence — 2026-11-10

Real WASM/Rapier executed in Chromium with WebGL/SwiftShader. No physical GPU,
audio/controller device, other browser engine or cross-OS certification is
implied. Package size remains informational per owner request (ADR-0062).

## Reproduce

Build matching native tools, run `just web-runtime`, then prepare a second browser
bundle with fonts but no physics for rejection tests (see BROWSER_PHYSICS_2D.md):

```sh
WEB_PHYSICS_MINIMAL_RUNTIME=target/web-no-physics-runtime node tests/web/physics.cjs target/debug/web-runtime
```

The local minimal bundle was target/task8-no-physics-runtime. The test copies
samples/browser_physics2d, invokes the real CLI with empty PATH, relocates output,
hides its authoring project copy and serves only the artifact under a nested URL.
It uses ordinary gameplay save slots to observe the simulation; there is no new
engine test/debug inspection API. result.json records completion; snapshot-summary
contains selected values derived from actual saved payloads. Full snapshots/logs
remain under target/task8-physics-evidence.

## Passed physics/visual behavior

- Hero falls through a sensor and an overlapping deliberately filtered collider,
  then rests on the floor at Y=29.995846. Enter/exit callbacks and layer/source-ID
  raycasts fire. Green body rests at Y≈48 on real solid tile geometry.
- Amber kinematic body advances; distance and revolute constraints hold their
  anchors, and the prismatic motor moves within authored limits. All three types
  remain reconstructed after restore.
- Checkpoint contains upward velocity (about 161.14); after full page reload and
  load, it still moves upward (about 112.09), using saved transform/state rather
  than restarting the scene. Attached Start count remains 1. Subsequent landing
  proves body/tile contacts were reconstructed.
- New scene has a higher floor; hero rests near Y=129.41 and a raycast below it
  finds no old floor. Old bodies/joints disappear. Loading the prior checkpoint
  returns to the saved scene and reconstructs its constraints.
- Screenshot pixel assertions find exactly 400 blue hero pixels, centered at
  (99.5,329.5) on the first floor and (99.5,229.5) on the next. Selected settled,
  restored, scene-change and missing-feature PNGs were inspected. The fixture's
  camera uses the correct lower-left origin so bodies are actually visible.
- Physics-disabled bundle fails CLI export without output, and substituting that
  valid JS/WASM pair at browser startup yields the visible missing-physics error
  instead of silently running bodies without simulation. No unexpected page/
  console errors or failed artifact requests occurred in the supported run.

## Regressions and packaging

- `just check`: formatting, strict native all-target Clippy, workspace/audio sample
  checks pass. `just test`: **660 passed, zero failed/skipped**. Added legacy JSON,
  legacy/new metadata, strict capability flags and body/joint/prefab/solid-tilemap
  detection tests; real CLI checks old-runtime rejection and capable export.
- Strict default/minimal WASM Clippy and native feature-opt-out checks pass.
  Actual font-only/physics-disabled WASM was built/prepared and used in rejection
  tests. Default JS/WASM was rebuilt into target/debug/web-runtime.
- Existing real browser movement, direct saved UI/audio/input/storage and actual
  native-editor Browser Save & Export pass with the new runtime. The editor saves
  X=88 and preserves prior output on failure; its relocated output passes nested
  loading/error/retry, consent, controller/touch, Web Audio API and save/resume
  regressions (editor-browser-export.json and exported-ui-regression.json).
- A new portable bundle includes both saved-UI and physics playground examples.
  Native editor/export/game save/restart acceptance passes again offline and
  unprivileged in clean Ubuntu 24.04 userspace, without Rust/repository/sysroot
  mounts (clean-linux.json). Browser gameplay is tested separately above, not in
  the native-only container. Container shares the host kernel.
- Local archive: target/dist/kiln-task8-browser-physics.tar.gz, SHA-256
  `234417f3390af10d7eb2b7e7ddd7cde829066a596cd99da21661a6f134d71336`.
  Source checkout was dirty on base 1349589; these are preparation observations,
  not binary provenance attestation. Generated archives/binaries are not in Git.
- CI prepares default and opted-out bundles and runs physics acceptance. Remote
  CI execution has not been observed. Sprite material overrides remain rejected;
  material integration, release provenance and physical/other-platform checks
  remain Task 8 work. Task 9/full 3D implementation have not started.
