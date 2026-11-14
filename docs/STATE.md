# Kiln Session Handoff

**Active work:** staged A02–A04 implementation, explicitly authorized by the
owner's “continue up to A04” request (ADR-0080/0081). Those milestones remain
PARTIAL; this batch completes concrete substeps, not their entire acceptance.
**Branch:** `fix/a01-ci-baseline`; batch baseline `16b0e7b`. Delivery is split into
CI-readiness, editor-authoring and scripting commits, with tests, CI coverage,
language contract and evidence versioned alongside the implementation.
**Owner constraints:** no release-size ceiling; retain resource/performance
limits; MIT OR Apache-2.0 settled; full 3D stays gated on G2D.

## Done

- A02: project-scoped entity clipboard captures selected subtrees, all supported
  components and detached world poses; remaps parents, joints and prefab-instance
  identities; paste is one atomic scene command/history step. System clipboard
  and egui platform output work. Multi-entity paste selects its new entities.
- A02: pre-write manifest guards retain externally changed/deleted/corrupt files;
  formatting-only changes are allowed. Save As refuses another existing project
  file in the same directory. Existing script draft/recovery guards remain active.
- A03: size/mtime polling detects external changes/additions/deletions, refreshes
  selected previews and rendered PNGs, retains filters and project asset IDs,
  excludes recovery/export outputs and rejects escaped preview paths.
- A03: atlas fit, pixels and UVs use the same stored placement. Failed additions
  do not mutate packing state; invalid data/names/configuration are errors;
  fallible construction checks allocation. Boundary/shelf pixel tests pass.
- A04: scoped counted loops, break/continue/increment rules and bounded dictionaries
  share the existing compiler/IR/host registry. Typed logical saves/preferences
  retain nested values and large integers. Syntax colors and searchable built-in
  API Reference are available in the shared Script Editor.
- A04: bytecode stepping executes, Continue resumes past a reported breakpoint,
  locals update, step-over/out follow real call depth, Restart clears bytecode
  state, and single steps/full execution share the configured instruction limit.
  Source/live gameplay debugging remains separate unfinished work.
- A01 followups: held input-binding presses repair the locally reproduced
  Skybridge timeout. Windows window selection waits for a valid client area and
  chooses the largest PID-owned visible window; regression tests pass.

## Validation

- `just check` PASSED: formatting, workspace all-target strict Clippy, workspace
  check and the standalone audio sample check.
- `cargo nextest run --workspace --locked`: **725 passed, zero skipped**.
  Python native/build/package regressions: **34 passed**.
- WASM strict Clippy passed with default and no-default features; browser runtime
  built and matching wasm-bindgen bindings/runtime manifest generated.
- Real editor Copy/Paste, undo/redo, second-scene paste and saved restart passed;
  new IDs remain unique and supported properties remain identical after restart.
- Real external PNG overwrite/restoration changed/restored scene framebuffer
  pixels without rewriting the manifest; source authored via real widgets uses
  counted loops and dictionaries to determine movement speed. The exact new
  authoring-to-browser CI path passed with local Chromium.
- Fresh portable installation verified (1546 files). Its native/browser exports
  retained complete notices and matching runtime bytes, and deliberate overwrite
  attempts failed without changing valid outputs. Native movement X119.5→137.5
  and browser X119.5→133.5 passed; both stopped after key release.
- Crash recovery rerun PASSED with real 30-second autosaves, forced kills, restart,
  Restore/Save, Escape preservation, recovery Discard, normal exit Cancel/Discard
  and unknown-version preservation against the new save guards.
- Full Skybridge authoring replay PASSED: all source/assets/scene properties/input
  bindings match the checked-in project; source template remains unchanged.
- actionlint 1.7.7, mdBook 0.4.52 and JavaScript syntax checks passed.
- Detailed contract/results/screenshots: docs/evidence/A02_A04_AUTHORING.md;
  raw generated logs/results under target/a02-*, target/a03-a04-*, target/a04-*.
  Two harness setup errors (missing installation notices and local Playwright
  browser path) were corrected; no assertions were removed.

## Hosted status / next work

- Source 16b0e7b: general CI **43520789419 PASSED**, Linux candidate
  **43520789200 PASSED**. Reference games **43520789363 FAILED** at Skybridge's
  first input binding; Windows candidate **43520789391 FAILED** because capture
  chose a too-small client. Current binding/window-readiness fixes need fresh
  matching-source hosted confirmation. See docs/evidence/A01_HOSTED_STATUS.json.
- A02: lock/hide and alignment; integrated two-scene transforms/selection/history/
  recovery acceptance. A03: renamed-art/multiple-scene prefab/atlas/export
  acceptance, nested prefabs/variants and platform import presets. Metadata-only
  polling cannot detect content changes preserving both file size and mtime.
- A04: enums, timers/coroutines, completion, project search/formatting/navigation,
  actual gameplay source breakpoints/statement stepping/imported stack frames/
  expression watches and editable/inspectable equivalent graph acceptance.
- Continue the authoritative docs/ENGINE_COMPLETION_PLAN.md; do not start full
  3D or mark A02–A04/G2D complete based on these substeps alone.

Toolchain: source /workspace/.kiln-tools/activate.sh; Rust 1.99.1, nextest 0.9.151,
mdBook 0.4.52. Set CARGO_INCREMENTAL=0, CARGO_PROFILE_DEV_DEBUG=0 and
CARGO_PROFILE_TEST_DEBUG=0 for this workspace. Local browser tests use
CHROMIUM_PATH=/usr/bin/chromium. Xvfb/software graphics/null audio and Chromium
software rendering do not establish physical-device or Windows GUI acceptance.
Git push and public GitHub HTML reads work; authenticated GitHub API remains
Forbidden, so do not use it to infer missing CI details.
