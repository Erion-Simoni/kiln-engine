# A07 source-backed Cutscene Composer — 2026-11-28

ADR-0116 adds real Animation Editor controls for per-entity position, rotation,
positive XY scale and tint keys. Timeline strips/value preview support scrubbing,
key times/values and incoming easing are editable, and draft undo/redo is bounded.
Generated KilnScript embeds a versioned authoring document. Load from Source Editor
restores keys and path; altered or unrelated source rejects regeneration. Existing
Script Editor stages/saves source with dirty-buffer and external-change guards.
Users attach the saved file to a director entity with existing Inspector controls.
The sample already uses it as the scene script.

Generated playback uses one saved script clock and zero-duration tween writes.
Sampling is analytical; large ticks cross multiple keys without accumulated tween
state. Completion stops writes; pause/seek/restart/cancel are rising-edge actions.
Limits: 16 unique bindings, 64 total keys, 60 seconds, explicit time-zero values,
positive scale >=0.000001 and physics-free transform bindings. Metadata decimal
strings preserve exact float values; literals avoid unsupported exponent syntax.
No dependency, runtime ABI or VM layout change; existing contract v15 retained.

Executed validation:

- Full locked Rust workspace: 934 tests pass, zero failed, 101 suites
  (`target/a07-composer-workspace.log`). Quality check and rebuilt native tools
  pass (`target/a07-composer-{quality,tools}.log`).
- Actual egui pointer events click composer open, Add Track, Add Key, Undo, Redo,
  Open Generated Source and Play Values. Generated source reloads to the same
  document. This is headless widget coverage, not a full OS project-authoring replay.
- Six integration tests cover preview/runtime agreement at keys and large ticks,
  completion relinquishing writes, controls and paused JSON save/resume, source
  disk reopen, manual/external edit preservation, invalid bindings/times/values,
  duplicate controls/tracks, physics ownership, and maximum 16-track/64-key
  execution at the unchanged default script budget. Extreme f32 destinations and
  subnormal literals round-trip exactly. Sample source regenerates reproducibly.
  Focused log: `target/a07-composer-focused.log`.

The new samples/authored_cutscene contains intermediate movement and scale keys,
a 450-degree spin, tint and camera-follow dolly. A delayed blue beacon gates the
final pose. Export receipts and final checks are recorded below.

A07 stays PARTIAL. Scene-view preview, general/audio/event tracks, blending,
skeletal bones/weights/skinning/IK, full editor/reimport acceptance and physical
performance/DPI/device validation remain open. Generated source has the existing
script memory-signature compatibility rules for logical saves. No physical
save-hotkey or full OS authoring flow is claimed.

Export results: authored cutscene passes relocated native and nested browser
exports, with authoring copies hidden; player PATH excludes developer tools.
Both render 8192 green pixels at [192,56,256,184] plus blue completion beacon 64.
The previous transform-tween sample also passes both exports with the same final
bounds/counts. All four replays ran serially after builds. Receipts/screenshots:
`target/a07-composer-{native,browser}` and
`target/a07-composer-transform-{native,browser}`. Scope: Linux Xvfb/software graphics
and Chromium/SwiftShader. Python helpers: 49 passing
(`target/a07-composer-python.log`). Final pre-commit quality and whitespace checks
pass (`target/a07-composer-precommit.log`). Generated outputs are not committed.
