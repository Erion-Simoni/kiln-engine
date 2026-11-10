# Kiln Session Handoff

**Active focus:** Task 8 clean Ubuntu runtime acceptance completed locally on 2026-11-10. Task 8 remains IN PROGRESS for physical-machine/other-OS, release provenance and browser physics/material parity.
**Branch:** `feat/2d-production-workflow`. Base: `37dc96e` (portable development editor). Task 9 and full 3D remain unstarted.
**Owner preference:** package/release size is not a goal. Editor/player/WASM/mobile size ceilings and completion gates are removed (ADR-0062). Size observations are optional; do not spend sessions chasing size thresholds or ask for size exceptions. Performance, startup, memory/resource and bounded-input requirements remain.

## Done

- Retired package-size goals from active budgets/roadmaps/agent guidance/README, export/distribution documentation and commands. `just size` builds optional artifacts and reports existing sizes via `tools/report_sizes.py`; it has no threshold or size-based pass/fail. Historical ADR/evidence measurements remain superseded by the new owner-requested policy. No tests or runtime performance/resource checks were weakened.
- Added a repeatable clean Ubuntu 24.04 runtime image/runner: only the prepared archive (read-only) and evidence directory are mounted; no repository, development sysroot or Rust/Cargo/compiler/Git is available. Container runs unprivileged with network disabled, read-only root, temporary work/home and no shared host display/GPU devices. Sanitized build context copies only four native test scripts, the Dockerfile and runtime requirements. Docker/Python/test drivers are infrastructure, not new engine dependencies.
- Clean startup exposed a real missing runtime-loaded X11 keyboard library: `libxkbcommon-x11.so could not be loaded` panicked inside winit despite successful native ldd resolution. Added shared `packaging/linux-runtime-packages.txt` including `libxkbcommon-x11-0`; the producer ships it with Ubuntu instructions and the test image installs that same list. Acceptance asserts shipped/tested lists match. CI GUI setup also installs the discovered dependency.
- Final container run passes installation identity/hash/mode/browser-descriptor verification and native ldd resolution, real installed-editor open/author/save/native export with empty child-tool PATH, failure preservation and UI Cancel. The game relocates with its authoring source copy unavailable, launches without archive arguments, and passes menus/settings/gameplay/inventory/save/fresh-process Continue/corrupt-slot preservation. Installed files still verify after use. Actual export/resumed-game captures were inspected.
- Added actual runtime package/image/archive identities and selected machine-readable/UI evidence in docs/evidence/task8-clean-linux. Rewrote CLEAN_MACHINE_TEST.md around supported host-target contracts; updated PORTABLE_EDITOR.md, plan, packaging/native/browser docs and budgets. ADR-0062 records policy and test architecture.
- CI now pins the portable artifact-producing job to Ubuntu 24.04 to match the tested GLIBC baseline, runs clean-userspace acceptance and uploads its evidence. Remote execution has not been observed.
- Existing matched editor/CLI/player/browser bundles, templates, dependency notices/MPL sources, asynchronous native/browser Save & Export and relocated browser loading/consent/input/audio/storage acceptance remain from earlier checkpoints. Browser still rejects authored physics/joints and sprite material overrides pending integration.

## Executed validation

- `just check` passed: formatting, strict workspace/all-target Clippy, workspace check and separate audio sample check.
- `just test`: **657 passed, zero failed/skipped**. Four Python producer regressions passed after packaging changes. Python syntax checks and optional no-threshold size report passed.
- New bundle prepared/verified with matching tools, shared Linux requirements, original notices and prepared browser runtime: `target/dist/kiln-task8-ubuntu-runtime.tar.gz`, SHA-256 `df258efc4c0dd6887101c22f337964f1283e7e7965c096f67b205030919f9cbb`. Size is informational; generated archives/binaries are not committed.
- `python3 tools/test_clean_linux.py target/dist/kiln-task8-ubuntu-runtime.tar.gz` passed through normal local-Docker approval. Base Ubuntu digest and image ID are in evidence/container-info.json; libc6 2.39-0ubuntu8.9 and all installed package versions are retained. UID 1000, no Rust/Cargo/compiler/Git, offline, no repository/sysroot mounts, read-only root. No TLS/proxy restriction was bypassed.
- Real editor saved Player X=88 during export, preserved old output on failed rebuild and cancelled an active job. Relocated native game retained health 66/inventory/settings, script Start count 1 and movement across restart/Continue; corrupt-slot load preserved the broken slot and live state. Xvfb startup now waits for an actual X11 connection in the gameplay helper, avoiding socket-existence races.
- This container run is native only. It verifies the browser descriptor but does not rerun browser gameplay or WASM compilation; the prior full relocated browser acceptance remains evidence for that unchanged runtime.

## Next

1. Task 8: validate physical Linux driver/device paths and appropriate Windows/macOS machines, including Intel HD 520 if available. Current support evidence is Linux x86_64 Ubuntu 24.04/X11 software graphics, not all distributions or physical machines.
2. Establish exact build provenance and resolve internal package license declarations before publishing release installers; packaging observations are not a binary/source attestation. Prepare release-profile compatibility/performance evidence where needed, with no package-size target.
3. Integrate/test browser physics/joints and sprite material parity; retain explicit export rejection until it works. Continue startup/frame-time/memory and actual browser/device validation where appropriate.
4. Finish remaining Task 8 acceptance before starting unasked Task 9 or full 3D implementation.

## Known issues and boundaries

- Containers share the host kernel. Xvfb/Mesa software graphics, ALSA null and simulated controller APIs do not prove physical Intel HD 520 performance, speakers/controllers, Wayland, other Linux distributions or Windows/macOS. No laptop/device was connected.
- Tested Ubuntu package list includes dynamic X11 support; exact image includes native GUI test drivers and their transitive libraries too. ldd alone cannot prove dynamic plugin closure. Executed paths pass, but this is not a universal/minimal dependency closure for every desktop/driver/plugin path. System libraries are not bundled. Binaries reference GLIBC 2.39 and need a compatible userspace; older baselines require a rebuild/test there.
- Package sizes are informational and never block completion. Technical archive/file-count/input bounds remain to keep parsing/copying resource use bounded.
- Some internal package manifests lack package license declarations. Workspace policy and original declarations/notices are preserved without inventing grants. Source revision/dirty/compiler fields are preparation observations; release licensing and exact/reproducible build provenance remain open.
- Browser export rejects physics/joints and sprite material overrides. Other browser engines/mobile/real controllers remain unverified. HTTP deployment passes; file:// is unsupported and was previously blocked by administrator policy in managed Chromium.
- Remote CI, signed installers/release uploads and startup/frame-time/memory on reference physical hardware have not been observed. Local software acceptance does not certify those gates.
