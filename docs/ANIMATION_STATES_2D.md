# Character animation states

Create clips in Animation Editor, then select a sprite and open **Inspector →
Animation States**. This replaces repeated script play-animation calls with a
parameter-driven graph. `samples/character_states` is a working example: its intro
runs idle → run → one-shot action → idle → run; Left/Right or A/D takes control and
moves the character, and Space plays the action. The color texels are placeholders
for your character artwork; the action is animation, not a physics jump.

1. Choose the idle clip for the initial `idle` state. Add `run` and `jump` states
   and choose their project clips.
2. Add `moving` and `jump` parameters, default zero. Script booleans use zero/one.
3. Add the ordered transitions below. The first matching transition wins.
4. Apply the validated draft, then save the project. Apply/remove participates in
   scene undo/redo. Unapplied drafts are editor-local. Remove the graph before
   removing its sprite; orphaning it is rejected without changing the document.

| From | To | Condition | After clip ends |
| --- | --- | --- | --- |
| Any state | jump | jump Greater 0.5 | No |
| idle | run | moving Greater 0.5 | No |
| run | idle | moving Less 0.5 | No |
| jump | idle | Always | Yes |

Graph-controlled clips must leave their ordinary `next` clip empty. Finish edges
require a specific non-looping source clip. You can reorder edges with Move up.
An edge targeting the current state is ignored; different states sharing a clip
restart that clip on state entry. Wildcard sources are useful for action/hit/death
interrupts. Looped states retain playback time when parameter requests repeat.

For an attached script, a minimal parameter driver is:

```text
if script_phase() == "update" {
    set_animation_parameter(self_entity(), "moving", get_input_x() != 0.0);
    let held = get_action("jump");
    set_animation_parameter(self_entity(), "jump", held && !state_get("held", false));
    state_set("held", held);
}
```

Declare the `jump` input action in the project. The rising-edge example prevents a
held key from replaying the one-shot after it returns to idle. A scene script can
use an explicit entity source ID instead of self_entity().

`set_animation_parameter` returns whether a declared update was queued. A failed
or debugger-suspended callback does not deliver pending updates. State/clip changes
occur at the next positive animation update, at most once per update; no zero-time
transition cascades. UI/global animation pause holds graph evaluation. One-shot
finish is observed on the next update after its final frame completes. Existing
frame-event callbacks continue to work and unconsumed crossed events are retained.

`get_animation_state(entity)` returns the state name, or empty string if absent or
manually detached. Manual play_animation/stop_animation hands ownership to manual
playback; use parameters for graph actors. Scene reset or runtime bind_graph enters
the graph again. Logical snapshots preserve definition, current state, parameters,
clip clock and speed; corrupt references/parameter sets reject before restoration.
Old snapshots without a graph remain valid manual playback snapshots.

Graphs are bounded to 64 states, 64 parameters and 256 transitions. Names are
nonempty, at most 64 bytes, with `*` reserved for wildcard sources; defaults and
thresholds must be finite. Unknown clips/parameters and conflicting clip.next
ownership reject. Graphs are inline components, reusable through authored entity/
prefab data; arbitrary prefab override authoring remains governed by A03 limits.

Use matching v13 editor/CLI/player/browser tools. No dependency was added. This is
a discrete sprite state machine; blending, arbitrary timelines, skeletal rigs/IK
and full A07 acceptance remain separate. [Executed evidence](evidence/A07_CHARACTER_STATES.md).
