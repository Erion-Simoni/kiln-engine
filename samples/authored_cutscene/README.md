# Source-backed authored cutscene

The actor moves through an intermediate key, spins 450 degrees, squashes then
stretches, and fades to green while the camera dollies. A delayed blue beacon
confirms the actor is settled: 8192 green pixels at [192,56,256,184].

Open scripts/main.ks in Source Editor, then **Animation Editor → Cutscene
Composer → Load from Source Editor** to edit the keys. Open Generated Source and
Save Script to keep changes. This sample already runs that source as its scene
script; do not attach another copy. timeline.json is the reproducible sample
recipe; the editor reloads the document embedded in main.ks.

Space pauses, Right seeks to midpoint, R restarts, C cancels. Logical game saves
preserve the timeline clock and pause flags. The editor offers a value preview;
Play runs the scene. See [composer guide](../../docs/CUTSCENE_COMPOSER.md).
