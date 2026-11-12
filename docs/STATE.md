# Kiln Session Handoff

**Active focus:** Task 9's three reference games, explicitly requested by the owner now. Game creation and local software acceptance are complete; independent tutorial/hardware/stress/release gates keep Task 9 IN PROGRESS. Task 8 platform/distribution review remains open. Full 3D is paused.
**Branch:** feat/task9-editor-reference-games, created from 8ec785237997147964580bd79cad603310c16235 at the start of this request. Feature commit/push and its new CI checkpoint are recorded at session completion.
**Owner preferences:** no package/release-size goal; performance/resource and bounded-input requirements remain. Project code uses the settled MIT OR Apache-2.0 license. No licensing decision is pending.

## Done

- Created games/emberwatch (top-down combat/relay), games/skybridge (two physical platforming scenes), and games/switchyard (three circuit puzzles). They have independent project identities and documented controls/goals. Small reference games with simple graphics/shared starter menus; no commercial-game breadth claim.
- Actual native editor Save As produced all manifests from existing examples. Names/IDs, gameplay text, scene scripts/names, animation frames/references, duplicated level/camera, stage property, physical gate X and input bindings were authored in real widgets. Ordinary script text/artwork were supplied; no manifest generator or game-specific engine entry point was added.
- Fixed the authoring blockers: general project-relative Script Editor with 1 MiB bounds, guarded atomic per-file saves, external-change detection, syntax checks and Save/Discard/Cancel protection; editable project identity/window fields; scene name/script controls; scene/camera duplication and stable input-action ordering (ADR-0073).
- Fresh GUI replay reproduces every saved project value and exact source bytes. Both targets export through the installed editor with no build tools in child PATH. Each job's background verification must complete; all runtime/notice bytes match their verified installation, and overwrite attempts preserve prior output.
- Actual native players run without an archive argument from unrelated cwd, with source paths unavailable. Combat defeats all enemies and activates its relay; puzzle solves all three circuits and continues in a new player process; platformer crosses both physical gates and saves/moves/loads with restored position verified.
- Chromium serves only browser exports under nested HTTP URLs. All game objectives pass, including puzzle reload between circuits two/three and platformer position restoration/second-level victory. Saves use actual localStorage; no winning state or engine hook is injected. Music starts after consent; JavaScript/console errors fail acceptance.
- Added a read-only reference-games.yml workflow for the new branch: strict gates, source/KAR regressions, matched tool packaging, real editor authoring/export and native/browser gameplay, retaining small evidence for seven days. No public release/merge or binary Git payload.
- Preceding Task 8 notice candidates at a0f1d85 both completed required gates successfully: Windows run 43227026649 and Linux run 43227026505. These predate Task 9 and do not validate the new source. Compact public observations retained separately.

## Validation

- just check passes: format, strict workspace/all-target Clippy, workspace/audio sample checks. just test: 680 passed, zero skipped. All 29 Python tests, WASM kiln-web check, workflow actionlint and changed Python/JS syntax checks pass.
- Four new gameplay regressions each replay source and a real KAR: natural combat victory/loss/restart, exact puzzle solutions/moves/save restoration, actual Rapier grounding/jumps, advancing animation UVs and both level gates. Four editor regressions cover source preservation/conflicts/paths/bounds, script-only lifecycle protection and independent scene/camera duplication.
- Final software evidence: docs/evidence/task9-games/local-validation.json. Sources match the final UI-produced projects/exports. Native result: target/task9-native-complete/result.json; browser result: target/task9-browser-verified/result.json; final UI report: target/task9-final/result.json. Generated captures/logs/archives stay outside Git.
- The local prebuilt tool archive records preparing HEAD 8ec7852 with source_dirty=true. It is development installation evidence, not captured clean release provenance. Cargo.lock is unchanged (SHA-256 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17); no engine dependency was added.
- Validation corrected dropped keystream text (clipboard now byte-exact), waits that accepted an older export success, animation Add Frame reflow timing and rapid native axis edges (visible release intervals). Platformer load triggers on release to avoid restoring in a loop while held; position comparison proves actual load behavior.
- A cloud restart killed the first interactive GUI. Saved games survived and reopened. Temporary restart/display issues were not counted as engine acceptance failures or ignored checks.

## Next / known issues

1. Commit/push this validated Task 9 work, verify the exact new-branch remote SHA and observe the new reference-games workflow. Local success is separate from hosted results; record actual hosted source/run identity.
2. Have another developer follow the game/editor tutorial unaided and fix ordinary workflows that still require engine or manifest edits. Automation is not independent human acceptance.
3. Measure actual reference hardware startup/frame time/memory; test long sessions, large asset sets, recovery and clean exports. No physical i7-6600U/Intel HD 520, controller or audible device is attached here.
4. Reconcile roadmap/support claims against executed evidence. Finish Task 8's full Windows GUI workflow, physical/other GPU tests, macOS, SDK/dependency/exact-linked-code review, independent artifact signature/capture review, OS signing and public release preparation. Existing Linux candidate requires GLIBC 2.39/system libraries; binaries are not Authenticode-signed.
5. Script drafts participate in normal save/exit protection; existing recovery snapshots cover project documents, not unsaved source buffers. Save source explicitly before relying on crash recovery. Atomicity is per script file, not a new multi-file project transaction.

## Boundaries

Native/Xvfb/SwiftShader/ALSA-null and Chromium checks are software evidence, not physical/Windows/macOS/hardware-performance proof. api.github.com remains excluded by enforced cloud policy; public Git status/push work, but artifacts/signatures were not independently downloaded here. Origin attestations are separate from OS signing and legal completeness. Remaining Task 8/9 release gates stay open; full 3D remains paused. Package size is informational. No credentials were extracted, policies bypassed, unrelated artifacts deleted or tests suppressed.
