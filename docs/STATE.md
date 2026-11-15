# Kiln Session Handoff

Active owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 transform snapping checkpoint, 2026-11-15. A02–A04 remain
PARTIAL. A01 configured baseline is accepted on source `6de428b`; matching general
CI 43606018834, reference games 43606018842 and Linux candidate 43606018785 passed.
Evidence: docs/evidence/A01_RESUMED_VALIDATION.md. No broader release gate is
accepted. Full 3D remains behind G2D; MIT OR Apache-2.0 settled; no size ceiling.

## Done

- Snap previously affected only translation. It now snaps world rotation to 15°
  and uniform scale to 0.1 steps, preserving proportions and a positive snapped
  magnitude. Invalid/disabled grids retain continuous rotation/scale. Existing
  press-state math, group application, history and save/recovery commands are used.
- Added toolbar hover guidance and user documentation for the three Snap modes.
  Fixed increments are a routine editor behavior choice; no new dependencies,
  architecture or project-format change was required.
- Added real widget acceptance to native CI with retained receipts/screenshots.
  New test moves/rotates/scales through gizmos, verifies one-step undo/redo,
  Escape cancellation without extra history and continuous scale when Snap is off.
- The same test copies into a newly created second scene, makes an unsaved Inspector
  scale edit, waits for actual autosave, force-kills/reopens/Restores/Saves, then
  restarts and compares the complete saved project. The first scene stays intact.
- Evidence: docs/evidence/A02_TRANSFORM_SNAP.md. Raw receipt/captures under
  target/a02-transform-snap; logs under target/a02-transform-logs.

## Executed validation

- just check PASSED (fmt, strict workspace/all-target Clippy, workspace and audio).
- Gizmo math: 8 PASSED. Targeted transform/property/scene-workspace/two-scene
  integration: 27 PASSED. Python regressions: 38 PASSED.
- Real Linux Xvfb widget test PASSED: 45° rotation, scale 1.1, snapped (64,96),
  continuous scale opt-out, history/cancel, second-scene recovered scale 1.7 and
  exact restart. Recovery screenshot inspected. No new hosted or cross-platform
  GUI/device acceptance is claimed yet; run matching hosted checks after push.

## Next

Continue A02: local-axis viewport tools are still absent (World toolbar indicator
is disabled; local values are edited in Inspector). Complete broader combined
transform/property/reparent/duplicate/delete acceptance with two scenes, recovery,
failed-save preservation and cross-platform checks. Earlier lock/hide, marquee,
alignment, clipboard, recovery and guarded-file-write work remains implemented.
Do not close A02 from this isolated snapping checkpoint.

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

Source /workspace/.kiln-tools/activate.sh for Rust 1.99.1, Cargo, rootless libraries,
software Vulkan and CHROMIUM_PATH (Debian Chromium 156). Run timed GUI/gameplay
checks serially: parallel local build/export work exposed timing failures in old
graph/physics captures, while unchanged serial checks and hosted CI passed.
Generated artifacts stay out of Git. Do not kill unrelated editor/Xvfb processes.
just check before every commit. No physical-GPU/audio/controller claims.
