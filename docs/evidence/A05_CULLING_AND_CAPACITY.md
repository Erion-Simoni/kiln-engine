# A05 transformed culling and visible queue diagnostics

2026-11-25, fix/a01-ci-baseline, following d6ead03. A05 is active; A03/A04
acceptance remains deferred under the user's bounded-attempt policy.

ADR-0105 fixes offscreen queue starvation: shared scene collection now rejects
non-intersecting actual sprite/particle/tile quads before sorting and submission.
Bounds include rotation, negative scale and pivot-adjusted position. Touching edges
remain included; invalid views reject before traversal, and invalid/zero-size
geometry is excluded. Bounds use f64 arithmetic to avoid finite f32 overflow.
Tilemaps retain existing chunk selection. Fixed vertex/custom fragment materials
preserve geometry. Retained draws keep their existing painter order.

The 16384-sprite GPU limit stays unchanged. All submission paths share per-frame
attempted/queued/dropped/missing-texture counters; unknown accepted IDs still use
white fallback but are diagnosed. Counters reset on begin_frame/library replacement.
The editor status bar and native/browser world overlays display compact warnings.
World overflow cannot consume the separate diagnostic HUD/overlay queue. General
UI/overlay overflow diagnostics and a new texture allocation cap are not supplied.

## Executed checks

- Locked Rust workspace: 874 PASS. just check PASS. Default and opt-out WASM strict
  Clippy PASS. Python helpers 49 PASS; py_compile, Node syntax and whitespace checks.
  Final culling checks include invalid-view rejection before world traversal.
- Required real-GPU test PASS on Mesa llvmpipe/Vulkan, with adapter information
  recorded. It constructs the real SpriteRenderer, submits through every API,
  checks exactly 16384 queued plus 3 rejected and 1 fallback texture, and verifies
  counter resets. KILN_REQUIRE_GPU_TESTS=1 forbids an adapter-missing skip.
- Runtime regressions PASS for 16448 offscreen sprites followed by visible sprites,
  retained painter order, pivot/rotation/negative scale, edge contact and invalid
  geometry/views. Existing renderer transform pixels/material/order checks pass.
- Source-only generated projects each contain 16449 entities. With the first
  16448 offscreen, relocated native and Chromium exports each render all 4096
  pixels of the final green sprite. With those sprites onscreen, both exports
  visibly report 65 dropped and produce 1260 orange warning pixels. Source copies
  are hidden; native player PATH has no developer tools. Screenshots inspected.
- Actual editor read-only inspection PASS: the status bar reports 85 dropped,
  including background/grid queue entries; manifest bytes stay unchanged. This
  does not claim full editor authoring or normal-exit acceptance for dense scenes.
- One compact-warning browser screenshot timed out while an editor stress run
  overlapped. An isolated unchanged browser replay passes. Editor OCR initially
  clipped the end of dropped; the prefix-tolerant isolated replay passes. Failures
  are retained; no blocked A03/A04 checks were retried.

Tools and CI: make_sprite_stress.py generates bounded manifests instead of checking
in multi-megabyte fixtures. Existing native/browser pixel observers add an explicit
warning-color mode without changing their default green scenario. CI runs both
stress cases and retains narrow receipts/logs/screenshots; hosted results are not
claimed. Matching editor/CLI/player/browser tools rebuilt, contract still v6.

Raw: target/a05-{capacity-gpu,capacity-final,culling-focused,culling-final,workspace,
quality,wasm,wasm-optout,python,precommit,tools-final,web-final}.log.
Final offscreen receipts: target/a05-offscreen-{native,browser}-final/ (4096 each).
Final overflow: target/a05-overflow-native-final/ and
 target/a05-overflow-browser-isolated/ (1260 each). Editor:
 target/a05-overflow-editor-replay/ (85, unchanged manifest). Sibling logs and
 earlier failed stress replay artifacts remain available. Prior prepared browser
 bundle retained in target/a05-prior-web-runtime.

This is functional Linux/software-graphics and Chromium evidence, not physical
frame-time/memory/GPU certification. A05 remains PARTIAL: camera effects/transitions,
split views/input mapping, remaining blend/mask/outline/material/texture-capacity
and combined production acceptance stay open. See SPRITE_CAPACITY.md.
