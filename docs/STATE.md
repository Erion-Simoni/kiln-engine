# Kiln Session Handoff

**Active focus:** Task 8 browser physics checkpoint completed locally on 2026-11-10. Task 8 remains IN PROGRESS for sprite material host integration, physical/other-platform checks and release provenance.
**Branch:** `feat/2d-production-workflow`. Base: `1349589` (size goals removed; clean Ubuntu acceptance). Task 9/full 3D implementation remain unstarted.
**Owner preference:** package/release size is not a goal. Do not chase size ceilings or request size exceptions (ADR-0062). Performance/startup/memory/resource and bounded-input requirements remain.

## Done

- Enabled existing kiln-runtime/Rapier physics in default kiln-web builds through an opt-out `physics` feature. Uses the shared native scene/tile collider, fixed-step, body/joint lifetime, collision/script query and logical save/restore paths. No new third-party dependency or separate browser solver.
- Added RuntimeInfo.physics with serde default false for legacy JSON. Extended prepared `kiln.runtime.v1` metadata with strictly validated physics=0/1; legacy three-line metadata remains readable with physics=false. Descriptor/hash/runtime equality still protects against changed/mixed files. Current native players declare physics=true (always-enabled dependency).
- Added shared project capability detection for declared physics, all scenes/prefab bodies/joints and referenced potentially solid tilemaps. CLI gates both native/browser exports against the selected runtime's physics support and cleans failed staging. Nonphysics projects can still use legacy bundles. Browser direct run/prepare also rejects authored physics in opted-out builds. Sprite material overrides remain explicitly rejected.
- Added playable samples/browser_physics2d: real contacts, filtered collider, sensor, raycasts, solid tile floor, kinematic mover, distance/revolute/prismatic joints and motor/limits; controls for jump/checkpoint/load/snapshot/scene transition. Uses an authored HUD and a correctly placed stationary camera. Portable producer now includes this example alongside saved UI and updates instructions.
- Added real browser CLI-export/relocation acceptance with empty tools PATH and unavailable authoring project copy. Uses ordinary gameplay snapshots to verify bodies/joints/tile collisions, upward velocity after page reload/load with Start once, rebuilt contacts, cleared old colliders/joints after scene change and cross-scene restore. Pixel assertions verify visible bodies at both actual floor heights. Real physics-disabled WASM fails both export and browser startup usefully.
- Added core/assets/CLI compatibility and detection regressions. CI checks default/minimal WASM, builds/prepares a font-only physics-disabled bundle, runs real physics acceptance and uploads evidence. ADR-0063 and BROWSER_PHYSICS_2D.md record contract; selected inspected evidence is in docs/evidence/task8-physics. Remote CI execution is unobserved.
- Existing browser movement, UI/fonts/input/audio/storage/consent/loading/error behavior and actual native-editor Browser export regressions pass with physics enabled. New portable bundle also passes native editor/gameplay acceptance again inside clean Ubuntu runtime userspace without Rust/repository/development sysroot.

## Executed validation

- `just check` passed: formatting, strict workspace/all-target Clippy, workspace/audio sample checks. `just test`: **660 passed, zero failed/skipped**. Four Python producer tests passed after bundle changes; JS syntax and git whitespace checks passed.
- Default WASM build/bindgen/descriptor preparation passed, plus a real font-only/physics-disabled build. Strict default and no-default-feature WASM Clippy passed. Minimal editor/player checks passed.
- `tests/web/physics.cjs target/debug/web-runtime` with WEB_PHYSICS_MINIMAL_RUNTIME=target/task8-no-physics-runtime passed: real gravity/floor/filtering, sensor enter/exit/raycast, tile collision, kinematics, all three joints, slider motor/limits, save/page reload/velocity/Start once, contact reconstruction, scene cleanup and cross-scene load. Rendered hero: 400 blue pixels, centroid Y329.5 then Y229.5. Missing-feature export and visible bootstrap error pass. No unexpected errors/failed artifact requests.
- Existing Chromium movement and direct UI/audio/input/storage tests passed. Real editor Browser Save & Export saves Player X=88 and preserves previous output on failure. Its relocated artifact passed full nested HTTP loading/error/retry, explicit audio consent/held-key, controller/touch, audio overlap/settings/save/reload/Continue/quota/corrupt regressions.
- Portable archive prepared/verified: target/dist/kiln-task8-browser-physics.tar.gz; SHA-256 234417f3390af10d7eb2b7e7ddd7cde829066a596cd99da21661a6f134d71336. Includes new playground, shared Linux runtime requirements, all tools/prepared runtime/notices. Generated binaries/archives are not committed; size is informational.
- `tools/test_clean_linux.py` on that archive passed with separate target/task8-physics-clean-linux-evidence: unprivileged/offline/read-only-root Ubuntu 24.04, only archive/evidence mounts, no Rust/Cargo/compiler/Git or repository/sysroot. Real installed native editor save/export/failure/cancel and relocated native menus/settings/save/restart/Continue/corrupt checks pass, then unchanged-installation verification. Browser gameplay was executed outside this native-only container. Selected browser captures were inspected.

## Next

1. Task 8: integrate/test authored sprite material overrides in actual runtime draw submission, shader/resource loading and pipelines. Keep explicit browser export rejection until the real host supports them; inspect native/shared host behavior as part of this work.
2. Validate physical Linux GPU/audio/controller and appropriate Windows/macOS/browser/mobile machines, including Intel HD 520 if available. Functional software acceptance is not reference-hardware performance evidence.
3. Establish exact/reproducible binary build provenance and resolve internal package license declarations before release installers/signing. Prepare release-profile compatibility/performance evidence where appropriate, with no size target.
4. Complete remaining Task 8 acceptance before unasked Task 9 or full 3D implementation.

## Known issues and boundaries

- Browser sprite material overrides remain rejected pending host integration. This physics checkpoint does not certify every script/platform feature, other browser engines/mobile or actual devices. Shader/material support should not be claimed from standalone renderer APIs alone.
- Xvfb/SwiftShader/Mesa/ALSA null and simulated gamepad APIs do not certify physical Intel HD 520 performance or audible/physical controls. Containers share the host kernel. Clean Linux evidence is Ubuntu 24.04/X11 software graphics; Wayland, older glibc baselines and other OS need separate runs. No laptop/device was connected.
- Legacy JSON/metadata defaults physics=false, deliberately preventing unsupported exports. Upgrade native tools and prepared web runtime together; older readers may reject extended metadata. Opted-out runtimes remain usable for nonphysics projects.
- Linux bundles require compatible glibc (current binaries reference 2.39), system libraries and dynamic X11 support from shipped requirements. Test drivers also add libraries; executed-path success is not universal/minimal system/driver/plugin closure. Libraries are not bundled.
- Internal packages with missing package-license declarations remain null; workspace policy/original notices are preserved without inventing grants. Revision/dirty/compiler fields are preparation observations, not arbitrary-binary/source attestation. Release licensing/provenance remain open.
- Package size never blocks completion. Technical bounded-file/input parsing/copy limits remain. No reference-hardware startup/frame-time/memory, remote CI, signed installer or release-upload result is implied.
