# Native editor design evidence

Captured on 2026-11-05 from the built native `kiln-editor`, on a 1440×900 Xvfb
display with software OpenGL. These are actual application frames, not the HTML
reference or composited mockups. The test edits a disposable copy of
`samples/editor_design`; the repository sample remains unchanged.

- `2d_editor.png`: real hierarchy selection, inspector and engine-rendered scene.
- `3d_preview.png`: isolated lit perspective preview, entered from the Game tab.
- `3d_orbit.png`: actual pointer-drag orbit and the changed orientation compass.
- `play_paused.png`: actual runtime Play after moving the player, then pausing.
- `result.json`: verified saved position, rendered player positions through
  edit/reopen/Play/pause/step/stop, and byte-for-byte 3D project isolation.

Reproduce with `tests/native/editor_smoke.py` as described in
[EDITOR_DESIGN.md](../../EDITOR_DESIGN.md). Full logs and intermediate frames are
written to the selected evidence output directory and retained by CI as artifacts.
These captures do not establish cross-platform or physical-device acceptance.
