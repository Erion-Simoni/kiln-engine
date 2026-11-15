# Kiln Session Handoff

Owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 stable floating-panel replay (2026-11-15), following 276524c.
A02–A04 remain PARTIAL. A01 baseline acceptance is pinned to green 6de428b
(CI 43606018834, reference 43606018842, Linux 43606018785). New source needs
matching results. Full 3D remains behind G2D; no broader release gate accepted.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Retrieved source-pinned hosted results: 276524c CI 43711940922, Linux candidate
  43711940942 and Windows 43711940872 PASSED. Reference 43711940851 FAILED
  Skybridge authoring with KeyError: 'run', matching c9958d7 reference failure.
- Reproduced the exact failure locally. The panel observer dragged from a
  transient title segment and left Animation at the wrong widget origin.
- The shared observer now waits for three consecutive matching rendered title
  origins and verifies the destination after dragging. Existing bounded waits,
  failure reporting and complete game/source/template comparisons remain intact.
  No application hook, project-data injection or assertion weakening.
- Added transient-frame/destination-observation regressions. No production code,
  dependency, project format or command architecture changes.
- Evidence: docs/evidence/A02_PANEL_REPLAY.md. Earlier parented group transforms,
  manifest rescue, Save As protection, graph dialog repair, Parent tree, World/Local
  axes, snapping, clipboard, flags/alignment and two-scene recovery remain.

## Executed validation

- just check PASSED: fmt, strict workspace/all-target Clippy, workspace/audio.
- Python regressions: 40 PASSED. Both new tests reject the old panel observer.
- Skybridge authoring FAILED before, PASSED after: real widgets reproduce exact
  complete project and source bytes, original template unchanged. Animation
  screenshot inspected: run clip, two frames, second X=16 and correct sheet.
- Raw evidence/logs: target/a02-animation-authoring-before,
  target/a02-animation-authoring-after, target/a02-animation-layout,
  target/a02-animation-authoring-logs.
- Initial unpackaged debug export correctly failed required license-tree checks;
  portable package was then prepared/verified with rebuilt native tools and the
  existing prebuilt browser runtime. Packaged all-three-game replay PASSED:
  exact authored project/source/template comparisons, all six native/web exports,
  full notice/runtime binding and overwrite protection. Report:
  target/a02-panel-packaged-replay/result.json. Gameplay was not rerun.

## Next / known issues

Obtain matching hosted results for the panel-observer repair. On prior c9958d7,
CI 43709956368 and Windows 43709956361 PASSED; reference 43709956415 FAILED
same missing clip and Linux 43709956395 was CANCELLED. Earlier artifact-preflight
failure on ac0a86c has no established service root cause; no check was weakened.

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
