# Kiln Session Handoff

Owner requested fixes across A02–A04 on `fix/a01-ci-baseline`.
Latest focus: A02 nested identity/data preservation, following 9bc2ace settings fix.
All A02–A04 remain PARTIAL. A01 accepted baseline stays 6de428b; 3D behind G2D.
MIT OR Apache-2.0 settled; no package-size ceiling.

## Done

- 9bc2ace fixes scene metadata/camera/nesting transactions, camera tab dirtiness/
  Discard and composition preview/recovery. Evidence: A02_SCENE_SETTINGS.md.
- Reproduced direct/transitive nested ID collisions and actual root-data overwrite
  from an ambiguous runtime snapshot. Before regressions failed, after pass.
- New nested links validate every affected ancestor before mutation. Shared IDs,
  repeated instances and missing links are refused with a dismissible inline error.
- Legacy ambiguous root snapshots preserve authored data; inspector/gizmo mapping
  excludes ambiguous handles. No guessed namespacing, file migration or automatic
  script-ID rewrite. Separate uncomposed scenes can still share entity IDs.
- CI/docs/evidence: docs/evidence/A02_NESTED_IDENTITY.md; nested_identity_smoke.py.

## Executed validation

- just check PASSED; full cargo test --workspace --locked: 773 PASSED, zero failed
  including doc tests. Python: 40 PASSED. New topology/snapshot regressions failed
  before fixes. Both final widget workflows pass; screenshots inspected.
- Real-widget invalid-composition regression failed before the guard. Afterward
  duplicate/collision rejection, project/history preservation, specific diagnostic
  and restart pass. Legacy colliding input cannot edit arbitrary targets and keeps
  complete authored project bytes semantically through save/restart.
- Valid distinct-ID scene-settings composition/camera/recovery rerun retained in
  target/a02-scene-settings-logs. Raw identity captures/receipts in
  target/a02-nested-identity and before directory. Linux Xvfb/software graphics only.

## Next / known issues

Obtain matching hosted results; earlier green code does not accept new source.
Prior d110141 PASSED CI 43717563394, reference 43717563395, Linux 43717563439.
Prior production 276524c Windows 43711940872 passed (d110141 was path-filtered).

Continue A02 all-property combined acceptance and cross-platform GUI acceptance.
Scoped/repeated nested entity instances need a deliberate runtime/script identity
contract; new links now refuse ambiguity, and old documents are preserved safely.
Inspector property edits still target the primary entity. Save As rescue still
requires source resources and explicit Save/Revert of dirty script buffers.
A03/A04 remain requested; no completion claimed from these A02 corrections.

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
