# Kiln Session Handoff

Owner scope remains A02–A04 on `fix/a01-ci-baseline`.
Current focus: A02 copied-joint references/recovery (2026-11-17), following 50b74d1.
A02–A04 remain PARTIAL. A01 accepted baseline stays 6de428b; full 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Reproduced clipboard rejection of valid nested joint targets, acceptance of
  ambiguous targets, Duplicate accepting a missing target, and prefab joint owner
  references retaining the template ID. Four Rust regressions failed before fixes.
- Entity commands now check newly published/changed joints on the fully staged
  composition before assigning the scene or recording history. Clipboard remaps
  internal references first; external IDs must resolve uniquely in the target scope.
- Shared owner remapping covers Duplicate, prefab placement and Revert. Unchanged
  legacy joints remain editable via unrelated operations. No new format/dependency,
  ID namespace or history architecture.
- New real-widget workflow/CI/evidence: docs/evidence/A02_COPIED_JOINT_REFERENCES.md.

## Executed validation

- just check PASSED. Full cargo test --workspace --locked: 781 PASSED, zero failed
  including docs. Python: 44 PASSED. py_compile/diff checks pass.
- Four before/after regressions pass, with prefab complete disk roundtrip and real
  runtime joint projection. Before claims are Rust logs, not the preliminary GUI
  run whose integration-test build had already relinked the editor.
- Final widgets PASSED: reject unresolved paste without mutation; nest target
  scene and paste joint/script data; new owner IDs, unchanged external ID/source;
  exact one-command Undo/Redo; second unsaved paste through actual autosave timer;
  forced crash/Restore/Save compares full expected project, then exact restart.
- Existing clipboard and entity-reference widget regressions PASSED again.
  Screenshots inspected. Linux Xvfb/software graphics only, not physics/device proof.
- Raw evidence: target/a02-copy-joint, target/a02-copy-reference-logs,
  target/a02-copy-reference-guards and target/a02-entity-clipboard.

## Next / known issues

Prior 50b74d1 CI 44078732105 and reference games 44078732134 PASSED, confirming
centered-error observation repair remotely. Linux candidate 44078732106 was still
IN_PROGRESS at observation; Windows did not trigger on that test/docs-only change.
Prior production 7796386 Windows 43725520747/Linux 43725520736 PASSED. Obtain
matching new-source checks; do not promote those results to changed production code.

Continue A02 combined every-property/reference acceptance and cross-platform GUI
coverage. Scoped/repeated nested identities need a deliberate runtime/script
contract; ambiguous links are refused and existing authored data preserved. Joint
form input requires Apply before becoming a saved/recoverable project edit.
Inspector edits remain primary-entity scoped. Save As rescue needs source resources
and explicit Save/Revert of dirty scripts. A03/A04 remain requested and PARTIAL.

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
