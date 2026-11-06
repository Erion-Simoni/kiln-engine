# Gameplay saves and player settings

Task 7 uses one versioned gameplay payload in editor Play, the native player and
`kiln-web`. Saves capture a completed fixed tick, independently of the old sample
score/coin state. The editor's project Save command still saves authored content;
F5/F9 during Play save/load gameplay slot 0.

## Script workflow

```ks
// Called by any scene or attached script, including a paused UI callback.
game_set("health", 73);
game_set("inventory", array_push(array_new(), "brass_key"));
if save_game(0) { ui_set_text("save", "Saving..."); }
```

`save_game(slot)` and `load_game(slot)` return whether the request was admitted,
not whether disk/storage finished successfully. Slots are integers 0–99. The host
services requests after the tick's commands have been applied. A successful load
replaces the timeline and discards subsequent requests from the old timeline.
Use `save_status()` (`idle`, `queued`, `saved`, `loaded`, `error`), `save_error()`
and UI events `save_complete`, `load_complete`, `save_failed`, `load_failed` to
report the result. Completion callbacks run while gameplay is paused.

`has_save(slot)` checks for a compatible gameplay payload; it does not guarantee
that every resource needed to restore it is currently available. `game_get(key,
default)` and `game_set(key, value)` share typed custom progress between scene and
entity scripts and across normal scene changes. Values support integers, finite
numbers, booleans, text and nested arrays. Script-local `state_get/state_set`
remains specific to each script instance.

A Continue callback usually calls `load_game(0)`, then closes the restored pause
screen or shows the HUD on `load_complete`. See the working callbacks in
`samples/ui2d_editor/scripts/ui.ks`. Loading does not rerun already-started entity
scripts. Do not initialize global progress unconditionally on every scene start.

## Saved contract

- Project and active scene identity; stable entity IDs, creation/deletion, parent
  links, local transforms including nonuniform scale, scene offsets/draw order,
  sprite appearance/resource references, UV/layout, roles and script properties.
- Rigid bodies, colliders, joints, velocities, live tile data and logical animation
  playback. Texture handles are resolved again from project resources.
- Root and attached script locals, instance state, lifecycle/timing, queued signals
  and scene request; shared custom game data and monotonic spawned-ID counter.
- Active UI screen, navigation stack/focus, dynamic text/visibility/enabled state.
  Pointer captures and held controls are cleared/suppressed until released.

Audio voices continue through an in-process load; they are not serialized.
Music starts through the normal startup callback in a fresh process. GPU objects,
input, collision contacts, solver caches and particle instances are transient;
particle definitions survive and restart. This is logical game restoration, not a
bit-for-bit replay of an engine process. Only the active world's entities are
saved; other scenes load their authored definitions when visited, with persistent
quest/inventory/progression carried in `game_*` values.

Expanded script source/locals signatures must match. Changing scripts, removing
required scripts/resources, using another project or loading a newer schema
reports an error before replacing the live world. The payload schema is version
1; there is no automatic migration from the old sample-only `SliceState` slots.
Back up/delete an incompatible slot explicitly to replace it. Existing corrupt or
future-version saves are preserved even when a new save is requested.

## Storage and failure behavior

Native slots use versioned `kiln-save` metadata and flushed temporary-file rename.
Editor Play stores them in `<project>/.kiln-save/`. The player uses its platform
user-data directory, scoped to the project ID by default; `--game-name NAME`
selects an alternate profile. Older players defaulted to `KilnGame`; use that
explicit profile name to retain their preferences. Legacy sample-only gameplay
slots still require explicit migration/replacement. Browser saves use same-origin localStorage keys
`kiln:<project-id>:slot:<slot>`. Browser preferences use
`kiln:<project-id>:preferences`; native preferences use `preferences.json` beside
the slots. A browser origin/profile change creates a separate storage area.

Preferences (`setting_get/setting_set`, audio master/buses) are versioned separately
and survive loading older gameplay. Failed preference writes roll back in-memory
changes. Corrupt preferences are retained and block replacement until repaired.
Quota/access errors and failed native writes report failure and retain the old
slot. Clearing browser data or deleting a native profile deletes its saves.

Limits: gameplay JSON 8 MiB, native envelope 16 MiB, 16,384 entities, 4,096 custom
keys, key length 256 bytes, text 64 KiB, arrays 4,096 items, nesting depth 16 and
16,384 nodes per value. Browser storage may enforce a smaller quota. No additional
storage service or dependency is required.

## Executed acceptance

- `kiln-runtime/tests/game_save.rs`: fresh-session/cross-scene restoration, typed
  inventory, stable IDs/deletion, parent/scale/velocity, attached lifecycle,
  completed-tick commands, animation rebinding/missing resources, corrupt/future
  data, changed scripts and independent preferences.
- `tests/native/task7_editor_smoke.py`: actual UI Designer add/drag/undo/redo,
  save/reopen and editor Play save/resume. Dragging uses the original press point
  and total displacement. The actual saved offset is verified in TOML.
- `tests/native/task7_smoke.py`: real player menus, audio commands/settings,
  health/inventory save, fresh-process Continue, script lifecycle and corrupt-slot
  preservation. Accepts `KILN_TASK7_ARCHIVE` for the same workflow from a KAR.
- `tests/web/task7.cjs`: rendered saved fonts/menus, browser Gamepad API polling,
  one touch/one event on an offset/scaled canvas, Web Audio looping/overlap,
  localStorage resume, independent preferences and quota/corrupt-save failures.
  Controller input is simulated through the browser API; it is not hardware proof.

Selected real captures/results are retained in `docs/evidence/task7-complete/`.
CI runs the native and browser acceptance scripts and uploads full artifacts.
Remote CI results, physical-controller behavior and audible output on real devices
remain separate checks. The real asynchronous export/distribution workflow is
Task 8; this checkpoint verifies the player/KAR and browser runtime directly.
