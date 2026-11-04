# Browser runtime smoke test

This is a functional regression for the actual WASM runtime, not a syntax-only
check. It serves generated JavaScript/WASM and the vertical-slice project over
a temporary loopback HTTP server. It checks startup error propagation, asset
HTTP responses, page errors, visible blue hero pixels, movement while a key is
held, and stopped movement after key release. The server/browser close on both
success and failure. No public preview is created.

## Prerequisites

- Rust with `wasm32-unknown-unknown` installed.
- `wasm-bindgen-cli` matching the resolved `wasm-bindgen` crate version. Inspect
  with `cargo tree -p wasm-bindgen --depth 0`. This checkpoint used 0.2.130:
  `cargo install wasm-bindgen-cli --version 0.2.130 --locked`.
- Node.js 24 (or a compatible supported Node version).
- Playwright Chromium: from this directory run `npm ci --ignore-scripts`, then
  `npx playwright install --with-deps chromium`. On a machine with Chromium
  already installed, set `CHROMIUM_PATH` to its executable instead.

From the repository root:

```sh
just web-smoke
```

The recipe builds WASM, generates bindings into ignored `target/web-smoke`,
installs the pinned development-only npm dependencies, and runs the browser test.
The npm lockfile preserves dependency integrity. Optional `WEB_SMOKE_DEBUG=1`
prints browser console messages. `WEB_SMOKE_SCREENSHOT=/path/screenshot.png`
saves the most recent canvas image for diagnosing failures.

The smoke test intentionally uses Chromium software WebGL, with WebGPU disabled,
to exercise fallback on GPU-less Linux machines. A missing WebGPU adapter warning
is expected; uncaught page errors, console errors, missing assets, an invisible
hero, or broken input fail the test. It is not a performance benchmark.

## Limits

This does not validate a complete CLI export/package, WebGPU on physical GPUs,
other browsers, mobile touch, long sessions, all UI widgets, or music/mixer/save
support. These remain separate items in the 2D completion plan. The GitHub Actions
job runs the same build/bindgen/browser operations; its remote result must be
checked separately from a local pass.

To run the same visible-pixel/input check against the Task 6 fixture, set
`WEB_SMOKE_PROJECT=samples/visual2d_editor`. Optional `WEB_SMOKE_PROFILE=1`
records 120 animation-frame intervals (cadence, not GPU timing). Set
`WEB_SMOKE_SCREENSHOT=/tmp/visual2d.png` to retain the rendered frame.
