# Kiln Session Handoff

Active owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 World/Local move-axis checkpoint, 2026-11-15. A02–A04 remain
PARTIAL. A01 baseline acceptance stays pinned to green `6de428b` (general CI
43606018834, reference games 43606018842, Linux candidate 43606018785). Newer
source requires matching results; no broader release gate is accepted. Full 3D
remains behind G2D. MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Enabled the formerly disabled World/Local toolbar selector in Edit mode. Local
  move handles follow the primary entity's world rotation, including parents.
  Drawing and hit testing share projected 42-point handle endpoints.
- Red X/green Y handle presses constrain movement; center/off-handle presses remain
  free planar drags. World snap is absolute, preserving the perpendicular coordinate;
  Local snap rounds displacement along the captured basis from the press position.
- Gesture space/orientation/axis are captured once. Existing group target ordering,
  world-to-local storage conversion, command validation and scene history remain
  in use. Switching space finishes pending edits like switching tools. Space is
  session-only; shared 2D rotation/uniform-scale tools are unchanged.
- Added parented-child storage integration and real widget axis acceptance to CI,
  retaining receipts, diagnostics and screenshots. User docs describe the controls.
- Prior 20ab011 rotation/scale snapping and two-scene recovery remain implemented.
  Evidence: docs/evidence/A02_LOCAL_AXES.md and A02_TRANSFORM_SNAP.md.

## Executed validation

- just check PASSED: fmt, strict workspace/all-target Clippy, workspace/audio.
- Gizmo math: 10 PASSED. Transform/property/workspace/two-scene integration:
  28 PASSED, including rotated/scaled-parent child local-axis roundtrip.
  Python regressions: 38 PASSED.
- Real local_axes_smoke.py PASSED: visible Local X upward for 90° actor vs World X
  rightward; X/Y constraints, local (40,92) vs world (64,60) snaps, free drag,
  one-step undo/redo, cancel preserving redo, exact full-project restart. Inspected
  Local/World screenshots. Fixture projection is 883×497, not 1:1 pixels/world.
- Existing transform_snap_smoke.py PASSED again: snapped move/rotate/scale, continuous
  opt-out, history/cancel, two scenes, actual autosave/crash recovery and restart.
- Logs/receipts: target/a02-local-logs, a02-local-axes and a02-transform-snap.
  No new hosted, Windows/macOS GUI or physical-device acceptance claimed.

## Hosted observation / next

Previous source 20ab011 Windows candidate 43701746095 PASSED. General CI
43701746146 FAILED native UI/browser; public annotation reports the known graph
Save-connected-host-call timeout. Job-specific logs/artifact contents were not
retrieved. Reference 43701746096 and Linux candidate 43701746100 were still running
at observation. This predates the local-axis work. Do not promote older green
results to current-source acceptance. Repair graph widget timing and review matching
hosted checks; run timed UI workflows serially without concurrent build/export load.

Continue A02 combined hierarchy/property/multiselect-parented-actor acceptance,
reparent/duplicate/delete, two-scene recovery, guarded failed writes and full
cross-platform checks. Local-axis movement is now implemented; 2D rotation/scale
share the existing controls. Do not close A02 from isolated feature checks.

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

Source /workspace/.kiln-tools/activate.sh for Rust 1.99.1, Cargo, rootless system
libraries/software Vulkan and CHROMIUM_PATH (Debian Chromium 156). Generated
artifacts stay out of Git. Do not kill unrelated editor/Xvfb processes.
just check before every commit. No physical-GPU/audio/controller claims.
