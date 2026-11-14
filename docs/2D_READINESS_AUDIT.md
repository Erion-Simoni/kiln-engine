# 2D readiness audit — 2026-11-05

> **Historical plan / technical reference.** The [unified engine completion plan](ENGINE_COMPLETION_PLAN.md) is authoritative for current scope, order, status and release gates. Older completion claims and package-size goals below are historical; preserve their evidence, but do not use them to close current milestones.


**Verdict: the complete 2D engine is not finished.** Task 7 is in progress; Tasks 8–9 remain pending,
native workflow acceptance is incomplete, and there are additional scripting
and browser integration gaps. Passing automated tests establishes the covered
behavior; it does not establish Unity/Godot/GameMaker feature parity.

Reviewed the feature branch at `20291fa` and GitHub master at `76da93f`
(merge PR #17). Their source trees were identical. The original audit was
documentation only; subsequent Task 7 audio/UI checkpoints update the resolved findings.

## Fresh verification

- `just check`: passed. Includes formatting, strict native workspace/all-target
  Clippy, workspace check and the separate audio sample compilation check.
- `just test`: **592 passed, zero failed/skipped**.
- Browser rendering/input and native/WASM builds passed in the Task 6 checkpoint.
  They were not rerun during this source audit. Audible playback, native GUI
  authoring, clean-machine distribution, other browsers/devices and release
  budgets are not certified by these results.

## Task 7 audio checkpoint verification

The subsequent native audio/settings implementation passes `just check` and
**603 workspace tests with zero skipped**, native editor/player builds, individual
no-audio package checks, WASM build and Chromium rendering/input smoke. The
standalone audible sample also opens/plays/completes through ALSA's null output;
that is device-path execution, not audible listening evidence. See STATE.md and
AUDIO_AUTHORING_2D.md. The following saved-UI checkpoint extends that implementation;
generic game restoration and browser mixer/preferences parity remain unfinished.

## Task 7 saved native UI checkpoint

Project-backed screens/widgets/fonts, responsive anchors/parent clipping, native
editor/player rendering, focus/keyboard/controller tokens, script UI callbacks,
paused gameplay scheduling and player settings integration are implemented.
`just check` and **622 workspace tests with zero skipped** pass. Real author/
save/reopen/KAR tests compare geometry/colors/UVs/events and frozen/resumed worlds. Xvfb native player mouse/keyboard flows pass, including custom
font rendering, dynamic text, persisted volume and Escape preserving the session;
see UI_AUTHORING_2D.md and retained PNGs. Full editor GUI click-through and physical
controller acceptance remain pending. Browser saved UI is still not integrated.

## Remaining planned work

| Area | Current result | Required before calling it finished |
| --- | --- | --- |
| Tasks 1–6: foundation and authoring | Local build/tests pass; scene, asset/prefab, native scripting, physics/cameras and visual authoring implementations exist | Complete native workflow acceptance and address the limits below |
| Task 7: audio, UI, saves/settings | Native audio/mixer/player settings and saved runtime UI/screens/fonts/navigation/events implemented | Audible/full editor GUI/controller acceptance, browser parity, general versioned game saves |
| Task 8: editor export/distribution | Real archive packing/player loading works; editor build UI still simulates completion | Real asynchronous build process, actual failures/logs/cancellation, project-specific assets/player packaging and clean-machine launches |
| Task 9: games/release gate | Small vertical-slice/physics/visual fixtures exist | Complete top-down action, platformer and puzzle games; unaided tutorial, recovery/long-session/large-project and hardware/export validation |

## Confirmed implementation gaps

1. **The Build/Export panel reports success without building.**
   `kiln-editor/src/build_panel.rs:228` sets the panel state and appends
   `Build completed!`; the code explicitly simulates execution. This is a
   shipping blocker. Real CLI packing elsewhere does not fix this UI path.

2. **Native sound-registration defect resolved; device acceptance remains.**
   Task 7 replaces empty players with decoded PCM, independent SFX voices and
   one replaceable looping music voice. Audio Editor now edits saved project
   assets/mixer, imports relative resources and previews independently of Play.
   Native editor/player enable audio by default and load from disk/archive bytes.
   Offline PCM and real archive/VM acceptance cover this path. Listening on real
   speakers/headphones, native GUI preview and browser audio parity remain
   pending. See AUDIO_AUTHORING_2D.md. Typed player preferences now persist
   independently of sample saves; this does not resolve finding 4.

3. **Saved native UI implemented; broader interaction acceptance remains.**
   Both UI editor tabs edit Project.ui with document undo/redo. Native Play and
   player share layout, clipping, focus, input, script events and alpha-atlas font
   rendering. A title/pause/settings/game-over fixture survives disk/archive
   roundtrips and runs in a real native player window on Xvfb. Full native editor
   panel/drag/save/Play interaction remains pending: the editor launched, but its
   automated toolbar/menu click-through was inconclusive. Physical-controller,
   browser UI and richer controls are not certified by these results.

4. **Gameplay saves remain tied to the vertical slice.**
   `SliceState` at `kiln-runtime/src/slice.rs:311` contains score, elapsed/status,
   collected pickup IDs and player coordinates. `save_state` serializes it and
   labels it `vertical slice`. Attached script state is stored separately and
   is not included. The save library has useful infrastructure, but a generic
   game/scene/entity/script-state save-and-restore workflow is still needed.

5. **Browser gameplay is behind native gameplay.**
   `kiln-web/src/lib.rs:216` loads only `scene.script` with
   `SliceSession::with_action_map`, rather than `with_scene_sources`. Its host
   does not process the native entity attachment/module lifecycle, scene-change
   requests or the shared gameplay camera controller. Its runtime dependency
   does not enable physics. The current WebGL sprite/input smoke tests do not
   establish parity for a physics game with attached enemy/trigger scripts.

6. **KilnScript still has general-gameplay limitations.**
   `kiln-script/src/lexer.rs:192` and `parser.rs:117` provide function/if/return/
   variable statements, but no `for`/`while` grammar. Source/instruction
   breakpoints and arbitrary watches are also outside the current frame-level
   debugger contract. Functions, modules, bounded arrays and per-instance state
   work within the tested native contract; they are not the full traditional
   engine scripting toolset.

7. **Scripted entity spawning ignores the requested texture.**
   `kiln-runtime/src/slice.rs:2075` discards `texture_path` and spawns a white
   32×32 sprite. This differs from the working image drag/drop and authored
   scene texture paths. Runtime instantiation/resource binding needs acceptance
   driven by the full reference games.

## Validation and release gaps

- Tasks 2–6 have explicit pending native graphical acceptance: actual drag/drop,
  gizmos/panels, Play, save/restart/reopen and packed-player interaction. The
  current runner lacks X11/Wayland; previous narrow GUI checkpoints do not
  validate all later authoring changes.
- Real audio playback and simultaneous/looping behavior need device evidence.
- A KAR archive is not a complete editor-driven, project-specific installer or
  portable build. Current native CLI export compiles/copies the player; the
  selected project's resources, launch configuration and clean-machine behavior
  still need the Task 8 workflow.
- No completed three-game release acceptance, independent tutorial validation,
  full platform matrix, physical reference-hardware testing or current release
  size/performance gate was established here.
- Older roadmap completion claims are not release evidence. Use
  `2D_COMPLETION_PLAN.md` plus executed acceptance records as the current scope.

## Recommended order

Proceed with Task 7, then Task 8, then Task 9. Track scripting/spawn and browser
parity as explicit follow-ups required before the release gate. Complete native
workflow checks as graphical machines become available. Keep 3D paused.
