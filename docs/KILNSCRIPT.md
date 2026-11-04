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

`if`, `else`, `return`, `let`, `call`

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

Numeric operations promote `I64` to `F64` when mixed with floats (e.g. `1 + 2.5` yields `3.5`). Integer division by zero is a runtime error; float division follows IEEE-754.

### Statements

- `let name = expr;` — declare a local variable (cannot redeclare in the same scope)
- `name = expr;` — assign to an existing variable
- `expr;` — expression statement (e.g. a function call)
- `return expr;` — return from the program
- `if cond { ... } else { ... }` — conditional; blocks get their own scope

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
| `get_collision_events()` | 0 | [[a, b, started]...] | Collision events this frame |

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
| `save_game(slot_id)` | 1 (i64) | bool | Save game to slot |
| `load_game(slot_id)` | 1 (i64) | bool | Load game from slot |


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

In the Inspector, use **Attach Script** to select a project `.ks` file. Add
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
