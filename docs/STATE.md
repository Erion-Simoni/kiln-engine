# Kiln Session Handoff

**Active focus:** Task 1 of `docs/2D_COMPLETION_PLAN.md`: build/test baseline.
**Branch:** `feat/2d-production-workflow`.
**Scope:** Continue the user's ordered 2D completion program one tested task at a time. 3D remains paused. Do not treat historical milestone claims as production-workflow evidence.

## Done

- Native baseline restored in the preceding checkpoint: workspace build, audio-enabled editor/sample checks, persistent script state, nonvisual entity lookup, and existing integration tests.
- Actual WASM compilation repaired: canvas ownership/configuration, browser API feature declarations, async audio decoding, public script-host access, widget APIs, default scene selection, and shared entity spawning.
- Sound callback access no longer depends on native Rodio. Native callbacks retain Send/Sync; browser callbacks can capture JS handles. ADR-0037 records this and test-only dependencies.
- Browser fallback probes WebGPU before claiming the canvas, then uses a separate WebGL instance if there is no adapter. Downlevel device limits work in software WebGL.
- Clock uses web-time only on wasm32; std Instant caused an actual browser panic.
- Separate overlay GPU buffers prevent HUD uploads from replacing world sprite data before submission. UI/HUD commands draw as a single overlay batch.
- Animation callback owns its Window, preserving future frames and input. Startup rejects on missing canvas/initialization errors instead of returning apparent success.
- Added reproducible `just web-smoke`, pinned Playwright tooling, and a browser CI job. See tests/web/README.md.

## Validation

- Actual WASM build plus matching wasm-bindgen output: passed.
- `CHROMIUM_PATH=/usr/bin/chromium just web-smoke`: passed. Loads real assets, checks visible hero pixels and movement/key release, rejects invalid startup, fails on browser/asset errors.
- Full native nextest suite: 453 passed, zero failed/skipped, including the new platform sound callback regression.
- `git diff --check`: passed.
- Remote CI execution is not yet verified. Full `just check` remains blocked by pre-existing workspace formatting differences; strict clippy has not been restored. Task 1 is still IN PROGRESS.

## Next

1. Restore full-workspace formatting and strict lint/CI gates without blanket suppressions or weakening tests. Re-run required build/test/browser checks after substantive changes.
2. Begin task 2 (scene authoring/persistence) only after baseline acceptance criteria pass.
3. Follow tasks 3–9 in the completion plan; none are complete.

## Limits and environment

The browser result is Chromium software WebGL, not physical WebGPU, Safari/Firefox,
touch-device validation, a complete CLI export, or full audio/music/save support.
Native audio checks establish compilation, not audible playback. Hardware frame
budgets, release size budgets, clean-machine packaging, and real-device checks
remain separate gates. Source `/workspace/.kiln-tools/activate.sh` for cloud Cargo
commands; `CHROMIUM_PATH=/usr/bin/chromium` uses the installed browser for smoke tests.
