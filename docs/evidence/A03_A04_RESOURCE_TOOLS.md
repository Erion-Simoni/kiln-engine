# A03/A04 resource and source tools checkpoint — 2026-11-14

Branch: `fix/a01-ci-baseline`. Evidence below records the implementation in this
checkpoint, on Linux x86_64 under Xvfb/software graphics, Rust 1.99.1. This is not
Windows/macOS or physical-GPU certification. A01–A04 remain PARTIAL.

| Check | Actual result | Reproduction / raw local evidence |
| --- | --- | --- |
| Full Rust workspace | 745 passed, zero skipped | `cargo nextest run --workspace --no-fail-fast`; `target/a03-a04-tests.log` |
| Python tooling | 34 passed | `python3 -m unittest discover -s tests/native -p 'test_*.py'`; `target/a04-python-tests.log` |
| Native import workflow | Passed actual widgets, GPU preview over multiple frames, separate Web/default settings, asset rename preserving ID/overrides/references, diagnostics and exact restart comparison | `python3 tests/native/asset_import_workflow_smoke.py`; `target/a03-asset-import/result.json` and captures |
| Editor → browser language | Passed live texture refresh and source authored through widgets; browser movement depends on enums, scheduled resume payload, one-shot timer, dictionaries and counted loops | `CHROMIUM_PATH=/usr/bin/chromium python3 tests/native/asset_script_workflow_smoke.py --web-runtime target/debug/web-runtime`; `target/a03-a04-workflow/result.json`, `browser-play.log` |
| WASM default build | Passed | `cargo build --target wasm32-unknown-unknown -p kiln-web`; `target/a04-wasm-build.log` |
| WASM feature opt-out | Strict Clippy passed | `cargo clippy -p kiln-web --target wasm32-unknown-unknown --no-default-features -- -D warnings`; `target/a04-feature-off.log` |

The preview test exposed a real wgpu lifetime error: an egui texture was dropped
before the queued draw used it. The AssetBrowser now retains the handle and the
renderer frees egui textures after submission/presentation. Import controls also
remain visible below a bounded shelf and within the scrollable Assets body.

Shared resource visitation now includes prefab overrides, clips, tilesets,
shader/material textures, scripts, audio and fonts. Unit regressions verify
reference-safe rename, target fallback, stable ID/settings retention and roundtrip;
diagnostics tests verify dependent names, duplicate content/IDs and no writes.

New language regressions cover qualified enums, invalid declarations/members,
argument limits, collection-depth rejection and deterministic tasks/timers across
logical save/restart. The runtime contract rejects older same-package-version
native/WASM bundles; fixtures exercise missing/different identities and existing
feature capability rejection independently.

New source formatting and literal search are tested for token preservation,
idempotence, invalid draft preservation, Unicode cursor offsets, excluded folders
and traversal bounds. `python3 tests/native/source_tools_smoke.py` passes actual
format/save, search navigation to a different file, shared-registry prefix
completion and exact source/manifest preservation across restart. Raw captures and
receipt: `target/a04-source-tools/result.json`, `editor/` and `reopened/`.
Live source breakpoints, imported-function frames/watches, editable gameplay
graph acceptance, nested/variant prefabs, integrated atlas rebuild/export and the
full combined cross-platform A02/A03 acceptance remain open.

Hosted source `fc78575`: Windows candidate 43576816845 passed. General CI
43576816906 failed in editor source authoring (source-save timeout; additional
specific diagnostics/held source controls are included in this checkpoint).
Matching-source reference/Linux candidate results still require verification.
These earlier hosted outcomes do not accept the current checkpoint.
