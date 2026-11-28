# A07 preview, cue, blend and cutout rig workflows

These workflows extend the [Cutscene Composer](CUTSCENE_COMPOSER.md). Open
[samples/rigged_cutscene](../samples/rigged_cutscene/README.md) for a complete
source-backed example with native/browser playback.

## Scene View preview

Enable **Preview in Scene View (draw only)**, then scrub or play the timeline.
The scene renderer samples movement, rotation, scale, tint, blending and IK using
the existing hierarchy and sprite renderer. Playback continues when switching to
Scene View. **Stop Scene Preview** returns to authored poses. Scene gizmos are
inactive during preview. Preview never emits cues, changes logical saves, or
records transient poses in undo history. Render failure also restores the poses.
Scene-camera navigation stays under editor control; following a tweened camera rig
is a gameplay view behavior, not a takeover of the editor camera.

## Audio and event cues

Expand **Audio / Signal cues**, add a time, choose Sound or Signal and enter its
name. Sound names must match a non-music project audio asset. Signals have a finite
numeric payload and use existing attachment `signal` lifecycle callbacks.
Signals are delivered on the next tick in stable order; scene root scripts do
not receive attachment lifecycle signals. Attach a listener script to an entity.

Cues fire in chronological order, once per playthrough, including when a large
tick crosses several cues. Time-zero cues fire at start/restart. Pause holds them;
seek is silent and does not rearm already-fired cues. Restart rearms them. Seeking
past an unplayed cue skips it unless later crossed after seeking backward.
Logical saves retain cue latches and pending signals. Native audio uses decoded
project assets; browser playback uses Web Audio after Start consent.

Generated sound cues use `queue_sound(name)` (host ID 141, arity 1), which returns
whether queued. Audio is delivered after successful callback commit; failed or
debugger-suspended callbacks cannot start the pending sound. One-shot voices finish
normally; this is not a music transport, audio waveform editor or voice-seeking API.

## Weighted animation blending

Enable **Blend second take** on a property track. Edit the second take's keys and
incoming easing, then set **Blend weight**. Weight 0 selects the first take;
weight 1 selects the second; intermediate weights mix positions, tint, positive
XY scale and unwrapped bone/entity rotation. Both takes share the saved clock.
The optional **Live weight game variable** uses `game_get(name, authored_weight)`;
scripts can change it with `game_set`, and logical saves retain it. Runtime weights
clamp to 0..1; use finite numeric values. Scene preview uses the authored weight.

This is pose/property blending and IK/FK mixing. Sprite-frame texture crossfades
and automatic parameter-driven blend trees are separate future extensions.

## Cutout skeletal rigs

Click **Create Two-Bone Cutout Rig** in Animation Editor. One undoable scene
transaction creates Upper/Lower/Tip bones, two rigid sprite skin attachments and
an IK Target. Bone names and stable IDs appear in Hierarchy. Existing Inspector
and hierarchy controls edit rest positions, rotations, sprite textures and parent
bindings; Save Project preserves them. Cutout skin attachments have one bone
influence with rigid weight 1. Add more child sprites/bones using hierarchy tools.

Animate bone-local rotations with composer tracks for FK. For IK, expand
**Cutout Skeletal IK**, **Add IK Chain**, choose Upper/Lower/Tip and the root-level
target entity, and animate that target's position. Bend +/− chooses elbow side;
**IK / FK weight** mixes the solved angles with the sampled FK pose. Targets out
of reach clamp to the closest reachable point. Save/reopen and logical game saves
retain hierarchy, local poses, target animation and weights.

Script API: `solve_ik_2d(upper,lower,tip,target_x,target_y,bend,weight)` (ID 140,
arity 7). Bend is -1 or 1, weight is 0..1, target is world space. The return value
means queued, not that a subsequently changed chain was still eligible. The
validated solver supports directly parented +X rest links and finite positive
uniform scale ancestry. Physics-owned bones, nonuniform/sheared chains and invalid
bindings reject without partial pose writes. Animated positions can change link
lengths but must retain +X rest alignment. IK owns the two solved rotation tracks.

This custom cutout approach supersedes ADR-0022's earlier deferral. It does not
implement soft multi-influence vertex mesh deformation or external Spine/
DragonBones import. Those capabilities and complete OS authoring/performance
acceptance remain separate from this implemented cutout workflow.

Matching runtime contract **kiln-authoring-v16** is required for IK and queued
sound calls. Rebuild editor, CLI, player and browser SDK together. Existing
unextended composer documents retain their original canonical generated source.
