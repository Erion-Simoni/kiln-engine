# Kiln Session Handoff

Owner scope remains A02–A04 on `fix/a01-ci-baseline`.
Current focus: A02 centered-error replay repair (2026-11-17), following 7796386.
A02–A04 remain PARTIAL; A01 accepted baseline stays 6de428b. Full 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Retrieved source-pinned hosted results: production 7796386 Linux candidate
  43725520736, Windows 43725520747 and reference games 43725520724 PASSED.
  General CI 43725520708 FAILED manifest rescue's visible-error wait.
- Reproduced that exact deletion-case timeout. Production error window is centered,
  but rescue's observer still searched top-left. The captured failure frame visibly
  contains the intended refusal and unsaved X=88. Old observer false/new true on
  the exact same frame. No unsafe Save or source-data mutation was observed.
- Shared centered project-error observer now used by manifest-rescue and joint/
  camera reference replays. Retains the latter's stricter red-pixel threshold.
- Four new observer regressions reject stale-region, inspector/gizmo, blank,
  malformed and sparse frames; accept centered error colors with sRGB variation.
- No production code, project format, assertion weakening or timeout/retry changes.
  Evidence: docs/evidence/A02_CENTERED_ERROR_OBSERVER.md.

## Executed validation

- just check PASSED. Python regressions: 44 PASSED. py_compile/diff checks pass.
- Manifest rescue PASSED all three cases: visible Save refusal, Escape retains
  draft, X=88/resources rescued, external originals untouched, subsequent Y=72
  writes only copy, exact complete-project restart. Before/failure screenshot
  inspected; raw exact-frame comparison recorded.
- Entity-reference replay PASSED again: joint draft correction/Undo/Cancel, visible
  joint/camera delete guards, reference resolution, deletion history and exact
  restart. Final screenshots inspected. Receipts/logs retained alongside rescue.
- Raw evidence: target/a02-error-observer-before, target/a02-error-observer-after,
  target/a02-error-observer-references, target/a02-error-observer-logs.
  Linux Xvfb/software graphics only. CI keeps its existing default artifact paths.

## Next / known issues

Obtain matching hosted checks for this observer repair. Prior 7796386 CI reached
manifest rescue, advancing through the earlier local-axis failure on bf206f9.
The older timeout's root cause is not established by a later successful step.
Public run annotations were available; authenticated logs/artifact contents were
not independently retrieved. Do not promote prior green source to newer code.

Continue A02 combined every-property/reference acceptance and cross-platform GUI
coverage. Scoped/repeated nested identities still need a runtime/script contract;
new ambiguous links are refused and legacy authored data preserved. Joint form
input requires Apply before it becomes a saved/recoverable project edit. Inspector
edits remain primary-entity scoped. Save As rescue needs source resources and
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
