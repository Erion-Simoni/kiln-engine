# A02 combined property and recovery oracle

2026-11-18, following `7f7c60e`. A02 remains PARTIAL.

The hierarchy widget test previously checked selected restored fields and then
used the observed restored document as its restart reference. It now builds the
complete expected project before the unsaved edit/crash, changes only the expected
child y coordinate, and compares the complete restored/restarted project. It also
checks that autosave leaves the saved project unchanged and script bytes survive.
The real-widget duplicate/reparent/detach/delete/undo/redo/two-scene subtree paste,
actual autosave timer, forced close, Restore/Save and restart all pass. Screenshots
were inspected; raw evidence is `target/a02-combined-hierarchy` and its sibling log.

The combined Rust acceptance now includes nondefault data across all 12 Component
variants: Transform, Sprite, Role, EditorFlags, SpriteLayout, Particles, Physics,
Script, Joint, PrefabInstance, Parent and Tilemap, plus scene camera settings.
Both scene projections compare complete documents. Duplicate has an independently
constructed expected entity/project, including joint owner and prefab owner
remapping, while preserving the external child endpoint. Delete/Undo/Redo,
sidecar recovery, saved-file preservation and complete disk restart/projection
compare those independent documents. The test explicitly loads tilemaps through
the separate runtime projection used by the editor; omitting that setup initially
caused a fixture failure. Locked actors are projected but editable test actors
retain hidden flags without the lock guard. No production defect is claimed.

Executed: both two-scene Rust tests PASS; `just check`, Python compilation and
diff checks PASS. Logs: `target/a02-combined-test.log`,
`target/a02-combined-check.log`. The GUI test is Linux Xvfb/software graphics;
the all-component construction/history test is headless, not proof that every
inspector field was authored through widgets. Materials, image and script paths
in that headless fixture are preservation data, not export/resource-resolution
certification. The broad combined every-inspector-property and cross-platform
GUI acceptance remains open.

Matching hosted checks for the prior `7f7c60e` all PASS:
[reference games 44370197155](https://github.com/Erion-Simoni/kiln-engine/actions/runs/44370197155),
[CI 44370197413](https://github.com/Erion-Simoni/kiln-engine/actions/runs/44370197413),
and [Linux candidate 44370197855](https://github.com/Erion-Simoni/kiln-engine/actions/runs/44370197855).
These confirm the preceding replay repair remotely, not this new test revision.
