# Cutscene movement and tint tweens

The intro moves a red actor to the right, fades it to green and dollies the camera. It pauses all three
tracks, seeks to their midpoint while paused, then resumes. A blue beacon confirms
those checks and completion. Final green bounds: [192,88,256,152].

Space pauses/resumes, Right seeks to the midpoint, R restarts, C cancels and holds.
F5/F9 use the existing game save/load controls; active track clocks and pauses
persist. The camera follows a tweened transform rig with zero smoothing, so saving its
rig pose/track also preserves the camera view. Direct camera transitions can be
composed separately; their controller history remains transient.
Script authoring uses the Source Editor and tween_* builtins, not a timeline GUI.
