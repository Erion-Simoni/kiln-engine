# Kiln Session Handoff

Active instruction: finish A01 through A04. Branch `fix/a01-ci-baseline`; last
pushed baseline `fc78575`. Full 3D remains gated on G2D. No release-size ceiling;
MIT OR Apache-2.0 remains settled. No milestone is accepted by this checkpoint.

## Done in this checkpoint

- Shared typed resource dependencies and rename cover sprite/prefab override,
  clip/tileset, script, audio/font and shader/material texture paths. Asset identity
  moves once, retaining global/target settings. Added target-specific texture
  options with default fallback and optional backwards-compatible project tables.
- Assets worker diagnostics name missing dependents and duplicated IDs/content
  without writing or deleting. SHA256 streams in bounded buffers. Import All now
  records actual project defaults instead of only cache metadata.
- Fixed real egui/wgpu preview use-after-free: retained preview TextureHandle and
  deferred texture frees until after submission/presentation. Bounded asset shelf
  and scrollable Assets body expose previews/import settings previously obscured.
- Qualified enums compile to stable tagged strings. Added parameter/argument and
  nested-array bounds. Cooperative staged tasks/one-shot timers reuse deterministic
  script time and typed state, survive logical save/restart and consume due values
  exactly once. No automatic suspended function stacks/yield syntax is claimed.
- Added token-preserving indent formatter, bounded saved-project source search,
  literal function navigation and shared-registry completion insertion. Navigation
  preserves unsaved buffers, uses Unicode cursor offsets and focuses/scans source.
- Shared runtime contract now rejects old same-version native/WASM tool bundles.
  Runtime sections, native identities and export regression fixtures agree.
- A01 source-save helpers hold relevant clicks across frames and name exact files
  and panel waits. Browser authoring publishes original exceptions/browser output.
  New native asset/source workflow smokes run in CI with retained evidence.

## Validation

- just check PASSED: fmt, strict workspace/all-target Clippy, workspace/native and
  standalone audio check. Full workspace: 745 passed, zero skipped; 34 Python tests.
- WASM default build and WASM no-default-feature strict Clippy PASSED. Fresh bound
  browser runtime plays real editor-authored enums/tasks/timers/dictionaries/loops;
  actual movement depends on the new scheduler results, rather than syntax alone.
- Real Xvfb/software graphics asset smoke PASSED: live GPU preview, separate Web
  defaults, move preserving IDs/settings/references, diagnostics, exact restart.
- Real source tools smoke PASSED: format/save, navigate to another saved source,
  completion prefix replacement and exact source/manifest restart preservation.
- Reproduction/evidence: docs/evidence/A03_A04_RESOURCE_TOOLS.md; generated logs,
  screenshots and receipts remain under target/ and are excluded from Git.

## Next / open gates

A01: fc78575 Windows candidate 43576816845 PASSED. General CI 43576816906 FAILED
in structured-scripting authoring with a source-save timeout; precise per-file waits
and held source controls need matching-source hosted confirmation. Public check logs
return 404 and authenticated API is Forbidden. Reference 43576816853 and Linux
43576816847 still need latest status. Earlier hosted results do not accept new code.
Avoid needless rapid pushes that continually cancel long candidate runs.

A02: implemented saved lock/hide, selection/marquee, alignment, copy/paste, recovery
and guarded writes with real two-scene evidence in prior fc78575 checkpoint.
Broad combined transform/property and cross-platform acceptance is still open.
A03: source imports are not included in typed manifest rename. Nested prefabs,
variants and integrated atlas author/rebuild/reopen/export acceptance remain open.
A04: replace hashed/stub graph host calls with shared registry semantics and wire
real editable nodes/pins; actual live source breakpoints, imported frames/locals/
watches and new enemy/projectile/trigger equivalent script/graph game remain open.
Legacy separate bytecode debugger is not the actual gameplay source debugger.

Tools: source /workspace/.kiln-tools/activate.sh; CARGO_INCREMENTAL=0,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0. Rust 1.99.1, nextest0.9.151;
local browser CHROMIUM_PATH=/usr/bin/chromium. just check before every commit.
Git push/public GitHub HTML work; authenticated API is Forbidden. Software graphics
is not physical GPU certification. Do not kill unrelated old editor/Xvfb processes.
