# Kiln Session Handoff

Owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 Save As failure/retry and source protection (2026-11-15),
following d012e01. A02–A04 remain PARTIAL. A01 baseline acceptance remains pinned
to green 6de428b (CI 43606018834, reference 43606018842, Linux 43606018785).
New source requires matching results. Full 3D remains gated on G2D; no broader
release gate accepted. MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Save As now closes only after success or explicit cancellation. Failures stay
  inline with the destination and draft available for retry. Cancel/Escape clears
  the copy error without resurrecting a generic failure popup. Guidance describes
  new-directory copies vs same-directory manifests sharing assets.
- Successful Save As rebuilds the active hierarchy and clears stale selection;
  the former view reset could leave no rows until another rebuild occurred.
- Save As now requires a clean source buffer via the existing guard, rather than
  implicitly writing a dirty script before later copy validation could fail.
  This restores documented Save/Revert behavior; no unsaved-source overlay copy
  is implemented. Explicit Save Script after cancellation targets the copied root.
- Added real Save As acceptance to native CI with retained evidence. No new
  dependency, format or command architecture. Prior graph parameter foreground
  fix, Parent tree/root drops, World/Local axes, snapping, clipboard/lock/hide/
  alignment and guarded recovery work remain implemented.
- Evidence: docs/evidence/A02_SAVE_AS_RETRY.md and prior A02 evidence files.

## Executed validation

- just check PASSED (fmt, strict workspace/all-target Clippy, workspace/audio).
- Source guard tests: 7 PASSED. Lifecycle/roundtrip/two-scene integration:
  20 PASSED. Python regressions: 38 PASSED.
- New save_as_failure_smoke.py PASSED: occupied destination rejection, Escape
  retaining X=88 draft, staged resource-symlink copy failure, no output/staging
  leaks, corrected retry from the same form, exact resource copying, rebuilt
  hierarchy editable, Y=72 saved only to the copy, guarded dirty-script rejection,
  cancelled buffer preserved and explicitly saved only in copy, exact restart.
- Original source manifest/assets and foreign project/sentinel remain byte-identical.
  Inspected inline copy error, copied-project and source guard screenshots.
- Existing clipboard smoke PASSED identity/history/cross-scene paste/restart.
  Raw evidence: target/a02-save-as-failure and target/a02-save-as-logs.
  No new cross-platform/device acceptance claimed.

## Hosted / next

Prior source d012e01 Windows candidate 43706643993 PASSED. CI 43706643952,
reference 43706643985 and Linux candidate 43706644024 were IN_PROGRESS at latest
observation. This predates Save As changes. Public workflow-list evidence only;
obtain matching confirmation for this commit and the graph-dialog fix. Do not
promote prior green source results to newer revisions.

Continue A02 full supported component/property and parented multiselect coverage,
additional external-file deletion/corruption and failed-save UI cases, and full
cross-platform acceptance. Dirty source is deliberately Save/Revert-before-copy;
Save As does not rescue unsaved code without that explicit action. Do not close
A02 from these isolated Linux workflows. Entity-parent tree support does not close
nested-scene authoring.

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
concurrent build/export load previously exposed graph/physics timing issues.
Generated target artifacts stay out of Git. Do not kill unrelated editor/Xvfb.
just check before every commit. No physical GPU/audio/controller claims.
