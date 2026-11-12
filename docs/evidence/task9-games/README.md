# Task 9 editor-authored games — 2026-11-12

The owner explicitly requested the three games on a new branch,
`feat/task9-editor-reference-games`, starting from `8ec7852`. Remaining Task 8
platform/release work stays open. This checkpoint covers the three reference
games and editor blockers they exposed, not all Task 9 release gates.

## Actual authoring

Projects under `games/` were created through native editor Save As from the
existing saved-UI and physics examples. Project names and independent save IDs
were entered in Settings. All new gameplay sources were pasted into the Script
Editor, saved through its normal controls, byte-compared with supplied source,
and syntax/runtime checked. No game manifests were written by a generator or
edited outside the editor. Artwork and documentation are ordinary supplied files.

Skybridge's two-frame loop was created in Animation Editor and assigned in the
inspector. Its first scene's script/name were set in Scene Editor. Duplicate
Scene copied the level and camera; the second hero's exposed stage property and
gate X were edited in the inspector. S/L/R bindings were entered through Input
Rebinding. Stop restores authoring state. The general source editor and scene
controls remain usable for other games; no game-specific engine entry point was
introduced (ADR-0073).

`tests/native/task9_editor_games.py --author` reproduces these operations into
fresh directories and compares **all serialized project values** to the tracked
projects. Source text is pasted through a real X11 clipboard and checked byte for
byte. It opens the saved projects with installed tools and no build tools in child
PATH, exports Native and Browser from the real panel, verifies all runtime/notice
bytes and tests overwrite preservation. Each export must finish its own background
verification; an older success message cannot satisfy the wait.

## Gameplay acceptance

- Rust `reference_games` replays each source project and a real packed KAR.
  Combat projectiles defeat all enemies and activate the relay; contact damage
  loses and restart restores a playable round. Puzzle inputs solve all three
  circuits with exact move counts and compare restored world/game data. Actual
  Rapier contacts ground/jump the platformer, its UV frames advance, and both
  physical level gates produce the expected transition/win.
- `task9_play_games.py` launches only standalone editor exports from unrelated cwd,
  with no archive argument/build tools and source project paths unavailable.
  It wins the games, saves/loads the second platformer scene and compares restored
  hero position, and continues the puzzle across a fresh player process.
- `tests/web/task9-games.cjs` serves only browser outputs under nested HTTP URLs.
  Real Chromium keyboard/mouse input wins each game. Puzzle reload/Continue occurs
  between circuits two and three, so the third solution proves progress restored.
  Platformer save/move/load compares hero position, then wins the far gate.
  Actual localStorage payloads establish progress; game state is never injected.
  Audio starts after browser consent; JavaScript/console errors fail the check.

## Reproduce

Build the editor/CLI/player and matching prepared browser runtime; prepare/extract
an intact portable installation as described in [PORTABLE_EDITOR.md](../../PORTABLE_EDITOR.md).
Then, with Linux GUI test dependencies and Playwright/Chromium available:

```sh
python3 tests/native/task9_editor_games.py --tools /path/to/installation --output target/task9-replay --author
python3 tests/native/task9_play_games.py target/task9-replay/result.json --output target/task9-native-play
node tests/web/task9-games.cjs target/task9-replay/result.json target/task9-browser-play
```

The branch's `reference-games.yml` workflow runs strict quality/tests, builds and
packages matched tools, repeats authoring/export, plays both targets and retains
small evidence artifacts for seven days. It has contents:read only and creates no
public release. A prepared local development archive is not a captured release or
independent reproducibility/OS-signing proof. Generated archives, logs, screenshots
and interactive desktop state stay under target/ outside Git; selected JSON below
records actual outcomes.

## Fixes found during validation and limits

- Simulated keystream entry dropped newlines/characters; ordinary clipboard paste
  now preserves exact source bytes. No truncated script is counted as a game.
- Export waiting must observe the new job's completion; clicking during background
  verification is disabled. The driver now waits for its own success increment.
- The animation driver waits for the New Clip form to collapse and checks the saved
  frame count before editing frame two; it never ignores an unexpected count.
- Platformer load now triggers on key release, preventing restored edge state from
  repeatedly reloading while the key is held. Acceptance compares actual saved
  positions rather than relying on a diagnostic message.
- A cloud restart terminated the first interactive GUI. Saved games survived and
  were reopened. Xvfb/SwiftShader/ALSA-null evidence is not a physical machine,
  audible sound/controller, Windows/macOS or reference-hardware performance test.
- Script drafts participate in normal Save/Discard/Cancel and atomic per-file
  writes, including external-change conflicts. Existing recovery snapshots still
  cover project documents; save source explicitly before relying on crash recovery.
- Independent unaided tutorial, long sessions/large projects, physical performance
  and final support/release reconciliation remain Task 9 work. Package size is not
  a goal; full 3D remains paused.

## Recorded local result

`local-validation.json` records all three final editor replays, six exports, native
and browser victories/restore checks, 680 passing Rust tests (zero skipped),
29 Python tests and strict/WASM/workflow/syntax checks. It binds final gameplay
sources by SHA-256 and retains the preparing tool archive's honest dirty/source
observations. `prior-task8-hosted-success.json` records the preceding a0f1d85
Windows/Linux candidates separately; those runs do not cover Task 9.
