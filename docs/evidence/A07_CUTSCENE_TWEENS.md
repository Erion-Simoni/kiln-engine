# A07 cutscene tween checkpoint — 2026-11-27

ADR-0114 implements bounded shared local-position and tint tracks, four easing
curves, replacement/cut/cancel, pause/seek/query and validated logical save/resume.
The source-authored sample includes a camera-follow rig; no new dependency.
A07 remains PARTIAL: timeline GUI, general/audio/event tracks, blending and skeletal
rigging/IK and complete editor/reimport acceptance remain open.

Executed checks:

- Full locked Rust workspace: 924 passing tests (`target/a07-tween-workspace.log`).
  Eight integration tests plus interpolation coverage exercise timing, endpoint
  precision, pause/seek/manual override, replacement, shared capacity, failed and
  debugger-suspended callbacks, UI pause, physics ownership, save corruption and
  legacy saves. Python helper suite: 49 passing.
- `just check`, strict default and opt-out WASM checks pass. Matching v14 tools and
  browser SDK rebuilt; actual prior v13 browser SDK export rejects. Logs:
  `target/a07-tween-{quality,wasm,wasm-optout,tools,web,stale,python}.log`.
- Relocated native and nested browser cutscene exports pass with authoring copies
  hidden. Native player PATH excludes developer tools. Automatic sequence proves
  held clocks/poses, seek while paused, resume and exact actor/tint/rig endpoints;
  green 4096 pixels at [192,88,256,152] and blue completion beacon 64.
  Receipts/screenshots: `target/a07-tween-rig-{native,browser}`. Earlier static-camera
  variant also passed. Native Xvfb/software graphics and Chromium/SwiftShader only.

GameSave JSON restoration is exercised by runtime tests. No physical F5/F9 replay,
physical device/DPI, performance budget or full A07 completion is claimed.
Generated logs, exports and screenshots stay untracked under target.

Final regressions pass in both exports: character-state real keyboard phases
(run movement, release idle, one-shot action/return, boundary) and five clipped
pointer phases (inside, letterbox invalid, inside, outside invalid, inside).
Receipts: `target/a07-tween-{states,pointer}-{native,browser}`. The six final
cutscene/character/pointer replays ran serially after all builds finished.
