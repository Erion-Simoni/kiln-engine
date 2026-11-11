# Kiln Session Handoff

**Active focus:** Task 8 artifact publication repair after the owner supplied run #1's exact error. Task 8 remains IN PROGRESS. Task 9/full 3D remain unstarted.
**Branch:** `feat/2d-production-workflow`. Workflow repair commit `0b9a67b21480a654e8d6fcc886e702dd1d2b1e33` is pushed and exact remote SHA verified. A documentation-only evidence checkpoint follows; it does not retrigger the build.
**Owner preference:** package/release size is not a goal. Existing performance/startup/memory/resource and bounded-input requirements remain (ADR-0062).

## Done

- Run 43088213774 at source 462d17f passed hosted native quality/tests, two fresh locked/offline release builds, actual six-payload comparison, browser material/physics, moved editor/export/gameplay and clean Ubuntu acceptance. Candidate staging passed. Both artifact uploads failed; attestation was not reached.
- The owner's screenshot now supplies the exact annotations: `Failed to CreateArtifact: Received non-retryable error: Failed request: (404) Not Found: workflow run not found`, twice. It also reports old Node 20 actions being forced onto Node 24. This does not establish quota exhaustion, expired tokens or runtime-migration causation.
- ADR-0068 updates the release workflow's immutable official pins: checkout v7, cache v6, setup-node v7, upload-artifact v7, download-artifact v8 and attest-build-provenance v4. JavaScript action metadata uses native Node 24; attestation remains a separate official composite action. Existing multi-file zipped uploads/retention and strict errors are preserved.
- Added a five-minute read-only artifact-preflight job before engine builds. It uploads a run/attempt/repository/source identity record, downloads it, and compares actual bytes. Failure stops expensive builds early; passing is not proof that later candidate upload/attestation passed. Probe retention is one day, with unique run/attempt identity.
- Cargo tool cache now includes .crates.toml and .crates2.json with executable files, and uses a new key. This preserves cargo install version tracking on restored runners.
- Updated BUILD_PROVENANCE, the 2D completion plan and historical hosted evidence with the screenshot diagnostics and precise remediation scope. Root Cargo.lock remains unchanged: SHA-256 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17, 649 packages.

## Validation

- `just check` passed: formatting, strict workspace/all-target Clippy, workspace and standalone audio sample checks.
- 19 Python producer/replay/comparison tests passed. Official checksum-verified actionlint 1.7.7 accepts the workflow.
- Executed the actual YAML preflight shell scripts locally: matching downloaded bytes pass; altered bytes and missing downloads fail. Parsed workflow checks confirm the candidate depends on preflight, build permissions remain contents:read and only the separate attester obtains OIDC/attestation write permission.
- No engine Rust source changed. Reran `just test`: **665 passed, zero failed/skipped**.
- New hosted run https://github.com/Erion-Simoni/kiln-engine/actions/runs/43107026471 at 0b9a67b: the actual artifact-service preflight **passed**, including upload, download and byte comparison. The candidate build job started and is in progress at this checkpoint. Candidate publication and attestation have not yet been observed. `docs/evidence/task8-hosted-ci/artifact-preflight-run2.json` records public step conclusions, not downloaded candidate contents or a complete-workflow success claim.

## Access and remaining work

1. Observe run 43107026471 through candidate publication and separate attestation. The new early upload/download already passed, but that alone does not establish long-job publication success or prove the original 404 cause. If a later upload fails, obtain the exact step annotation/log; keep errors strict and do not assume quota or token expiry.
2. api.github.com remains outside this environment's enforced network policy. The earlier network-only settings draft requires owner review/save/publish and is not active. Public GitHub status pages and Git push work; anonymous detailed logs are unavailable. No credential values are requested or extracted. The screenshot provides actionable error text without waiting for API access.
3. Resolve README License:TBD versus workspace MIT OR Apache-2.0; eight internal crates omit license metadata. The earlier owner question has no submitted answer, so no licensing grant is invented.
4. Execute PHYSICAL_DEVICE_TEST.md on actual Intel HD 520/other physical machines and required platforms/browsers. No laptop/device is connected here. Finish licensing, remaining SDK/dependency provenance and OS installer signing as appropriate before calling Task 8 complete.

## Boundaries

- The builder remains read-only. The separate attester checks out/executes no engine code. No public GitHub Release, unrelated artifact deletion, permission expansion or error masking is introduced.
- Prior local repetition uses the same host/toolchain/dependency cache; it is not independent-builder or hermetic certification. Build-origin attestations are separate from OS signing/licensing/physical compatibility.
- Software acceptance uses Ubuntu 24.04/X11/GLIBC 2.39, Xvfb/SwiftShader/Mesa/ALSA null/simulated gamepads. It cannot certify physical GPU/audio/controller performance or other OS/browser/Wayland/older glibc/mobile. System libraries are not bundled.
- License policy is unchanged. Generated archives/binaries remain outside Git. Package size remains informational. No unobserved hosted upload, attestation or physical result is implied.
