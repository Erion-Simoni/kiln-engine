# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current checkpoint: authorized progression through
A07, 2026-11-27. A05/A06/A07 remain PARTIAL; this session completed concrete
checkpoints, not all milestone acceptance. User authorized crossing milestones
through A07, overriding local single-milestone session guidance. Stop at A07.
At most three focused attempts per blocked check; retain evidence and advance.
A03/A04 gates remain deferred. Full 3D behind G2D. MIT OR Apache-2.0.

## Done

- Environment execution disconnected and later resumed with edits intact. A05
  commit was verified absent and completed as 898ba25: collapsed projection pointer
  rejection and cleared draw-call diagnostics on texture replacement. Prior A05
  prepared views, clipped pointers, parallax, camera effects/dead zones and culling
  remain intact. Full authored split-screen/rendering acceptance remains open.
- A06 54521e7 /ADR-0111: render/collision work follows stored intersecting chunks,
  not enormous empty coordinate spans. Finite f64 bounds/wide tile coordinates,
  stable chunk draw/global-row collision order, saturating map size and checked
  atlas dimensions prevent hangs/overflow. Source sparse_tilemap fixture added.
- A07 ADR-0112: defaulted ping-pong/speed controls reach editor preview/history,
  runtime and logical snapshots. Loop endpoints are not duplicated; one-shot
  returns to frame zero. One precise timeline drives frame/event/seek. Zero dt/
  speed is silent; leftover transition time uses the next clip's speed. Old fields
  default to forward/speed one. Source ping_pong event/speed proof fixture added.
- Matching editor/CLI/player/browser rebuilt, contract kiln-authoring-v12. Actual
  prior-v11 SDK export rejects. No dependency, VM-memory or save-layout change.
  Docs: evidence/A06_A07_SPARSE_MAPS_AND_PLAYBACK.md. CI adds both export fixtures.

## Executed validation

- Full locked workspace 907 Rust PASS (serial), Python helpers 49 PASS; just check,
  strict default/opt-out WASM Clippy, targeted editor control history, Node syntax,
  py_compile and whitespace PASS.
- A06 core/runtime regressions: full finite-world sparse queries, tile coordinate
  2,000,000,000, row priority across chunks, offsets/edge contact, bad geometry,
  map/atlas overflow. Initial chunk-edge culling mismatch fixed before final tests.
- A07 regressions: ping-pong boundaries/hold/seek, old defaults/roundtrip, invalid
  speed, pause/entry silence, event sequence, different-speed transition leftovers,
  snapshot resume and nondefault editor undo/redo.
- Native/browser sparse-map pixels: 4096 green, bounds [128,112,192,176]. Ping-pong:
  4096 green, bounds [128,88,192,152], gated by zero/one/two/one/zero event order and
  tick-15 speed checkpoint. Existing five-phase pointer regression passes both
  runtimes. Six replays run serially after builds, with relocation/hidden source,
  no native developer PATH and no browser page errors.
- Raw target/a06-sparse-*.log and target/a07-*.log. Receipts/screenshots under
  target/a07-{sparse_tilemap,ping_pong}-{native,browser} and
  target/a07-pointer-{native,browser}. Prior v11 bundle target/a07-prior-web-runtime.
  Generated outputs excluded from commits. New PNG sample texels are tiny generated
  fixture assets, not external art.

## Open / deferred

- A05 full split-screen authoring/host integration and rendering/capacity acceptance.
- A06 stamps/streaming, further tile asset validation and complete editor-authored
  scrolling/collision/reimport/export acceptance. Sparse traversal is not an index;
  unsupported negative/malformed chunk data is ignored. Stored-data cost remains.
- A07 state graphs/conditions, arbitrary property/camera/audio/event tracks/curves,
  script tween authoring and 2D rigs/weights/skinning/IK with full editor acceptance.
  Existing 4096-cycle update cap remains; extreme event/cycle overload and physical
  performance acceptance are open. Controls compile/history/persistence/pixel tested;
  no new OS widget-drag acceptance claimed.
- A04 source widget gate remains stopped after three attempts; saved-physics pose
  divergence remains. A03 identities/overrides/crash transactions and A02 combined
  inspector acceptance deferred. No physical device/DPI, production performance,
  release, certification or full 3D acceptance implied.

## Next

Continue feasible A07 work or consolidate its acceptance. Keep A05/A06 open items
accurate, preserve the bounded-attempt skip policy, and do not advance to A08 unless
requested. Do not resume deferred A03/A04 gates by default.
