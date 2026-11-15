# Kiln Session Handoff

Owner scope remains A02–A04 on `fix/a01-ci-baseline`.
Current focus: A02 reference-safe entity deletion/joint editing, following bf206f9.
A02–A04 remain PARTIAL; A01 baseline accepted on 6de428b only. 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Reproduced deletion leaving joint endpoints/camera targets dangling and Add/Set
  Joint accepting missing/ambiguous endpoints. Three integration regressions failed
  before and pass after shared composition-aware reference validation.
- Deletion refuses dependent camera/joints in containing scenes, keeps document/
  history/redo unchanged, and permits ordinary deletion once references are removed.
  Independent scenes may reuse IDs without being incorrectly treated as dependents.
- Joint inspector retains a form draft, validates on explicit Apply, commits fields
  together once, retains invalid input/errors and supports Cancel preserving redo.
  Endpoint fields/numeric controls wrap in narrow panes. Unapplied form input is
  not a project edit; Apply before save/recovery.
- Generic errors are centered/foreground; deterministic raised-panel hit regression
  fails before the layer fix and passes after. No new format/dependency/history stack.
- CI/docs/evidence: docs/evidence/A02_ENTITY_REFERENCES.md and entity_reference_smoke.py.
  Previous camera/settings/nested-identity protections remain implemented.

## Executed validation

- just check PASSED. Full cargo test --workspace --locked: 777 PASSED, zero failed
  including docs. Python regressions: 40 PASSED. py_compile/diff checks pass.
- Final widget workflow PASSED: joint invalid/corrected Apply, one Undo, Cancel
  preserves redo, specific joint/camera deletion diagnostics and visible errors,
  dependency resolution, delete/Undo/Redo and exact full-project restart.
- Existing scene-settings regression PASSED real camera-only autosave, forced
  crash/recovery, composition preview/history and exact restart. Screenshots inspected.
- Existing local_axes_smoke.py PASSED isolated/serial on new source. Hosted failure
  remains unresolved; a local pass does not establish its cause.
- Raw evidence: target/a02-entity-references, target/a02-entity-references-logs,
  target/a02-entity-references-layout. Failed setup attempts remain retained.

## Next / known issues

Prior bf206f9 Windows 43723198796 and reference 43723198790 PASSED.
CI 43723198838 FAILED local_axes_smoke.py: Saved constrained position (40,60)
timed out. Linux candidate 43723198819 was IN_PROGRESS at observation. Do not
claim that hosted failure resolved from unrelated green runs. Obtain matching
new-source checks and investigate retained local-axis evidence; no checks weakened.
Prior 9bc2ace CI 43722214097/Windows 43722214082 PASSED; reference/Linux CANCELLED.

Continue A02 combined every-property/reference acceptance and cross-platform GUI
coverage. Scoped/repeated nested identities need a deliberate runtime/script contract;
new ambiguous links are refused, existing authored data preserved. Inspector edits
remain primary-entity scoped. Save As rescue needs existing source resources and
explicit Save/Revert of dirty scripts. A03/A04 remain requested and PARTIAL.

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
