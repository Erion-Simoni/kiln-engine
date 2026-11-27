# Scripted cutscene tweens

Author sequences in the Source Editor using `tween_*` builtins. See
[samples/cutscene_tweens](../samples/cutscene_tweens/README.md) for movement,
tint, a camera dolly, automatic checks and interactive pause/seek/restart/cancel.
This checkpoint provides script authoring; a graphical timeline remains open.

| Builtin | ID | Arguments |
| --- | --- | --- |
| tween_position | 131 | entity, x, y, seconds, ease |
| tween_color | 132 | entity, r, g, b, a, seconds, ease |
| tween_cancel | 133 | entity, property |
| tween_seek | 134 | entity, property, seconds |
| tween_active | 135 | entity, property |
| tween_remaining | 136 | entity, property |
| tween_pause | 137 | entity, property, paused |

Property names are `position` and `color`. Easing names are `linear`, `smooth`
(smoothstep), `in` (quadratic) and `out` (quadratic). Durations are finite seconds
from 0 through 60; destinations must be finite f32-representable values. Color
destinations are RGBA channels from 0 through 1. Position means local Transform
position and rejects entities with any RigidBody2D; use physics tools for those.
Tint requires a Sprite. Unknown or ineligible targets return false.

Mutations return whether their command was queued. They commit after a successful
callback; failed or debugger-suspended callbacks do not leak pending changes.
Queries read committed state, so starting and querying inside one callback does
not observe that pending start. Cancellation, pause and seek can queue for an
eligible property even when it has no active track. Invalid eligible requests
raise a host error. The shared root/attachment service allows 64 active property
tracks, counting queued starts; pending cancellations can free capacity. Other
writes and seek-to-end free capacity when applied, so reservation is conservative.

A replacement starts from the current property value when applied. Zero duration
cuts immediately. Completion writes the exact destination and removes the track;
cancel holds the current value. Manual position/color writes cancel the matching
track, and despawn/component removal clears affected bindings. Track pause holds
its clock. Seek clamps to the track duration and writes immediately even while
paused; seeking to the end completes it. `tween_active` returns false and
`tween_remaining` returns zero after removal. Scripts own sequence stages and
completion events: poll active state, then use existing signal/task APIs as needed.
There is no internal completion-event queue to duplicate during seeking.

Tracks advance once per simulation update, shared by root and attachments. UI
pause and source-debugger pause hold progression. GameSave records sources,
destinations, easing, elapsed time and pause flags. Old saves without tracks load;
invalid or missing saved bindings reject before replacing the running session.

For a saved camera dolly, follow a Transform-only rig with zero smoothing and
tween its position. The sample saves the rig pose and its active track, preserving
view continuity. Existing direct camera transitions/shake can be composed with
sequences, but their controller history remains transient. Runtime contract
`kiln-authoring-v14` requires matching editor, CLI, player and browser SDK.
