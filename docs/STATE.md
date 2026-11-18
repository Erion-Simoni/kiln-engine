# Kiln Session Handoff

Owner scope remains A02–A04 on `fix/a01-ci-baseline`.
Current focus: A02 combined property/recovery oracle (2026-11-18), following 7f7c60e.
A02–A04 remain PARTIAL; A01 accepted baseline stays 6de428b; full 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Hierarchy recovery now compares the complete project to an independent expected
  document prepared before the crash; autosave preserves the saved project and
  script bytes survive. Duplicate/reparent/detach/delete/history/two-scene copy,
  actual autosave timer, forced close, Restore/Save and exact restart pass.
- New headless combined acceptance preserves all 12 component variants, camera
  settings and references across both scene projections, independent duplicate
  oracle, delete/Undo/Redo, recovery and disk restart. No production code changes.
- Evidence and limits: docs/evidence/A02_COMBINED_PROPERTY_ORACLE.md.

## Executed validation

- Both two-scene Rust tests PASS; just check PASS; Python compilation/diff PASS.
- Strengthened hierarchy widgets PASS, screenshots inspected. Linux Xvfb/software
  graphics; no every-field inspector, gameplay or physical-device certification.
- Logs: target/a02-combined-test.log, target/a02-combined-check.log,
  target/a02-combined-hierarchy.log; images/result target/a02-combined-hierarchy.
- 7f7c60e hosted reference games 44370197155, CI 44370197413 and Linux candidate
  44370197855 all PASS, confirming the earlier panel-header replay repair remotely.
  Its 49 Python checks and all three authoring/six exports previously passed locally.

## Next / known issues

Obtain matching checks for this revision. A02 every-inspector-property combined
widget acceptance and cross-platform GUI coverage remain open. Headless fixture
paths are preservation data, not resource-resolution/export certification.
Scoped/repeated nested identities need a deliberate runtime/script contract;
ambiguous links are refused and authored data preserved. Joint form requires
Apply before save/recovery; inspector edits remain primary-entity scoped. Save As
rescue needs source resources and explicit Save/Revert of dirty scripts. Source
Editor can consume Ctrl+S; explicit File / Save Project remains available.

Next requested work: A03 source-module move/import refactoring. User requested
finishing A02–A04; proceed through focused, separately validated checkpoints.

A03: 3daaa1c adds typed dependencies/reference-safe moves, per-target import rules,
worker diagnostics and fixed GPU preview lifetime. Full nested prefab/variants,
integrated atlas build/rebuild/reopen/export, source-module import refactoring and
combined renamed-art/prefab/override/multiple-scene production acceptance remain open.
A04: actual live source breakpoints, statement stepping, imported function frames/
locals/watches, safe reload specification and the new enemy/projectile/trigger game
with equivalent graph and complete exports remain open. Cooperative task payloads
are explicit state machines, not automatic suspended function stacks/yield syntax.
The legacy separate bytecode debugger is not the live gameplay source debugger.

## Tools / boundaries

Source /workspace/.kiln-tools/activate.sh (Rust 1.99.1, rootless libraries/software
Vulkan, CHROMIUM_PATH Debian Chromium 156). Run timed UI/gameplay checks serially;
concurrent build/export load previously exposed graph/physics timing issues.
Generated target artifacts stay out of Git. Do not kill unrelated editor/Xvfb.
just check before every commit. No physical-GPU/audio/controller claims.
