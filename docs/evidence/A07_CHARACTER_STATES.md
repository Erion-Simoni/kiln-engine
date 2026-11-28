# A07 character animation state machines — 2026-11-27

ADR-0113 adds a gameplay-facing state machine: idle/run/action clips respond to
script parameters and one-shot completion. Graphs are inline sprite components;
shared runtime and scene loading serve editor Play, native and browser exports.
This is a discrete sprite controller, not blending, skeletal animation or physics.

## Implemented behavior

- Named states reference project clips; entry state and ordered edges are authored.
  Numeric/bool parameters, greater/less/equal/not-equal/always comparisons,
  wildcard interrupts and after-finished edges support character actions.
- First match wins, at most one state change per positive unpaused animation update.
  Current-state targets are ignored. Entering another state restarts the clip,
  even when two states share a clip. Repeated moving=true retains run time.
  One-shot finish is observed on the following update after clip completion.
- Existing frame events remain available; transitions preserve unconsumed crossed
  events. Graph clips cannot also use clip.next. Valid manual play/stop detaches
  graph ownership; unknown manual clip requests leave it intact.
- set_animation_parameter (129) accepts a finite number/bool and returns whether a
  declared update was queued. Unknown entity/graph/parameter returns false.
  Callback failure/source suspension does not deliver pending updates.
  get_animation_state (130) reads current state from world facts, or empty string.
- Graph definitions, active state, parameters and clip clock/speed are logically
  saved/restored. Bad states, missing clips, altered parameter sets and non-finite
  values reject. Legacy snapshots without graph data still load as manual clips.
- Graphs validate before binding, authoring, export and save restoration. Limits:
  64 states/parameters, 256 edges, 64-byte names and finite numeric values. Inspector
  persistent drafts use explicit validated Apply through existing scene history.
  Component metadata survives project save/reopen. Removing a dependent sprite
  rejects before document/history mutation; remove Animation States first.
- Contract v13 tools rebuilt; an actual export with prior v12 browser runtime
  rejects. No dependency or VM-memory layout change. Graph snapshot data is an
  optional backward-compatible extension; incompatible graph state fails closed.

## Tests and actual input/pixel evidence

Full locked workspace: 915 Rust tests PASS with serial execution; Python helper
suite 49 PASS. just check, strict default/opt-out WASM Clippy, Node syntax,
py_compile and whitespace PASS. Editor rebuilt after the final dependency guard.

New runtime integration tests cover movement transitions without restart, pause,
priority, wildcard action interruption, one-shot finish, alias-state clip restart,
manual override, corrupt snapshot rejection, parameter validation, unconsumed
events, source-stop callback commit and general logical save restoration.
Editor component transaction test covers undo/redo, runtime projection, disk reopen
and rejected dependent sprite removal with unchanged history. Controls compile;
no full OS graph-creation widget replay is claimed.

`samples/character_states` uses actual red/green/blue clip textures. Its automatic
intro observes idle → run → action → idle → run and enables a separate blue beacon
only after the entire sequence. The actor's color comes from the selected clip,
not script tinting. Both native/browser replays wait for that proof, then drive real
keyboard input and assert actual pixels:

1. Right held: green run clip and horizontal movement.
2. Right released: red idle, no green pixels.
3. Space pressed: blue one-shot action (more than 1000 blue pixels; beacon is 64).
4. Space released/action completes: red idle, no green and only the small beacon blue.
5. Right held to movement boundary: 4096 green pixels at [256,88,320,152], 64 blue.

Both exports pass all phases. Source is hidden, exports relocated, native PATH
contains no developer tools, browser has no page errors. Browser captures phase
screenshots states-running/idle/action/finished.png and final rendered_scene.png.
The source sample is interactive: Left/Right or A/D takes control, Space triggers
one-shot playback. It does not implement a physics jump. Replace texels with real
character clips through the existing clip editor.

Existing ping-pong and five-phase clipped-pointer fixtures pass both exports too,
using default observer flags. Six pixel replays ran serially after tool builds.
CI adds native/browser real-input graph cases. An optional observer flag
KILN_PIXEL_TEST_ANIMATION_STATES enables these phases.

Logs target/a07-graph-{runtime-tests,editor-tests,workspace-complete,precommit,
wasm-final,wasm-optout-final,tools-final,web-final,editor-final,python,stale}.log.
Receipts/screenshots target/a07-graph-{native,browser},
target/a07-graph-ping_pong-{native,browser},
target/a07-graph-world_pointer-{native,browser}. Prior SDK:
target/a07-graph-prior-web-runtime. Linux Xvfb/software graphics and Chromium/
SwiftShader only; no physical device, production performance or release acceptance.
A07 stays PARTIAL: blending, arbitrary tracks/curves, rigs/IK and full authoring
acceptance remain open. Earlier A03/A04 deferred gates were not retried.
