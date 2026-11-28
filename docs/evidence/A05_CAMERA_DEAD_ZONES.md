# A05 authored camera dead zones and finite update recovery

2026-11-25, fix/a01-ci-baseline, following c21915b. A05 remains active/PARTIAL;
A03/A04 deferred gates were not retried. ADR-0106 records this change.

## Implementation

Per-scene CameraSettings.dead_zone persists full world-unit width/height, defaults
to zero for existing centered follow, and is editable in Gameplay Camera. Initial
creation/reset centers on the target plus follow offset. Later updates hold the
origin inside the box and move the goal minimally outside it. Effective dimensions
are capped to zoomed viewport extent; exponential smoothing, world bounds and pixel
snapping retain their roles. Bounds take priority over the pixel grid.

Camera candidate state is computed locally with f64 intermediates and committed
only after finite ordered view/projection validation. Invalid targets/settings or
unrepresentable coordinates cannot poison subsequent updates. Orthographic matrix
translation avoids intermediate f32 addition overflow. Screen/world conversion
avoids intermediate overflow and rejects non-finite/unrepresentable outputs while
preserving existing outside-viewport extrapolation.

Contract advances to kiln-authoring-v7 because old runtimes could ignore the new
persisted setting. Matching editor/CLI/player and prepared browser runtime rebuilt;
prior v6 bundle retained in target/a05-camera-prior-web-runtime. Actual CLI export
with that v6 bundle rejects as an incompatible engine build. VM-memory signatures
and save-slot layout are unchanged. No dependency added.

## Executed checks

- Locked Rust workspace: 879 tests PASS. just check PASS. Default/opt-out WASM
  strict Clippy PASS. Python helpers 49 PASS; Node syntax/py_compile/diff checks.
- Old camera documents default to zero; settings round-trip and reject negative/
  non-finite zone dimensions. Two-scene editor acceptance fixtures include a
  nonzero zone, covering persistence through existing project/scene workflows.
- Runtime tests check holding/minimal movement, missing/bad target recovery,
  failed first-update recovery, extreme finite matrices, smoothing at 30/120 Hz,
  zoomed zone caps, offset viewport/Y-orientation round trips, bounds and pixel
  snapping. Focused tests rerun after final validation-message/bounds assertions.
- New source-only camera_dead_zone sample delays its green marker until tick 3,
  preventing initial-frame false positives. Both relocated native and Chromium
  exports render 4096 green pixels at exact bounds [128,88,192,152] when target
  moves 160→180 inside the box. Moving it 160→220 produces exact bounds
  [100,88,164,152], proving the minimal camera shift x=28. The marker stays at a
  fixed world position; centered follow would give different pixels. Source copies
  hidden, native player PATH without developer tools; no browser page errors.
- Existing projectile game also passes native/browser exports without a dead zone
  or bounds-observer mode: 3932 green pixels each. This checks default camera and
  shared observer compatibility. All GUI/browser replays ran serially.

Native/browser CI adds both exact-position camera cases and retains receipts via
existing A05 artifact patterns. Hosted execution is not claimed. The new inspector
controls are compiled and persistence-tested, not claimed as a new OS widget-drag
acceptance run. Camera follow history is not added to logical saves. Shake,
transitions/parallax, multiple views/routed input and full A05 production acceptance
remain open; no physical-device/DPI performance certification is claimed.

Raw: target/a05-camera-{focused,final-focused,workspace,quality,wasm,wasm-optout,
python,precommit,tools-final,web-final,stale-runtime}.log. Pixel receipts/screenshots
under target/a05-camera-{inside,edge,default}-{native,browser}/ with sibling logs.
See CAMERAS_2D.md and samples/camera_dead_zone/README.md.
