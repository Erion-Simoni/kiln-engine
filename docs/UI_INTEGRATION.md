# UI (kiln-ui) In-Game Integration — Design Doc

**Problem:** `kiln-ui` (`Button`, `Label`, `List`, `Panel`, `UiRenderer`, `TextRenderer`, `NineSliceRenderer`) exists as a library but is not connected to:
- the player (HUD text is drawn directly with `TextRenderer`, not through the full widget system),
- the slice runtime (no UI interaction from KilnScript — buttons can't trigger script actions),
- the ECS world (no `UILayer` / `Canvas` component for in-game UI),
- input handling (no widget click/hover state from mouse/touch).

This doc specifies the integration path. `cargo` is not available here, so this is a design doc to implement when `cargo` is present.

---

## Current state (verified from source)

| Item | Status |
|------|--------|
| `kiln-ui/src/ui_renderer.rs` | `UiRenderer` converts `Panel`/`Button`/`Label`/`List` → `Vec<UiQuad>` (texture + SpriteInstance). `UiQuad::rect()` for solid rects. `FONT_SIZE = 14.0`, `ROW_HEIGHT = 24.0`. |
| `kiln-ui/src/widget.rs` | `WidgetId`, `Widget` trait, `Panel`, `Label`, `Button` (with normal/hovered/pressed nine-slice states), `List` (rows, selection, scroll). |
| `kiln-ui/src/text.rs` | `TextRenderer` — built-in 5×7 bitmap font, `measure`/`render` returns glyph quads. |
| `kiln-ui/src/render.rs` | `NineSlice`, `NineSliceRenderer` — up to 9 quads with UV rects. |
| `kiln-ui/src/lib.rs` | Re-exports all widget/render/text/layout/localization/navigation types. |
| `kiln-render/src/sprite_renderer.rs` | `SpriteRenderer` accepts `SpriteInstance` with `TextureId` + `uv_rect` — `UiQuad` draws directly into it. |
| `kiln-player/src/main.rs` | HUD drawn via `TextRenderer::new()` + `text.render()` directly, not through `UiRenderer`/widgets. |
| `kiln-runtime/src/input.rs` | `InputState` tracks keys, mouse position, mouse buttons — can feed widget hover/click. |

---

## Design

### UI layer in the player

`PlayerApp` gains a `UiRenderer` and a root `Panel` (the UI canvas). Each frame:

```rust
// After scene render, before present
let mut ui_pass = encoder.begin_render_pass(&...);
ui_renderer.begin_frame();  // if needed
for quad in ui_renderer.render_ui(&root_panel, &self.world, &self.slice) {
    ui_sprite_renderer.push_textured(quad.texture, quad.instance);
}
ui_sprite_renderer.flush(&mut ui_pass, &ui_camera, &self.queue);
```

The UI camera is `Camera::new_top_left(width, height)` (y-down, screen space) so UI coordinates match pixel space.

### Widget → game action wiring

Widgets need a way to trigger game actions. Add an `on_click` callback or action id to `Button`:

```rust
pub struct Button {
    /* ... existing fields ... */
    pub on_click: Option<String>,  // action id, e.g. "pause", "restart", "menu"
}
```

The player's input handler maps mouse clicks to widget hits:
1. On `mousedown`, find the topmost widget under the cursor.
2. If it's a `Button` with `on_click`, set a "clicked" flag.
3. After the slice step, dispatch the action (e.g. `set_status("paused")`, toggle pause).

### KilnScript host calls for UI

Add UI builtins to `kiln-runtime/src/slice.rs` `BUILTINS`:

| Function | Args | Returns | Description |
|----------|------|---------|-------------|
| `set_status(msg)` | 1 (str) | — | Already exists — shows text on HUD |
| `show_ui(panel_id)` | 1 (str) | — | Show a named UI panel |
| `hide_ui(panel_id)` | 1 (str) | — | Hide a named UI panel |
| `ui_button_clicked(panel_id, button_id)` | 2 (str, str) | bool | Poll whether a button was clicked this frame |

The slice runtime owns the UI panel hierarchy and routes button clicks to script logic.

### HUD (existing)

The player already draws `slice.hud_lines()` via `TextRenderer`. This can be refactored to use `UiRenderer.render_label()` for consistency, but the current approach works — keep it as the default HUD and add `UiRenderer` for richer UI (menus, inventories, dialogs).

### Text rendering

`TextRenderer` uses a built-in 5×7 bitmap font (A–Z, digits, punctuation). For richer fonts, load a bitmap font atlas PNG and use `TextRenderer` with custom glyph UVs, or render text via `Label` widgets (which already use `TextRenderer` internally).

### Acceptance criteria (testable)

1. **Command:** `cargo build -p kiln-ui` compiles (already true).
2. **Command:** `cargo test -p kiln-ui` passes (44 tests already pass per `fixes m1-29.md`).
3. **Unit:** `UiRenderer::render_button` produces background quad + label glyphs; hover/pressed change color.
4. **Unit:** `UiRenderer::render_list` renders visible rows, clips scrolled-off rows, highlights selected.
5. **Runtime:** A `Button` widget under the mouse changes to hovered state; clicking triggers `on_click`.
6. **Runtime:** HUD score from `SliceSession` displays via UI widgets.

### Out of scope

- Full UI layout engine (constraint-based layout is in `kiln-ui/src/layout.rs` but not yet connected to auto-sizing)
- UI animations / transitions
- Touch input for mobile (Part of mobile milestones)

## See Also

- `kiln-ui/src/ui_renderer.rs` — `UiRenderer`, `UiQuad` (widget → sprite instances)
- `kiln-ui/src/widget.rs` — `Button`, `Label`, `List`, `Panel`, `Widget`
- `kiln-ui/src/text.rs` — `TextRenderer` (bitmap font)
- `kiln-ui/src/render.rs` — `NineSlice`, `NineSliceRenderer`
- `kiln-render/src/sprite_renderer.rs` — `SpriteRenderer` (UI draw target)
- `kiln-player/src/main.rs` — current HUD drawing (to refactor)
- `kiln-runtime/src/input.rs` — `InputState` (mouse for widget hits)
- `kiln-runtime/src/slice.rs` — `BUILTINS` list to add UI host calls to