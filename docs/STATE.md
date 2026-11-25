# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A05 deterministic camera shake
(2026-11-25), following 1453c19. A05 active/PARTIAL. A03/A04 remaining acceptance
is deferred, not completed. User policy: at most three focused attempts per blocked
step, retain failure evidence and advance to feasible work. A01 baseline 6de428b;
A02 PARTIAL. Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0107 implements camera_shake(x,y,seconds,hertz,seed) and camera_shake_stop().
  Validated commands commit with callbacks, share a bounded 32-request queue across
  scene/attached scripts and return false when full. Suspended/failed callbacks do
  not expose uncommitted effects. Active mixer holds 32 seeded decaying effects;
  overflow replaces oldest. No dependencies added.
- Shared SliceSession camera adapter delivers effects in editor/native/browser,
  holds time during UI pause and rejects suspended tick updates. Offsets do not
  feed back into follow/dead-zone history. Pixel snapping and bounds apply after
  shake; camera validation failures cannot advance effect clocks.
- Runtime contract now kiln-authoring-v8. Matching editor/CLI/player and prepared
  browser rebuilt. Actual export using previous v7 browser bundle rejects. VM/save
  layout unchanged; transient shake history is deliberately not logically saved.
- Source-only camera_shake sample, optional motion/beacon pixel observer gates,
  and native/browser expiry CI cases. See CAMERAS_2D.md and
  evidence/A05_CAMERA_SHAKE.md for contracts and evidence.
- Prior A05 work remains: authored dead zones/finite camera mapping (1453c19),
  transformed culling and bounded sprite queue stats/warnings (c21915b). Prior A04
  scripting/debugger/source continuation improvements remain intact.

## Executed validation

- Full locked workspace 885 Rust tests PASS (serial test execution), Python helper
  suite 49 PASS. just check PASS; strict default/opt-out WASM Clippy PASS; Node syntax,
  py_compile and whitespace PASS. Initial concurrent workspace run hit an atlas
  publication lock; final serial run passes. A pause-test namespace was corrected.
- Actual native/browser expiry replays observe 12/13 distinct green bounds, then
  [128,88,192,152], 4096 green pixels and 64 blue beacon pixels. Explicit script
  cancellation observes 14/17 bounds, then the same exact resting pixels. Existing
  no-shake dead-zone fixture also passes both runtimes. Replays run serially after
  all builds, with relocated exports and authoring copies hidden; native player
  PATH excludes developer tools, browser has no page errors.
- Tests cover shared pending limits, argument rejection, callback failure/debug
  suspension, cancellation delivery, authored UI pause, deterministic decay,
  active replacement, follow isolation and recovery after bad camera settings.
- Raw logs target/a05-shake-*.log; screenshots/receipts under
  target/a05-shake-{native,browser}, target/a05-shake-cancel-{native,browser},
  target/a05-shake-default-{native,browser}. Prior v7 runtime retained in
  target/a05-shake-prior-web-runtime. Generated outputs are not committed.

## Deferred / limits

- A04 source widget gate incomplete after three attempts (setup-baseline assertion).
  Exact saved-physics pose continuation has its prior divergence. Do not retry
  these by default. A03 nested/variant identities, arbitrary overrides, multi-file
  crash transactions and combined production acceptance remain deferred. A02 full
  inspector combined widget acceptance remains open.
- Shake history resets with controllers, is not serialized in logical saves and
  can be suppressed by final camera bounds. Identical seeds/time reproduce the
  oscillator without a cross-platform bit-identical pixel guarantee. No physical
  DPI/device, release, 3D or external certification acceptance is claimed.
- Culling still visits ECS entities and uses conservative quads, not a spatial
  index. Dense visible scenes over 16384 drop with diagnostics; independent
  overlay/texture allocation limits remain open.

## Next

- Continue A05 transitions/parallax, multiple views/split input routing, remaining
  sorting/mask/outline/blend/material/texture-capacity acceptance. Inspect actual
  code for substantial feasible work; keep A03-dependent pieces deferred and use
  the same bounded-attempt skip policy. Do not begin A06 unasked.
