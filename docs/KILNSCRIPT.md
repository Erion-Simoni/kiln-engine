# KilnScript Language Reference

KilnScript is Kiln's text-based gameplay language. A KilnScript program compiles to the engine's **IR bytecode** and runs on the same VM as node graphs (`kiln-graph`). A script runs **once per frame**, reads *facts* about the current frame, and emits *commands* that the engine applies after the script returns. The script never touches the ECS world directly — that keeps gameplay logic safe, serialisable and identical between the editor and the standalone player.

## Source File

KilnScript source lives in `.ks` files, referenced from a scene in `kiln.toml`:

```toml
[scenes.main]
script = "scripts/main.ks"
```

## Language Surface

### Literals

| Kind | Example | Notes |
|------|---------|-------|
| Integer | `42` | 64-bit signed |
| Float | `3.14`, `2.5` | 64-bit float |
| String | `"hello"` | UTF-8 |
| Boolean | `true`, `false` | |

### Keywords

`if`, `else`, `return`, `let`, `call`, `fn`, `while`, `break`, `continue`

### Operators

| Operator | Meaning |
|----------|---------|
| `+` | Add |
| `-` | Subtract / negate |
| `*` | Multiply |
| `/` | Divide |
| `==` | Equal |
| `!=` | Not equal |
| `<` | Less than |
| `<=` | Less than or equal |
| `>` | Greater than |
| `>=` | Greater than or equal |
| `=` | Assign to existing variable |
| `+=`, `-=`, `*=`, `/=` | Update an existing variable |
| `!` | Boolean negation |
| `&&` | Short-circuit boolean AND |
| `\|\|` | Short-circuit boolean OR |

Numeric operations promote `I64` to `F64` when mixed with floats (e.g. `1 + 2.5` yields `3.5`). Integer division by zero is a runtime error; float division follows IEEE-754.

### Statements

- `let name = expr;` — declare a local variable (cannot redeclare in the same scope)
- `name = expr;` — assign to an existing variable
- `expr;` — expression statement (e.g. a function call)
- `return expr;` — return from the program
- `if cond { ... } else if cond { ... } else { ... }` — conditional; blocks get their own scope
- `while cond { ... }` — repeat while a boolean condition is true
- `for (let i = 0; i < count; i += 1) { ... }` — scoped counted loop
- `break;` — exit the innermost loop in this function
- `continue;` — test the innermost loop again (run its increment first for `for`)

### Gameplay control flow

`while` tests its condition before each iteration; its body may run zero times.
Block-local declarations are initialized on each iteration and are not visible
outside the loop. Assignments can update existing outer locals. `break` and
`continue` apply to the innermost loop in the same function; an invoked helper
cannot control its caller's loop. A function's `return` exits that function even
inside nested loops.

```kilnscript
let i = 0;
while i < 5 {
    i += 1;
    if i == 2 { continue; }
    else if i == 5 { break; }
    print(i); // prints 1, 3, 4
}
```

The operators `!`, `&&` and `||` require booleans for evaluated operands and
return booleans. `&&` skips its right side when the left is false; `||` skips it
when the left is true. Thus a skipped host call has no side effects.
For example, `false && probe()` does not call a registered `probe` host function.

Precedence, from highest to lowest: parentheses/calls, unary `!` and `-`,
`*` and `/`, `+` and `-`, comparisons, `&&`, `||`. Operators at the same
binary precedence associate left to right. Compound assignments are statements,
not expressions, and require an existing local.

The existing VM instruction budget also applies to loops: runaway execution
returns an instruction-budget error. Keep per-frame work bounded; do not use
a loop to wait for time or input. Parser limits reject excessive statement/unary
nesting and boolean chains.

Compatibility: `while`, `break` and `continue` are newly reserved words.
Rename old variables/functions using those names. Chained division now groups
left to right: `8 / 4 / 2` is `1`; the previous parser incorrectly produced
`4`. Use explicit parentheses when a right grouping is intended. Unary minus
now binds before multiplication/division.

### Functions (Builtins)

Builtins are host functions registered by the runtime. The standard slice builtins:

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `print(...)` | 1 | — | Print to console |
| `get_input_x()` | 0 | f64 | Horizontal input axis (-1 to 1) |
| `get_input_y()` | 0 | f64 | Vertical input axis (-1 to 1) |
| `get_elapsed()` | 0 | f64 | Seconds since slice start |
| `get_score()` | 0 | i64 | Current score |
| `add_score(n)` | 1 (i64) | — | Add to score |
| `get_coin_count()` | 0 | i64 | Pickups still on the field |
| `collect_coin()` | 0 | bool | Collect one pickup in reach (24 units) |
| `move_player(dx, dy)` | 2 (f64, f64) | — | Move player by delta |
| `set_status(msg)` | 1 (str) | — | Set HUD status message |
| `get_status()` | 0 | str | Current status message |
| `get_delta_time()` | 0 | f64 | Seconds this frame |

### Audio builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `play_sound(name)` | 1 (str) | — | Play a sound effect |
| `stop_sound(name)` | 1 (str) | — | Stop a sound effect |
| `play_music(name)` | 1 (str) | — | Start music playback |
| `stop_music()` | 0 | — | Stop music playback |
| `set_volume(volume)` | 1 (f64) | — | Master volume 0–1 |
| `set_sfx_volume(volume)` | 1 (f64) | — | SFX volume 0–1 |
| `set_music_volume(volume)` | 1 (f64) | — | Music volume 0–1 |
| `pause_sound(name)` | 1 (str) | — | Pause a sound effect |
| `resume_sound(name)` | 1 (str) | — | Resume a sound effect |
| `pause_music()` | 0 | — | Pause music |
| `resume_music()` | 0 | — | Resume music |
| `stop_all_sounds()` | 0 | — | Stop all sounds |
| `sound_playing(name)` | 1 (str) | bool | Whether a sound is playing |
| `music_playing()` | 0 | bool | Whether music is playing |
| `set_bus_volume(name, volume)` | 2 (str, number) | — | Set named bus or Master gain; persist native player override |
| `get_bus_volume(name)` | 1 (str) | f64 | Current configured bus or Master gain |

Native audio assets and buses are authored in the Audio Editor and saved with
Project. Repeated `play_sound` calls create overlapping voices; `play_music`
replaces the previous track and respects its Loop option. Master/bus volume
setters persist to player preferences when a settings directory is configured.
A preferences write failure produces a script error. Browser audio parity is
still pending; see [Audio authoring](AUDIO_AUTHORING_2D.md).

### Player settings

`setting_get(key, fallback)` returns a saved setting or the fallback.
`setting_set(key, value)` persists an integer, finite float, boolean, string or
array and returns that value. These settings are shared across scene/entity
scripts and survive restart. They are separate from `state_get`/`state_set`,
which remain per-instance memory, and from game save slots. Setters require a
configured writable directory; failures are reported as script errors. Corrupt
or unsupported preferences files block writes until repaired and reloaded.

```ks
setting_set("language", "sq");
let language = setting_get("language", "en");
```

Native hosts configure the directory before the first script callback. The
browser localStorage adapter remains unfinished.

### Entity builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `spawn_entity(name, x, y, texture_path?)` | 4 (str, f64, f64, str?) | — | Spawn a new sprite entity |
| `despawn_entity(source_id)` | 1 (str) | — | Despawn an entity |
| `get_entity_pos(source_id)` | 1 (str) | [x, y] or false | Read entity position |
| `set_entity_pos(source_id, x, y)` | 3 (str, f64, f64) | — | Set entity position |
| `get_entities_with_role(role)` | 1 (str) | [source_id...] | Find entities by role |
| `get_entity_component(source_id, component_name)` | 2 (str, str) | value | Read a component value |
| `set_entity_component(source_id, component_name, value)` | 3 (str, str, value) | — | Set a component value |
| `find_nearest(x, y, role)` | 3 (f64, f64, str) | source_id or false | Find nearest entity with role |
| `raycast(x, y, dx, dy)` | 4 (f64, f64, f64, f64) | [source_id, distance, normal_x, normal_y] or false | Solid geometry query, maximum distance 100 |
| `raycast_query(x, y, dx, dy, distance, mask)` | 6 (numbers; integer mask) | [source_id, distance, normal_x, normal_y] or false | Ray query with range and membership bitset |
| `overlap_circle(x, y, radius, mask)` | 4 (numbers; integer mask) | [source_id...] | Actual shape overlap; includes sensors, sorted unique IDs |
| `get_frame_count()` | 0 | i64 | Current frame number |
| `get_time_scale()` | 0 | f64 | Current time scale |
| `set_entity_vel(source_id, vx, vy)` | 3 (str, f64, f64) | — | Set entity velocity |
| `get_entities_at_position(x, y, radius)` | 3 (f64, f64, f64) | [source_id...] | Find entities near position |
| `get_entity_role(source_id)` | 1 (str) | str | Read entity role |
| `add_component(source_id, component_type, value_json)` | 3 (str, str, value) | — | Add a component |
| `remove_component(source_id, component_type)` | 2 (str, str) | — | Remove a component |

### Animation builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `play_animation(source_id, clip_name)` | 2 (str, str) | — | Play an animation clip |
| `stop_animation(source_id)` | 1 (str) | — | Stop animation |
| `set_animation_speed(source_id, speed)` | 2 (str, f64) | — | Set playback speed |
| `animation_event_name()` | 0 | str | Frame event name during the attachment's `animation_event` phase |
| `animation_event_value()` | 0 | str | Frame event text payload, or empty string |
| `get_collision_events()` | 0 | [[a, b, started]...] | Collision events this frame |

Authored clips use decoded sheet dimensions, per-frame duration/pivot, optional
frame events, looping and a non-looping next-clip transition. Attached scripts
receive each crossed event once, including skipped frames and loop boundaries:

```kilnscript
if script_phase() == "animation_event" {
    if animation_event_name() == "footstep" {
        state_set("surface", animation_event_value());
    }
}
```

Repeated `play_animation` requests for the currently playing clip preserve time.
All project clips are available to a scene sprite, including clips using another
sheet. See [Visual authoring](VISUAL_AUTHORING_2D.md) for the shared preview and
runtime contract. Particle start/stop APIs toggle emission and preserve emitter
configuration while live particles finish their lifetimes.

### UI builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `ui_button(id, x, y)` | 3 (str, f64, f64) | bool | Draw a button, returns clicked |
| `ui_label(id, x, y, text)` | 4 (str, f64, f64, str) | — | Draw a label |
| `ui_panel(id, x, y, w, h)` | 5 (str, f64, f64, f64, f64) | — | Draw a panel |
| `ui_clicked(id)` | 1 (str) | bool | Check if button was clicked |

### Input builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `get_action(action_name)` | 1 (str) | bool | Whether an action is pressed |
| `get_action_axis(action_name)` | 1 (str) | f64 | Axis value -1 to 1 |
| `get_mouse_world_x()` / `get_mouse_world_y()` | 0 | f64 | Pointer mapped through the gameplay camera and physical viewport |

Physics queries use world coordinates. Directions are normalized, distances are
in world units, and attached scripts exclude their own body from rays. Ray
queries skip sensors. Membership/mask values are integer bitsets (1, 2, 4, …);
zero matches nothing. Without the optional physics backend, rays return false
and overlaps return an empty array. Task 5 replaces the old provisional ray
result layout: use stable source ID at index 0 and distance at index 1.

`get_entity_component(id, "velocity")` reads a body's world linear velocity as
`[vx, vy]`. `set_entity_vel` changes the physics body for the next fixed tick;
preserve its current Y velocity when changing horizontal movement. Position
getters/setters address authored local transforms. The mouse world APIs are
provided by the native editor/player; custom hosts must call `set_mouse_world`.
See [Physics and cameras](PHYSICS_2D.md) and the checked-in editor sample.

### Localization builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `localize(key)` | 1 (str) | str | Localized string for key |

### Save/load builtins

| Function | Arguments | Returns | Description |
|----------|-----------|---------|-------------|
| `save_game(slot_id)` | 1 (i64, 0–99) | bool | Queue a completed-tick gameplay save; true means admitted |
| `load_game(slot_id)` | 1 (i64, 0–99) | bool | Queue an atomic gameplay restore; true means admitted |
| `game_get(key, default)` | 2 (str, value) | value | Shared persistent game data, independent of sample score/coins |
| `game_set(key, value)` | 2 (str, value) | value | Set validated typed game data shared across scene/entity scripts |
| `has_save(slot_id)` | 1 (i64, 0–99) | bool | Compatible gameplay payload exists |
| `save_status()` | 0 | str | idle / queued / saved / loaded / error |
| `save_error()` | 0 | str | Last storage/restore failure, empty after success |

UI completion events: `save_complete`, `load_complete`, `save_failed`,
`load_failed`. They dispatch even while gameplay is paused. F5/F9 in native Play
and the browser use slot 0. See [GAME_SAVING_2D.md](GAME_SAVING_2D.md) for the
snapshot contract, version limits, storage profiles and Continue example.


```ks
let speed = 120.0;
let dt = get_delta_time();
let dx = get_input_x() * speed * dt;
let dy = get_input_y() * speed * dt;
move_player(dx, dy);

if collect_coin() {
    add_score(10);
    set_status("PICKED UP A COIN");
}

if get_coin_count() == 0 {
    set_status("ALL COINS COLLECTED");
}
```

## Values and Types

The VM supports four value types:

- **`I64`** — 64-bit signed integer
- **`F64`** — 64-bit float
- **`Bool`** — boolean
- **`Str`** — heap-allocated UTF-8 string

Integers and floats compare numerically (`1 == 1.0` is true). Booleans and strings only compare with their own type.

## Local Variables

Variables live in the VM and are **reset every time the program starts running**. A per-frame script must keep persistent state in the host (the slice state: score, elapsed time, player position, collected coins). Use `let` for frame-local temporaries.

Variables are block-scoped: `if` blocks get their own scope, so a `let` inside an `if` does not leak out.

## Compilation Pipeline

```
KilnScript source (.ks)
    → lexer (tokens)
    → parser (AST)
    → compiler (IR bytecode)
    → VM (executes once per frame, calls host builtins)
```

Compile errors produce diagnostics with source spans (undefined variable, undefined function, duplicate variable, wrong argument count). Unknown functions are a compile error.

## Entity scripts and modules

In the Inspector, choose **Create Movement Script** to create and attach an arrow/WASD starter, or **Attach Script** to select a project `.ks` file. **Open Script** opens the native Script Editor; Validate and Save Script apply source changes for the next Play. Double-click `.ks` assets to edit other scripts. Add
numeric property names and values there, then read them with
`get_property("speed")`. Attachments, property defaults, duplication, undo, and
save/reopen use the authored scene document. Each entity gets its own runtime
state store and stable `self_entity()` reference.

```kilnscript
import "helpers/movement.ks";

if script_phase() == "start" {
    state_set("distance", 0.0);
}
if script_phase() == "update" {
    let x = state_get("distance", 0.0) + get_property("speed") * get_delta_time();
    state_set("distance", x);
    set_entity_pos(self_entity(), x, 0.0);
}
if script_phase() == "collision_enter" {
    emit_signal("hit", event_entity());
}
if script_phase() == "signal" {
    if signal_name() == "level_complete" { change_scene("level_2"); }
}
```

Lifecycle phases are `start` once, `update` each gameplay tick, `signal` for
queued signals, `collision_enter`/`collision_exit` for backend collision events,
and `stop` when the attachment/world is removed. `event_entity()` identifies
the other collider; `signal_value()` reads a signal's payload. Signals emitted
during a tick are delivered on the next tick in stable entity order, with a
256-event limit. Collision generation requires a physics backend; the runtime
also exposes `queue_collision_event` for custom backends.

Modules use top-level `import "relative/path.ks";` directives, on their own
line. Paths are relative to the importing resource and must stay inside the
project. Repeated imports expand once; import cycles are errors. Helpers may
declare named functions:

```kilnscript
fn advance(x, speed) {
    if speed > 0 { return x + speed; }
    return x;
}
```

Functions can call other named functions and builtins, receive evaluated
arguments, and have isolated locals. Recursive/nested functions and closures
are rejected; no explicit return yields integer zero. `array_new()`,
`array_push(array, value)`, `array_length(array)`, and `array_get(array, index)`
provide collections for entity queries and persistent state. Arrays contain
at most 4096 items; `array_push` returns a new array. Bad indices report runtime
errors with source file/line/column. `state_set` stores values independently
per attachment across ticks, with at most 4096 keys; Stop/Restart/scene changes
create fresh instance state. Game-save serialization of arbitrary attachment
state is separate from this per-instance runtime persistence.

Existing top-level scene scripts still execute once per gameplay tick and need
no migration. `fn` is now reserved for function declarations and top-level
`import` is a module directive; rename legacy variables using those names.
The native editor and player both use `SliceSession::with_scene_sources`.
The live debugger controls real gameplay at frame granularity and displays
actual instance state; instruction breakpoints/watch expressions are not part
of this debugger surface.

## See Also

- `docs/TUTORIAL.md` — build your first game with KilnScript
- `kiln-script/src/` — lexer, parser, compiler source
- `kiln-ir/src/vm.rs` — bytecode VM
- `kiln-runtime/src/slice.rs` — slice runtime and builtins

## Saved native UI screens

Author screens through UI Layout Editor/UI Designer, then handle buttons in
`script_phase() == "ui_event"` separately from normal `"update"` gameplay.
`ui_event_name()`, `ui_event_widget()` and `ui_event_screen()` identify the callback.
`ui_show_screen(id)`, `ui_close_screen()`, `ui_screen()` and `ui_clicked(id)` manage
or inspect state. `ui_set_text(id, value)` formats an active-screen label/button;
`ui_set_visible(id, bool)` and `ui_set_enabled(id, bool)` change flags and repair
focus. Missing widgets/invalid text changes return false; invalid argument types
or unknown screens produce script diagnostics.

An initially paused title starts attached controllers once. UI callbacks continue
while gameplay/physics/animation are paused; their delta is zero. Guard ordinary
gameplay with the update phase so menu events cannot run movement twice. Screen
actions apply before callbacks, whose event screen still names the originating
screen. See [Saved UI](UI_AUTHORING_2D.md) and `samples/ui2d_editor` for the full
native contract, limits, fonts, navigation and remaining browser/save acceptance.


## Counted loops and dictionaries

A counted loop requires `let name = value`, a boolean condition, and an assignment
increment, separated by semicolons inside parentheses. `i++`, omitted clauses and
`for-in` are not supported. The initializer may shadow an outer local; its name
is visible only in that loop. The body has its own scope. `continue` runs the
increment before testing again; `break` skips it. Functions isolate loop controls.
`for` is now a reserved keyword; rename older identifiers using that spelling.

```kilnscript
let enemy = dict_new();
for (let i = 0; i < 4; i += 1) {
    if i == 1 { continue; }
    enemy = dict_set(enemy, "health", dict_get(enemy, "health", 0) + 10);
}
state_set("enemy", enemy); // health = 30
```

Dictionaries hold string-keyed values, including arrays and other dictionaries.
Like arrays they have value semantics: assign the result of `dict_set` or
`dict_remove` to keep a change. Independent copies do not share mutable storage.

| Function | Behavior |
| --- | --- |
| `dict_new()` | Empty dictionary |
| `dict_get(dictionary, key, fallback)` | Value or fallback for an absent key |
| `dict_set(dictionary, key, value)` | Copy with one key inserted/replaced |
| `dict_has(dictionary, key)` | Boolean key presence |
| `dict_remove(dictionary, key)` | Copy without the key; absent keys are allowed |
| `dict_keys(dictionary)` | Array of keys in sorted order |

Keys are at most 256 UTF-8 bytes, with at most 4096 entries per dictionary. A
mutation validates the existing saved-value limits (16 levels, 16384 total values,
65536-byte strings, finite numbers). Invalid types or limits return runtime
errors. Equality compares keys and recursively compares values, including the
existing numeric-aware integer/float comparison. Logical game saves and player
preferences preserve nested values and large integers. Save files containing
new dictionary variants require an engine version that supports them.

A script's `let` variables are local to its program execution. Persistent
`state_get`/`state_set` storage belongs to the current scene program or individual
attached entity script; it is not a shared cross-scene global singleton. Use the
existing save/scene mechanisms deliberately. No implicit process-global scope or
timer behavior is implied by dictionary mutation; explicit tasks are described below.

The Script Editor colors keywords, literals, comments and built-ins, including
unfinished text. Its API Reference filters the same built-in registry used by
scripts, graphs, native games and browser games. The reference currently displays
argument counts and call insertion. Formatting and bounded project search/navigation
are described below.

Execution defaults to 1,000,000 bytecode instructions per tick. The host's VM
instruction-limit setter now governs both full runs and debugger steps, including
a zero limit that prevents execution. A restart gets a fresh configured budget.
The bytecode debugger executes steps, resumes past the breakpoint it just
reported, exposes local slots and follows actual bytecode call depth for Step
Over/Out. Restart clears bytecode locals and stacks while retaining host state.
Conditional bytecode breakpoints return an explicit unsupported diagnostic.

The live gameplay panel currently pauses/steps whole frames and inspects entity
state. Source breakpoints, statement stepping, imported-function call frames,
expression watches and a debugger attached to the actual gameplay VM remain
required before A04 is accepted. Bytecode repairs do not establish those workflows.

## Enums, tasks and source tools (A04)

Top-level enums have 1–256 unique variants. Values are immutable qualified
constants backed by stable strings (`Stage::Warmup`), so saved task payloads keep
meaning if declarations are reordered. Nested enum declarations and member
assignment are rejected. `enum` is reserved; rename older identifiers using it.
Functions/calls accept at most 256 parameters/arguments.

```kilnscript
enum Stage { Warmup, Fire }
if !state_get("started", false) {
    task_schedule("enemy", 0.25, Stage.Warmup);
    state_set("started", true);
}
if task_ready("enemy") {
    let stage = task_take("enemy", Stage.Warmup);
    if stage == Stage.Warmup {
        task_schedule("enemy", 0.25, Stage.Fire);
    } else {
        state_set("projectiles", state_get("projectiles", 0) + 1);
    }
}
```

Tasks are cooperative state machines: take a due resume payload, execute a helper,
then schedule the next stage explicitly. There are no automatic suspended function
stacks or `yield` syntax. Each scene/entity script has its own task scope. Game
pause stops task time; logical save/restore retains deadlines and typed payloads.
Use different names for independent tasks; rescheduling replaces an existing one.

| Function | Behavior |
| --- | --- |
| `task_schedule(name, delay, payload)` | Replace/schedule a task; returns its name |
| `task_ready(name)` | True when a live task's deadline is reached |
| `task_take(name, fallback)` | Consume a due payload once; otherwise fallback |
| `task_cancel(name)` | Remove a task; returns whether it existed |
| `task_exists(name)` | Test presence, including tasks that are already due |
| `task_remaining(name)` | Nonnegative seconds remaining; absent tasks return 0 |
| `timer_start(name, delay)` | Schedule a one-shot timer using the same task scope |
| `timer_finished(name)` | Consume a due timer once; otherwise false |

Names require 1–240 UTF-8 bytes; delays must be finite and nonnegative. Each script
allows 256 live tasks within its existing state limit. Payloads obey saved-value
bounds, including the scheduler's wrapper. Avoid reserved `__kiln_task:` state keys.
Zero-delay tasks are ready within the current invocation after scheduling.

The Script Editor's **Format Indent** changes indentation/trailing whitespace
without changing tokens, comments or valid escaped string contents. Invalid quoted
or unbalanced drafts are preserved. **API Reference** offers insertion of calls
from the shared registry; replace the `arg1` placeholders with actual expressions.
**Project Search** searches saved `.ks` files literally and case-sensitively;
**Find Function** searches `fn name(` for the identifier under the source cursor.
Navigation requires Save or Revert first, preserving unsaved drafts. Search skips
symlinks and hidden/generated folders, limits traversal to 4096 entries/64 levels,
and reads at most 512 files/8 MiB with 256 results. This is literal navigation,
not a language-server symbol resolver. Source breakpoints/stepping/watches and
interactive gameplay graph acceptance remain open.

New exports require the matching `kiln-core::export::RUNTIME_CONTRACT` in both
native and browser runtime identities. Missing/older same-version runtimes are
rejected explicitly; install the matching tool bundle before exporting again.
