# Kiln Session Handoff

**Active focus:** Task 8 runtime-matched export notices. Task 8 remains IN PROGRESS for remaining distribution/platform review; Task 9/full 3D remain unstarted.
**Branch:** feat/2d-production-workflow. Runtime notice feature is validated locally and ready for the feature commit/push; hosted feature results remain separate. Prior accepted licensed Linux/Windows candidate source is 3ff72d37f2a05641eac78098e5bc4e928ff8581f; later documentation HEAD was 358df2a.
**Owner preference:** package/release size is not a goal. Performance/resource and bounded-input requirements remain. Owner-delegated project license is MIT OR Apache-2.0; no further licensing decision is pending.

## Done

- Added runtime notice propagation from the actual selected native player/browser runtime's verified portable installation. Complete original project/dependency/font/Rust notice tree is copied, including verified MPL sources. Runtime payloads must match that installation's hashes.
- New licenses/runtime-notices.json hashes each notice and selected native player or three browser runtime files. Optional export receipt SHA-256 binds this manifest. CLI and editor background verification reject changed/missing/extra/unreceipted notice files, invalid paths, linked parents, malformed coverage and runtime mismatches before success. No dependency or execution protocol change (ADR-0072).
- Loose developer runtimes without an installation inventory remain supported with an explicit missing-notices warning; they have no verified notice claim. Damaged/incomplete portable installations fail without publishing output; existing export staging/cancellation remains authoritative. Working projects belong outside the immutable installation.
- Added seven integrity tests and updated Linux/Windows portable acceptance to compare every notice/runtime byte against the selected installation for all templates. Real corruption export must fail without output/staging. GUI smoke publication waits remain bounded while allowing the new complete inventory hashing.
- Refreshed licensed Linux run 43215982528 at 3ff72d3 completed successfully in 45m 2s: all three jobs/required gates passed. Fresh captured build, frozen replay, actual six-payload equality, browser materials/physics, moved installation/export/gameplay, clean Ubuntu without source/Rust, uploads and separate origin attestation passed. Evidence records this prior candidate separately: it predates automatic notice copying.
- Prior Windows run 43215982526 at 3ff72d3 passed in 7m 29s: static MSVC CRT/DLL inspection, matched ZIP/notices, moved installation, all-template CLI native/browser exports with no build tools, visible editor/player startup/capture/normal close, uploads and separate origin attestation. Full Windows GUI authoring/gameplay and physical devices were not exercised.
- Documentation now describes automatic current-source exports versus historical candidate manual notice copying, updates the accepted licensed Linux checkpoint and retains platform/provenance limitations.

## Validation

- just check passed: formatting, strict workspace/all-target Clippy, workspace and audio sample checks. just test: 672 passed, zero skipped. Seven new notice integrity tests and existing seven real CLI export tests pass. All 29 Python tests, both candidate workflows' actionlint and changed Python syntax checks pass. WASM kiln-web check passes.
- Rebuilt actual development editor/CLI/player, prepared a local portable archive with compatible existing browser runtime and verified 1546 installation files. It contains 1514 original notice files (7846211 bytes). All three templates export native/browser with matching notice manifests/receipts; damaged-install export rejects publication and cleans staging. Full local GUI Save & Export, saved X=88, overwrite failure preservation, native UI Cancel, source-unavailable relocation and native/browser gameplay/save/resume all pass. Final installation verification passes; 1514 original notice files are byte-preserved in every export.
- First sandboxed local GUI attempt could not bind Xvfb sockets; no existing display occupied the paths. The complete rerun outside the display restriction passed. This is environment access evidence, not an editor regression.
- Cargo.lock unchanged: SHA-256 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17. Local archive source observation is preparing HEAD 358df2a with source_dirty=true, not a captured clean release or hosted candidate.

## Next / known issues

1. Commit/push the validated feature and verify exact remote SHA. Observe its new Windows/Linux workflows; prior 3ff72d3 successes do not validate new source. Local evidence is docs/evidence/task8-runtime-notices/local-validation.json.
2. Execute full Windows GUI author/save/undo/restart/export/gameplay and PHYSICAL_DEVICE_TEST.md on actual Intel HD 520/other devices. Check actual GPU/driver, audible output/controller and startup/frame-time/memory/session/recovery. No physical laptop/device is connected here.
3. Finish macOS distribution, remaining SDK/dependency/exact-linked-code review and OS installer signing. Windows binaries are not Authenticode-signed. Origin attestation is separate from OS signing and hermetic/independent-builder reproducibility.
4. Independently download/verify candidate signatures and review retained captures on an appropriately connected machine. api.github.com remains excluded by enforced cloud policy; the earlier network draft is unapplied. Public status/Git push work; signatures/PNGs were not independently downloaded/reviewed here. Use documented source/signer SHA constraints with current authenticated gh.
5. Candidate/attestation artifacts retain 14 days, diagnostics seven. No public GitHub Release was created. Complete Task 8 before Task 9/full 3D.

## Boundaries

Build/test remains contents:read; only separate no-engine/no-checkout attesters receive OIDC/attestation write permission. No credential extraction, policy bypass, unrelated artifact deletion or runtime dependency. Windows 2025/DX12 startup/CLI export is not full GUI/older-Windows/device proof. Linux software acceptance uses Ubuntu/X11/SwiftShader/ALSA null, not physical devices. Linux release candidate needs GLIBC 2.39 and documented system libraries; drivers/libraries are not bundled. Checksums verify integrity, not publisher authenticity or legal completeness. Historical archives retain original contents; generated binaries/logs/captures stay outside Git. Package size remains informational.
