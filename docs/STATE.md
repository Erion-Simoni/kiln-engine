# Kiln Session Handoff

Owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 entity-parent tree/combined recovery checkpoint (2026-11-15).
A02–A04 remain PARTIAL. A01 baseline acceptance stays pinned to green `6de428b`:
CI 43606018834, reference 43606018842, Linux candidate 43606018785. New source
requires matching evidence. Full 3D remains gated on G2D; no broader release gate
is accepted. MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Hierarchy now follows entity Parent links in stable parent-first order with
  indented children. Actual folding replaces inert expand buttons; triangle
  painting avoids missing arrow glyphs. Iterative traversal keeps malformed
  cyclic/missing-parent entities inspectable without recursive traversal.
- Search reveals matching descendants with ancestor context even when folded.
  Clearing search restores branch state; same-scene rebuilds retain branch state
  and selected flags. Scene switches reset branches; creating/opening projects
  clears the hierarchy view.
- A drop on SCENE detaches an entity to root through existing guarded Reparent,
  world/local transform conversion and scene history. Entity drops remain the
  normal reparent path. No new dependency, format or command architecture.
- New CI widget workflow combines duplication, reparenting under a rotated/scaled
  parent, filtered child-property editing, detach/Undo/Redo, Delete/Undo with stable
  IDs, parent-subtree copy to scene two, real negative-property autosave/crash
  recovery and whole-project restart comparison. Numeric test input now places
  -- before xdotool values, supporting negative numbers.
- Earlier World/Local constrained move tools, rotation/scale snapping, clipboard,
  lock/hide, alignment and guarded-file/recovery work remain implemented.
- Evidence: docs/evidence/A02_HIERARCHY_TREE.md, A02_LOCAL_AXES.md and
  A02_TRANSFORM_SNAP.md. Generated logs/receipts stay under target/.

## Executed validation

- just check PASSED (fmt, strict workspace/all-target Clippy, workspace/audio).
- Hierarchy: 5 PASSED including egui selection/reparent. Entity-command/two-scene/
  workspace/project-lifecycle integration: 30 PASSED. Python: 38 PASSED.
- Final open-path reset correction: just check and 9 lifecycle tests PASSED.
- New hierarchy_tree_smoke.py PASSED end-to-end with real widgets and actual
  recovery timer. Copied parent reference and negative local Y=-15 recovered;
  first scene untouched; exact complete project after restart. Screenshots inspected.
- Existing entity_clipboard_smoke.py and scene_controls_smoke.py PASSED again:
  clipboard/identity/history/restart and selection/alignment/lock/hide/two-scene
  recovery/restart. All UI tests run serially on Xvfb/software graphics.
- Raw evidence: target/a02-hierarchy-tree, a02-entity-clipboard, a02-scene-controls;
  logs: target/a02-hierarchy-logs. No new Windows/macOS GUI/device certification.

## Hosted / next

Previous source bdab5e4 Windows candidate 43703292617 and reference games
43703292602 PASSED. General CI 43703292628 FAILED browser graph authoring with
Render-constant-parameter-dialog timeout; Linux candidate 43703292634 was still
IN_PROGRESS at observation. Public summaries only; no job-specific logs/artifact
contents retrieved. This predates entity-tree changes. Prior greens do not validate
new source. Repair graph widget timing and review matching hosted checks.

Continue A02 full supported component/property and parented multiselect transform
acceptance, guarded failed-write UI workflows and cross-platform coverage. Entity
Parent trees now work; nested-scene authoring is a separate remaining boundary.
Do not close A02 from this Linux combined editing checkpoint.

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
Vulkan, CHROMIUM_PATH Debian Chromium 156). Run timed GUI/gameplay checks serially;
parallel local build/export work exposed timing issues in old graph/physics tests.
Generated artifacts stay out of Git. Do not kill unrelated editor/Xvfb processes.
just check before every commit. No physical GPU/audio/controller claims.
