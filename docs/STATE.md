# Kiln Session Handoff

Owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 parented selection/group transforms (2026-11-15), following c9958d7.
A02–A04 remain PARTIAL. A01 baseline acceptance is pinned to green 6de428b
(CI 43606018834, reference 43606018842, Linux 43606018785). New source needs
matching results. Full 3D remains behind G2D; no broader release gate accepted.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Reproduced and fixed Ctrl-deselect leaving a stale inspector/gizmo target.
  Empty hierarchy selection now synchronizes to no selected entity.
- Reproduced group scaling changing child-local floats. Shared group transforms
  now update selected roots; selected descendants inherit without rewriting their
  local documents. Ancestor traversal includes intermediate unselected parents
  and is bounded. Existing general target ordering is unchanged.
- Added unit/integration and real-widget coverage to CI with retained evidence.
  No dependencies, project format or command architecture changes.
- Evidence: docs/evidence/A02_PARENTED_SELECTION.md. Earlier manifest rescue,
  Save As protection, graph dialog foreground repair, Parent tree, World/Local
  axes, snapping, clipboard, flags/alignment and two-scene recovery remain.

## Executed validation

- just check PASSED: fmt, strict workspace/all-target Clippy, workspace/audio.
- Targeted gizmo math: 11 PASSED. Transform/entity-command/scene-workspace/
  two-scene integration: 35 PASSED. Python: 38 PASSED.
- parented_multiselect_smoke.py PASSED: empty-selection safety, descendant-first
  selection, move/rotate/scale, exact child properties, single-gesture undo/redo,
  selection retained across history, cancel preserves redo, exact project restart.
- Existing local_axes_smoke.py PASSED again. Screenshots inspected.
- Raw receipts/logs: target/a02-parented-selection, target/a02-parented-selection-logs,
  target/a02-local-axes. Linux Xvfb/software graphics only.

## Next / known issues

At observation on prior c9958d7, reference 43709956415, Windows 43709956361,
CI 43709956368 and Linux 43709956395 were IN_PROGRESS. Obtain matching new-source
checks; earlier green source results cannot accept newer code. Prior ac0a86c Linux
candidate 43708624752 failed artifact-probe download before application build;
root cause unestablished and preflight unchanged.

Continue A02 full component/property coverage and cross-platform acceptance.
Inspector property edits retain primary-entity semantics. Entity-parent trees do
not close nested-scene authoring. Source resources must still exist for Save As
rescue; whole-resource-tree reconstruction is not implemented. Dirty source still
requires explicit Save/Revert before copying. Do not close A02 from isolated flows.

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
