# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A05 authored sprite parallax
(2026-11-27), following b0dc7e2. A05 active/PARTIAL. A03/A04 acceptance is deferred,
not completed. User policy: at most three focused attempts per blocked step, retain
failure evidence and advance to feasible work. A01 baseline 6de428b; A02 PARTIAL.
Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0109 extends defaulted SpriteLayout with parallax factors/anchor. Defaults
  [1,1]/[0,0] preserve ordinary scrolling and old files/saves. Finite factors -8..8
  support fixed, slow, foreground and reverse motion; anchors must be finite.
- Shared gameplay collector applies camera-relative offset to transformed sprite
  geometry before culling, retaining ordering/materials/UVs. f64 intermediates
  reject unrepresentable draw positions. World transforms, hierarchy and physics
  stay unchanged. Authoring collector retains logical geometry for picking/gizmos;
  Play previews actual parallax in the same path as native/browser exports.
- Inspector/Sprite Editor expose validated undoable factor/anchor controls. Existing
  history/projection/disk and two-scene snapshot/restart fixtures include nondefault
  parallax. Logical gameplay snapshots capture runtime fields and restore them.
  Export validates scene/prefab layouts before publication; invalid layouts reject.
- Contract now kiln-authoring-v10. Matching editor/CLI/player/browser rebuilt;
  actual export with previous v9 bundle rejects. VM/save schema unchanged, no new
  dependency or render pass. Source-only samples/parallax and native/browser CI
  cases added. See CAMERAS_2D.md and evidence/A05_SPRITE_PARALLAX.md.
- Prior A05 camera transitions/shake/dead zones/finite mapping, transformed culling
  and bounded sprite diagnostics remain intact. A03/A04 deferred gates untouched.

## Executed validation

- Full locked Rust workspace 896 PASS (serial), Python helper suite 49 PASS;
  just check and strict default/opt-out WASM Clippy PASS. Node syntax/whitespace
  PASS. Initial development findings (collector inference and unused test binding)
  were corrected before final checks; no blocker required repeated gate retries.
- Tests: old-layout defaults/roundtrip/validation, anchors/negative/foreground
  factors, pivot/rotation/scale/UV preservation, finite overflow, parallax before
  culling, world-transform isolation, authoring geometry, logical-save JSON/restore
  and invalid runtime-save rejection. Actual CLI transaction rejects invalid
  scene/prefab layouts without publication/staging leftovers.
- Native/browser exact green bounds: anchored slow [78,104,142,168], fixed with
  camera [4096,2048] [128,88,192,152], ordinary [48,128,112,192], reverse/foreground
  [168,160,232,224]. Each case has 4096 green/64 blue beacon pixels. Moving cases
  require multiple distinct bounds before completion. Fixed case proves pre-cull
  handling; it would disappear at its logical world position. Prior camera
  transition sample also passes motion/beacon/exact-endpoint native/browser checks.
- All replays run serially after builds, with relocated exports, hidden source
  copies, native developer tools removed from PATH and no browser page errors.
  Raw logs target/a05-parallax-*.log. Receipts/screenshots:
  target/a05-parallax-{native,browser},
  target/a05-parallax-{fixed,default,reverse}-{native,browser},
  target/a05-parallax-transition-{native,browser}.
  Previous v9 bundle: target/a05-parallax-prior-web-runtime. Generated outputs
  are not committed. New controls compiled/history/persistence tested; no new
  OS widget-drag acceptance claimed.

## Deferred / limits

- Sprite parallax only: tiles/particles and automatic repeated backgrounds remain
  separate. Factors are not inherited by descendants. Zoom scales sprites even
  with factor zero; use UI for HUDs. Authoring view deliberately uses logical world
  coordinates; Play shows camera-relative scrolling. Invalid runtime geometry
  rejects draw submission; source layouts are validated by authoring/export/save.
- A04 source widget gate remains incomplete after three attempts (setup-baseline
  assertion), and saved-physics pose continuation retains its prior divergence.
  Do not retry by default. A03 nested/variant identities, arbitrary overrides,
  multi-file crash transactions and combined production acceptance remain deferred.
  A02 full inspector combined widget acceptance remains open.
- Camera transition/shake/follow histories reset with controllers and are not saved
  in logical slots. Culling still visits ECS entities with conservative quads, not
  a spatial index; visible sprites beyond 16384 drop with diagnostics. Independent
  overlay/texture allocation limits remain open. No physical DPI/device, production
  performance, release, 3D or external certification acceptance is claimed.

## Next

- Continue A05 multiple views/split input routing and remaining sorting/masks/
  outlines/blends/materials/texture-capacity acceptance; tile/particle/repeated
  backgrounds remain open too. Inspect actual code for substantial feasible work;
  apply the bounded-attempt skip policy. Keep A03/A04 gates deferred and do not
  begin A06 unasked.
