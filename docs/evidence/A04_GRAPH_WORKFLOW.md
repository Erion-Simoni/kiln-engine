# A04 editable graph checkpoint — 2026-11-14

Branch `fix/a01-ci-baseline`, following pushed checkpoint `3daaa1c`. Linux x86_64,
Xvfb/software graphics and Chromium via `CHROMIUM_PATH=/usr/bin/chromium`; Rust
1.99.1. No physical-GPU or cross-platform graph acceptance is claimed.

- `python3 tests/native/graph_authoring_smoke.py --web-runtime target/debug/web-runtime`
  PASSED real graph creation, pin connections, registered add_score argument,
  parameter Cancel, Undo/Redo, one-step multi-frame drag undo, scene assignment,
  Play/F5 save, live node values, reopen and exact document equality. Browser also
  rendered 392 hero pixels, saved three actual node values and restored an edited
  ordinary gameplay save through F9/F5. Native score 252, browser score 378; both
  reflect the committed argument 9. The test observes the visible parameter dialog
  before typing; it does not inject graph/project state.
- Raw receipts/captures: `target/a04-graph-authoring/result.json`, `browser-play.log`,
  `editor/`, `reopened/`. The saved native slot uses its normal metadata/data envelope.
- `cargo test -p kiln-editor --test graph_gameplay` PASSED graph/text movement parity
  over multiple inputs, actual root VM value inspection and logical save/restoration.
- Graph regressions execute host IDs/arity, argument order, conditional side effects,
  fan-out once per tick, shared registry calls, exact strings/i64 MIN, deterministic
  metadata roundtrip, edited-body guards, invalid draft recovery and graph bounds.
- `cargo nextest run --workspace --no-fail-fast`: 751 passed, zero skipped.
  `just check`, WASM default build, WASM feature-off strict Clippy and 36 Python
  regressions pass. Logs: `target/a04-graph-full-tests.log`, `a04-graph-final-check.log`,
  `a04-graph-wasm.log`, `a04-graph-feature-off.log`, `a04-graph-python-tests.log`.

Graph source goes through the same frontend/runtime as scripts and ordinary exports.
The leading metadata comments are bounded lines to avoid huge editor text layouts.
Invalid graphs generate an intentional compile error, so a draft cannot silently run
as an empty game. Out-of-document-budget edits are rejected, retaining the prior draft.

Open A04 acceptance: actual source breakpoints/statement stepping, imported function
frames/locals/watches, safe reload specification and the new enemy/projectile/trigger
equivalent script/graph game with complete export evidence. Last-executed graph state
and frame stepping do not establish source debugging. A01–A04 remain PARTIAL.

Hosted A01 followup: on `3daaa1c`, Windows candidate 43601654013 passed; general
CI 43601654042 failed specifically while observing the floating Script Editor
(title-run wait), before authoring. The egui shader dithers sRGB pixels; the old
observer required an uninterrupted exact RGB run. A narrow two-channel-unit color
tolerance now recognizes that rendered bar while rejecting gray canvas/short runs,
with two Python regressions. A matching hosted rerun must still establish whether
this resolves that failure; the current local authored-source/browser rerun passes.
