# A03 source-module move/refactor checkpoint

2026-11-18, branch `fix/a01-ci-baseline`, following `6ddcc44`. A03 remains PARTIAL.

## Change

The Assets shelf previously selected only images for Move / Rename. It now selects
source and other file cards while retaining decoded previews for images. Selection
survives file polling. Script double-click still opens the source editor.

Moving a `.ks` module now stages import rewrites and original backups alongside
the filesystem move. Incoming top-level imports follow the moved module. Imports
inside the moved file rebase relative to its new directory. The scanner follows
the runtime's line-based, top-level import grammar and preserves unrelated source
bytes, comments, strings, indentation, Unicode and line endings. Manifest resource
references and stable IDs use the existing typed visitor. The open source buffer
reloads at its new path/baseline. Dirty source refuses a source move before files
change; users Save or Revert first, then Save Project after moving.

All changed sources are staged/synced and their original bytes rechecked before
the move. Publication errors roll back published edits and the moved path; rollback
errors identify retained backups. The operation retains `.ks`, refuses unsafe paths,
symlinks, malformed top-level imports and oversized/deep scans instead of silently
missing importers. Bounds: 512 scripts, 4096 entries, depth 16, 1 MiB/file, 8 MiB
source total. Hidden/generated folders are excluded. Files commit sequentially;
process-crash atomicity across the move and later manifest Save is still open.
This checkpoint adds no format, dependency or runtime-language change.

## Executed evidence

- Full locked Rust workspace: 790 tests PASS, zero failures, including docs.
  `just check` PASS (format, strict Clippy, workspace/audio checks).
- Eight new unit regressions cover source selection/polling, caller and moved-file
  import rebasing, actual runtime module compilation before/after, untouched bytes,
  comments/strings/nesting, no final newline, collisions/invalid paths/extensions,
  symlink rejection and injected failures both before and after publishing a moved
  module's own rewritten source. Complete source/path rollback is compared.
- Native widget baseline using the retained previous editor fails the same source
  move scenario: script selection leaves Move / Rename disabled. Failure screenshot
  inspected. This control establishes the old UI gap; it does not independently
  exercise old incoming-import rewriting after a programmatic raw rename.
- Current final widgets PASS: open helper, select it in Assets, import stable IDs,
  move it and verify exact caller/own-import bytes; move the scene entrypoint and
  save its manifest reference; preserve both source IDs; reopen the retargeted
  buffer and save/check syntax without recreating the old path; refuse another
  move while source is dirty without changing any source bytes; Revert, restart,
  and compare the complete saved project and all source files. Screenshots inspected.
- CI runs the new widget test and uploads its result/screenshots/logs.

Raw local evidence: `target/a03-source-move-workspace.log`,
`target/a03-source-move-quality.log`, `target/a03-source-move-widget-final.log`,
`target/a03-source-move` and `target/a03-source-move-baseline`.
Generated binaries and captures stay outside Git. Python compilation/diff checks
pass. Linux Xvfb/software graphics only; matching hosted checks are still required.

## Remaining A03 acceptance

Nested prefab/variant production workflows, integrated atlas build/rebuild/reopen/
export and combined renamed-art/prefab/override/multiple-scene visual acceptance
remain open. This source-refactor checkpoint does not complete those gates.
