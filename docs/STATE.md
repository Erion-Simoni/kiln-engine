# Kiln Session Handoff

**Active focus:** Task 8 Windows x64 portable candidate implementation prepared; first hosted Windows execution still pending. Task 8 remains IN PROGRESS. Task 9/full 3D remain unstarted.
**Branch:** feat/2d-production-workflow. Prior accepted Linux run: 43107026471, source 0b9a67b; overall success includes candidate upload and origin attestation. Session starts after documentation commit 70a0f3a.
**Owner preference:** package/release size is not a goal. Existing performance/resource and bounded-input requirements remain.

## Done

- ADR-0069 adds native Windows preparation alongside the existing captured Linux driver, without broadening that driver's replay/provenance claims.
- Extended prebuilt packaging with deterministic Windows ZIP, exclusive output publication and rejection of linked/special inputs. Windows manifests classify .exe tools explicitly; Windows README uses native commands and omits Linux dependency instructions. Existing Linux tar format remains available.
- Added Windows ZIP extraction regressions for exact payload preservation, existing-output refusal, traversal, case aliases and links. All 25 Python preparation/build/comparison/archive tests pass locally.
- Added Windows installed acceptance: move outside checkout into a path with spaces, remove Rust/Node/bindgen from child PATH, validate tool identities/inventory and corruption rejection, create all templates and export Windows/web, move a standalone game with original project path unavailable, and require visible editor/player windows, nonblank captures and normal zero-status close. This script must execute on Windows; its GUI results are not established locally.
- Added pinned Windows 2025 workflow: Rust 1.99.1, Python 3.13, lockfile-matched bindgen, static MSVC CRT, DLL import inspection, release tools/browser builds, original notices, installed acceptance and strict artifact publication. A separate no-checkout job attests ZIP/preparation record. Build remains contents:read; only the attester gains OIDC/attestation write permission.
- WINDOWS_PORTABLE.md describes extraction, integrity/origin verification, runtime requirements and exact acceptance scope. Hosted results remain explicitly pending.

## Validation

- just check passed: format, strict workspace/all-target Clippy, workspace/audio sample checks. Actionlint 1.7.7 accepts the new workflow. Bash snippets and parsed signing permissions pass local validation. Official setup-python v7 action metadata uses native Node 24.
- Actual local Linux packaging with the changed producer passed the existing installation verifier for all 1542 files. Archive target/task8-windows-session-linux-regression.tar.gz has SHA-256 76a598c1af3ffb3fb4adddc626c313b33d5ca655eef3d3ef796afe89126cd20c. This uses existing release binaries/prebuilt preparation, not a new captured Linux build.
- All 25 Python tests pass. Rust suite rerun passed 665 tests with zero failures/skips. No Rust source/dependency or Cargo.lock changed.
- Windows build/DLL/startup/signing gates are not yet observed. Local tests must not be reported as a Windows pass.

## Next / known issues

1. Push this tested checkpoint to the existing branch, verify remote SHA and observe Windows candidate workflow. Fix actual hosted compile/package/desktop failures, then record observed acceptance and upload/attestation. The Linux candidate workflow also triggers because shared packaging changed.
2. Windows acceptance currently covers CLI export and startup, not full graphical author/save/undo/export/gameplay or physical GPU/audio/controller behavior. Windows ZIPs remain unsigned by Authenticode. Captures/color diversity are startup evidence, not complete semantic UI proof.
3. The Linux captured source/replay builder remains Linux-only. Windows uses preparation-observations-only inventory plus its hosted preparation record and separate origin attestations; no Windows captured replay/equality claim is made.
4. Project licensing remains unresolved: README:TBD versus workspace MIT OR Apache-2.0, eight engine crates and three samples lack license metadata, and root project license texts are absent. The prior owner policy question has no submitted answer; no grant is invented.
5. Physical/reference-machine, macOS/full Windows authoring, remaining SDK/dependency provenance and OS signing keep Task 8 open. No laptop is connected. api.github.com remains excluded by enforced cloud policy; its older network draft is unapplied. Public status/Git push work; no credentials or policy bypass are used.

## Boundaries

- No release-size ceiling, runtime dependency, public GitHub Release, credential extraction or unrelated milestone. Generated archives/logs/screenshots remain outside Git.
- Earlier Linux software evidence is Ubuntu 24.04/X11/GLIBC 2.39 with virtual/software devices, not physical acceptance. Independent downloaded-signature verification is separate from observing official CI attestation steps.
