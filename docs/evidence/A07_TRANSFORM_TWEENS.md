# A07 rotation and scale cutscene checkpoint — 2026-11-28

ADR-0115 extends the saved cutscene service with local rotation and XY scale.
Rotation interpolates unwrapped radians, preserving intentional multi-turn spins.
Scale sources/destinations are positive and at least 0.000001, preserving existing
non-singular saved geometry. Negative-scale mirroring and zero crossings are
outside this checkpoint. All transform tracks reject RigidBody2D ownership.

The same pause/seek/cancel/query APIs and shared 64-property capacity apply.
Position-only manual writes preserve spin/stretch; Transform removal clears all
transform tracks. An integration test exposed a pre-existing removal bug:
Transform and Sprite derive ECS Bundle, so remove::<T> removed their bundle fields
instead of the component. Script removal now uses remove_one::<T>.

Executed validation:

- Full locked Rust workspace: 927 passing tests, zero failures, 100 test/doc-test
  suites (`target/a07-transform-workspace.log`). All 11 tween integration tests
  pass. New cases cover unwrapped spins, scale midpoint/endpoints, paused seek,
  JSON save/resume, corrupt dimensions, replacement, cut/cancel, position-write
  independence, Transform removal, physics exclusion and shared pending capacity.
- Python helper suite: 49 passing. Quality check and strict WASM default/opt-out
  checks pass. Logs: `target/a07-transform-{focused,python,quality,wasm,wasm-optout}.log`.
- Matching native tools and browser SDK use runtime contract v15. Older v14 SDKs
  cannot implement the new builtins and are intentionally incompatible.

The source fixture samples/transform_tweens composes position, tint, a 450-degree
spin, XY stretch and camera-follow rig. Its automatic sequence pauses all five
tracks, checks held clocks, seeks while paused and resumes. A blue beacon gates
completion. Expected final actor bounds are [192,56,256,184], 8192 green pixels.

A07 remains PARTIAL: graphical timeline, general/audio/event tracks, blending,
skeletal rigging/IK and complete editor/reimport acceptance remain open. No
physical device/DPI, performance budget or full A07 acceptance is claimed.

Actual relocated native and nested browser transform exports both pass: 8192 green
pixels at [192,56,256,184], blue completion beacon 64. Authoring copies are hidden;
native player PATH excludes developer tools. Receipts/screenshots:
`target/a07-transform-{native,browser}`. Actual prior v14 browser SDK export
rejects with the expected incompatible-engine diagnostic
(`target/a07-transform-stale.log`). Matching tools and SDK builds pass.

The earlier movement/tint/camera-rig cutscene also passes both exports: 4096 green
pixels at [192,88,256,152], blue 64. Receipts:
`target/a07-transform-cutscene-{native,browser}`. All four replays ran serially
after builds on Linux Xvfb/software graphics and Chromium/SwiftShader. Final
pre-commit just check and whitespace checks pass; log
`target/a07-transform-precommit.log`. Generated receipts are not committed.
