# Task 8 runtime notice propagation — 2026-11-12

Current native/browser exporters copy original notices from the actual selected
runtime's verified installation. A receipt-bound manifest inventories complete
notice bytes and the native player or browser JS/WASM/descriptor hashes. The CLI
and editor's background verifier check coverage and contents before success.
Loose developer runtimes warn explicitly; they do not claim verified notices.
ADR-0072 records the compatibility, staging and integrity contract.

## Validation scope

- `just check`: formatting, strict all-target workspace Clippy and workspace/audio
  checks pass. The unchanged Cargo.lock SHA-256 is
  `042a1a29202fab2e8940632cbc9f23f17e88a42572f9b2930e8d303e8de0ac17`.
- `just test`: 672 tests pass, zero skipped. Seven new notice tests exercise
  native/browser byte preservation, runtime binding, changed/missing/extra files,
  incomplete installations, malformed metadata even with a recomputed hash,
  linked parents, cancellation and existing destination preservation. Existing
  real-CLI export tests cover the explicit loose-runtime warning and transactions.
- WASM `kiln-web` check, all 29 Python tests, workflow actionlint and Python syntax
  checks pass. No dependency was added and the execution protocol is unchanged.
- The portable acceptance scripts now compare every installed notice against
  actual native/browser export bytes and the receipt; a damaged installation's
  export must fail without publishing output or leaving private staging.

Local software acceptance uses a freshly rebuilt development editor/CLI/player
with an already prepared compatible browser runtime, a moved installation,
unrelated cwd and no build tools in child PATH. It is prebuilt installation
testing, not a captured release rebuild, clean-machine or physical-device claim.
The first sandboxed attempt reached CLI checks but could not open Xvfb sockets;
the complete rerun outside that display restriction passed. `local-validation.json`
records both GUI Save & Export flows (unsaved X=88 saved into output), overwrite
preservation, native UI cancellation, source-unavailable relocation and actual
native/browser gameplay/settings/save/resume/failure behavior. All exports preserve
1514 original notice files (7846211 bytes), with one extra receipt-bound manifest.
Final installation integrity verification passes. The local archive records
preparing HEAD 358df2a with source_dirty=true; this is not a clean captured release.
Its actual hash is retained in the JSON. Local editor capture was visually
inspected; generated archives/logs/PNGs stay outside Git.

## Prior licensed Linux release candidate

[Run 43215982528](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43215982528)
at `3ff72d37f2a05641eac78098e5bc4e928ff8581f` completed successfully in 45m 2s.
`prior-licensed-linux-success.json` records all three jobs, complete required
gates, no failures and public overall success. Fresh locked/offline captured
build, frozen replay, six-payload comparison, browser materials/physics,
moved editor/export/gameplay, clean Ubuntu without source/Rust, candidate upload
and separate origin attestation/bundle upload all passed.

That candidate includes the new project grants but **predates automatic export
notice copying**. It does not validate the new code. Historical artifacts remain
unchanged; signature verification and retained capture review were not performed
through artifact downloads here because api.github.com is excluded by the active
cloud policy. Public workflow conclusions establish hosted execution, not legal
completeness, OS signing or physical-device acceptance.
