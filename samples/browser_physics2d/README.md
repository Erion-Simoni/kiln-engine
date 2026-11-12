# Browser 2D physics acceptance playground

Export with a prepared physics-enabled web runtime and serve the exported folder
over HTTP(S). Native export uses the same authored scene/scripts and physics.

- Blue hero falls through a sensor and a deliberately filtered collider, then
  settles on the floor. J jumps, C checkpoints, L loads, S writes an inspection
  snapshot to slot 0, N changes to a fresh scene with a higher floor.
- Green ball lands on an actual solid tile. Amber kinematic body moves right.
  Purple bob hangs from a distance joint; a hinge bob and motorized slider
  exercise revolute/prismatic constraints.
- Root/attached script state, body velocities, joints, tilemaps and scene identity
  survive logical save/reload. Checkpoint and inspection use separate slots.

`node tests/web/physics.cjs RUNTIME_DIR` exports a disposable copy with empty tools
PATH, relocates the output, hides its source copy and serves only that artifact at
a nested URL. It tests real WASM/Rapier rendering/simulation, collision filtering,
sensor transitions/raycast, kinematics/joints/tilemap collision, save/reload/Start
once, collider reconstruction and scene/cross-scene restore. This fixture is an
acceptance playground, not a finished reference game or physical hardware test.
