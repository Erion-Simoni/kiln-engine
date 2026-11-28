# A07 soft skins, texture crossfades and automatic trees — 2026-11-28

ADR-0118 implements the three requested feature areas. The 2D sprite renderer now
accepts per-corner triangle positions/UVs and dual-texture premultiplied color
blending through compact attributes below the 16-attribute downlevel limit.
Painter order, custom vertex path, prepared views, culling and queue capacity
warnings remain shared. Texture pair cache is bounded at 1024 and resource
replacement invalidates prepared views as before. Native GPU resources use Arc;
WASM uses Rc for its non-Send handles.

Soft skins use world bind matrices and inverse bone binds with up to four
normalized influences per vertex. Mesh bounds: 64 bones, 256 vertices and 512
triangles; each triangle uses one existing frame queue entry. Inspector offers grid
creation, bind capture/recapture, editable bindings and per-vertex weights/UVs,
normalization and validated undoable Apply. Bone deletion/Transform removal rejects
when referenced. Scene documents, export and logical snapshots retain mesh data.

Timed graph transitions and crossfade_animation (ID 142,3 args) advance both clip
samples with a saved fade clock. Only destination/reference frame events fire.
Pause holds, seek cancels, manual play/stop cuts. Active fades finish before new
graph edges. Automatic 1D state-local trees select adjacent clip thresholds from
declared parameters, preserve clock continuity and synchronize normalized phase.
Reference clips own events. Trees accept 2..16 points per state; a timed edge
involving a tree rejects to preserve the two-texture limit. These are automatic
sprite-clip trees; authored bone pose tracks retain their existing two-take/IK
blending workflows. Custom fragments use sample_sprite to honor a texture blend.

Executed validation:

- Locked Rust workspace: 953 passing, zero failed, 104 suites. Five new editor
  integration tests cover bind reconstruction and actual multi-weight deformation,
  fade endpoint/pause/seek/corrupt restore, automatic tree thresholds and clock,
  weight Apply/undo/disk reopen, and logical mesh/fade save/restore. Restore now
  rebuilds the blended Sprite immediately, including when paused.
- Actual GPU readback verifies premultiplied blending of a half-alpha red texel
  toward a transparent green texel and prepared-view lifetime after begin_frame.
  A second GPU test checks deformed triangle pixel bounds/culling. Existing renderer
  material, sprite transform, capacity and prepared multi-view tests pass.
- just check, strict default and opt-out WASM Clippy pass. Python helper tests: 49
  pass; Node syntax, py_compile and whitespace checks pass. Matching editor, CLI,
  player and browser SDK v17 rebuilt. Actual previous v16 browser SDK export rejects.
  Logs target/a07-advanced-{focused,render,quality,workspace,wasm,wasm-optout,tools,
  web,python,stale}.log. No dependency or VM layout change.
- New advanced_animation exports pass native and browser checks, with source hidden
  and exports relocated. Both observe mixed red/green pixels separately for the
  timed fade and automatic tree before accepting final mesh bounds [80,104,272,136],
  green 6144 pixels, blue completion beacon 64. These checks exercise intermediate
  rendered textures, not only final states. Receipts/screenshots:
  target/a07-advanced-{native,browser}.
- Prior rigged_cutscene passes native/browser regression: green tip 1024 at
  [224,104,256,136], blue 64; browser Web Audio starts exactly once. Receipts:
  target/a07-advanced-rig-{native,browser}. All four final GUI replays ran serially
  after builds. Scope: Linux Xvfb/software graphics and Chromium/SwiftShader.

No full OS weight-painting/project-authoring session, external rig import,
physical device/DPI or production performance acceptance is claimed. The custom
weighted vertex workflow is implemented; full A07 release acceptance remains
PARTIAL. Generated artifacts stay untracked under target. Final pre-commit
just check/whitespace receipt: target/a07-advanced-precommit.log.
