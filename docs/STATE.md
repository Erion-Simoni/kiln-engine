# Kiln Session Handoff

**Active focus:** Resolve PR #21's master conflicts and verify the combined Task 9 editor/reference-game work. Task 9 remains IN PROGRESS for independent tutorial, hardware, stress and release gates. Task 8 distribution/platform review remains open; full 3D is paused.
**Branch:** feat/task9-editor-reference-games. Integration commit f7c8f9da50fa2d8e32e8811250c4e199870838cf is pushed and its exact remote SHA verified; public PR #21 head matches. It merges reference-game HEAD 4628429 and master 9da5551 without rewriting history or merging the PR. New source-pinned CI run 43404644543 is in progress; later documentation-only checkpoint commits do not change that source identity.
**Owner preferences:** no package/release-size goal; performance/resource and bounded-input requirements remain. Kiln code uses MIT OR Apache-2.0; no license decision is pending.

## Done

- Reconciled the five conflicting files: DECISIONS.md, STATE.md, app.rs, lib.rs and script_editor.rs. Preserve master's Quick Actions, workspace polish, default source tab, inspector/asset script opening, movement starter and KilnScript control flow/short-circuit changes alongside all three reference games and general authoring improvements.
- One ScriptEditor buffer now lives in EditorShell and serves inspector, assets, tabs, Save/Save As, Play and export. Keep generic project-relative Open/New, bounded text, import-aware validation, atomic saves and external-change detection. Refocusing the same dirty file preserves the draft; switching files remains guarded.
- Include source drafts in normal project Save/Discard/Cancel. A failed save keeps the prompt/draft; Discard clears the shared buffer before loading a replacement. Existing crash snapshots cover project documents, not unsaved source text.
- Actual GUI replay exposed duplicated Animation Editor panels/egui widget-ID collisions. Menu routes now reuse an existing panel; regression covers default and floating panel reopening. Keep the upstream default layout. The authoring driver follows the merged theme, closes default authoring tabs before floating them, and uses visible title-bar pixels plus ordinary dragging to handle staggered windows.
- Preserve all upstream ADRs (0073–0075); renumber the reference-game decision to 0076 and document this integration in ADR-0077. No engine dependency, save/project format or performance-budget exception.
- Keep games/emberwatch (combat/relay), games/switchyard (three circuits), games/skybridge (two physical levels), real editor authoring/export and native/browser gameplay drivers. These are small reference games, not all Task 9 acceptance.

## Validation

- Integrated just check passes: format, strict workspace/all-target Clippy, workspace and separate audio sample checks. just test: 700 passed, zero skipped, including all tests from both branches and the new duplicate-panel regression. All 29 Python tests, workflow actionlint and changed Python/JS syntax checks pass.
- Rebuilt native editor/CLI/player and WASM kiln-web from the merged source; packaged and verified a matching installation. Development archive records preparing HEAD 4628429 with source_dirty=true, not captured clean release provenance. Cargo.lock is unchanged; no dependency added.
- Fresh authoring replay reproduces all serialized project values and exact source bytes for all three games through real widgets. All six Native/Browser exports pass notice/runtime verification and overwrite preservation, with no build tools in child PATH. All native and browser gameplay passes: combat victory, puzzle fresh-process/reload Continue, and both physical platformer levels with saved-position restoration. See docs/evidence/task9-games/merge-validation.json. Incomplete virtual-display startup attempts are excluded; native acceptance reran with Xvfb -noreset and failed-start cleanup.
- Logs/captures stay under target/task9-merged-acceptance, target/task9-merged-native-accepted, target/task9-merged-browser and target/task9-merge-quality-commit.log. Earlier docs/evidence/task9-games/local-validation.json remains the historical pre-master 680-test checkpoint.
- Previous hosted run 43317157963 (source 135c7f1) passed strict gates/tool packaging but failed authoring; gameplay was skipped. Recorded in pre-merge-hosted-failure.json. Authenticated logs/artifacts were unavailable, so no exact failure cause or hosted fix is claimed from it.

## Next / known issues

1. Observe https://github.com/Erion-Simoni/kiln-engine/actions/runs/43404644543 for source f7c8f9d to completion. It is in progress at merge-hosted-checkpoint.json; no completed hosted acceptance is claimed. Local software acceptance is complete. PR #21 remains open; current master is included and the remote source was verified.
2. Another developer must follow the tutorial unaided. Automation is not independent human acceptance. Test long sessions, large projects, recovery and clean exports.
3. Measure actual reference-hardware startup/frame time/memory. No physical i7-6600U/Intel HD 520, audible audio device or controller is attached here.
4. Finish Task 8's full Windows GUI workflow, other physical GPUs, macOS, SDK/dependency/exact-linked-code review, independent artifact/signature/capture review, OS signing and public release preparation. Previous Task 8 a0f1d85 Linux/Windows candidates passed but do not validate Task 9. Local Linux tools require GLIBC 2.39/system libraries; binaries are not OS signed.

## Boundaries

Xvfb/SwiftShader/ALSA-null/Chromium checks are local software evidence, not physical/Windows/macOS/performance proof. api.github.com is excluded by enforced cloud policy; public GitHub observations and managed Git fetch/push work. No credentials were extracted or policies bypassed. No PR merge, release publication or binary Git payload is authorized by this conflict fix. Package size remains informational.
