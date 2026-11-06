# Task 7 local acceptance — 2026-11-06

These are actual native editor/player and Chromium captures, using the real
engine renderer and saved `samples/ui2d_editor` project. Native tests use Xvfb,
software graphics and an ALSA null output; browser tests use SwiftShader and real
Web Audio nodes. Each run uses disposable storage. Controller buttons are injected
through the browser Gamepad API, then polled by production input code.

- [Dragged label](editor-label-dragged.png): actual UI Designer add/drag, with
  offset `[104,80]` verified in saved TOML after undo/redo and process restart.
- [Editor resumed HUD](editor-resumed-hud.png): a fresh Play session restores
  health 66 and inventory `brass_key`; attached start count stays 1.
- [Browser settings](browser-settings.png): touch activates once, plays a sound
  and persists 25% volume using the saved TTF font/layout.
- [Browser saved pause](browser-saved-pause.png): scene, progress, script and menu
  state survive page reload and Continue; later player settings remain unchanged.
- [Browser quota failure](browser-save-error.png): readable failure feedback;
  exact previous slot bytes retained.
- [Browser corrupt load](browser-load-error.png): failure feedback without
  replacing live state or rewriting the corrupt slot.
- [Packed player resume](player-kar-resume.png): native player resumes the same
  saved health/inventory/script lifecycle in a fresh process from a real KAR.

`editor-result.json`, `browser-result.json` and `player-kar-result.json` record
assertions and observed values. Full logs/screenshots are generated under `target`
and uploaded by the CI workflow. Reproduce with `tests/native/task7_editor_smoke.py`,
`tests/native/task7_smoke.py` (`KILN_TASK7_ARCHIVE` selects a KAR) and
`tests/web/task7.cjs` after building native binaries/WASM bindings.

These checks do not certify physical controller compatibility, audible output,
remote CI results or clean-machine distribution. Task 8's real editor exporter
remains pending. See [GAME_SAVING_2D.md](../../GAME_SAVING_2D.md) for the complete
save contract and source/format compatibility limits.
