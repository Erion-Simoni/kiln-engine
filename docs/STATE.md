# Kiln Session Handoff

Owner requested fixes for remaining A02 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 scene settings/camera/composition correctness, following d110141.
A02–A04 remain PARTIAL. A01 accepted baseline remains 6de428b; full 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Found Scene Editor metadata/composition/cameras bypassing history, stale nested
  preview and missing camera dirty/discard protection. Fixed through existing
  scene transaction/history with optional camera snapshots (ADR-0088).
- Scene-name/script/camera/nested settings now undo/redo; cancellation restores
  camera presence/value and retains redo. Ordinary entity commands preserve cameras.
- Camera-only tabs dirty correctly; close requires a decision; Discard restores
  disk baselines including absent/recovered/new cameras and other-scene isolation.
- Nested-scene picker is explicit; offset/removal rebuilds live projection, child
  documents remain independent. Controls wrap and respect Play/project guards.
- Evidence/docs/CI: docs/evidence/A02_SCENE_SETTINGS.md and scene_settings_smoke.py.

## Executed validation

- Before fix, prior packaged editor FAILED new real-widget scene-name Undo.
- Final widgets PASSED name/script and camera exact-project Undo/Redo, live camera
  Escape/cancel preserving redo, explicit two-scene link/offset/remove/Undo with
  observed rendered bounds, unchanged child, actual autosave/forced-crash camera
  recovery and exact full-project restart. Screenshots inspected.
- just check PASSED. Full cargo test --workspace --locked: 770 PASSED, zero failed
  including doc tests. nextest command unavailable; not counted as success.
- Python: 40 PASSED; py_compile and diff checks pass. Raw logs/receipts:
  target/a02-scene-settings-logs, target/a02-scene-settings and before/layout dirs.

## Next / known issues

Obtain matching hosted checks for new production changes. Prior d110141 PASSED
CI 43717563394, reference games 43717563395 and Linux candidate 43717563439.
Windows did not trigger on that test/docs-only change; prior production 276524c
Windows 43711940872 passed. New source cannot inherit acceptance from these.

Continue A02 combined every-supported-property/reference recovery coverage and
cross-platform GUI acceptance. General nested-scene identity semantics remain open;
Parent hierarchy authoring alone does not complete scene composition. Inspector
property edits retain primary-entity semantics. Save As rescue needs existing
source resources and explicit Save/Revert of dirty source. A03/A04 remain requested
but are not closed by this A02 fix; proceed in milestone order.

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
