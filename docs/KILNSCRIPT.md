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

### Example

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

## See Also

- `docs/TUTORIAL.md` — build your first game with KilnScript
- `kiln-script/src/` — lexer, parser, compiler source
- `kiln-ir/src/vm.rs` — bytecode VM
- `kiln-runtime/src/slice.rs` — slice runtime and builtins