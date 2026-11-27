# Kiln Session Handoff

Branch fix/a01-ci-baseline. A07 saved cutscene tween checkpoint, 2026-11-27,
following 6d5f74f (character animation states). A05/A06/A07 remain PARTIAL.
Authorized scope ends at A07; do not start A08 unasked. At most three focused
attempts per blocked check, then record and move on. A03/A04 gates stay deferred.
Full 3D remains behind G2D.

## Done

- ADR-0114: shared bounded script cutscene service for local-position and sprite
  tint, four easing curves, exact endpoints, replacement, zero-duration cuts,
  pause/seek/cancel and active/remaining queries. IDs 131–137; see CUTSCENE_TWEENS.md.
  Root/attachments share 64 active property tracks with pending-start reservations.
  Mutations commit only after successful callbacks; queries read committed state.
- Position excludes all RigidBody2D entities. Manual property writes cancel matching
  tracks; despawn/component removal cleans bindings. Simulation advances once;
  UI and source-debugger pauses hold progression. Stop clears service.
- Optional GameSave tween snapshot retains source/destination, clock, easing and
  paused state. Missing legacy field defaults empty; invalid bindings reject
  before replacing a running session. No dependency or VM memory-layout change.
- Runtime contract v14; matching editor/CLI/player and browser SDK rebuilt.
  Actual prior v13 browser SDK export rejects.
- Source-authored samples/cutscene_tweens: actor movement/tint plus camera-follow
  Transform rig at zero smoothing. Automatic pause/held-clock/seek/resume/endpoint
  proof turns beacon blue. Space pauses, Right seeks, R restarts, C cancels.
  Saved rig pose/track preserves camera view; direct camera effect history remains
  transient. CI native/browser replay steps added. Existing character states,
  ping-pong/speed, camera and pointer features retained.

## Executed validation

- Full locked Rust workspace: 924 passing; Python helpers: 49 passing. just check,
  strict default and opt-out WASM checks, Node syntax, py_compile and whitespace
  checks passed, including the final pre-commit just check after documentation edits.
- Eight tween integration tests plus math coverage: exact finite interpolation,
  pause/seek, replacement/manual override, shared capacity, callback rollback,
  debugger suspension, UI pause, physics exclusion, corrupt/legacy/resumed saves.
- Camera-rig cutscene native/browser exports pass: green 4096 pixels at
  [192,88,256,152], blue beacon 64. Earlier static-camera variant passes too.
- Character-state real keyboard input regression passes both exports: run moves,
  release idles, one-shot action returns, boundary reached; final green 4096 at
  [256,88,320,152], blue 64. Pointer five-phase regression passes both exports,
  including invalid letterbox/outside positions, green bounds [288,88,352,152].
- All six final replays ran serially after builds, with relocated exports and
  hidden authoring sources. Native player PATH excludes developer tools.
  Receipts/screenshots target/a07-tween-{rig,states,pointer}-{native,browser}.
  Logs target/a07-tween-*.log. See evidence/A07_CUTSCENE_TWEENS.md.

## Open / deferred

- A07 graphical timeline, general/audio/event tracks, blending/blend trees,
  skeletal bones/weights/skinning/IK and complete editor/reimport acceptance.
  Tween completion is script-owned polling; no internal completion-event queue.
  Direct camera transitions/shake history remains transient. Physical F5/F9
  replay not performed; logical JSON save/resume is tested.
- A05 full split-screen authoring/host and remaining rendering/capacity acceptance;
  A06 stamps/streaming and complete level-authoring acceptance remain open.
  Animation 4096-cycle cap/extreme overload and physical performance acceptance
  remain open; no budgets bypassed.
- A04 source widget gate stopped after three attempts; saved-physics pose divergence
  remains. A03 identity/override/crash-transaction and A02 combined inspector gates
  deferred. No physical device/DPI, production performance, release/certification
  or full 3D acceptance claimed.

## Next

Continue useful A07 gameplay/authoring work within authorized scope. Preserve
bounded attempts; do not retry deferred A03/A04 gates or advance past A07 unasked.
