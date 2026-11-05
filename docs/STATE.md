# Kiln Session Handoff

**Active focus:** 2D readiness audit completed on 2026-11-05; the full 2D engine is unfinished.
**Branch:** `feat/2d-production-workflow`. 3D remains paused. Tasks 7–9 have not been implemented.

## Done

- Audited feature commit `20291fa` and fetched current GitHub master `76da93f` (merge PR #17). Their source trees are identical, so the audit includes the latest merged Task 6 work.
- Recorded code-backed findings in `docs/2D_READINESS_AUDIT.md` and linked them from the completion plan. This session changes documentation only.
- Tasks 1–6 provide local build stability, scene/project/prefab/asset authoring, native entity scripting/Play, physics/cameras and visual authoring within their documented contracts. Native interaction acceptance for Tasks 2–6 remains pending.
- Task 6 includes real image/clip preview, sprite layout/sorting/filtering, terrain/layer/paint tools, saved VFX and automatic static physics for painted solids. Its generated sample and real archive/native-player parity acceptance remain available. See VISUAL_AUTHORING_2D.md, PHYSICS_2D.md and ADR-0054.

## Fresh validation

- `just check`: passed (formatting, strict native workspace/all-target Clippy, workspace and standalone audio sample compilation).
- `just test`: **592 passed, zero failed/skipped**.
- No implementation changes were made. Browser smoke, native/WASM builds and CLI packing passed in the Task 6 checkpoint; they were not rerun for this source audit.
- Audible audio, complete native GUI authoring, clean-machine distribution and full device/release performance acceptance remain unverified. Test success does not certify these workflows.

## Confirmed remaining work

- Task 7: native sound registration ignores its file path and does not append decoded audio; audio/editor mixer persistence is incomplete. Visual UI authoring remains editor-only. The current gameplay save API serializes sample score/pickups/player coordinates, excluding general entity/scene/attached-script state. Complete the runtime/authoring/settings workflow.
- Task 8: Build/Export still appends simulated success without executing a build. Implement actual process execution, logs/failure/cancellation, project resources and player packaging, and clean-machine launches. Real KAR packing/player loading already works but is not this complete workflow.
- Task 9: finish the top-down action/platformer/puzzle reference games, unaided tutorial and recovery/scale/device/release gates.
- Earlier-work follow-ups: KilnScript lacks for/while grammar, scripted spawning discards its texture argument, and debugger support is frame-level. Browser hosting still loads the older scene-script path, lacks the native entity/module/transition/camera workflow and does not enable physics.
- Full native drag/drop/gizmo/panel/Play/save/reopen and graphical packed-player acceptance remains pending without a display server. Browser sprite/input checks do not establish physics/script gameplay parity.

## Next

- Task 7 is the next planned implementation step when the user requests continuation, followed by Tasks 8 and 9.
- Treat scripting/spawn and browser parity as explicit release blockers; do not infer completion from earlier checked boxes or old roadmap claims.
- Do not start 3D or claim the complete 2D engine is production ready.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. GitHub operations use the supported `require_escalated` permission in this runner. This audit's source baseline is `20291fa` / merged master `76da93f`.
