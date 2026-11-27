# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A05 prepared view uploads and clipped
world pointers (2026-11-27), following 9e18e5f. A05 active/PARTIAL. A03/A04 acceptance
is deferred, not completed. User policy: at most three focused attempts per blocked
step, retain evidence and advance to feasible work. A01 baseline 6de428b; A02 PARTIAL.
Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0110 adds immutable prepared sprite views with separate camera/instance GPU
  buffers and shared texture/material bindings. Bounded queue/painter order remains;
  empty queues allocate nothing. Resource tokens reject stale/foreign views.
  Clearing project textures also discards pending sprites with invalidated IDs.
  Default single-view host loops keep their existing reusable flush path.
- Shared ViewRect/route_pointer helpers validate up to four fitted rectangles and
  route half-open boundaries, gaps and overlaps through the correct camera. New
  pointer_to_world clips gameplay pointers; screen_to_world still extrapolates for
  tools. Full authored split-screen configuration/host scheduling remains open.
- Editor/native/browser clear stale world input outside the fitted viewport.
  Native cursor leave/focus loss and browser absent/touch-completed pointers clear
  presence. mouse_world_valid() (128) reports validity, coordinate reads return zero
  when invalid. Pending/source-debug ticks capture validity with coordinates and
  propagate to attachments/UI callbacks. No new dependency or logical-save layout.
- Contract now kiln-authoring-v11. Matching editor/CLI/player/browser rebuilt;
  actual export using prior v10 runtime rejects. Source-only samples/world_pointer
  and optional five-phase pixel observer mode added. CI includes strict real GPU
  prepared-view test and native/browser pointer checks. Docs: CAMERAS_2D.md and
  evidence/A05_VIEW_UPLOADS_AND_POINTERS.md.
- Prior A05 parallax, camera transitions/shake/dead zones/finite mapping, transformed
  culling and bounded sprite diagnostics remain intact. A03/A04 gates untouched.

## Executed validation

- Full locked workspace 901 Rust tests PASS (serial), Python helpers 49 PASS;
  just check, strict default/opt-out WASM Clippy, Node syntax/py_compile/whitespace
  PASS. Initial test-only camera borrow corrected before final checks.
- Strict real GPU test on llvmpipe/Vulkan: two different cameras/zooms/instance sets
  survive one render pass/submission, 256 green and 256 red pixels in independent
  expected boxes. Empty/invalid views, queue cleanup, stale/foreign resources tested.
  WASM compiles prepared-view API; no two-view browser host completion claimed.
- Pointer tests cover non-finite clearing, arity, attached/UI dispatch and debugger
  resume with changed physical input retaining the frozen tick. Rect tests cover
  offsets, resize/letterbox, zoom, Y direction, shared boundary, overlap, gaps and
  invalid/excessive layouts.
- Actual native/browser five-phase pointer replay passes: inside green, letterbox
  red, inside green, outside window/canvas red, inside green. Final bounds
  [288,88,352,152], 4096 green pixels in 640×240 hosts. Browser test applies wider
  host-stage CSS before startup. Existing parallax and default dead-zone samples
  pass both runtimes at [78,104,142,168] and [128,88,192,152]. Six export replays
  run serially after builds, with relocation, hidden source copies, native developer
  PATH removed and no browser page errors.
- Logs target/a05-views-*.log. Receipts/screenshots:
  target/a05-views-pointer-{native,browser}, target/a05-views-parallax-{native,browser},
  target/a05-views-default-{native,browser}. Prior v10 browser bundle:
  target/a05-views-prior-web-runtime. Generated outputs are not committed.

## Deferred / limits

- Authored split-screen configuration, per-view controller scheduling, editor view
  authoring and complete multi-view host integration remain open. Prepared views
  are explicit GPU APIs; callers set viewport/scissor and bound retained view count.
  They must prepare new geometry when scene/camera changes. Physical performance/
  memory acceptance is not established. Tools retain extrapolating mapping by design.
- Sprite parallax excludes tiles/particles/repeated backgrounds and does not inherit
  to descendants. Zoom scales sprites even at factor zero. Authoring keeps logical
  geometry; Play previews parallax. Camera histories reset and are not logically saved.
- A04 source widget gate incomplete after three attempts (setup-baseline assertion)
  and saved-physics pose continuation retains prior divergence. Do not retry by
  default. A03 nested/variant identities, arbitrary overrides, multi-file crash
  transactions and combined production acceptance remain deferred. A02 combined
  inspector/widget acceptance remains open.
- Culling visits ECS entities with conservative quads, not a spatial index. Visible
  sprites beyond 16384 drop with diagnostics; independent overlay/texture allocation
  limits remain open. No physical DPI/device, production performance, release, 3D
  or external certification acceptance is claimed.

## Next

User explicitly authorized continuing through A07 in this session. Checkpoint
A05 as PARTIAL and proceed sequentially through A06 and A07; this overrides the
local one-milestone restriction for this requested scope. A03/A04 remain deferred.


- Continue A05 authored split views and host integration using the prepared GPU
  API/routing helpers, or remaining sorting/masks/outlines/blends/materials/texture
  capacity acceptance. Tile/particle/repeated backgrounds remain open. Inspect code
  for substantial feasible work and keep the bounded-attempt skip policy. Do not
  retry deferred A03/A04 gates or proceed beyond A07 unasked.
