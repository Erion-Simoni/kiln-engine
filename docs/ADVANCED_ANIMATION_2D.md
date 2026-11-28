# Soft skins, texture crossfades and automatic blend trees

## Soft mesh skinning

Select a sprite and expand **Inspector → Soft Mesh Skinning**. Choose two bone
entities, set grid rows/columns, and **Generate Grid and Capture Binds**. The grid
uses the sprite's pivot and pixel scale. Bind capture records authored mesh/bone
world transforms. Animate the bones afterward with normal hierarchy tracks or IK.

Expand **Bone bindings** to add/edit references. **Vertices / Weights** edits local
positions, UVs and up to four influences per vertex. Bone indices refer to the
binding list. Weights must be positive and normalized; **Normalize weights** helps
after editing. **Recapture Bind Poses** deliberately rebinds at current authored
poses. **Apply Skinned Mesh** validates and records one undoable component edit.
Project save/reopen retains geometry, triangles, references and binds. Removing a
referenced bone or its Transform rejects; remove/rebind the skin first.

The renderer performs linear-blend skinning from current world bone transforms
and inverse binds. Vertices shared by triangles deform continuously. Textured
triangles use the existing material, culling, painter-order and prepared-view
paths, in native and browser players and Scene View preview. Skin motion comes
from its bones; changing the skin entity transform alone does not rebind vertices.
Sprites can still use animated UVs and texture blends on deformed geometry.

Limits per mesh: 64 bones, 256 vertices, 512 triangles, four influences per vertex.
Each triangle consumes one of the existing 16,384 frame queue entries; normal
capacity warnings still apply. Missing/nonfinite live bones do not submit unsafe
geometry. Export and logical-save validation reject invalid bindings. No external
rig importer is required; this extends the custom cutout rig workflow with actual
multi-influence vertex deformation.

## Timed texture crossfades

In **Inspector → Animation States**, set a transition's **Crossfade seconds**.
Zero retains the previous immediate transition; positive durations (up to 60
seconds) blend the old and new textures/UV rectangles. Both clips advance, while
the destination/reference owns crossed frame events. Pivot values interpolate.
Pause/zero speed holds the fade; logical saves preserve source time and fade clock.
Seek cancels the fade and samples the requested destination position immediately.

Scripts can queue `crossfade_animation(entity, clip, seconds)` (host ID 142,
arity 3). The return means queued. Unknown/ineligible requests leave playback
unchanged. A fade finishes before another graph edge is evaluated; another
explicit fade is rejected while one is active. Manual play/stop can cut immediately.
This bounded two-texture design does not silently discard a third interruption
sample. A timed edge involving a blend-tree state rejects during authoring.

Blending happens in premultiplied color, then converts to the renderer's existing
straight-alpha output. Transparent texels therefore do not introduce dark fringes.
Texture-pair bindings are cached with a ceiling of 1024 pairs and participate in
resource replacement. Custom fragment shaders should use `sample_sprite(in)` to
honor a blend; direct `textureSample(sprite_tex,...)` intentionally samples only
the primary texture. The new default shader template already uses the helper.

## Automatic 1D blend trees

Open **Animation States → Automatic Blend Trees**, enable a tree for a state,
choose a declared parameter and add ordered clip thresholds. Each tree accepts
2..16 points. **Sort thresholds**, then **Apply Animation States**. Existing graph
undo/redo and project persistence apply. Duplicate/nonfinite thresholds reject.

`set_animation_parameter` now automatically chooses and blends the two adjacent
clips. Outside the threshold range it clamps to an endpoint. Changing the value
preserves playback time; scripts do not need to switch clips or compute weights.
Children synchronize to the state's reference clip normalized phase; the reference
clip's duration and speed define their shared clock. That reference
owns events, preventing duplicate child events. Live parameters and graph state
survive logical save/load. State transitions and manual ownership retain their
existing rules. These are parameter-driven 1D sprite-clip trees; authored bone
pose tracks retain their separate two-take/FK/IK blending workflows.

[samples/advanced_animation](../samples/advanced_animation/README.md) demonstrates
soft bending/stretching, a timed texture fade and an automatic parameter blend.
Native/browser acceptance checks observe both intermediate mixed textures and the
final deformed mesh, not just startup or final clip names. Matching **v17** editor,
CLI, player and browser SDK are required. No new dependency or 3D scope is added.
