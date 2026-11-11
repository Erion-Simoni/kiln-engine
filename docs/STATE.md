# Kiln Session Handoff

**Active focus:** Task 8 hosted engine acceptance observed on 2026-11-11; artifact publication failed and attestation was not reached. Exact upload error still needs diagnostics. Task 8 remains IN PROGRESS.
**Branch:** `feat/2d-production-workflow`. Workflow/source commit: `462d17f5cbede3af7993e260281215db8a0d65c3`, pushed and remote-verified. Final documentation/evidence commit follows it. Task 9/full 3D remain unstarted.
**Owner preference:** package/release size is not a goal. Performance/startup/memory/resource and bounded-input requirements remain; no size ceilings/exceptions (ADR-0062).

## Done

- Tracked the root Cargo.lock used by tested release builds: SHA-256 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17, 649 locked packages. Nested standalone lockfiles retain ignore behavior. Actual source capture confirms one root lock entry with that checksum, supporting current tracked and older ignored snapshots.
- Added release-candidate.yml on selected build/test changes for master/main/feat/2d-production-workflow and manual dispatch. Pins Rust 1.99.1, nextest 0.9.151 and official Actions commits; bindgen comes from Cargo.lock. Includes native quality/tests, two fresh locked/offline release builds, six-payload comparison, browser material/physics, moved installation/export/gameplay and clean Ubuntu acceptance.
- Build/test has contents:read only. The dependent attestation job downloads accepted files and invokes the official build-origin action with OIDC/attestation permission, without checking out/executing engine code. Artifacts and attestation are intended Actions outputs; no public GitHub Release is created. ADR-0067 and BUILD_PROVENANCE.md describe scope and verification.
- Executed GitHub run #1: https://github.com/Erion-Simoni/kiln-engine/actions/runs/43088213774. Hosted native quality/tests, first release build, fresh frozen rebuild, actual six-payload equality, browser material/physics, moved editor/native/browser games and clean Ubuntu all reported success. Candidate staging succeeded too.
- Added PHYSICAL_DEVICE_TEST.md: actual desktop/device identification, author/export/save/restart, material/physics examples, audible output/controller, browser checks and honest measurement/result recording for Intel HD 520 or other machines. This is a manual checklist, not new hardware evidence.

## Executed validation and failure

- Local `just check` passed: formatting, strict workspace/all-target Clippy and workspace/audio sample checks. `just test`: **665 passed, zero failed/skipped**. **19 Python tests passed**. Official checksum-verified actionlint 1.7.7 validates the workflow. Local parsed-permission and real source-capture preflight pass.
- Public hosted job step conclusions establish the successful engine gates above. The private/authenticated detailed logs and accepted artifact contents were not downloaded. No precise remote test-count or artifact digest claim is inferred from the public step label alone. Prior actual local release/repetition evidence remains in task8-release-build / task8-repeated-build.
- **Upload accepted candidate** and **Retain acceptance and failure evidence** reported failure. The dependent attestation stage was not reached. Root cause is unknown; do not guess quota/storage/permissions from labels. Public status snapshot was still in_progress at the observed failure, and later refreshed run pages intermittently returned 502. No complete workflow success or signed artifact is claimed.
- Saved local preflight, public hosted step/timestamp observation and diagnostics blocker under docs/evidence/task8-hosted-ci. The evidence belongs to source/workflow revision 462d17f, while this final checkpoint also adds documentation only. Existing source/lock/workflow changes were already pushed and exact remote SHA checked.

## Blocker and configuration draft

- This selected cloud environment's enforced revision 1 excludes api.github.com; the public check-annotation request received CONNECT 403. Anonymous GitHub pages expose conclusions but detailed log requests returned 404. The system gh CLI reports invalid API authentication and lacks the attestation command; Git push/public status access work. Credential values were neither printed nor requested.
- A restricted-network settings draft adding only api.github.com was saved successfully, requires_publish=true. It is **not applied**; refreshed runtime status still reports revision 1, no custom hosts and no API host. The owner needs to review/save/publish through environment settings before relying on this access. Then refresh status and make a fresh connection.
- An async question also requests the exact error from Upload accepted candidate in run #1, which can supply diagnostics without waiting for API access. No answer has arrived. Use the provided error or approved access to repair the publication cause and observe a successful upload/attestation.

## Next

1. Obtain actual upload diagnostics, fix the cause, then verify publication and attestation; never treat the current run as fully green. Root workflow steps remain strict and report publication failures.
2. Resolve README License:TBD versus workspace MIT OR Apache-2.0. Eight internal crates omit package license metadata. The owner was asked whether to use the existing dual license for all Kiln crates or keep licensing undecided; no submitted answer has arrived, so no grant/metadata policy was invented.
3. Execute PHYSICAL_DEVICE_TEST.md on physical Linux/Intel HD 520 and appropriate Windows/macOS/browser/mobile machines; collect actual startup/frame-time/memory/session results. No laptop/device is connected here.
4. Complete licensing, any remaining SDK/dependency provenance and OS installer signing. GitHub build-origin attestation is separate from OS signing/hermetic certification. Complete Task 8 before Task 9/full 3D.

## Boundaries

- Source/Rust/runtime dependencies did not change beyond tracking the existing lockfile and a source-capture comment. Documentation-only follow-up does not retrigger the expensive candidate workflow, and does not claim a new artifact for the final documentation SHA.
- Local records are unsigned. Same-host repeated builds share toolchain/SDK/dependency source cache; physical/independent-machine/general reproducibility is not established. Materials retain the documented fixed-vertex/custom-fragment/static-parameter contract.
- Existing software evidence uses Ubuntu 24.04/X11/GLIBC 2.39, Xvfb/SwiftShader/Mesa/ALSA null/simulated gamepads; it cannot certify physical GPU/audio/controller performance or other OS/browser/Wayland/older glibc/mobile. System libraries are not bundled.
- Original notices/workspace license policy remain preserved. Generated archives/binaries stay outside Git. No signed installer, public release, unobserved CI attestation or physical/reference-hardware result is implied. Package size remains informational.
