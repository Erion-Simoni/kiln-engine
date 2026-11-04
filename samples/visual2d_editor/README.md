# Visual authoring 2D sample

Open `kiln.toml` with File → Open Project. Left/right moves the blue character;
the wall blocks it. The sample includes a two-frame animation, an idle
transition, frame-event script state, a particle trail and two tile layers.

Use View → Sprite Editor / Animation Editor / Tilemap Editor / VFX Editor to
inspect or edit it. See [the authoring guide](../../docs/VISUAL_AUTHORING_2D.md)
for save/reopen, archive packing and the pending native visual acceptance.

`kiln-player/tests/visual_workflow.rs` generates this project through authoring
APIs and exercises it from disk and a real archive. Chromium also renders and
drives it in the browser smoke test.
