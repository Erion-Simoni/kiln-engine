# Kiln Session Handoff

Owner scope: A01 through A04 on `fix/a01-ci-baseline`.
Current focus: A02 external-manifest draft rescue (2026-11-15), following ac0a86c.
A02–A04 remain PARTIAL. A01 baseline acceptance is pinned to green 6de428b
(CI 43606018834, reference 43606018842, Linux 43606018785). New source needs
matching results. Full 3D remains behind G2D; no broader release gate accepted.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- Reproduced missing-manifest Save As failure in a disk regression: normal Save
  refused the external deletion but Save As canonicalized the missing file and
  failed too. The new test failed before the fix, passes after it.
- Resource copying now excludes the loaded manifest's filename under its already
  canonical source root. The copy's manifest comes from the in-memory draft.
  It need not read/resolve a deleted/corrupt/replaced input manifest. Unrelated
  TOML resources still copy exactly; linked resources remain rejected.
- Project-operation errors are visibly marked and Escape-dismissable without
  dropping the draft. Existing Save As form retry/cancel, clean-script requirement,
  hierarchy rebuild and all copy publication guards remain in place.
- Added disk/symlink regressions and three real widget rescue cases to CI with
  retained evidence. No dependency, format or command architecture changes.
- Evidence: docs/evidence/A02_MANIFEST_RESCUE.md. Earlier Save As source protection,
  graph dialog foreground repair, Parent tree, World/Local axes, snapping, clipboard,
  flags/alignment and actual two-scene recovery remain implemented.

## Executed validation

- just check PASSED: fmt, strict workspace/all-target Clippy, workspace/audio.
- Roundtrip/lifecycle/two-scene integration: 22 PASSED. Python: 38 PASSED.
- Before/after missing-manifest regression; deleted/corrupt/externally changed
  source cases; replaced manifest link exclusion and resource-link rejection pass.
- manifest_rescue_smoke.py PASSED all three cases: unsaved X=88, normal Save visibly
  refused, Escape retained draft, Save As rescued data/resources without changing
  the original external state, Y=72 saved only to copy, exact complete restart.
- Existing save_as_failure_smoke.py PASSED again: occupied/staged copy failures,
  cancel/retry, original preservation, copied hierarchy editing, dirty-source guard,
  explicit Save Script and exact restart. Screenshots inspected.
- Raw logs/receipts: target/a02-manifest-rescue-logs, target/a02-manifest-rescue,
  target/a02-save-as-failure. No new cross-platform GUI/device acceptance.

## Hosted / next

On prior source ac0a86c, Linux candidate 43708624752 FAILED before application
build: artifact-service preflight download reported probe artifact not found.
No service root cause was established; no preflight check was weakened. Windows
43708624748, CI 43708624755 and reference 43708624764 were still IN_PROGRESS
at observation. Obtain matching new-source checks and a successful candidate
artifact preflight; do not promote earlier green source results to newer code.

Continue A02 remaining full component/property and parented multiselect transform
coverage and cross-platform acceptance. Source directory/resources must still
exist for Save As rescue; whole-resource-tree reconstruction is not implemented.
Dirty code deliberately requires explicit Save/Revert before a copy. Entity-parent
trees do not close nested-scene authoring. Do not close A02 from isolated workflows.

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
