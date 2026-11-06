# Kiln Session Handoff

**Active focus:** Task 8 started; native export checkpoint completed on 2026-11-06. Task 8 remains IN PROGRESS.
**Branch:** `feat/2d-production-workflow`. Base: `0f89e1a` (Task 7). No Task 9 or full 3D work started.

## Done

- Replaced the Build / Export panel's fake success with Save & Export through the existing project save transaction, then an actual asynchronous CLI process. Scene-tab edits are saved before export; Play must stop first. Fixed the discarded output-directory text field. Exporter/player paths default beside the editor and can be edited under Runtime tools.
- Added nonblocking process polling even with the panel hidden, bounded stdout/stderr diagnostics, actual exit status and validation of a job-specific receipt plus packed project before success. Zero exit with missing/stale output is failure. UI Cancel requests cooperative termination; after three seconds the editor kills/reaps the owned process and removes private staging. Drop/exit also reaps it.
- Added host-native `kiln-cli export --project ... --player ... --output ...` using prebuilt tools; export/play never invoke Cargo. Source manifests normalize to kiln.toml. Project-relative visible resources are snapshotted, shared scene/module/entity scripts/hierarchy/UI and required media are validated, a real KAR is packed and byte-verified, and a matching runtime is copied. Complete output publishes by rename with a receipt. Previous nonempty exports are refused and preserved. Excludes hidden saves/recovery/cache/git and standard build outputs; rejects escaping/symlink resources.
- Added shared protocol/runtime/receipt types in kiln-core and a no-device player `--runtime-info` handshake with host/version/audio/fonts/gamepad checks and a two-second deadline. The player now defaults to assets.kar beside its executable, so exported game/game.exe launches directly after relocation and from another working directory. Explicit --archive remains supported.
- Removed broken source-dependent web/native/mobile CLI export scaffolds from the export command. Unsupported targets and old packaging/signing flags fail explicitly. This checkpoint is host-native only, exercised on Linux x86_64. Browser exporter and packaging/distribution follow; old manual/platform guides do not establish implementation.
- ADR-0059 records the architecture before implementation. No new third-party dependency: CLI adds the existing runtime crate/audio feature for shared script validation/device-free audio decoding and uses existing image/font libraries. Added NATIVE_EXPORT_2D.md and corrected current export/packaging instructions.
- Added real CLI transaction/media/path/cancellation tests, process supervision tests and native editor export/relocation CI acceptance. Selected inspected screenshots/results are in docs/evidence/task8-native/.

## Executed validation

- `just check`: formatting, strict native workspace/all-target Clippy, workspace check and separate audio sample check passed.
- `just test`: **649 passed, zero failed/skipped**. Includes five real CLI transaction tests and three process supervisor tests; previous Task 7 suites remain green.
- Default editor/player/CLI binary builds passed. Minimal editor/player check (`--no-default-features`) and strict WASM-target Clippy passed after the shared-core addition.
- Real editor/Xvfb export smoke passed with an empty tools PATH: inspector X edit remains unsaved until Save & Export, then saved X=88 appears in TOML; edited destination and receipt/file sizes verify. Rebuilding into that existing directory fails and preserves archive bytes. Actual Cancel button cancels a deliberately slow runtime handshake without leaving staging/output.
- The resulting artifact was moved outside source/repository. Direct launch from /tmp, no --archive and no Cargo/rustc on PATH passed real saved menus/fonts, audio commands/settings, health/inventory save, fresh-process Continue, lifecycle/position preservation and corrupt-slot handling. Evidence uses isolated profiles and ALSA null output/software graphics.
- GUI and process evidence is not physical GPU/controller/audible-device, other OS, clean-machine dependencies or remote CI certification. CI was configured, not observed remotely in this workspace. Debug runtime sizes are observations, not release-budget results.

## Next

- Finish Task 8: test a release runtime/export on an actual clean Linux machine without Rust/editor/development dependencies; establish minimum glibc/system-library/GPU requirements and packaging closure. The relocated/no-PATH cloud check is useful but not a replacement.
- Implement a real browser export that includes project resources, matching built WASM/JS, correct run(canvas,project) loading shell, meaningful loading/errors and archive/input/audio-unlock/storage acceptance outside the source tree. Existing browser runtime smoke remains valid; its old CLI HTML scaffold was broken and removed.
- Ship portable editor/exporter/runtime bundles with licenses and verified OS/architecture/runtime metadata; installers/signing and other desktop/mobile target-machine acceptance remain open. Current repository development builds tools once; shipping prebuilt editor bundles is not complete.
- Run physical-device checks on the user's older Intel GPU laptop, real speakers/controller and review remote CI. Only after Task 8 continue Task 9/reference-game release gate.

## Known boundaries

- Export preserves existing nonempty output instead of replacing it. Choose a new/versioned output directory; safe update/installer flows are future work. Cancellation racing final publication may leave a complete new artifact. File/diagnostic limits and exclusions are documented in NATIVE_EXPORT_2D.md.
- The exported native executable still relies on OS graphics/audio/window-system libraries. It does not require Rust; minimum OS/ABI and clean-machine closure remain unverified. Debug binaries are not release size/performance measurements; no budget was waived.
- Native export includes supported 2D project data. The editor-only 3D preview remains isolated and is not exported as 3D gameplay. Browser physics/renderer feature parity, rich UI controls/shaping and full traditional-engine parity remain bounded by previous tasks.
- Task 7 implementation remains complete: shared logical game saves/UI/audio/settings across editor/player/browser. Source/format migration, physical controllers and audible device behavior retain the limits documented in GAME_SAVING_2D.md. Active-world saves use game_* values for persistent progress in other scenes.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Builds use CARGO_INCREMENTAL=0 and debug-info overrides for cloud disk usage. Rootless native test tools live under the workspace sysroot; display/browser sockets and GitHub use supported require_escalated execution. Local displays used isolated profiles; no user projects or real saves were changed.
