# Kiln Session Handoff

**Active focus:** Task 8 browser export checkpoint completed locally on 2026-11-10. Task 8 remains IN PROGRESS for distribution, clean-machine and platform/runtime parity gates.
**Branch:** `feat/2d-production-workflow`. Base: `979a137` (native export). Task 9 and full 3D remain unstarted.

## Done

- Added Browser to the native editor's real Save & Export workflow. Matching prebuilt runtime bundles default beside the editor/exporter and can be selected under Runtime tools. Export preserves relative project resources, normalizes the manifest to project/kiln.toml and publishes a complete staged folder with a job/target-specific receipt. Native player/KAR export remains supported. No Cargo, Node or wasm-bindgen runs during export.
- Added `kiln-cli web-runtime --input DIR` and `just web-runtime` to prepare a trusted generated JS/WASM pair once. WASM carries bounded custom engine/protocol/font metadata; the descriptor records SHA-256 for both files. Missing, changed/mixed, malformed, incompatible or snippet-dependent bundles fail verification. Uses existing SHA-256 and internal core protocol types in kiln-assets; no new third-party dependency. Browser instantiation remains the semantic/module compatibility check.
- Added a relative HTML/JS shell with visible loading/ready/error/reload UI and HTTP guidance. Minimal WASM prepare/start APIs load resources without any gameplay tick/entity Start/audio voice, then resume Web Audio from the user's Play gesture before starting simulation. Consent cannot activate an underlying game button; keyboard repeats not pressed since input installation/blur are ignored until a fresh press. Existing direct `run(canvas, project)` API remains functional.
- Added failure/retry feedback for both the loader and runtime/project assets. Browser slots/settings retain project/origin scoping and survive page reload. Exported output runs at a nested HTTP path using only relocated files; source access and development tools are unnecessary.
- Browser export explicitly rejects authored physics bodies/joints and sprite material overrides because those existing browser host integrations are missing. It does not silently export those projects with altered behavior. Native export remains the supported target for them. General script/platform parity beyond the tested browser contract remains bounded by prior tasks.
- Moved output/large-runtime hash verification off the editor UI thread, keeping polling/cancellation responsive. Receipt validation handles native and browser file sets, rejects invalid/duplicate paths and checks job/target/resource identities. Added cancellation-during-verification regression.
- GUI export tests now clear logs per run so old success/failure messages cannot satisfy current verification waits; software-rendered captures wait for the verified frame and Xvfb startup waits for a real X11 connection.
- Added CLI/bundle tests for hashes/metadata/truncation, raw project paths, escaped titles, prior-output preservation, unsupported browser features and changed runtime failure cleanup. Added actual native-editor Browser export and relocated HTTP browser acceptance; CI now runs them alongside direct browser regressions. ADR-0060 and BROWSER_EXPORT_2D.md record architecture/contract; selected inspected evidence is in docs/evidence/task8-browser/.

## Executed validation

- `just check`: formatting, strict native workspace/all-target Clippy, workspace check and separate audio sample check passed.
- `just test`: **652 passed, zero failed/skipped**. Includes native export/process regressions and new browser bundle/CLI/async-verification cases.
- Default native binaries built; minimal editor/player (`--no-default-features`) check passed. WASM build/bindgen/runtime preparation and strict WASM-target Clippy passed.
- Real editor/Xvfb Browser export with an empty tools PATH passed: save unsaved X=88 edit, honor output field/target, validate receipt/resources, preserve prior output after failure, relocate outside the repository and make source unavailable. New async verification remains supervised when the panel is visible/hidden.
- Chromium served only that artifact at /games/nested/demo/. Passed visible stalled loading, zero audio starts before consent, held-Enter consent, controller API navigation, single touch activation, saved TTF UI, real Web Audio looping/overlap, independent preferences, reload/Continue with health 66/brass_key/start-count 1, quota/corrupt preservation, asset/loader HTTP failure feedback and successful reload retry. No unhandled page errors.
- Native editor export/cancel and relocated native gameplay/save/restart regressions passed. Direct browser movement and UI/audio/storage API regressions also passed after prepared startup/input changes.
- The interrupted native regression launch was retried through normal approval once the usage limit cleared; no review restriction was bypassed. Managed Chromium rejects file:// navigation with ERR_BLOCKED_BY_ADMINISTRATOR. Its guidance branch is therefore unverified; file launching is unsupported and the HTTP path passes. Xvfb/SwiftShader/ALSA null/simulated gamepad evidence is not hardware or other-browser certification. Remote CI has been configured, not observed here.

## Next

- Finish Task 8 portable editor/exporter/native-player/browser-runtime bundles with licenses, verified version/architecture metadata and a repeatable distribution build. Current repository development still prepares tools once; prebuilt distribution delivery is not finished.
- Validate a release native artifact on an actual clean target without Rust/editor/development dependencies; establish minimum glibc/OS/system-library/GPU requirements. The relocated/no-tools cloud check is not a clean-machine replacement.
- Integrate/test browser physics and material host support before allowing those authored projects on Browser. Verify additional browsers and real GPU/controller/audio devices. The user's Intel integrated-GPU laptop is a useful target; it has not been accessed here.
- Review remote CI artifacts. Keep Task 9/reference games/full release gate and full 3D paused until Task 8's remaining acceptance is handled.

## Known boundaries

- Existing nonempty output is preserved; choose a new/versioned directory. Cancellation racing publication can retain a complete new artifact. Read-only verification may finish after cancellation; owned exporter processes are killed/reaped as documented. The portable native executable still depends on OS graphics/audio/window-system libraries.
- Browser export uses the current browser host's supported contract; authored physics/joints/material overrides are refused and physics-query/platform script behavior is not universally certified. Browser data is scoped to origin/profile/project; no service worker/offline or file:// game claim. Prepared bundles with extra JS snippet imports are currently rejected.
- Debug runtime/output sizes are recorded in BUDGETS.md, not release-budget results. No budget was waived, source excluded from checks or warning suppressed. The editor-only 3D preview remains isolated from runtime/project export. Task 7 logical saving/UI/audio/settings remains intact, with its documented migration/device limits.

Source `/workspace/.kiln-tools/activate.sh` for Cargo. Builds use CARGO_INCREMENTAL=0 and debug-info overrides for disk usage. Rootless test tools are in the workspace sysroot. Display/browser sockets and GitHub operations use supported require_escalated execution. Local Chromium uses CHROMIUM_PATH=/usr/bin/chromium; CI installs the matching Playwright browser. Test projects, saves and profiles are disposable.
