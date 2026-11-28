# Projectile Range (script and editable graph)

Open this folder's kiln.toml in the editor and press Play. The projectile advances
automatically. Hold Left/A to stop it or Right/D to double its speed; release to
return to base speed. The orange enemy moves toward the cyan projectile. Their
sensor collision removes the enemy and stores hits=1 in shared game data. The
projectile continues through the purple victory trigger; it turns the large red
indicator green only after the enemy was hit. The HUD score becomes 1 after the hit. Restart resets the range.

All logic is ordinary KilnScript on authored entities. motion.ks is an imported
function used by both moving entities. No engine change or compiled plugin is
needed to edit gameplay. Collision masks exclude the indicator and ensure the
enemy/trigger only receive the projectile. The projectile remains alive after a
hit so it can reach the trigger.

The equivalent editable graph project is ../projectile_range_graph. Open each
scripts/*.ks in View → Graph Editor to inspect/change nodes, pins and host calls.
Its generated body goes through the same frontend, VM, scheduler and physics.
To regenerate the checked-in deterministic graph documents from the repo root:

```sh
cargo run --locked -p kiln-graph --example projectile_range -- samples/projectile_range_graph/scripts
```

The real-physics regression compares both variants at every tick, including input,
enemy removal, shared game data, activation and rendered indicator color. Logical
saves taken before impact restore and reach the same victory outcome. Exact pose
continuation across physics restore remains an open check: an initial resumed
kinematic step differs from uninterrupted play. This sample does not establish
full A04 debugger or equivalent-game production acceptance.
