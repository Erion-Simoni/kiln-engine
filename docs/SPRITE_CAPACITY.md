# Sprite visibility and queue diagnostics

Shared scene drawing culls offscreen quads before sorting or GPU submission.
Bounds use the actual pivoted, rotated and scaled draw geometry, including negative
scale. Particle quads and collected tile quads receive the same visibility check;
tilemaps also retain their existing chunk selection. Quads touching the view edge
are retained. Invalid camera bounds reject before scanning the world; non-finite
or zero-size geometry is excluded. Culling does not despawn entities or stop their
scripts/physics. Custom fragment materials keep Kiln's fixed vertex geometry.

Each sprite renderer remains capped at **16384 queued sprites per frame**.
Submissions beyond that bound are dropped deterministically in submission order;
capacity has not increased. SpriteFrameStats reports attempted, queued, dropped and
missing_textures (accepted invalid IDs replaced with the white texel). Counters
reset at begin_frame and texture-library replacement. Flush retains diagnostics.

The editor status bar and native/browser world overlays show dropped/missing counts
when nonzero. World diagnostics use a separate HUD/overlay queue so filling the
world queue cannot suppress the warning. Editor background/grid submissions count
toward its scene queue. This checkpoint does not impose a new texture allocation
cap or complete diagnostics for every independent UI/overlay queue.

Source-only stress projects can be generated without committing large manifests:

```sh
python3 tools/make_sprite_stress.py target/offscreen-example
python3 tools/make_sprite_stress.py target/overflow-example --visible
```

The default places 16448 gray sprites before a final green sprite. Offscreen mode
must render the green sprite; visible mode exceeds capacity by 65 and must show
that drop diagnostic. Counts can be varied with --count (1..65536); the generator
refuses to overwrite an existing directory. Native/browser pixel helpers accept
KILN_EXPECT_SPRITE_DROPS=1 to observe warning pixels instead of green scene pixels.
See evidence/A05_CULLING_AND_CAPACITY.md for actual local checks and limits.
