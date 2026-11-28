# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current A07 checkpoint: rotation and scale cutscene
tracks, 2026-11-28, following 140ed9b (saved movement/tint tweens).
A05/A06/A07 remain PARTIAL. Authorized scope stops at A07; do not start A08
unasked. At most three focused attempts per blocked check, then record and move
on. A03/A04 gates remain deferred. Full 3D remains behind G2D.

## Done

- ADR-0115 extends ADR-0114's shared saved tween service with local rotation and
  XY scale. tween_rotation(entity,radians,seconds,ease) ID 138, arity 4;
  tween_scale(entity,x,y,seconds,ease) ID 139, arity 5. Property controls/query
  accept position, color, rotation and scale. Four easing curves, bounded shared
  capacity, callback atomicity, pause/seek/cancel and exact endpoints retained.
- Rotation uses unwrapped radians for deliberate multi-turn spins. Positive scale
  sources/destinations >= 0.000001 preserve the existing non-singular logical-save
  geometry contract. Zero crossings/mirroring are deferred. All transform tracks
  reject RigidBody2D ownership; tint remains usable on bodies.
- Position-only writes preserve rotation/scale tracks. Removing Transform clears
  all transform tracks, preserving tint. Fixed pre-existing script removal of
  Transform/Sprite: remove_one removes components instead of Bundle-derived fields.
- Logical GameSave records rotation/scale sources/destinations, clocks, easing and
  pause state through the existing optional snapshot. Corrupt shape/bindings reject;
  legacy optional-field behavior and VM memory layout unchanged. No new dependency.
- Runtime contract v15; matching editor/CLI/player and browser SDK rebuilt.
  Actual prior v14 browser SDK export rejects.
- samples/transform_tweens composes actor movement, tint, 450-degree spin, XY
  stretch and camera-follow dolly. It automatically pauses five tracks, checks
  held clocks, seeks while paused, resumes and gates completion with blue beacon.
  Space pauses/resumes, Right seeks, R restarts, C cancels. Source Editor authoring;
  no graphical timeline. CI native/browser replay steps added.
- Existing character states, ping-pong/speed, saved movement/tint, camera effects,
  sparse tilemaps, prepared views/pointer and parallax work remains intact.

## Executed validation

- Full locked Rust workspace: 927 passing tests, zero failed, 100 suites.
  All 11 tween integration tests pass; new coverage includes unwrapped spins,
  scale midpoint/endpoints, paused seek and JSON save/resume, corrupt dimensions,
  replacement/cancel/cut, position-write independence, Transform removal, physics
  ownership and pending shared capacity. Python helper suite: 49 passing.
- just check and strict WASM default/opt-out Clippy pass. Matching native/browser
  tools built serially before GUI replays. Actual stale v14 export rejects.
- Transform demo native/browser exports pass: 8192 green pixels at
  [192,56,256,184], blue completion beacon 64, relocated/hidden sources and
  native player PATH without developer tools. Receipts target/a07-transform-
  {native,browser}.
- Earlier movement/tint/camera-rig demo also passes both exports, green 4096 at
  [192,88,256,152], blue 64. Receipts target/a07-transform-cutscene-{native,browser}.
  All four replays ran serially after builds. Final pre-commit just check and
  whitespace checks pass (target/a07-transform-precommit.log).
- Logs target/a07-transform-{focused,workspace,python,quality,wasm,wasm-optout,
  tools,web,stale}.log. Final export replay outcomes and pre-commit receipt are
  recorded in evidence/A07_TRANSFORM_TWEENS.md. Generated artifacts stay in target.

## Open / deferred

- A07 graphical timeline, general/audio/event tracks, blending/blend trees,
  skeletal bones/weights/skinning/IK and complete editor/reimport acceptance.
  Transform-only camera-follow rig is not a skeletal rig. Tween completion remains
  script-owned polling; no internal completion-event queue. Direct camera effect
  controller history remains transient. No physical save-hotkey replay claimed;
  logical JSON save/resume is tested.
- A05 full split-screen authoring/host and remaining rendering/capacity acceptance;
  A06 stamps/streaming and complete level-authoring acceptance remain open.
  Animation 4096-cycle cap/extreme overload and physical performance acceptance
  remain open; no budget bypass introduced.
- A04 source widget gate stopped after three attempts; saved-physics pose divergence
  remains. A03 identity/override/crash-transaction and A02 combined inspector gates
  deferred. No physical device/DPI, production performance, release/certification
  or full 3D acceptance claimed.

## Next

Continue useful A07 gameplay/authoring work within authorized scope. Keep bounded
attempts; do not retry deferred A03/A04 gates or advance past A07 unasked.
