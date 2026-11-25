# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A05 transformed culling and visible
sprite queue diagnostics (2026-11-25), following d6ead03. A05 is active. A03/A04
remaining acceptance stays deferred, not completed; do not retry those gates to
delay plan progress. User policy: at most three focused attempts per blocked
step/check, retain failures, continue the next feasible step. A01 accepted baseline
6de428b unchanged. A02/A03/A04/A05 remain PARTIAL. Full 3D behind G2D.
MIT OR Apache-2.0.

## Done

- ADR-0105 fixes offscreen sprite queue starvation. Shared scene collection culls
  actual pivoted/rotated/scaled sprite and particle quads and collected tile quads
  before sorting/submission. Negative scale/edge contact are supported. Invalid
  views reject before traversal; invalid/degenerate geometry is excluded. f64
  bounds arithmetic avoids finite f32 overflow; retained painter order unchanged.
- Keep GPU capacity 16384. All sprite submission APIs share attempted/queued/
  dropped/missing-texture counters. Invalid accepted texture IDs retain white
  fallback but become diagnosable. Reset counters on begin_frame/library reset;
  flush retains diagnostics. No new dependency, pass or capacity increase.
- Visible compact world-queue warnings in editor status bar, native HUD and browser
  overlay. Editor counts include background/grid. This does not complete warnings
  for every independent UI/overlay queue or add a texture allocation cap.
- Source-only bounded stress generator; native/browser pixel observers support an
  explicit orange-warning mode alongside existing green scenarios. CI includes
  generated offscreen and overflow projects with narrow evidence uploads.
- Matching editor/CLI/player/prepared browser tools rebuilt. Contract remains v6;
  prior bundle target/a05-prior-web-runtime. See docs/SPRITE_CAPACITY.md,
  docs/PLAN_PROGRESS.md and evidence/A05_CULLING_AND_CAPACITY.md.

## Executed validation

- Full locked Rust workspace: 874 PASS. just check PASS. Default/opt-out WASM strict
  Clippy PASS. Python helper suite 49 PASS; py_compile/Node syntax/whitespace PASS.
- Strict real-GPU queue test PASS on Mesa llvmpipe/Vulkan (not skipped): actual
  Renderer, all push variants, 16384 queued/3 dropped/1 fallback and reset behavior.
- Runtime culling/order tests PASS with 16448 offscreen sprites and transformed
  bounds, including invalid-view rejection before world traversal.
- Generated projects have 16449 entities. Offscreen case: actual relocated native
  and Chromium each render 4096 green pixels from the final sprite. Visible case:
  each shows 65 dropped and 1260 orange warning pixels. Authoring copies hidden;
  native PATH has no developer tools. Screenshots inspected.
- Actual editor status bar PASS: 85 dropped including grid/background; manifest
  bytes unchanged. Read-only inspection, not full dense authoring/normal exit.
- One overlapping browser/editor stress capture timed out; isolated unchanged
  browser replay passes. First editor OCR clipped dropped; prefix-tolerant replay
  passes. Keep failures; do not use them as a reason to retry A03/A04 gates.
- Raw target/a05-{capacity-final,culling-final,workspace,quality,wasm,wasm-optout,
  python,precommit,tools-final,web-final}.log. Final receipts/screenshots:
  a05-offscreen-{native,browser}-final/, a05-overflow-native-final/,
  a05-overflow-browser-isolated/, a05-overflow-editor-replay/.

## Deferred / scope limits

- A04 source widget gate incomplete after three attempts (setup-baseline assertion);
  saved-screen UI source dispatch atomic. Actual source/statement core and Rust
  continuation tests pass. Do not repeat the widget gate by default.
- Exact saved-physics continuation remains blocked (prior tick 46 pose divergence).
  Restored final outcomes pass with narrower scope; no general physics-save fix.
- A03 nested/variant identities, arbitrary overrides, multi-file transactions and
  combined production acceptance deferred. A02 every-inspector-property combined
  widget acceptance remains open. External device certification is outside scope.

## Next

- Continue A05 camera/rendering work: camera dead zones/shake/transitions/parallax,
  split viewport input mapping, remaining sorting/mask/outline/blend/material and
  texture-capacity/large-scene acceptance. Inspect actual code before selecting
  substantial feasible fixes; keep A03-dependent work deferred where necessary.
- Culling is conservative quad AABB, not occlusion culling or a spatial index.
  It avoids offscreen queue starvation but still visits ECS entities. Extremely
  dense visible scenes still drop beyond the explicit bounded queue and report it.
  No reference-hardware performance or release acceptance is implied.
