# Kiln Session Handoff

Owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 authoring reliability/CI followup (2026-11-15), following
4270bd6. A02–A04 remain PARTIAL. A01 baseline acceptance is pinned to green
6de428b: CI 43606018834, reference 43606018842 and Linux candidate 43606018785.
Newer code requires matching results. Full 3D remains behind G2D; no broader
release gate accepted. MIT OR Apache-2.0 settled; no size ceiling.

## Done

- Reproduced an actual graph parameter-window layering defect: raising the
  containing floating graph panel covered its Middle-order parameter editor.
  The new deterministic widget/layer hit-test FAILED before the fix, PASSED after
  parameter editors were placed in the foreground. Active parameter state remains
  unchanged; dialogs now stay above their containing graph panel.
- Host-function widget replay waits for the dialog's measured/recentered layout,
  visible registry menu and closed picker before committing. No blind retries,
  increased timeout, engine hooks or removed source/gameplay assertions.
- Existing native/browser CI already exercises full graph authoring and retains
  screenshots/receipts. The new Rust regression runs with workspace tests.
- Evidence: docs/evidence/A02_GRAPH_DIALOG_LAYERING.md. This is editor interaction
  repair, not new A04 gameplay/compiler/debugger functionality.
- Prior A02 work remains: Parent-link tree/fold/search/root-drop, World/Local move
  axes and constraints, move/rotate/scale snapping, clipboard/lock/hide/alignment,
  guarded file writes and actual two-scene crash recovery. See prior A02 evidence.

## Executed validation

- just check PASSED: fmt, strict workspace/all-target Clippy, workspace/audio.
- New occlusion regression: before FAIL, after PASS. Graph/text gameplay/inspection
  save/restore integration: 1 PASSED. Python regressions: 38 PASSED.
- Full native + browser graph replay PASSED twice with separate fresh projects and
  profiles: real host arguments, Cancel, Undo/Redo, one-gesture drag undo, scene
  assignment, actual execution/F5 save, saved values, exact restart and browser
  F9/F5 restoration. First native/browser scores 261/360; repeat 252/342. Browser
  rendered 392 hero pixels and saved three node values in both runs.
- Raw evidence: target/a02-dialog-logs, target/a02-graph-dialog and
  target/a02-graph-dialog-repeat. No new hosted/cross-platform/device claim yet.

## Hosted / next

At start on prior source 4270bd6: Windows candidate 43705134977 PASSED; CI
43705135020, reference 43705134990 and Linux candidate 43705134979 were still
running. This predates the dialog fix. Earlier bdab5e4 CI failed the browser graph
constant-dialog wait. The local occlusion reproduction is a sufficient cause, not
proof of every hosted timeout; obtain matching hosted confirmation for this fix.

After CI review, continue A02 full component/property and parented multiselect
acceptance, failed-write UI flows and cross-platform coverage. Current entity-tree
combined workflow and previous constrained-transform/recovery workflows passed
locally in prior checkpoints; they do not close full A02. Nested-scene authoring
is separate from the now-functional entity-parent tree.

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

Source /workspace/.kiln-tools/activate.sh for Rust 1.99.1, rootless libraries/software
Vulkan and CHROMIUM_PATH (Debian Chromium 156). Run timed UI/gameplay serially;
concurrent local build/export load previously exposed graph/physics timing issues.
Generated target artifacts stay out of Git. Do not kill unrelated editor/Xvfb.
just check before every commit. No physical-GPU/audio/controller claims.
