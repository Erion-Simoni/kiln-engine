# Saved 2D UI screens

Task 7 now has project-backed native UI: panels, labels and buttons, responsive
anchors, custom fonts, focus/navigation, script events and pausing. Native editor
Play and the player use the same layout, state machine and draw quads. Generic
world/game saves and browser UI integration remain unfinished.

## Author a screen

1. Save/open a project and choose **Window → UI Layout Editor** (or **UI Designer**).
   Both tabs edit the same `Project.ui` document. The old unsaved parallel models
   are removed. Saving, recovery and Save As use ordinary project persistence.
2. Enter a screen name and click **Add screen**. Choose the global Initial screen
   and Pause screen; optional per-scene initial overrides support levels/HUDs.
   Empty initial screen means no active UI. IDs remain stable across saves.
3. Add a Panel, Label or Button. Select it in the widget list or on the canvas.
   Drag to move, with configurable snapping; edit size, colors, font/alignment,
   visibility and enabled state in Properties. Earlier/Later changes paint order.
4. Set a Parent and an anchor/pivot preset. Anchors/pivots range from 0 to 1;
   offsets/sizes use logical pixels. Stretch adds parent size to the authored
   size, so a negative size creates margins. Children are clipped by ancestors.
   Parent cycles and broken references are rejected.
5. Buttons can emit an event, Show screen, Back, or Close UI. An optional event
   name replaces the widget ID. Navigation links override automatic directional
   selection; default focus falls back to the first visible enabled button.
6. Use **Undo UI / Redo UI** for UI document edits. These are separate from scene
   histories and the editor's scene Undo shortcut. Removing a widget also removes
   its descendants and repairs focus links; removing a screen repairs references.
   Undo restores the whole document, including those references.
7. Preview exercises the shared layout/button navigation; scripts execute in Play.
   Reference size controls logical scale; Preview size tests other aspect ratios.
   Runtime UI mutations affect a session copy, leaving the authored document intact.

Pause gameplay stops ordinary scene/entity updates, physics, animation and VFX.
UI input and `ui_event` callbacks continue. Controllers start once, including on
an initially paused title screen. Capture input also suppresses gameplay axes and
actions when simulation continues behind a modal screen. Back restores prior
screen/focus. Escape cannot dismiss a screen with Back allowed disabled.

## Fonts

The built-in font needs no assets. Import a TTF/OTF by font ID and file path in
Fonts, then assign that ID to labels/buttons. External resources copy to
`assets/fonts/`; collisions preserve the existing file. Undoing a font import
removes its reference, leaving the imported resource file available for reuse.

Custom fonts use real alpha-atlas glyphs, not the older incomplete SDF renderer.
ASCII, authored text and Extra characters are baked at 48 pixels into a 1024²
RGBA atlas. Each glyph draws one textured quad with real advance/bearing metrics;
newlines/alignment and geometry/UV clipping are shared across both native hosts.
Dynamic characters not baked in the atlas use `?`. Choose Extra characters for
non-ASCII runtime text. Missing/invalid resources display/log errors and use the
bitmap fallback. Reload fonts refreshes the authoring preview after external edits.

Limits: 128 screens, 1024 widgets/screen, 32 parent/screen-stack depth, 16 fonts,
2048 baked characters/font, 16 MiB input font size and 16384 UI draw quads. Startup
validation conservatively budgets up to 21 quads per bitmap character and one
per atlas glyph, including hidden widgets. Oversized runtime text changes fail
without replacing the previous text. Native atlas reloads release old textures.
Large physical font sizes may soften the fixed-resolution atlas; streaming/SDF
text, shaping, lists, scrolling, nine-slice styling and richer controls are outside
this saved-widget checkpoint.

## Input and scripts

Arrows move focus spatially. Tab/Shift+Tab cycles it; Enter/Space activates.
Mouse/touch activates on release over the captured button, with topmost/clipped
hit testing. Focus loss cancels capture. Native hosts retain bounded input edges,
so quick clicks/key taps between fixed ticks survive. Holding an activation key
does not trigger the next screen's default button.

Native builds enable the existing gamepad backend by default. D-pad navigates,
South/A/Cross activates, East/B/Circle goes back, and Start toggles pause/back.
Polling replaces held tokens on every frame, including release/disconnect; it
does not append stale input. Physical-controller acceptance remains pending.
Linux builds need `libudev-dev` and `libasound2-dev`. `--no-default-features`
disables audio/fonts/controller; selectively add `--features fonts,gamepad` etc.

Scene and entity scripts receive `script_phase() == "ui_event"`. The event has
its original screen/widget/name, even if its built-in action already switched
screens. Ordinary gameplay should be guarded separately:

```ks
if script_phase() == "update" {
    move_player(get_input_x() * 120.0 * get_delta_time(), 0.0);
}
if script_phase() == "ui_event" {
    if ui_event_name() == "volume_quiet" {
        set_volume(0.25);
        ui_set_text("volume_label", "Volume 25%");
    }
}
```

| Function | Result |
| --- | --- |
| `ui_show_screen(screen)` | Show/push an existing screen, or error for a missing ID |
| `ui_close_screen()` | Close UI and clear its back stack |
| `ui_screen()` | Current active screen ID, or empty |
| `ui_event_name()` / `ui_event_widget()` / `ui_event_screen()` | Current callback fields; empty outside that callback |
| `ui_clicked(id)` | Whether that saved widget activated this tick |
| `ui_set_text(id, value)` | Format a script value for an active-screen widget; false if missing/over budget |
| `ui_set_visible(id, bool)` / `ui_set_enabled(id, bool)` | Update an active-screen widget; repair focus; false if missing |

UI callbacks use zero gameplay delta and do not increment the ordinary gameplay
frame counter. UI state/context is returned to the scene even after a VM error;
source-path/line diagnostics remain available. Explicit UI callbacks may still
issue intentional world changes, signals or scene requests. Existing projects
without saved screens keep their prior scheduling and launcher menu behavior.
The older procedural `ui_button/ui_label/ui_panel` API is not this saved workflow.

## Demo and executed evidence

Open `samples/ui2d_editor/kiln.toml`, or run:

```bash
cargo run -p kiln-player -- --archive samples/ui2d_editor --game-name ui2d-demo
```

The player starts the authored title automatically. Settings changes text and
persists 25% master volume. Start closes UI and moves the blue actor; Escape
opens Pause. Pause → Game over opens the fourth screen without replacing the
world. Resume demo continues that world; it does not claim generic save/restore.
Title music and button effects reuse the small synthesized audio fixture.
The included font is a renamed DejaVu subset with its license beside it.

```bash
just check
just test
KILN_TASK7_UI_SAMPLE_OUT=samples/ui2d_editor cargo test -p kiln-player --test ui_workflow authored_screens_survive_restart_archive_and_editor_player_gameplay
cargo build -p kiln-player --bin kiln-player
python3 tests/native/ui_smoke.py
```

The native smoke requires Xvfb, xdotool, ImageMagick import and a software graphics
backend. A Linux CI job runs it and retains evidence; remote execution is pending. It starts an isolated profile, opens a real window, clicks the menus,
checks actual event traces/settings and verifies Escape preserves the session.
Screenshots/logs go to a temporary directory or `KILN_UI_EVIDENCE_DIR`.
Executed player screenshots: [Title](evidence/task7-ui/title.png),
[Settings](evidence/task7-ui/settings_changed.png), [Pause](evidence/task7-ui/pause.png),
[Game over](evidence/task7-ui/game_over.png). These are real native captures.

Disk/KAR tests compare project persistence, editor/player geometry/colors/UVs,
script callbacks and frozen/resumed gameplay. Additional tests cover focus,
clipping, rapid input, invalid documents, real font coverage/metrics, atomic UI
transactions and opening the panel without dirtying data. Full native editor
panel/drag/save/Play interaction acceptance is still pending; a virtual-display
launch succeeded, but its automated toolbar/menu click flow was inconclusive.
See STATE.md for current validation counts and limits. Browser saved UI/audio/
settings parity, general game saves and real export/distribution remain open.
