# Kiln Session Handoff

Owner scope: A02–A04 on `fix/a01-ci-baseline`.
Current focus: A03 source-module moves (2026-11-18), following 6ddcc44.
A02–A04 remain PARTIAL; A01 accepted baseline stays 6de428b; full 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Assets now selects source files for Move / Rename. Script moves rewrite caller
  imports and the moved file's relative imports, retain typed manifest references
  and stable IDs, and reload clean open source buffers at their new path.
- Unsaved source blocks source moves before mutation. A bounded scan stages/syncs
  changed sources/backups, checks originals and rolls back publication failures.
  No new dependency, project format or runtime language change.
- New real-widget CI regression and docs/evidence/A03_SOURCE_MODULE_MOVES.md.
- 6ddcc44 strengthened A02 complete precrash project/source-byte oracles and added
  headless preservation across all 12 component variants, cameras and references.
  Evidence: docs/evidence/A02_COMBINED_PROPERTY_ORACLE.md.

## Executed validation

- Full cargo test --workspace --locked: 790 PASS, zero failures including docs.
  just check PASS; Python 49 PASS; py_compile/diff PASS.
- Source move widgets PASS: exact incoming/outgoing source edits, scene entrypoint
  reference, stable IDs, open-buffer retarget/save/check syntax, dirty-move refusal
  without source writes, complete project/source restart. Screenshots inspected.
- Retained previous editor fails the same source-selection/move control; its
  Move / Rename remains disabled. Evidence scopes that failure to old UI selection.
- Existing art/import GUI regression PASS again: preview lifetime, per-target
  presets, stable ID/reference move, diagnostics and complete restart.
- Raw: target/a03-source-move{,-baseline}, sibling workspace/quality/widget-final/
  widget-baseline/python logs, and target/a03-source-move-art-regression.
- 7f7c60e hosted reference 44370197155, CI 44370197413, Linux candidate
  44370197855 all PASS. Matching checks for newer commits remain required.

## Next / known issues

User requested finishing A02–A04; continue in separately validated checkpoints.
Next A03: integrated atlas build/rebuild/reopen/export and combined renamed-art/
prefab/override/multiple-scene acceptance; full nested prefabs/variants remain open.
Source moves commit multiple files sequentially; process-crash atomicity across
publication and later manifest Save remains open. Scripts must retain .ks, portable
paths; hidden/generated trees are excluded. Oversized/deep scans, symlinks and
malformed import directives refuse the move before mutation.

A02: every-inspector-property combined widget acceptance and cross-platform GUI
coverage remain open. Headless paths prove preservation, not export resolution.
Scoped/repeated nested identities need a deliberate runtime/script contract;
ambiguous links are refused and authored data preserved. Joint form requires
Apply; inspector edits remain primary-entity scoped. Save As rescue needs source
resources and explicit Save/Revert of dirty scripts. Source can consume Ctrl+S;
explicit File / Save Project is available.

A04: actual live source breakpoints, statement stepping, imported function frames/
locals/watches, safe reload specification and the new enemy/projectile/trigger game
with equivalent graph and complete exports remain open. Cooperative task payloads
are explicit state machines, not automatic suspended function stacks/yield syntax.
The separate bytecode debugger is not the live gameplay source debugger.

## Tools / boundaries

Source /workspace/.kiln-tools/activate.sh (Rust 1.99.1, rootless libraries/software
Vulkan, CHROMIUM_PATH Debian Chromium 156). Run timed UI/gameplay checks serially;
concurrent build/export load previously exposed graph/physics timing issues.
Generated target artifacts stay out of Git. Do not kill unrelated editor/Xvfb.
just check before every commit. No physical-GPU/audio/controller claims.
