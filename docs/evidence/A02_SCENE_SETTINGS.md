# A02 scene settings, cameras and composition — 2026-11-15

Branch `fix/a01-ci-baseline`, following `d110141`. A02 remains PARTIAL.
Implementation decision: ADR-0088.

## Defects and implementation

Scene Editor directly changed scene names/scripts, nested links and camera tables
without history. Camera-only edits did not dirty their scene tabs, and Discard
could leave camera changes behind. Nested link changes did not rebuild the live
projection. Add Nested Scene arbitrarily picked the first other scene. Nested
controls bypassed Play/project-operation guards and clipped Remove in narrow panes.

Scene settings now use the existing preview/gesture transaction with optional
camera snapshots in scene history. Entity-only entries preserve the current camera;
settings undo/redo/cancel restore its exact prior presence/value. Unchanged gestures
retain redo. Camera baselines follow saved and recovered scene baselines; tab close
checks camera dirtiness, and Discard restores/removes the camera appropriately.
No manifest format change, dependency or separate undo stack.

Choose the nested child explicitly. Nested changes rebuild through the existing
scene loader; child documents remain independent. All composition controls disable
during Play or blocked project operations. Nested controls wrap in narrow panes;
errors reach the console. Escape cancels an active settings/inspector transaction
through the existing cancellation path, including camera restoration.

## Executed validation

- Before repair: real-widget scene-name Undo FAILED on the prior packaged editor
  (`One Undo restores complete scene settings timed out`).
- After repair: `scene_settings_smoke.py` PASSED actual scene-name/script-path
  edits and exact whole-project Undo/Redo; camera creation Undo removes its table
  entry, Redo restores it; Escape cancels a live camera drag and retains prior redo.
- Two scenes authored through widgets; explicit nested link add/Undo/Redo; X/Y
  offsets preserve the entire child document; Remove immediately removes the
  child's pixels, Undo restores exact document and rendered bounds. Observed
  offset child bounds agree with its intended scene position. Screenshots inspected.
- Real autosave timer stores unsaved camera-only zoom without writing the manifest.
  Forced process kill/restart, Restore/Save and a further ordinary restart compare
  the complete project, including both scenes, nested offset, script and camera.
- New Rust regressions cover camera-only dirty/close protection; Discard of saved,
  recovered and newly created cameras; other-scene isolation; camera-aware history
  interleaved with entity edits; cancel/no-op retaining redo.
- `just check` PASSED. Full `cargo test --workspace --locked`: 770 PASSED,
  zero failures, including doc tests. `cargo-nextest` was unavailable in this
  environment; its attempted invocation is retained and not counted as a pass.
- Python suite: 40 PASSED. Script compilation and `git diff --check` PASSED.
- CI runs the new real-widget regression and retains logs/screenshots/receipts.

Raw evidence: `target/a02-scene-settings/`, `target/a02-scene-settings-before/`,
`target/a02-scene-settings-layout/`, `target/a02-scene-settings-logs/`.
These are Linux Xvfb/software graphics results, not cross-platform/device acceptance.
Full combined every-property authoring/recovery and general nested-scene identity
semantics remain open. A03/A04 acceptance is unchanged by this A02 repair.

## Source-pinned hosted results

Prior source `d110141` PASSED general CI 43717563394, reference games 43717563395
and Linux candidate 43717563439. Windows was not triggered for that test/docs-only
change; prior production `276524c` Windows 43711940872 passed. Those results do not
accept these new production changes; matching hosted results remain required.
