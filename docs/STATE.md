# Kiln Session Handoff

**Active focus:** Task 1 of `docs/2D_COMPLETION_PLAN.md`: build/test baseline.
**Branch:** `feat/2d-production-workflow`, based on master `cd62b63`.
**Scope:** The user authorized a new branch, a full 2D completion plan, and sequential implementation. Continue that plan one tested task at a time. 3D remains paused. Historical milestone claims are not evidence that authoring/export is production-ready.

## Done

- Published the ordered 2D plan with acceptance criteria and explicit pending work.
- Repaired malformed player test code and stale core/player Sprite fixtures.
- Fixed nonvisual entity lookup: gameplay roles no longer require a Sprite.
- Fixed script state persistence: stale host state no longer erases commands applied in the same frame. Added a multi-frame regression.
- Corrected the integration test input path to reach the sample's actual coin position without weakening scoring/save/archive assertions.
- Updated the GPU instance-layout expectation for the material field and bounded the worker scheduling test by elapsed time.
- Restored audio-enabled editor and standalone sample compilation against Rodio 0.23. Playback/file-decoding completeness is a later task, not claimed here.
- Confined browser-only APIs to wasm32 and separately checked the real WASM target.

## Validation

- Default native workspace build passed.
- Full native nextest suite: 452 passed, zero failed/skipped.
- Audio-enabled editor and standalone audio sample checks passed.
- `git diff --check` passed.
- `just check` still fails on existing formatting differences. It did not reach strict clippy; the global quality gate remains pending.
- Actual WASM-target check fails with 35 API errors; web support is not ready. No browser execution, audio playback, or new hardware validation is claimed.
- Prior merged editor fix visually verified welcome/workspace rendering with software OpenGL.

## Next

1. Continue Task 1: repair the actual WASM API errors, validate browser execution, and restore formatting/strict lint/CI gates without suppressing failures. Do not mark Task 1 complete prematurely.
2. Begin scene-authoring and persistence work only after the baseline acceptance criteria pass.
3. Follow the remaining ordered tasks in `docs/2D_COMPLETION_PLAN.md`; none of tasks 2–9 are complete.

## Environment

Source `/workspace/.kiln-tools/activate.sh` before Cargo commands in this cloud machine. GUI checks require a display; previous cloud rendering used a local Xvfb and `WGPU_BACKEND=gl`. Physical device and clean-machine release verification remain separate gates.
