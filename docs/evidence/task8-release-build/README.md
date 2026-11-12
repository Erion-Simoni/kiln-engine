# Task 8 captured release candidate — 2026-11-10

The new build driver performed a fresh native and default-WASM `--release`
build with offline/locked Cargo, then prepared and verified the portable archive.
All three installed native identities report debug_assertions=false. Browser
runtime advertises fonts/physics/materials. This is local execution/software
acceptance, not signed attestation, repeated-build equality or physical hardware
certification. Package size has no threshold.

## Build identity

- Archive: target/task8-release-build/kiln-portable.tar.gz; SHA-256
  `d5ca032d975c5761a267c24c60d03b97efad1d019e46915b89baab5c4ce15b4a`.
- Source archive: target/task8-release-build/source.tar.gz; SHA-256
  `966223e8534b274670c62bb0d1e139741bd5781ee48fdf6a08f0449aec9914e0`.
- Build record SHA-256:
  `0044c2a27117602a5c416010964a345110a2948e5e99145e9b5584f18f6e92eb`.
  Full record retained here; original logs/source/archive remain under target.
- Source snapshot: 540 files plus their content/mode inventory, base a0e199b,
  dirty=true. Captured before final documentation/test-harness updates and minor
  build-driver input guards; the record's builder hash matches its captured
  source. No engine Rust source changed in this checkpoint. The captured archive,
  not a claim about the final Git HEAD, identifies the exact build inputs.
- Cargo.lock SHA-256:
  `042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17`.
  Compiler/Cargo/bindgen versions, commands, log hashes and six native/browser
  artifact hashes are in build-record.json. Archive verifies 1543 listed files
  plus bundle.json, includes the checked record and declares release-candidate /
  unsigned-local-build. Linux ELF observations still require GLIBC 2.39.

## Executed acceptance

- `just check` passed; `just test`: **665 passed, zero failed/skipped**. Ten Python
  packaging/build tests pass: source/lock/modes, ambient override clearing,
  existing-output/path protection, failure-log handling and changed/incomplete
  source/payload/profile rejection. Syntax/whitespace and recipe dry run pass.
- Native release material pixels/save/scene restoration and editor preview passed
  directly, then again using only installed tools and the shipped material example
  in clean Ubuntu (clean-native-materials.json). Inspected editor-materials.png
  shows the actual installed editor viewport.
- clean-linux.json/container-info.json record the same archive's pinned Ubuntu
  24.04 acceptance: offline, unprivileged, read-only root, only archive/evidence
  mounts, no Rust/compiler/Git/repository/development sysroot. Real editor native
  Save & Export, failure preservation/cancel, relocated gameplay/settings/save/
  fresh-process Continue/corrupt-slot behavior pass. Installed files still verify
  unchanged after all tests. Test drivers/system packages are present; host kernel
  and software graphics are shared. No physical device or browser in this container.
- browser-materials.json: current release CLI exports with empty tools PATH, output
  is relocated/source copy hidden and served under nested HTTP. Numeric/texture
  defaults/overrides, transparent pipeline reset/order, flip/rotation and cross-
  scene save/load pass; malformed shader feedback and actual legacy-runtime
  rejection pass. browser-materials.png was inspected.
- browser-physics.json: release CLI/WASM gravity/filtering/sensors/raycast, tile
  collisions, kinematics/all three joints, reload/velocity/Start once/contact
  reconstruction, scene cleanup/cross-scene load and real opted-out-runtime
  rejection pass. Supported run has no unexpected browser errors.
- The extracted, relocated release installation's editor performs actual Browser
  Save & Export with no Rust on PATH, saves X=88 and preserves previous output on
  failure (editor-browser-export.json). Its moved artifact passes full nested
  loading/error/retry, consent/held input, controller/touch, audio API and settings/
  save/reload/Continue/quota/corrupt checks (exported-browser-ui.json). Installation
  verifies unchanged afterward. Direct release-runtime UI regression also passes;
  its direct-mode export/loading flags are false by design (browser-ui-direct.json).

See [BUILD_PROVENANCE.md](../../BUILD_PROVENANCE.md) to reproduce. Prebuilt packaging
still works with preparation-only observations. Local records are unsigned and
cannot authenticate a publisher; Cargo caches, external configuration, compiler/
SDK/system provenance and bit-identical rebuilding are not certified. Internal
license declarations, trusted CI/signing and physical/other-platform/reference-
hardware startup/frame-time/memory checks remain Task 8 work. Remote CI execution
was not observed. Task 9/full 3D implementation have not started.
