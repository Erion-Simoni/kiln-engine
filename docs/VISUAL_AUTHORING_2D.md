# Sprite, animation, tilemap and VFX authoring

Open `samples/visual2d_editor/kiln.toml` in the native editor. The small blue
character has a two-frame walk clip, a transition to idle, frame events, a
particle trail, and a multilayer tilemap whose wall blocks movement. Left/right
moves the character. The sample was generated through the same authoring APIs
used by editor controls, then saved normally; its scene file needs no manual edits.

## Sprites

Import a PNG through the Asset Browser, then drag its image entry onto the Scene
View. The editor decodes its dimensions and creates an undoable sprite entity at
the pointer's world position. Image payloads are distinct from prefab payloads.
The texture path remains project-relative after saving and packing.

Use **Inspector → Sprite Pivot / Pixel Scale**, or **View → Sprite Editor**, to
set the pivot, horizontal/vertical flips and pixels per world unit. A pivot of
`[0,0]` is the image's top left; `[0.5,1]` anchors its feet. Higher pixel density
reduces drawn world size. Rotation and negative transform scale apply around the
anchor; framing uses the actual pivoted quad. Animated frames use their own
per-frame pivots while playing.

Sorting layers interleave sprites, tile layers and particles. Lower layers draw
first. Equal layers retain authored hierarchy order; transparent sprites keep
that order even when textures or materials differ. This can require more draw
batches than grouping every sprite by texture.

The Asset Browser's per-asset **Nearest filtering** and **Mipmaps** options reach
the actual GPU texture upload. Mips are generated with box filtering, including
odd edge texels. Existing images without configured import metadata retain
nearest sampling without mipmaps. Sprite edits refresh the scene projection;
new paths and filter changes refresh GPU bindings without accumulating old
project textures. Resource authoring is disabled during Play.

## Animation

In the Sprite Editor, choose the sprite, enter a new clip name and sheet grid
cell size, optional spacing and offset, then **Create Clip and Attach**. The
grid slicer validates decoded image bounds and caps a clip at 4096 frames.

Use **View → Animation Editor** to select the clip, add/remove/reorder frames,
edit pixel rectangles, duration and pivots, and add named frame events with
optional text payloads. Invalid frame rectangles or durations are rejected.
**Undo Clip Edit / Redo Clip Edit** restores clip authoring separately from scene
history. A non-looping clip can choose a next clip; looping clips wrap.

The preview draws the decoded sheet with the same playback clock, frame UVs,
events and transitions as gameplay. It supports Play/Pause/Stop and scrubbing.
UVs use actual sheet dimensions; changing clips can change the bound texture.
Repeated `play_animation` requests for the current clip preserve elapsed time.
Crossed frame events are delivered once, including skipped frames and loop
boundaries. Attached scripts receive `script_phase() == "animation_event"`,
`animation_event_name()` and `animation_event_value()`.

## Tilemaps

Open **View → Tilemap Editor**. Create a map and a tileset from a project PNG,
with positive tile dimensions that divide the sheet's tight grid. Select the map,
layer and tileset; the palette always belongs to that selected layer. All tiles
are reachable through palette pages, with 64 cells drawn per page.

- **Brush:** drag to paint; interpolated strokes fill gaps between pointer samples.
- **Erase:** remove tiles; right-drag also erases.
- **Fill:** replace a connected region within the visible 16×16 editing area.
  The underlying command accepts explicit finite bounds up to 65536 cells.
- **Terrain:** select/create a terrain palette and assign its 16 variants. Mask
  bits are up=1, right=2, down=4, left=8. Painting or erasing updates adjacent
  variants on the selected layer.

Add layers, set their names, Z and world offsets, and set the map's world tile
size. The grid has world Y increasing upward. Page through the map with the
direction buttons. Toggle **Solid** for the brush tile; **Collision preview**
draws red outlines using the same solid flags queried by gameplay. Paint strokes
have independent undo/redo history. Map cells, layers, tilesets, solid flags and
terrain palettes persist in the project.

Attach the map to a scene entity with **Inspector → + Tilemap**. A tilemap
owner can be a nonvisual entity. Tile layers use world offsets consistently for
culling, drawing, bounds and collision queries. Attaching a map preserves the
owner's local transform and hierarchy; nested scene offsets apply once. Map
translation follows the owner. Tile collision is axis-aligned; use world tile
size and layer offsets to lay out grids.

With physics enabled, solid maps whose owner has no explicit rigid-body component
also generate static Rapier geometry. Physical characters can land on painted
tiles; backend contacts and rays identify the map's stable entity ID. Adjacent
solids merge into row runs. Map edits/translation/removal update this geometry.
Automatic map colliders use membership bit 1 and accept all body memberships, so
a physical character's mask must include bit 1 to collide with them. An explicit
rigid-body component takes precedence over automatic map geometry.

## Particles

Select a scene entity and use **Inspector → + Particles**, then open
**View → VFX Editor**. Add/remove emitters and edit shape, rate, speed/angle,
lifetime, size, color, gravity and fading. Set the effect's initial emission,
particle capacity and sorting layer. Configurations are validated and saved as
an undoable scene component; removing a component is also undoable.

The preview and gameplay use the same fixed-tick particle simulation. Each
emitter has a reproducible random stream. Each particle keeps its originating
gravity, fade rule and initial alpha, even if that emitter is removed. In games,
emitters follow their entity's global position. Start/stop controls toggle
emission while existing particles finish; they preserve emitter configuration.
The renderer draws the same square particles used by the preview.

The existing renderer has a 16384-quad frame budget shared by visible sprites,
tiles and particles. Particle simulation capacity is configured separately;
preview particle counts do not raise that render budget.

## Saved and packed game acceptance

The player integration test authors an animated character, effects and two tile
layers, verifies scene undo/redo and paint undo/redo, saves/reopens, packs a real
KAR archive, and uses the native player's loader on both sources. Simulations at
30 and 144 render FPS produce identical movement, frame events and draw geometry.
It asserts wall collision, cross-layer ordering and unchanged authoring data.
Other tests verify terrain/fill, actual textured preview UVs, clip event history,
per-emitter particle parity, pivots, nested offsets, painter order and mip texels.

Pack the sample and launch it on a graphical machine:

```sh
cargo run -p kiln-cli -- pack --source samples/visual2d_editor --output target/visual2d_editor.kar
cargo run -p kiln-player -- --archive target/visual2d_editor.kar
```

The real archive/runtime path is exercised here. Automating the editor's
Build/Export panel and bundling distributions remains Task 8.

Chromium WebGL smoke tests run the checked-in sample and the original vertical
slice. They assert visible character pixels, keyboard movement, release stopping,
and zero browser runtime/asset errors. A separate copy exercises linear filtering
with generated mipmaps. Native editor/player window creation was attempted but
is blocked by this runner's missing X11/Wayland display. Native drag/drop and
panel interaction are **unverified**, not inferred from browser rendering.

On a graphical machine, record the build revision, OS/GPU, results and screenshots:

1. Import/drag an image into a new scene. Change pivot/flips/scale/sorting and
   filtering; undo/redo and verify alignment and overlapping transparency.
2. Slice a sheet and edit frames/events/transition in the Animation Editor.
   Compare its texture playback with Play and the live script event state.
3. Paint/erase/fill two map layers, terrain edges and solids. Save, restart and
   reopen; verify images, palette selection, collisions and offsets in Play.
4. Author an effect with two emitters of different gravity/alpha. Compare preview
   and Play, then pack and launch the saved archive with the native player.
