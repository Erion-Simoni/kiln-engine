# Kiln Session Handoff

**Active focus:** Task 8 Windows x64 portable candidate accepted and attested; owner-delegated project license resolved. Task 8 remains IN PROGRESS for remaining distribution/platform review. Task 9/full 3D remain unstarted.
**Branch:** feat/2d-production-workflow. Accepted Windows source/workflow: 3ff72d37f2a05641eac78098e5bc4e928ff8581f, pushed and exact remote SHA verified. Later documentation-only commits do not retrigger candidates; use the run's SHA for downloads/verification.
**Owner preference:** package/release size is not a goal. Existing performance/resource and bounded-input requirements remain.

## Done

- Windows run https://github.com/Erion-Simoni/kiln-engine/actions/runs/43215982526 completed successfully in 7m 29s. Both candidate and separate attestation jobs completed; all required gates succeeded with no failed/missing gates.
- Deterministic ZIP packs matched x64 MSVC release editor/CLI/player, default physics/material browser runtime, templates/examples and complete original/project notices. MSVC CRT is statically linked and actual DLL inspection rejects dynamic CRT imports.
- Actual Windows acceptance moved the installation outside the checkout into a spaced path, used unrelated cwd and removed Rust/Node/bindgen from child PATH. Tool/profile/inventory, corruption rejection/restoration, all-template native/browser exports and source-unavailable standalone relocation passed.
- Visible editor and exported-player startup, foreground nonblank client captures, normal zero-status close and final installation verification passed. These are hosted startup/CLI-export gates, not full GUI author/save/undo/restart/export/gameplay or physical-device evidence.
- Candidate/diagnostic uploads, no-checkout origin attestation and attestation-bundle upload passed. Preparation record includes actual compiler/bindgen/target flags, image, lock/archive hashes, DLL observations and acceptance. Compiler outputs may be cached; this is prebuilt preparation, not captured/replayed Windows reproducibility.
- Diagnosed/fixed Windows ZIP normalization, explicit UTF-8 Cargo/tool JSON handling and SYSTEMROOT casing in the screenshot helper. Failure annotations and bounded command/startup/capture diagnostics remain enabled. Successful tool/compiler caches save before later acceptance failures (ADRs 0069–0071).
- The owner delegated license selection. Retained existing MIT OR Apache-2.0, added full root grants, replaced README:TBD and fixed missing inheritance. All 34 workspace package declarations resolve consistently. Packaging includes both project grants and rejects missing grant files/unlicensed path metadata. Third-party/font grants remain unchanged.
- Added Windows download/extract/verify/start instructions, source-pinned origin verification and physical-test guidance. Recorded complete public success and honest failure history under docs/evidence/task8-windows. The plan now distinguishes completed Windows candidate/license checkpoints from remaining full-platform release work.

## Validation

- just check passed: formatting, strict workspace/all-target Clippy, workspace/audio checks. Local Rust suite: 665 passed, zero failures/skips. All 29 Python tests pass, including Unicode under simulated CP1252, Windows ZIP normalization and uppercase environment capture regression. Actionlint 1.7.7 and Bash/permission checks pass.
- Actual Windows native/WASM builds, static-CRT inspection, packaging, installed exports/captures/normal close, publication and origin attestation passed. Source 3ff72d3 is the candidate identity, not later documentation HEAD.
- Actual local Linux prebuilt packaging with licensed metadata verified 1544 files and byte-identical project grant payloads. SHA-256 6eade796b6a095813d20ea0d52bbaa89bdc5aa769dae658ff3e2cb1bab3d4fb6 belongs to that historical local archive; local-validation.json records its true preparing checkout/dirty flag. It is not a Windows hash or fresh captured build.
- Earlier Linux run 43107026471 at 0b9a67b completed all captured-build/replay/six-payload/browser/moved/clean-Ubuntu/upload/attestation gates. Its original inventory lacks the new project grant files. Refreshed Linux run 43215982528 for 3ff72d3 is still in progress at this checkpoint; no complete new Linux candidate success is claimed.
- No Rust source/dependency or root Cargo.lock changed. Lock SHA-256 remains 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17.

## Next / known issues

1. Observe refreshed Linux run 43215982528 to completion and record its actual result. Superseded intermediate Linux runs were cancelled by later fixes; do not confuse cancellation with a newly diagnosed engine regression.
2. Automate matched runtime project/dependency notice propagation into game exports. For now, copy the complete installation licenses directory beside native/browser output and retain project asset notices; that manual addition is outside the export receipt.
3. Execute full Windows GUI author/save/undo/restart/export/gameplay and PHYSICAL_DEVICE_TEST.md on actual Intel HD 520/other devices. Verify actual GPU/driver, audible output/controller and startup/frame-time/memory/session/recovery. No laptop/device is connected here.
4. Finish macOS distribution, remaining SDK/dependency/exact-linked-code review and OS installer signing. Executables are not Authenticode-signed. Windows origin attestation is separate from OS signing, clean-machine/hardware certification and hermetic/independent-builder reproducibility.
5. Independently download/verify candidate signatures and review retained captures on an appropriately connected machine. api.github.com is still excluded by enforced cloud policy; the earlier network draft is unapplied. Public status/Git push work, but candidate contents/signatures/PNGs were not independently downloaded/reviewed here. Use the documented source/signer SHA constraints with current authenticated gh.
6. Candidate/attestation artifacts retain 14 days, diagnostics seven. No public GitHub Release was created. Complete Task 8 before Task 9/full 3D.

## Boundaries

- Build/test remains contents:read; only the separate no-engine/no-checkout attester obtains OIDC/attestation write permission. Errors stay strict. No release-size ceiling, unrelated artifact deletion, credential extraction, network-policy bypass or runtime dependency.
- Windows 2025/DX12 virtual-desktop acceptance is not older-Windows/physical-device/full-GUI proof. Linux software evidence uses Ubuntu 24.04/X11/GLIBC 2.39 and virtual/software devices, not physical acceptance. System drivers/libraries are not bundled.
- Historical artifacts retain original metadata and contents. New project grants are not retroactively claimed inside old archives. Generated archives/logs/captures remain outside Git. Package size remains informational.
