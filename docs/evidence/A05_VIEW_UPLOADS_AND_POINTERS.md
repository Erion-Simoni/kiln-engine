# A05 prepared view uploads and clipped world pointers — 2026-11-27

ADR-0110 addresses shared GPU upload storage and extrapolated/stale gameplay
pointers. This is a multi-view renderer/input foundation. It does not complete
authored split-screen configuration or host scheduling, which remain open.

## Renderer and resource fixes

`prepare_view(device,camera)` snapshots the existing bounded painter-sorted queue
into independent immutable GPU camera/instance buffers. `draw_prepared(pass,view)`
uses shared texture/material resources, preserving adjacent batching and transparent
order. Empty queues allocate nothing; each view remains bounded by MAX_SPRITES.
Prepared views expose queued/batch counts and survive CPU queue clearing. Resource
identity tokens reject replaced or foreign renderer resources. Project texture
clearing now also clears pending sprites that referenced invalidated IDs, avoiding
later invalid texture indexing. Default hosts retain their reusable single-view
flush path, with no new per-frame allocation. Prepared views are explicit opt-in.

Strict real GPU test (`KILN_REQUIRE_GPU_TESTS=1`) passes on llvmpipe/Vulkan. It
prepares a green quad with origin [0,0]/zoom 1 and a red quad with origin [128,0]/
zoom 2, clears the CPU queue, then draws both into separate viewports in one pass
and one submission. Readback finds 256 green pixels only within [8,8,24,24] and
256 red pixels only within [40,8,56,24]. This fails when camera or instance uploads
alias. It also checks empty/invalid-camera preparation, pending texture cleanup
and stale/foreign view rejection. CI now requires this GPU test instead of allowing
an absent adapter to count as success. Browser prepared-view code is WASM-compiled;
no two-view browser host integration is claimed.

## Pointer routing and presence

`ViewRect` validates finite positive representable extents. `route_pointer` accepts
at most four already fitted rectangles and selects the frontmost containing view.
Half-open edges route shared boundaries to one adjacent view; gaps map to none.
Tests cover zoom, resize/letterbox, offsets, Y direction, overlap priority, invalid
rectangles, invalid/non-finite pointers and excessive view counts.

`Camera::pointer_to_world` clips gameplay input; `screen_to_world` still extrapolates
for authoring tools. Editor Play/native/browser now set or clear world input each
tick. Native cursor leave/focus loss, browser absence/touch completion, and absent
editor input clear presence. `mouse_world_valid()` (host 128) reports validity;
coordinate getters return zero when invalid. Finite custom-host set_mouse_world
remains supported; set_world_pointer(None) explicitly clears. Validity is captured
in pending ticks and propagated with coordinates to attached/UI callbacks. Tests
verify invalid/NaN clearing, arity, callback propagation and source-debug resume
with input changed while suspended. Existing paused UI dispatch checks validity.

## Actual relocated export replays

`samples/world_pointer` is green only for a valid world pointer near [160,120].
Both native and browser run in a 640×240 host area around a 320×240 world. Native
resizes the window; the browser observer embeds the export with a test-only wider
stage CSS rule before startup to exercise actual canvas letterboxing. Each replay
checks five phases by actual pixels:

1. Inside: green.
2. Letterbox bar: zero green, more than 200 red pixels.
3. Re-enter: green.
4. Outside native window/browser canvas: zero green, more than 200 red pixels.
5. Re-enter: 4096 green pixels at exact bounds [288,88,352,152].

Positive-before-negative ordering catches stale retained coordinates, not just an
initial invalid state. Screenshots include pointer-valid.png, pointer-invalid.png,
pointer-outside.png and the final rendered scene. Browser records no page errors.
Native PATH excludes developer tools; exports are relocated, authoring copies
hidden. Existing default observer modes also pass: native/browser parallax
[78,104,142,168] (4096 green/64 blue), and dead-zone sample [128,88,192,152]
(4096 green). Six actual export replays run serially after all builds.

## Validation and limits

Full locked workspace: 901 Rust tests PASS (serial); Python helper suite 49 PASS.
just check, strict default/opt-out WASM Clippy, Node syntax, Python compilation and
whitespace PASS. Matching editor/CLI/player/browser rebuilt, contract v11; actual
export using prior v10 browser bundle rejects as incompatible. No dependencies,
extra render pass, persistent project schema or logical-save layout change.
An initial test-only camera borrow was corrected before final checks.

Raw logs target/a05-views-{gpu,pointer,workspace-final,quality-final,wasm-final,
wasm-optout-final,tools-final,web-final,python,stale,precommit}.log. Receipts/screenshots:
target/a05-views-pointer-{native,browser}, target/a05-views-parallax-{native,browser},
target/a05-views-default-{native,browser}. Previous bundle:
target/a05-views-prior-web-runtime. Linux Xvfb/software graphics and Chromium/
SwiftShader only; no physical-device/DPI, production frame-time, split-screen
editor interaction, or release acceptance. A05 stays PARTIAL; A03/A04 gates remain
deferred under the user's bounded-attempt policy.
