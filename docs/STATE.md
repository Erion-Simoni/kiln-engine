# Kiln Session Handoff

Branch fix/a01-ci-baseline. Current A07 advanced-animation checkpoint, 2026-11-28,
following c9dc49f. User explicitly requested soft mesh skinning, texture crossfades
and automatic blend trees; those bounded 2D workflows are now implemented.
A05/A06/A07 release acceptance remains PARTIAL. Do not advance to A08 unasked.
At most three focused attempts per blocked check; record and move on. A03/A04
gates remain deferred. Full 3D remains behind G2D.

## Done

- ADR-0118: actual soft triangle skins with world mesh/bone bind poses, inverse
  binds and four normalized influences per vertex. Bounds: 64 bones, 256 vertices,
  512 triangles. Inspector Soft Mesh Skinning generates a grid, captures binds,
  edits references/positions/UVs/weights, normalizes and applies through undoable
  component transactions. Bone deletion/Transform removal rejects if referenced.
  Scene projection, export and logical save retain mesh data.
- Shared sprite renderer submits deformed triangles/per-corner UVs and blends two
  textures in premultiplied color. Correct transparent crossfade demonstrated by
  GPU readback. Compact attributes fit the downlevel limit. Existing culling,
  painter order/materials, prepared views and MAX_SPRITES warnings retained.
  Bounded 1024 texture-pair cache; native Arc/WASM Rc handles. Custom fragments
  call sample_sprite to honor fades. Each mesh triangle uses a queue entry.
- Authored graph transition Crossfade seconds and queued script API
  crossfade_animation(entity,clip,seconds), ID 142. Saved source/target clocks,
  exact endpoints, paused hold, seek cancellation and manual cuts. Destination
  owns events; old source keeps advancing silently. Active fade finishes before
  another graph edge, preventing unsupported three-texture interruption.
- Automatic per-state 1D blend trees use a declared parameter and 2..16 sorted
  clip thresholds. Adjacent clips mix without restarting time, normalized to the
  reference clip clock; reference owns events. Inspector authors thresholds and
  parameters through existing graph Apply/undo. Timed edges involving trees reject
  because each tree already uses two samples. Logical saves retain parameters.
- Restore applies the saved animation sample to Sprite immediately, including
  paused fades/trees. Legacy optional fade/tree fields default empty. Runtime
  contract v17; matching native/browser tools rebuilt; actual prior v16 SDK rejects.
  No dependency or VM layout change. New sample advanced_animation and CI checks
  observe both intermediate texture blends and final soft mesh deformation.
- Existing cutout FK/IK, audio/signal cues, Scene View preview, composer, saved
  tweens, character states, tilemaps and camera/view functionality retained.
  Guide: ADVANCED_ANIMATION_2D.md; evidence/A07_ADVANCED_ANIMATION.md.

## Executed validation

- Locked Rust workspace: 953 passing, zero failed, 104 suites. Five new editor
  integration tests cover multi-weight bind/deform, crossfade pause/seek/snapshot,
  automatic thresholds/time, editor Apply/undo/reopen and mesh/fade logical saves.
  GPU tests verify transparent blend readback and deformed triangle bounds, plus
  all prior renderer/material/prepared-view/capacity regressions.
- just check, strict WASM default/opt-out, 49 Python helper tests, Node syntax,
  py_compile and whitespace pass. Native and SDK builds complete before replays.
- Native/browser advanced sample passes: observed fade and tree mixed-pixel phases,
  final mesh green 6144 at [80,104,272,136], blue 64. Prior rig sample passes both:
  green tip 1024 at [224,104,256,136], blue 64, browser audio starts exactly once.
  Four final replays serial, relocated/authoring hidden; native PATH lacks tools.
- Logs target/a07-advanced-{focused,render,quality,workspace,wasm,wasm-optout,tools,
  web,python,stale}.log. Receipts/screenshots target/a07-advanced-{native,browser},
  target/a07-advanced-rig-{native,browser}. Final pre-commit check receipt:
  target/a07-advanced-precommit.log. Generated outputs are untracked.

## Open / deferred

- Full OS authoring/reimport acceptance, external Spine/DragonBones import and
  physical device/DPI/performance acceptance remain separate A07 release gates.
  Implemented automatic trees are 1D sprite-clip trees; bone pose tracks keep
  their existing two-take and IK/FK mixing. Renderer uses at most two texture
  samples per draw, with documented crossfade interruption/graph restrictions.
- A05 full split-screen authoring/host and remaining rendering/capacity acceptance;
  A06 stamps/streaming and complete level-authoring acceptance remain open.
  Animation 4096-cycle cap/extreme overload and physical performance acceptance
  remain open; no budgets bypassed. Direct camera-effect history stays transient.
- A04 source widget gate stopped after three attempts; saved-physics pose divergence
  remains. A03 identity/override/crash-transaction and A02 combined inspector gates
  deferred. No release/certification or full 3D acceptance claimed.

## Next

Consolidate A07 authoring/reimport acceptance or follow explicit user direction.
Preserve bounded attempts and deferred gates; do not advance past A07 unasked.
