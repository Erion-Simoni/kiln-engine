# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: A05 interruptible camera pan/zoom
(2026-11-27), following bf5826f. A05 active/PARTIAL. A03/A04 remaining acceptance
is deferred, not completed. User policy: at most three focused attempts per blocked
step, retain evidence and advance to feasible work. A01 baseline 6de428b; A02
PARTIAL. Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- ADR-0108 adds camera_transition(x,y,zoom,seconds) and camera_transition_stop().
  Finite origins, zoom 1/64..64 and duration 0..60 validate before enqueue. Both
  use the existing shared 32-command callback queue. Failed/suspended callbacks
  cannot deliver pending effects. Smoothstep pan/zoom holds its endpoint; zero
  seconds cuts. New requests start from the last constrained unshaken view.
- Authored follow/dead-zone history advances independently. Stop returns to the
  current authored view; UI pause holds clocks. Transitioned zoom controls final
  bounds/pixel rounding. Shake remains independent. Failed projections commit no
  follow/transition/shake clock state. No dependency/render-pass/schema addition.
- Contract now kiln-authoring-v9. Matching editor/CLI/player/browser tools rebuilt;
  actual export with prior v8 browser runtime rejects. VM/save layout unchanged.
  Transient transition/shake history is deliberately not saved in logical slots.
- Source-only camera_transition sample plus native/browser CI pan/zoom cases.
  Docs: CAMERAS_2D.md and evidence/A05_CAMERA_TRANSITIONS.md. Prior A05 dead zones,
  finite mapping, shake, transformed culling and queue diagnostics remain intact.

## Executed validation

- Full locked Rust workspace 891 PASS (serial), Python helpers 49 PASS; just check
  and strict default/opt-out WASM Clippy PASS. Node syntax/whitespace PASS.
- Tests cover smooth endpoint/hold, cuts, pause, interruption, invalid replacement,
  finite extreme interpolation, 60/120 Hz equivalence, independent follow/shake,
  cancellation, bounded commands, failed/debug-suspended callbacks and recovery
  after projection validation failure without clock advancement.
- Actual native/browser exports verify motion plus delayed blue beacon and exact
  final bounds: pan/zoom [96,16,224,144], cancel [128,88,192,152], interruption
  [96,104,160,168]. Existing shake sample verifies motion and original resting
  bounds. All replays run serially after builds, with relocated exports, authoring
  copies hidden, native developer PATH removed and no browser page errors.
- First pan/zoom expectations used y-down coordinates and failed; corrected y-up
  expectations passed on the second native/browser attempts. Original failure
  evidence retained. No repeated attempts against deferred A03/A04 gates.
- Logs target/a05-transition-*.log; receipts/screenshots under
  target/a05-transition-{native,browser}, target/a05-transition-cancel-{native,browser},
  target/a05-transition-interrupt-{native,browser}, target/a05-transition-shake-{native,browser}.
  Prior v8 browser bundle: target/a05-transition-prior-web-runtime. Generated files
  are not committed.

## Deferred / limits

- A04 source widget gate incomplete after three attempts (setup-baseline assertion).
  Exact saved-physics pose continuation has prior divergence. Do not retry these
  by default. A03 nested/variant identities, arbitrary overrides, multi-file crash
  transactions and combined production acceptance remain deferred. A02 full
  inspector combined widget acceptance remains open.
- Transition endpoints are origins, not target centers. Pixel-perfect zoom is
  stepped; bounds may suppress movement. Stop returns immediately to authored
  follow rather than blending back. Controllers reset transient motion history;
  logical saves do not resume it. No physical DPI/device, release, performance,
  3D or external certification acceptance is claimed.
- Culling visits ECS entities with conservative quads, not a spatial index. Dense
  visible scenes beyond 16384 drop with diagnostics; independent overlay/texture
  allocation limits remain open.

## Next

- Continue A05 parallax, multiple views/split input routing, remaining sorting/
  masks/outlines/blends/materials/texture-capacity acceptance. Inspect actual code
  for substantial feasible work and apply the bounded-attempt skip policy. Keep
  A03/A04-dependent gates deferred; do not begin A06 unasked.
