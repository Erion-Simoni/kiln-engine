# Kiln Session Handoff

Active owner scope: A01 through A04 on `fix/a01-ci-baseline`.
A01 configured baseline accepted on pushed source `6de428b168aff0b3cbad86038461e7c2731ad081`
(2026-11-15). A02 is next; A02–A04 and broader release gates remain open.
No full 3D work; G2D remains open. MIT OR Apache-2.0 is settled; no size ceiling.

## Done

- Startup fix `6de428b` waits for the painted editor menu before closing default
  tabs and holds menu clicks across frames. Delayed-render GUI regression is in CI.
- Matching hosted SUCCESS: general CI 43606018834; reference games 43606018842;
  Linux candidate 43606018785. General CI includes three OS check/build/archive
  matrices, native UI, browser/graph/portable flows and the delayed startup test.
  Candidate checks include fresh locked replay/payload comparison, artifact-service
  roundtrip, clean Ubuntu runtime acceptance and origin attestation.
- Reconnected after environment suspension; checkout, toolchain and target evidence
  survived. Temporary /tmp logs did not. No experimental graph patch was retained.
- Local complete three-game widget authoring/native+browser export receipt and moved
  portable integrity/template/export/relocated-gameplay acceptance survived and pass.
- Evidence and public artifact digests: docs/evidence/A01_RESUMED_VALIDATION.md and
  A01_HOSTED_STATUS.json. Public summaries inspected; hosted artifact contents and
  attestation signatures were not independently downloaded/verified here.

## Validation

- Before suspension: cargo test --workspace --locked, 752 passed including doc tests
  (751 ordinary tests), zero failures/ignored. Default WASM build, matching bindings,
  strict WASM default/feature-off Clippy and editor/player feature opt-out passed.
- After reconnection: just check PASSED; 38 Python regressions PASSED.
- External texture refresh and structured source authoring → browser movement pass.
  Browser menus/audio/controller/save-resume and exported materials pass.
- Parallel local GUI/build/export workloads exposed graph-dialog and physics-capture
  timing failures. An attempted graph timing patch was reverted after unsuccessful
  verification; no assertions or engine behavior were changed. Both unchanged tests
  passed in isolated reruns after reconnection, matching hosted CI. Graph native
  score 252/browser 324, 392 rendered pixels and three saved node values; physics
  restored upward velocity 121.900024 and correct collision/joint/scene state.
- Run timed GUI/gameplay checks serially. Parallel-resource-contention reliability
  is not established. Raw new logs: target/a01-resume-logs; graph receipt/screenshots:
  target/a01-resumed-graph; other receipts: target/a01-reference-replay and
  target/task8-portable-evidence. No new physical-GPU/audio/controller claim.

## Next

A02: prior fc78575 implements saved inherited lock/hide, selection/marquee,
alignment, clipboard, recovery and guarded writes with real two-scene evidence.
Broad combined transform/property and cross-platform acceptance remains open.
A03: 3daaa1c adds typed dependencies/reference-safe moves, per-target import rules,
worker diagnostics and fixed GPU preview lifetime. Full nested prefab/variants,
integrated atlas build/rebuild/reopen/export, source-module import refactoring and
combined renamed-art/prefab/override/multiple-scene production acceptance remain open.
A04: actual live source breakpoints, statement stepping, imported function frames/
locals/watches, safe reload specification and the new enemy/projectile/trigger game
with equivalent graph and complete exports remain open. Cooperative task payloads
are explicit state machines, not automatic suspended function stacks/yield syntax.
The legacy separate bytecode debugger is not the live gameplay source debugger.

## Known boundaries / tools

Windows candidate did not trigger for the Linux-test-only startup change; the older
3efb687 candidate does not validate this revision. Matching Windows check/build/
archive jobs in general CI did pass. No fresh full Windows GUI/candidate acceptance
is claimed. Baseline CI acceptance does not close any broader engine release gate.

Source /workspace/.kiln-tools/activate.sh for Rust 1.99.1, Cargo, rootless system
libraries/software Vulkan and CHROMIUM_PATH. Chromium 156.0.8078.79 is installed
rootlessly from Debian; Playwright's browser download domain is network-denied.
CARGO_INCREMENTAL=0 and debug-info overrides keep disk use bounded. Generated
artifacts stay out of Git. Do not kill unrelated editor/Xvfb processes.
just check before every commit. Continue A02 implementation/acceptance next.
