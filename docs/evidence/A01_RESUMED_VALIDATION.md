# A01 resumed validation — 2026-11-15

Source: `6de428b168aff0b3cbad86038461e7c2731ad081`, branch
`fix/a01-ci-baseline`. The startup fix was pushed before the environment was
suspended. Reconnection recovered the checkout and target evidence; temporary
`/tmp` logs were not retained. No experimental graph timing change was retained.

## Matching hosted results

- [General CI 43606018834](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43606018834):
  **SUCCESS**, 18m 59s. Includes Linux/Windows/macOS checks, builds and experimental
  archives; native/Python tests; documentation; native UI including delayed-render
  startup; browser/graph/portable compatibility. The startup fix has hosted confirmation.
- [Reference games 43606018842](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43606018842):
  **SUCCESS**, 16m 15s. All three widget authoring/export and native/browser game
  replays passed. Public evidence archive digest:
  `2b543efe301c58a47e42c8b4d69329368c47d885a0a4765f81f27b71ccc95204`.
- [Linux candidate 43606018785](https://github.com/Erion-Simoni/kiln-engine/actions/runs/43606018785):
  **SUCCESS**, 46m 18s. Artifact-service roundtrip, locked release build, fresh
  rebuild/payload comparison, relocated editor/export gameplay, clean Ubuntu
  runtime acceptance and origin attestation completed. Public candidate archive
  digest: `4bdd602de659fb28c902ca2e622ae8bcd8fbf3b61c2ab8ae3ba86f000cfb6cc0`.
  Public attestation artifact digest:
  `5b92e4c56ab446ae1851de9219b278945809535dd621f0fe8f928a9083414c29`.

These are public run-summary observations and workflow-defined checks, not a local
independent download/verification of hosted artifacts or attestation signatures.
The Windows candidate workflow did not trigger for this Linux-test-only change;
its prior `3efb687` pass is not a matching-source candidate result. General CI's
Windows check/build/archive matrix did pass on the current source.

## Recovered local evidence

- `cargo test --workspace --locked`: **752 passed**, zero failed/ignored, including
  documentation tests (751 ordinary tests). Count observed before suspension.
- Strict default/feature-off WASM Clippy and native editor/player feature opt-out
  passed. The default WASM build and matching wasm-bindgen 0.2.130 bundle passed.
- Structured scripting authoring → browser movement passed, including exact source
  save and external artwork refresh (`target/a03-a04-workflow/result.json`).
- All three games completed actual widget authoring and native/browser export,
  preserving existing output on rejected overwrite. The complete three-game
  receipt survived at `target/a01-reference-replay/result.json`.
- Portable integrity/tampering/permission/version/symlink rejection, all starter
  templates, native/browser editor export, notices, relocation and real game
  save/restart acceptance passed (`target/task8-portable-evidence/result.json`).
  Debug portable archive: 60,944,065 bytes; SHA-256
  `9e4ae154d0da4528ade77e59b58ab16fb87a94e682519d89024125f096f04fd8`.
  This is same-machine relocation, not clean-machine or release-budget evidence.
- Browser menus/audio/controller/save-resume and exported material rendering passed.
  Chromium 156.0.8078.79 was installed rootlessly from Debian; the Playwright
  download domain is denied by the environment's network policy.

## Local timing boundaries

Concurrent local build/export/GUI workloads exposed two failures despite green
hosted results. No assertion was removed and no engine behavior was changed.

1. Graph authoring saved the default `print` call rather than `add_score`.
   Captures show the newly opened host-function dialog moving during initial
   measurement. An observation run with screenshots passed complete native/browser
   graph gameplay, argument 9, history, save/restart and node-value restoration.
   Proposed waits/retries then failed later dialog-opening checks; those unverified
   edits were reverted. Evidence: `target/a04-graph-authoring`,
   `target/a01-graph-observation`, `target/a01-graph-settled`, `target/a01-graph-verified`.
2. Physics saved upward velocity 170.95001 but the post-reload capture saw
   -127.529976 after advancing into descent. An isolated rerun after reconnection
   passed the unchanged test: restored velocity 121.900024, valid collision/joint
   rebuilding, next-scene and cross-scene loading, no browser errors. The optional
   physics-disabled runtime rejection was not exercised by this local command.

The unchanged graph replay also passed in isolation after reconnection: native
score 252, browser score 324, 392 hero pixels and three saved node values; actual
host arguments, Cancel, Undo/Redo, one-gesture drag undo, exact restart and browser
save restoration passed (`target/a01-resumed-graph/result.json`). Both formerly
failing flows now pass in the configured serial mode, matching hosted CI. A serial
pass does not establish reliable execution under resource contention; avoid
running concurrent GUI/build/export workloads when collecting these timed checks.

A01's configured baseline acceptance is recorded on `6de428b`: matching required
hosted checks, all three replays and candidate artifact verification passed.
The Windows-specific candidate was not a required triggered check for this
Linux-test-only change; no fresh Windows candidate or physical-device acceptance
is inferred from its older result. Independent downloads/signature verification
were not performed locally. A02–A04 and broader release gates remain open.

Resumed check logs and public summary extracts: `target/a01-resume-logs/`.
