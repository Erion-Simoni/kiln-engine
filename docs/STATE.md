# Kiln Session Handoff

Active owner instruction: finish A01 through A04. Branch `fix/a01-ci-baseline`;
last pushed checkpoint before this graph batch: `3daaa1c`. Continue implementation,
not checkbox closure. Full 3D remains gated on G2D; no release-size ceiling;
MIT OR Apache-2.0 settled. A01–A04 remain PARTIAL.

## Done in this checkpoint

- Replaced graph hash IDs/stubs with bounded KilnScript lowering through the same
  shared host registry/frontend/VM as text. Registry moved into kiln-script with
  backwards-compatible runtime re-export; kiln-graph reuses the existing crate.
- Correct host argument pins/return values and separate optional trigger/done pins;
  constants, arithmetic/comparison, lifecycle events and conditional event merges.
  Each node computes once per invocation despite fan-out. Missing/duplicate inputs,
  cycles, unsupported reachable nodes, depth/expansion/value bounds fail explicitly.
- Wired the existing interactive graph widget into the actual editor. Parameter
  dialogs commit on OK, Cancel preserves values, Delete removes nodes/connections,
  signature changes retain compatible pin IDs and remove obsolete wires. Fixed
  multi-frame dragging, added visible pin names and actual shared-registry chooser.
- Visual metadata and ordinary generated source are one guarded .ks resource with
  bounded comment lines. Graph drafts share existing script Save, dirty guards and
  crash recovery. Incomplete graphs retain metadata but intentionally fail compile;
  manually changed generated bodies are preserved as text. Out-of-budget edits
  reject the transaction. New graph resources assign to scenes through the UI.
- Graph Undo/Redo is scoped to the file, bounded jointly to 32 snapshots/8 MiB;
  each drag is one gesture. Graph controls are read-only during Play; actual last-
  executed root VM node values are inspectable and survive logical saves.
- Added real editor → native/browser graph acceptance and retained evidence in CI.
- A01 observation fix: floating-panel detection tolerates egui sRGB dithering by
  two channel units while rejecting canvas gray/short runs. Parameter-dialog waits
  also observe the rendered dialog before typing. Added two Python regressions.

## Validation

- just check PASSED: fmt, strict workspace/all-target Clippy, native/workspace and
  standalone audio check. Workspace: 751 passed, zero skipped. Python: 36 passed.
  actionlint, mdBook build and Node syntax check passed.
- Fresh default WASM build and no-default-feature strict WASM Clippy PASSED.
- Real graph widgets/native/browser acceptance PASSED: wire actual add_score with
  argument 9, parameter Cancel, Undo/Redo, one-step drag undo, scene assignment,
  Play/F5 save, actual node values, exact restart and browser F9/F5 restoration.
  Native score 252/browser 378; browser rendered 392 hero pixels, saved 3 node values.
  Reproduction: tests/native/graph_authoring_smoke.py --web-runtime target/debug/web-runtime.
- Real session graph/text movement parity and logical save/restart pass. Graph
  regressions cover host IDs/arity/order, branch side effects, once-per-tick fan-out,
  exact strings/i64 MIN, deterministic metadata, manual-edit guards and bounds.
- Repaired floating-panel observer rerun PASSED real editor source authoring and
  browser enum/task/timer/dictionary/loop movement. Baseline movement assertions and
  missing-canvas rejection remain active. No renderer behavior was changed.
- Evidence: docs/evidence/A04_GRAPH_WORKFLOW.md and A03_A04_RESOURCE_TOOLS.md.
  Logs/screenshots/receipts are under target/, excluded from Git. Xvfb/software
  graphics is not physical hardware or Windows/macOS graph certification.

## Next / remaining gates

A01: source 3daaa1c Windows candidate 43601654013 PASSED. General CI 43601654042
FAILED while observing the floating Script Editor (exact title-run timeout, before
source entry). The new dither-tolerant observer requires matching hosted confirmation;
this is a targeted fix, not proof of the hosted root cause. Reference 43601653998
and Linux candidate 43601653996 were still IN_PROGRESS at last observation. Earlier
fc78575 Linux candidate 43576816847 passed; PR #22 general/reference runs also passed,
but current source still needs all required matching results. Public job logs return
404; authenticated API is Forbidden. Push/public HTML work. Avoid rapid repeated
pushes that continually cancel long candidate runs.

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

Tools: source /workspace/.kiln-tools/activate.sh; CARGO_INCREMENTAL=0,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0. Rust 1.99.1/nextest0.9.151;
local browser CHROMIUM_PATH=/usr/bin/chromium. just check before every commit.
Do not kill unrelated editor/Xvfb processes. Generated artifacts stay out of Git.
