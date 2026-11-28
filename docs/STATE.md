# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current A07 checkpoint: source-backed graphical
Cutscene Composer, 2026-11-28, following 914f1ed (rotation/scale tracks).
A05/A06/A07 remain PARTIAL. Scope stops at A07; do not start A08 unasked.
At most three focused attempts per blocked check; record and move on. A03/A04
gates remain deferred. Full 3D remains behind G2D.

## Done

- ADR-0116: Animation Editor → Cutscene Composer authors position, rotation,
  positive XY scale and tint keyframes. Editable keys/times/incoming easing,
  timeline-strip/value scrub and Play Values; bounded draft undo/redo.
  Limits: 16 unique entity/property tracks, 64 total keys, duration <=60 seconds.
  Explicit zero-time starts, validated bindings, physics-free transform ownership.
- Generate source into guarded Script Editor buffer, then Save Script. Versioned
  JSON comment restores editable keys/path from canonical source. Altered/unrelated
  files reject regeneration; external edits reject atomic save. No silent gameplay
  attachment or replacement. Attach once to a director entity through Inspector;
  samples/authored_cutscene already uses it as its scene script.
- One saved script clock analytically samples all tracks with existing zero-duration
  tween writes. Large ticks cross keys correctly. Rising-edge action pause/seek/
  restart/cancel; completion stops writes so later manual edits persist. Generated
  timeline clock lives in script state, separate from active tween snapshots.
- Exact decimal-string metadata avoids JSON float-roundtrip drift. Emitted literals
  use supported decimal notation, including extreme/subnormal f32 values. No new
  dependency, runtime ABI or VM layout change; existing v15 runtime retained.
- Composer drafts persist when generated/saved, not independently. Scene/project
  changes reset draft; Source Editor retains existing dirty/recovery protections.
  Value preview does not change scene poses. Guide: CUTSCENE_COMPOSER.md.
- Existing saved movement/tint/rotation/scale tweens, character state graphs,
  ping-pong/speed, camera effects, sparse maps and prepared views/pointer retained.

## Executed validation

- Full locked Rust workspace: 934 passing, zero failed, 101 suites. Quality check
  and matching native tool rebuild pass. Actual egui pointer-event widget test
  adds tracks/keys, undo/redoes, generates reloadable source and plays values.
- Six integration tests: preview/runtime agreement, multi-key/large-tick sampling,
  completion ownership, controls, paused JSON save/resume, disk reopen, manual and
  external edit protection, invalid metadata/bindings and maximum 16-track/64-key
  playback at default budget. Extreme/subnormal literals persist exactly.
  Generated sample source has a reproducibility check.
- New authored cutscene and prior transform-tween regression each pass native
  and browser exports: 8192 green pixels at [192,56,256,184], blue beacon 64.
  All four run serially after builds, with relocated exports/hidden authoring
  sources and native player PATH excluding developer tools. Receipts/screenshots
  target/a07-composer-{native,browser},
  target/a07-composer-transform-{native,browser}. Python helpers: 49 passing.
  Final pre-commit just check and whitespace checks pass.
- Logs target/a07-composer-{focused,widgets,quality,workspace,tools,python}.log.
  Final native/browser replay receipts and pre-commit checks are recorded in
  evidence/A07_CUTSCENE_COMPOSER.md. Generated artifacts stay under target.

## Open / deferred

- A07 scene-view animation preview, broader/general/audio/event tracks,
  blending/blend trees, skeletal bones/weights/skinning/IK and complete editor/
  reimport acceptance. Composer uses a source-backed property timeline, not full
  sequencer or skeletal rigging. No full OS project-authoring replay claimed.
  Direct camera-effect controller history remains transient; follow-rig pose and
  generated script clocks save normally. Source edits retain existing logical-save
  memory-signature compatibility checks. No physical save-hotkey replay claimed.
- A05 full split-screen authoring/host and remaining rendering/capacity acceptance;
  A06 stamps/streaming and complete level-authoring acceptance remain open.
  Animation 4096-cycle cap/extreme overload and physical performance acceptance
  remain open; no budgets bypassed.
- A04 source widget gate stopped after three attempts; saved-physics pose divergence
  remains. A03 identity/override/crash-transaction and A02 combined inspector gates
  deferred. No physical device/DPI, production performance, release/certification
  or full 3D acceptance claimed.

## Next

Continue useful A07 gameplay/authoring within authorized scope. Preserve bounded
attempts, do not retry deferred A03/A04 gates or advance past A07 unasked.
