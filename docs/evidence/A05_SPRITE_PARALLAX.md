# A05 sprite parallax — 2026-11-27

ADR-0109 extends defaulted SpriteLayout with `parallax` factors and
`parallax_origin` anchor, copied to runtime Sprite and logical save snapshots.
Factors are finite -8..8, anchors finite. Defaults [1,1]/[0,0] retain normal world
scrolling, including old layouts/saves without these fields. Inspector and Sprite
Editor controls use validated component commands with existing undo/redo and disk
persistence. Scene/prefab layouts reject invalid export before publication.

For each axis, transformed draw geometry receives
`(camera_origin-anchor)*(1-factor)` before culling and deterministic painter sorting.
f64 intermediate arithmetic rejects unrepresentable positions. World transforms,
hierarchy, physics and particles remain unchanged. Authoring draws logical world
geometry for picking/gizmos; Play/native/browser use camera-relative geometry.
Zoom still scales sprites. This is sprite parallax, not HUD, tile/particle parallax
or automatic repeated backgrounds. No dependency or new render pass.

## Executed validation

- Full locked workspace 896 Rust tests PASS (serial), Python helper suite 49 PASS.
  just check and strict default/opt-out WASM Clippy PASS; Node syntax and whitespace
  PASS. Initial development compile/quality findings (collector function inference
  and an unused test binding) were corrected before final checks.
- Tests: legacy defaults/roundtrip/bounds validation; per-axis anchors/reverse/
  foreground offset; pivot/rotation/negative-scale and UV preservation; finite
  overflow rejection; pre-cull visibility; no transform mutation; logical authoring
  geometry; logical-save JSON/restore and invalid runtime-factor rejection.
- Existing editor history/projection/disk fixture and two-scene snapshot/restart
  fixture now include nondefault parallax. Real CLI transaction test rejects invalid
  scene and prefab layouts with no publication or leftover staging. No new OS
  widget-drag acceptance is claimed.
- Matching native editor/CLI/player and prepared browser rebuilt with contract v10;
  actual export using the prior v9 browser runtime rejects as incompatible. VM/save
  schema unchanged; SpriteLayout serde defaults preserve older logical saves.

## Actual export pixels

`samples/parallax` moves the camera to [80,40] over two seconds. The green 64×64
sprite at [160,120] has factors [0.5,0.25], anchor [20,8]. A blue completion beacon
appears only at tick 150. Observers require multiple green positions before exact
final bounds (except the deliberately stationary fixed case). Both axes use world
coordinates; world Y grows upward. Generated variants test fixed, ordinary and
reverse/foreground factors. These checks do not merely assert project metadata.

| Scenario | Factors / anchor | Exact green bounds |
| --- | --- | --- |
| Anchored slow | [0.5,0.25] / [20,8] | [78,104,142,168] |
| Fixed, camera [4096,2048] | [0,0] / [0,0] | [128,88,192,152] |
| Ordinary | [1,1] / [20,8] | [48,128,112,192] |
| Reverse / foreground | [-1,2] / [20,8] | [168,160,232,224] |

Each case passes native and browser exports with 4096 green and 64 blue pixels.
The fixed case would disappear if culling happened before the parallax offset.
The existing camera-transition sample also passes exact endpoint and motion checks.
Replays run serially after all builds; exports are relocated and source copies
hidden, native player PATH excludes developer tools, browser reports no page errors.
CI runs the anchored slow fixture. Linux Xvfb/software graphics and Chromium/
SwiftShader only; no physical DPI/device or production performance acceptance.

Receipts/screenshots: target/a05-parallax-{native,browser},
target/a05-parallax-{fixed,default,reverse}-{native,browser},
target/a05-parallax-transition-{native,browser}. Logs:
target/a05-parallax-{workspace-final,quality-final,wasm,wasm-optout,python,tools-final,
web-final,stale,precommit}.log. Prior runtime: target/a05-parallax-prior-web-runtime.
A05 stays PARTIAL. Split views/input, tile/particle/repeated backgrounds and
remaining rendering/capacity acceptance remain open. A03/A04 gates stay deferred.
