# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current A07 checkpoint: character animation states,
2026-11-27, following a9b72c1. User asked to prioritize features that matter; this
session chose character animation control. A05/A06/A07 remain PARTIAL. Current
scope stops at A07; do not advance to A08 unasked. At most three focused attempts
per blocked check; record and move on. A03/A04 gates stay deferred. Full 3D behind G2D.

## Done

- ADR-0113: inline AnimationGraph sprite component with named states, declared
  numeric/bool parameters, ordered conditions, wildcard interrupts and one-shot
  finish edges. First match wins, one transition per positive unpaused update.
  State entry restarts its clip, including aliases; repeated movement parameters
  do not restart running. Existing events remain chronological/unconsumed events
  survive transitions. Graph clips cannot also use clip.next.
- Script APIs set_animation_parameter(entity,name,value) (129, returns queued bool)
  and get_animation_state(entity) (130, empty if absent/detached). Facts include
  graph state/parameters. Failed or suspended callbacks do not deliver pending
  changes. Manual valid play/stop detaches; unknown manual clip requests preserve
  graph ownership. Pause holds graph evaluation.
- Inspector Animation States drafts support states/clip selection, parameters,
  condition/finish edges and priority reordering. Validated Apply uses undoable
  component transactions. Project snapshot/reopen retains graphs. Removing an
  attached sprite rejects atomically until its graph is removed.
- Logical snapshots preserve graph definition/state/parameters plus clip clock.
  Invalid references, parameter sets or state/current-clip mismatch reject.
  Optional graph fields keep legacy snapshots readable. Limits: 64 states,
  64 parameters, 256 edges; finite numbers/names up to 64 bytes. Export validates
  scene/prefab graph references and requires a sprite. No new dependency or VM
  memory layout change. Runtime contract v13; matching tools rebuilt, v12 SDK rejects.
- Interactive samples/character_states: automatic state proof, then Left/Right or
  A/D moves and Space plays one-shot action. This is animation, not physics jump.
  Native/browser CI tests exercise real input and pixels. See ANIMATION_STATES_2D.md
  and evidence/A07_CHARACTER_STATES.md. Prior A05/A06/A07 work remains intact.

## Executed validation

- Full locked Rust workspace 915 PASS (serial), Python helpers 49 PASS; just check,
  strict default/opt-out WASM Clippy, Node syntax/py_compile/whitespace PASS.
- Runtime tests: movement continuity, pause, priority, finish, same-clip state entry,
  manual override, parameter validation, corrupt/resumed snapshots, preserved events,
  failed/debug-suspended callback commit, and general logical saves. Editor test:
  undo/redo, projection, disk reopen and atomic dependent-sprite-removal rejection.
- Both native/browser exports observe automatic idle/run/action/idle/run with a
  delayed blue beacon, then real Right/Space input: run moves, release idles,
  one-shot action plays, finish idles, run reaches boundary. Final green 4096 at
  [256,88,320,152] plus blue beacon 64. Source hidden/exports relocated/native PATH
  lacks developer tools/browser no page errors. Existing ping-pong and pointer
  fixtures also pass both exports. Six replays ran serially after builds.
- Actual prior v12 browser SDK export rejects. Controls compile and have transaction/
  persistence coverage; no full OS graph-creation widget replay claimed.
- Raw target/a07-graph-*.log; receipts/screenshots target/a07-graph-{native,browser},
  target/a07-graph-ping_pong-{native,browser},
  target/a07-graph-world_pointer-{native,browser}. Prior bundle
  target/a07-graph-prior-web-runtime. Generated outputs are not committed.

## Open / deferred

- A07 blending/blend trees, arbitrary property/camera/audio/event tracks/curves,
  tween authoring, rigs/weights/skinning/IK and full editor acceptance. Conditions
  are level values; scripts should pulse one-shot parameters (sample shows rising
  edge). Current-target edges are ignored; different states sharing clips restart.
  Graphs are inline components; arbitrary prefab override authoring remains A03.
- A05 full split-screen host/authoring and remaining rendering/capacity acceptance.
  A06 stamps/streaming, further tile validation and complete level-authoring gate.
  Existing animation 4096-cycle cap/extreme event overload and physical performance
  acceptance remain open; no budget bypass introduced.
- A04 source widget gate remains stopped after three attempts and saved-physics pose
  divergence remains. A03 identity/override/crash-transaction and A02 combined
  inspector acceptance remain deferred. No physical device/DPI, production
  performance, release/certification or full 3D acceptance claimed.

## Next

Continue useful A07 gameplay/authoring work or consolidate state-machine acceptance.
Keep all remaining milestones honest, preserve bounded-attempt policy and do not
retry deferred A03/A04 gates or advance beyond A07 without user direction.
