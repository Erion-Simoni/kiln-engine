# Browser game export — Task 8 checkpoint

**View → Build / Export → Browser → Save & Export** now creates an actual
browser game folder. It saves the same project/scene-tab edits as native export,
validates a prepared runtime and project, then publishes a complete directory.
Existing nonempty output is preserved. Export diagnostics, real process status,
job-specific receipts, cancellation and staging cleanup use the native workflow.
Runtime/hash verification runs off the editor UI thread and has a distinct
Verifying exported files stage.

## Prepare the runtime once

A browser runtime bundle contains `kiln_web.js`, `kiln_web_bg.wasm` and
`runtime.json`. The WASM embeds engine/protocol/font metadata; `runtime.json`
records the SHA-256 identity of the generated JS/WASM pair. Missing, altered,
mixed, malformed or incompatible bundles fail verification. Browser instantiation
still checks full WASM/module semantics; prepare bundles from trusted matching
wasm-bindgen output. Bundles needing additional JS snippet imports are rejected.

For repository development, build/install matching native tools and prepare the
browser runtime once (Rust wasm32 target and matching wasm-bindgen CLI required):

```bash
cargo build -p kiln-editor -p kiln-cli -p kiln-player --bins
just web-runtime
```

This writes `target/debug/web-runtime` beside development editor/exporter binaries.
The editor locates that folder by default; **Runtime tools → Browser runtime**
selects another prepared bundle. Exporting then needs no Cargo, wasm-bindgen or
Node on PATH. Portable editor packages must include this bundle; package delivery
remains Task 8 work. Runtime preparation can also be run explicitly:

```bash
kiln-cli web-runtime --input /path/to/wasm-bindgen-output
```

Export the saved project directly, from any working directory:

```bash
/path/to/kiln-cli export --platform web \
  --project /path/to/project/kiln.toml \
  --web-runtime /path/to/prepared-runtime \
  --output /path/to/new-browser-game
```

Relative CLI paths resolve from its working directory. Relative editor output
paths resolve from the project. Choose a new or empty destination for each export.
TOML/RON manifests normalize to `project/kiln.toml`; project-relative paths survive.
Private saves/recovery/cache/git data and build outputs are excluded using the
same resource policy/limits as [native export](NATIVE_EXPORT_2D.md).

## Deploy and play

Upload the **complete** output folder to an HTTP(S) static host:

- `index.html` and `game.js`: loading/start/error UI and relative entry points.
- `runtime/`: verified prebuilt JS/WASM and its descriptor.
- `project/`: saved manifest, scripts, textures, audio, fonts and other resources.
- `export.json`: job/project/runtime receipt; `_headers`: optional isolation headers.

Nested URLs work, for example `https://example.com/games/demo/`. Serve `.wasm` as
`application/wasm` and JS as JavaScript. No source repository, Rust, Node or editor
is needed by the player. For a local check, serve the directory using an HTTP
server; double-click/file:// opening displays HTTP guidance and is not supported.

Loading does not start gameplay, entity Start callbacks or sound voices. Once all
resources are ready, Play resumes Web Audio from the trusted user gesture before
starting the first tick. The start gesture, including held Enter repeats, cannot
also activate an underlying game button. The canvas receives focus afterwards.
Missing modules/assets and a missing loader show failure/reload feedback. A failed
start can be retried; a failed load reloads a fresh page and GPU context.

Settings and gameplay slots remain scoped to browser origin plus project ID.
Reload/Continue preserves progress; quota/access/corrupt saves report failure and
retain previous data. A different origin/profile creates a separate storage area.
The output has no service worker or offline-cache claim. The managed test browser
blocks file:// navigation by administrator policy, so direct execution of its
HTTP-guidance branch is unverified; HTTP deployment is the supported path.

## Supported contract and remaining gates

The checkpoint exercises the existing browser runtime's scripts, sprites,
animation/tile resources, saved UI/fonts, camera/input, Web Audio and localStorage.
It does not expand the renderer/physics contracts from earlier tasks. **Authored
physics bodies/joints and sprite material overrides are rejected for Browser**:
those host integrations are still missing. Use native export for those projects.
Physics-query scripts and other runtime/platform feature parity remain bounded by
the existing browser host; no universal native/browser gameplay parity is claimed.

Automated evidence includes actual native-editor Browser export with an empty
tools PATH, an unsaved scene edit saved into the artifact, previous-output
preservation, relocation and an unavailable source project. Only that artifact is
served to Chromium under a nested path. The test checks visible loading, no
simulation before consent, keyboard/held-key consent, controller API navigation,
touch activation, real Web Audio looping/overlap, independent preferences,
page-reload save/resume, quota/corrupt-save preservation, asset/loader failures and
reload retry. Selected captures/results: [evidence/task8-browser](evidence/task8-browser/README.md).

This is Chromium/SwiftShader and simulated controller input; real GPU/audio/
controller devices, other browsers/OS, clean-machine desktop dependencies, release
size/performance budgets and portable editor packages still require acceptance.
Task 8 remains in progress. Task 9 and full 3D implementation are not started.
