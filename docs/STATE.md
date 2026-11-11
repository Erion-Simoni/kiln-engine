# Kiln Session Handoff

**Active focus:** Task 8 hosted Linux candidate publication and build-origin attestation now pass. Task 8 remains IN PROGRESS for release review and physical/other-platform distribution. Task 9/full 3D remain unstarted.
**Branch:** `feat/2d-production-workflow`. Accepted candidate source/workflow: `0b9a67b21480a654e8d6fcc886e702dd1d2b1e33`. Later documentation-only commits do not retrigger the candidate workflow; use the run's SHA for artifact lookup/verification.
**Owner preference:** package/release size is not a goal. Performance/startup/memory/resource and bounded-input requirements remain (ADR-0062).

## Done

- Observed run #2 through completion: https://github.com/Erion-Simoni/kiln-engine/actions/runs/43107026471. Overall GitHub status is completed/success, duration 47m 9s. All three jobs completed; all required gates succeeded, with no failed/missing gates.
- Native quality/tests, first captured locked/offline release build, fresh frozen rebuild and actual six-payload equality passed. Release browser material/physics, moved installation/editor/export/gameplay and clean Ubuntu acceptance passed.
- Accepted candidate and diagnostics uploaded successfully. The separate job downloaded the candidate, created GitHub build-origin attestation and uploaded its bundle. This closes the observed hosted publication/attestation execution gate for source 0b9a67b.
- Saved complete public job step conclusions/timestamps and overall success in docs/evidence/task8-hosted-ci/hosted-run2-complete.json. Earlier run #1 and run #2's early preflight observations remain historical records. This is public status evidence, not independent downloaded-signature verification.
- Added a concrete completed-run download/verification recipe: select a successful run, use its SHA, refuse an existing output directory, download its candidate, verify all five files with repository/workflow/source/signer constraints, extract, verify the installation and launch the editor. Linked it from README, GETTING_STARTED, PORTABLE_EDITOR and PHYSICAL_DEVICE_TEST.
- Updated the 2D plan with a completed hosted Linux candidate publication/attestation item and a separate remaining public-release/platform item. Overall Task 8 is not declared finished.
- Audited current locked workspace metadata: eight engine crates (kiln-ai/assets/cli/gameplay/graph/ir/platform/script) and animation/ai/gameplay sample packages lack license values. README says TBD, workspace says MIT OR Apache-2.0, and root project license texts are absent. Documented exact gaps; no licensing policy/grant changed.

## Validation

- This session: just check passed (formatting, strict workspace/all-target Clippy, workspace and standalone audio sample checks). Download recipe passes Bash syntax checking; verification flags match current official GitHub CLI source, and the actual local release archive root matches the documented extraction path.
- Actual hosted native tests, replay/comparison, browser, moved installation, clean Ubuntu, upload and attestation gates all passed. Public step labels do not establish a precise hosted test count or candidate digest, so none is inferred.
- Prior current-code local suite: 665 Rust tests passed, zero failures/skips; 19 Python tests passed. No engine source, workflow, runtime dependency or Cargo.lock changed this session. Root lock SHA-256 remains 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17 (649 packages).
- The artifact preflight and end-of-job candidate upload both passed after ADR-0068's native Node 24 action update. Run #1's exact error was CreateArtifact HTTP 404, workflow run not found; successful remediation does not prove quota/token-expiry/runtime causation.

## Next / known issues

1. Independently download and verify candidate signatures with current authenticated gh on an appropriately connected machine. This environment's enforced revision 1 excludes api.github.com; the earlier network-only settings draft is not applied. Public status/Git push work, but authenticated detailed logs and candidate contents were not fetched. No credential values are requested/extracted or policy bypassed. Consumer instructions check all five signed files against the selected run's source SHA.
2. Resolve project licensing before calling this a reviewed public release. The owner was previously asked whether to use existing MIT OR Apache-2.0 or keep licensing undecided; no submitted answer exists. Preserve existing declarations/notices until policy is resolved, then add approved root license texts and missing package metadata.
3. Execute PHYSICAL_DEVICE_TEST.md on actual Intel HD 520/other machines, including GPU selection, audible output/controller and startup/frame-time/memory/session/recovery measurements. No laptop/device is connected here.
4. Finish matching Windows/macOS distribution and appropriate platform tests, remaining SDK/dependency provenance and OS installer signing. GitHub origin attestation is separate from OS signing, physical compatibility and hermetic/independent-builder certification.
5. Complete Task 8 before starting Task 9/full 3D. Candidate/attestation Actions artifacts retain 14 days, diagnostics 7 days; use a later successful candidate run after expiry. No public GitHub Release was created.

## Boundaries

- Build/test remains contents:read; only the separate no-checkout/no-engine-execution attester has OIDC/attestation write permission. Upload errors remain strict. No permission expansion, unrelated artifact deletion or release-size ceiling.
- Local build records remain unsigned execution records; the successful hosted job created separate origin attestations for its specific candidate. Those signatures were not independently verified here. Same-host repetition shares toolchain/SDK/dependency caches.
- Acceptance uses Ubuntu 24.04/X11/GLIBC 2.39, Xvfb/SwiftShader/Mesa/ALSA null/simulated gamepads. It does not certify physical GPU/audio/controller performance, Windows/macOS/Wayland/older glibc/mobile. System libraries are not bundled.
- Original notices/license policy are preserved. Generated archives/binaries remain outside Git. Package size remains informational.
