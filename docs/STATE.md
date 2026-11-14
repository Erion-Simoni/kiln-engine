# Kiln Session Handoff

Active owner instruction: continue A01 through A04 on `fix/a01-ci-baseline`.
This environment resumed from pushed graph checkpoint `3efb687` on 2026-11-14.
Current focus: A01 startup synchronization. A01–A04 remain PARTIAL; full 3D stays
behind G2D. MIT OR Apache-2.0 is settled; no release-size ceiling.

## Done this checkpoint

- Fetched and checked out the requested branch in `/workspace/kiln-engine`,
  preserving the graph/script/resource/scene work from the prior environment.
- Restored Rust 1.99.1, rustfmt/Clippy/just and rootless native test libraries,
  Xvfb/xdotool/software Vulkan under `/workspace/.kiln-tools`.
- General hosted CI 43603619952 on `3efb687` still failed at the floating Script
  Editor wait in structured-scripting authoring. The prior dither-tolerance fix
  alone did not resolve it. Matching Windows candidate 43603619881 and reference
  games 43603619867 PASSED; Linux candidate 43603620009 was still IN_PROGRESS
  at the recorded observation. Public HTML only; no authenticated artifact/log access.
- Reproduced the exact floating-panel timeout by pausing the owned editor after
  X11 window creation and resuming after 2.5 seconds. The old 0.7-second wait sent
  default-tab close clicks before rendering, leaving Script Editor docked.
- Shared GUI startup now waits for the visibly painted menu. Tab closes and panel
  menu selections use held press/release events. All floating-title, source,
  movement and missing-canvas postconditions stay active; no engine hooks added.
- Added delayed-render real GUI acceptance to native CI and retained screenshots,
  diagnostics and receipts. Added unpainted/solid/truncated/dithered-frame regressions.
- Evidence: `docs/evidence/A01_EDITOR_STARTUP.md` and updated hosted status JSON.

## Executed validation

- Default editor/player/CLI build PASSED.
- `just check` PASSED: formatting, strict workspace/all-target Clippy, workspace
  check and standalone audio sample check. Python regressions: 38 PASSED.
- Delayed first-render reproduction FAILED before the fix, PASSED after it;
  committed `tests/native/editor_startup_smoke.py` PASSED. Inspected screenshot
  shows actual floating Script Editor with default source/animation tabs closed.
- Real external texture refresh and enum/task/timer/dictionary/loop source
  authoring PASSED. Real source format/search/navigation/completion/save/restart
  smoke PASSED. Forced-crash recovery/restore/save, explicit discard, exit
  Cancel/Discard and unsupported-sidecar preservation smoke PASSED.
- No engine source changed. Full Rust tests and browser runtime were not rerun in
  this fresh environment; prior checkpoint recorded 751 Rust/36 Python passes
  plus graph native/browser gameplay and saved node-value restoration.

## Next / remaining gates

A01: obtain matching hosted results for this startup fix; the local delayed-render
reproduction establishes a sufficient cause, not proof of all hosted failures.
Follow Linux candidate 43603620009 without repeatedly canceling long runs. Keep
all three authored games, general CI, artifact checks and candidates source-pinned.
Do not close acceptance from prior-source green runs.

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


## Environment / evidence

Source `/workspace/.kiln-tools/activate.sh`; it sets CARGO_HOME/RUSTUP_HOME,
CARGO_INCREMENTAL=0, debug-info overrides, rootless pkg-config/library paths and
software Vulkan. Local tools use Debian 13 rootless packages. The previous
`CHROMIUM_PATH=/usr/bin/chromium` installation is absent in this environment.
Raw current evidence lives under target/a01-delayed-start, a01-editor-startup,
a03-a04-workflow, a04-source-tools and a02-script-recovery; generated artifacts stay
out of Git. No physical hardware or new Windows/macOS GUI certification claimed.
Do not kill unrelated editor/Xvfb processes. `just check` before every commit.
