# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A05 authored dead zones and finite
camera updates/mapping (2026-11-25), following c21915b. A05 active/PARTIAL. A03/A04
remaining acceptance deferred, not completed; do not retry their gates to delay
progress. User policy: at most three focused attempts per blocked step/check,
retain failure evidence, continue the next feasible step. A01 baseline 6de428b;
A02 remains PARTIAL. Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0106 adds persisted per-scene dead_zone=[width,height] in world units,
  default zero. Initial/reset centers on target plus follow offset; later goal
  stays put inside the centered box and moves minimally outside. Effective zone
  dimensions cap to zoomed viewport extent. Existing response/bounds/pixel snap
  apply, with bounds taking priority. Inspector exposes both dimensions.
- Controller computes/validates candidates before committing stored position or
  initialization. Invalid targets/overflow cannot poison later valid updates.
  f64 intermediates avoid coordinate/matrix sum overflow; invalid/unrepresentable
  projections reject. Screen/world mapping returns finite points or None,
  retaining outside-viewport extrapolation for tools.
- Runtime compatibility is now kiln-authoring-v7; older bundles could ignore the
  new persisted field. Matching editor/CLI/player/prepared browser rebuilt. Old
  v6 bundle retained in target/a05-camera-prior-web-runtime; actual stale export
  rejection verified. VM-memory/save-slot layout unchanged. No dependency.
- Source-only camera_dead_zone sample plus native/browser exact pixel-bounds
  observer mode and CI cases. Red marker becomes green only after 3 ticks to
  avoid initial-frame false positives. Default observer modes stay compatible.
- Prior A05 transformed culling, bounded sprite queue stats/warnings and 16449-
  entity native/browser/editor evidence remain intact. See PLAN_PROGRESS.md,
  CAMERAS_2D.md and evidence/A05_CAMERA_DEAD_ZONES.md.

## Executed validation

- Locked full Rust workspace: 879 PASS. just check PASS. Default/opt-out WASM strict
  Clippy PASS. Python helper suite 49 PASS; Node syntax/py_compile/whitespace PASS.
- Old settings default to zero; nonzero zones round-trip and validate. Existing
  two-scene editor persistence fixture includes nonzero dead zone. Runtime tests
  cover inside/outside movement, bad/missing target recovery, failed initial
  update recovery, extreme finite matrices, 30/120 Hz smoothing, zoom/viewport
  caps, offset/scaled/Y-direction mappings and bounds/pixel snapping.
- Exact native/browser pixels PASS: target 160→180 keeps origin 0, green bounds
  [128,88,192,152]; target 160→220 moves origin to 28, bounds [100,88,164,152].
  Each has 4096 green pixels, relocated export, authoring copy hidden, native PATH
  without developer tools, no browser page errors. Existing default projectile
  game also passes both exports (3932 green pixels). Replays ran serially.
- Stale v6 browser bundle export rejects as incompatible engine build. Matching
  v7 tools prepared. New inspector fields are compiled/persistence tested; no new
  OS widget-drag, physical DPI/device or performance acceptance is claimed.
- Raw target/a05-camera-{focused,final-focused,workspace,quality,wasm,wasm-optout,
  python,precommit,tools-final,web-final,stale-runtime}.log. Pixel evidence under
  target/a05-camera-{inside,edge,default}-{native,browser}/.

## Deferred / limits

- A04 source widget gate incomplete after three attempts (setup-baseline assertion);
  saved-screen UI source dispatch atomic. Exact saved-physics continuation still
  blocked from its prior pose divergence. Do not retry these by default.
- A03 nested/variant identities, arbitrary overrides, multi-file crash transactions
  and combined production acceptance deferred. A02 every-inspector-property combined
  widget acceptance open. No release/3D/external certification approval implied.
- Dead zones track a point, not the target's full bounds. Camera follow history
  resets with its controller and is not persisted in logical saves. Mapping
  extrapolates outside a viewport; gameplay callers must choose/clip their view.

## Next

- Continue A05 shake/transitions/parallax, multiple views/split input routing and
  remaining sorting/mask/outline/blend/material/texture-capacity acceptance. Select
  substantial feasible work by inspecting actual code; keep A03-dependent pieces
  deferred as needed. Apply the same bounded-attempt skip policy.
- Culling still visits ECS entities and uses conservative quad bounds, not a spatial
  index/occlusion system. Dense visible scenes beyond 16384 drop with diagnostics;
  complete independent overlay-queue/texture allocation limits remain open.
