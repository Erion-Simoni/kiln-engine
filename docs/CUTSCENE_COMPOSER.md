# Cutscene Composer

Open **Animation Editor → Cutscene Composer** to author position, rotation,
XY scale and tint keyframes. This is a source-backed property timeline; skeletal
rigging, audio/event tracks and scene-view animation preview remain open work.

1. Choose an entity and property, then **Add Track**. The first key captures its
   authored value at time zero. Change the second key's time and destination;
   **Add Key** adds another segment. Rotation values are radians without wrapping.
2. Each key's easing controls the segment arriving at that key. Choose Linear,
   Smooth, In or Out. Times must strictly increase; use **Sort Keys by Time** after
   changing their order. Time-zero keys cannot be removed. Duplicate times reject.
3. Click or drag the timeline strip, use the Seconds slider, or **Play Values** to
   preview interpolated values. This previews values without changing scene poses.
   **Undo Timeline / Redo Timeline** restore draft edits.
4. Set a project-relative `.ks` source path. Its parent directory must exist.
   **Open Generated Source** stages the script and opens Source Editor.
   **Save Script** uses the normal atomic save and external-change checks.
5. Attach the saved script to one director entity using **Inspector → Attach
   Script**. Save the project. Avoid attaching multiple copies targeting the same
   properties. A scene that already runs this source should not attach it again.
6. Reopen that source from Assets or Source Editor, return to Animation Editor and
   choose **Load from Source Editor**. The path and editable keys reload from the
   embedded document. Generate and Save Script again after changing keys.

Draft edits remain in the composer until generated and saved. Switching scene or
project resets that draft; generated source drafts use existing Source Editor
save/recovery behavior. Manual edits to generated source are preserved: reloading
or replacing altered source rejects. Keep the custom script and generate to a new
path instead. Unrelated scripts are never replaced by the composer.

The default control actions are `pause`, `restart`, `seek` and `cancel`. Bind them
in the project's action map, or change the names under **Runtime control actions**.
Empty names disable controls; nonempty names must be distinct. Controls use rising
edges. Pause holds the clock and poses; seek jumps to the timeline midpoint even
while paused; restart resets to zero and resumes; cancel holds and stops writing.
Completed timelines hold their final sampled pose and relinquish subsequent
writes until restarted or sought. A later manual write therefore stays in place.

All tracks share a single clock saved in script state. Keyframes are sampled
analytically, so a large tick can cross multiple segments and still reach the
correct value. UI pause and source-debugger suspension use existing script runtime
behavior. Logical saves retain clock, pause/control flags and entity poses. The
source's memory signature must still match when loading a saved game.

Limits: 16 unique entity/property tracks, 64 total keys, 60 seconds. Every track
starts at zero. Transform bindings need a Transform and reject all RigidBody2D
entities. Tint needs a Sprite; channels must be 0..1. Scale axes must be positive
and at least 0.000001. Other destinations must be finite and f32-representable.
The metadata comment stores decimal strings to preserve authored values exactly;
source numeric literals avoid unsupported exponent notation. Existing v15 tools
execute the generated script; this adds no runtime ABI or dependency.

[samples/authored_cutscene](../samples/authored_cutscene/README.md) demonstrates
multi-key motion, a 450-degree spin, squash/stretch, tint and a camera-follow dolly.
See also [script tween APIs](CUTSCENE_TWEENS.md).
