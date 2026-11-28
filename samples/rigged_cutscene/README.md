# Blended cutout skeleton with audio and event cues

The target blends two keyframe takes while a two-bone cutout arm follows it with
IK. Gray rigid skin sprites attach to bones. A chime plays once; two marker signals
and a final completion signal turn the beacon blue. Final green tip is 32×32 at
[224,104,256,136], with 64 blue beacon pixels.

Open scripts/director.ks in Source Editor, then **Animation Editor → Cutscene
Composer → Load from Source Editor**. Preview in Scene View, edit either take,
blend weight, cues or IK bindings, then Open Generated Source and Save Script.
The director is already attached. scripts/listener.ks receives next-tick signal
callbacks on the beacon entity. timeline.json is the reproducible source recipe.

Space pauses, Right seeks silently, R restarts/rearms cues, C cancels. The optional
saved game variable reach_blend controls the live take weight (default 0.5).
See [workflows](../../docs/A07_ANIMATION_WORKFLOWS.md) for cutout rig creation,
FK/IK blending, event semantics and supported chain constraints.
