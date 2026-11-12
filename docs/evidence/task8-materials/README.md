# Task 8 authored sprite material evidence — 2026-11-10

Real material fragments executed in the editor, native player and browser using
software graphics. No physical GPU/device, other OS/browser or release provenance
certification is implied. Package size is informational (ADR-0062).

## Reproduce

Build matching native tools and prepare the default browser runtime, then run:

```sh
node tests/web/materials.cjs target/debug/web-runtime
python3 tests/native/materials_smoke.py
cargo test -p kiln-render material_target_tests
```

Local Chromium used /usr/bin/chromium and SwiftShader; native tests used Xvfb and
software graphics. WEB_MATERIAL_LEGACY_RUNTIME=target/task8-physics-runtime also
exercised real rejection of the prior valid runtime without material support.
See [SPRITE_MATERIALS_2D.md](../../SPRITE_MATERIALS_2D.md) for the fragment contract.

## Passed behavior

- browser.json records real CLI export with an empty tools PATH, relocated output
  and hidden source copy, served at a nested HTTP path. Pixel assertions check
  numeric and named-texture defaults/overrides, default/custom/default pipeline
  reset, stable transparent order, flip/rotation and cross-scene save/restore.
- native.json records corresponding relocated native game pixels and save/scene
  restoration, followed by actual editor preview. The alpha sample is
  [136,187,136,255] native and [137,188,137,255] browser; other listed colors match.
  editor-materials.png contains 23364 saturated red and 7744 saturated green pixels.
- browser-materials.png, native-materials.png, editor-materials.png and
  invalid-shader.png were inspected. Corrupting exported WGSL produces an explicit
  material error/reload screen. Supported browser run had no unexpected errors.
- Required software GPU test renders a custom fragment to Rgba16Float and reads
  back [0x3c00,0,0,0x3c00], catching the editor float post target mismatch. New shader
  template is directly validated against the shared contract. GUI automation does
  not individually exercise every Shader Editor control.

## Regressions and packaging

- `just check` passed. `just test`: **665 passed, zero failed/skipped**. Four
  producer tests and syntax/whitespace checks passed. CPU/CLI tests cover invalid
  WGSL/stages/bindings/types/nonfinite parameters, metadata/capability compatibility
  and preservation of prior export output on failure.
- Strict default/minimal WASM Clippy and native feature-opt-out checks passed.
  Default JS/WASM was rebuilt/prepared with materials=true and physics=true.
  Existing browser physics and direct UI/audio/input/storage regressions passed.
- Actual native-editor Browser Save & Export saves X=88 and preserves prior output
  on failure (editor-browser-export.json). The relocated result passed nested
  loading/error/retry, consent, controller/touch, Web Audio API and save/settings/
  resume/quota/corrupt acceptance (exported-ui-regression.json).
- Final archive target/dist/kiln-task8-materials-final.tar.gz, SHA-256
  `d496940843632a63832e88935af10190d343172f9aa6e94f2672df9bd65e8f39`,
  includes the material example and matching tools/runtime. Verified 1542 listed
  files. Prepared from a dirty source checkout on ea21962; revision/compiler fields
  are observations rather than arbitrary-binary/source attestation. Binaries and
  archives are not committed.
- clean-linux.json/container-info.json record this final archive's native saved-UI
  acceptance in offline, unprivileged, read-only-root Ubuntu 24.04. Only archive/
  evidence mounts, no Rust/repository/development sysroot. Editor save/export/
  failure/cancel, relocated game save/restart/Continue/corrupt and unchanged-install
  checks pass. Material pixels/browser behavior were tested separately above,
  outside this native-only container; its kernel is shared with the host.
- CI now runs material native/browser acceptance and provides software Vulkan
  drivers for the required float-target GPU test. Remote CI was not observed.
  Physical/other-platform checks, release licensing and reproducible provenance
  remain Task 8 work. Task 9/full 3D implementation have not started.
