# Kiln Session Handoff

**Active focus:** Task 8 hosted release-candidate workflow prepared locally on 2026-11-11. Hosted execution/attestation is pending observation after push. Task 8 remains IN PROGRESS.
**Branch:** `feat/2d-production-workflow`. Base: `51628a6` (repeated release payloads). Task 9/full 3D remain unstarted.
**Owner preference:** package/release size is not a goal. Performance/startup/memory/resource and bounded-input requirements remain; no size ceilings/exceptions (ADR-0062).

## Done

- Tracked the exact root Cargo.lock used by tested release builds (SHA-256 042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17). Nested standalone lockfiles retain ignore behavior. Source capture still supports legacy ignored locks and now handles the tracked root lock exactly once.
- Added release-candidate.yml for selected build/test changes on master/main/feat/2d-production-workflow and manual dispatch. Pins tested Rust 1.99.1, nextest 0.9.151 and official Actions commits; bindgen comes from Cargo.lock. Gates include strict native checks/tests, two fresh captured release builds, actual six-payload comparison, release browser materials/physics, moved installation/native/browser gameplay and clean Ubuntu acceptance.
- Build/test has contents:read only. A separate dependent job downloads accepted files and invokes the official build-origin attestation action with OIDC/attestation permission. It executes no engine code and checks out no repository. Candidate/evidence/bundle are Actions artifacts; this workflow creates no public GitHub Release. ADR-0067 and BUILD_PROVENANCE.md describe scope and verification.
- Official actionlint 1.7.7 archive was checksum-verified into workspace tools and validates the new workflow. Official action tags were resolved to immutable commits; the pinned provenance action exposes the used subject-path and bundle-path interface. The tested Rust version's official manifest is available. No engine Rust source/runtime dependency changed this checkpoint.

## Executed local validation

- `just check` passed: formatting, strict workspace/all-target Clippy and workspace/audio sample checks. `just test`: **665 passed, zero failed/skipped**. **19 Python tests passed**. actionlint and whitespace checks pass.
- Actual source capture verifies exactly one root Cargo.lock with the known release checksum. Parsed workflow preflight confirms read-only build permission, no PR trigger, and the dependent attestation job has no run step or checkout. Hosted completion is not established by these local checks.
- Existing same-host repeatability evidence remains in docs/evidence/task8-repeated-build: two fresh source/target directories, source archive 966223e8534b274670c62bb0d1e139741bd5781ee48fdf6a08f0449aec9914e0, all six editor/CLI/player/JS/WASM/descriptor bytes equal. Whole archives differ due to per-execution records. Original snapshot is dirty on a0e199b; it is not a claim about this new Git HEAD's input identity.
- Those exact payloads already passed release-profile clean Ubuntu native editor/export/failure/cancel/gameplay/save/Continue/corrupt/material tests and real installed-editor Browser export/relocated loading/consent/input/audio/storage checks (docs/evidence/task8-release-build). This workflow is designed to execute the same gates on a hosted runner; it has not yet been observed completing.

## Next

1. Push and observe the hosted candidate workflow for the exact commit. Record actual run outcome and artifact/attestation evidence; fix any failures before claiming hosted acceptance. The system GitHub CLI is old and its API token reports invalid; Git push/public Actions page access work through the managed proxy. No credential was printed or altered.
2. Validate physical Linux/Intel HD 520 GPU/audio/controller, appropriate Windows/macOS/browser/mobile machines and reference startup/frame-time/memory/long sessions. No laptop/device is connected.
3. Resolve internal missing package license declarations with ownership/licensing decisions; establish any remaining SDK/dependency provenance and OS installer signing. GitHub build-origin attestation is not OS signing or a hermetic build certification.
4. Complete Task 8 before Task 9/full 3D.

## Known issues and boundaries

- Local records are unsigned; hosted attestation requires a successful actual workflow and repository Actions/attestation support. Prepared workflow code does not prove remote success. Current local repetition shares host/toolchain/SDK/dependency source cache; no general independent-machine reproducibility guarantee is made.
- Materials remain the documented fixed-vertex/custom-fragment contract with typed static constants/named textures; old placeholder shaders need migration. See SPRITE_MATERIALS_2D.md.
- Software Xvfb/SwiftShader/Mesa/ALSA null/simulated gamepads cannot certify physical GPU/audio/controller performance. Existing runtime evidence is Ubuntu 24.04/X11 with GLIBC 2.39/system-library requirements; other OS/browser/Wayland/older glibc/mobile need their own checks. System libraries are not bundled.
- Internal license omissions remain null; original notices/workspace policy are preserved without inventing grants. No signed installer, public release, physical/reference-hardware or unobserved remote CI result is implied. Generated archives/binaries stay outside Git. Size remains informational.
