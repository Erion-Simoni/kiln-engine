// SWITCHYARD: a reusable five-lamp XOR puzzle, played with keyboard and mouse.
// Select a switch with Left/Right (or A/D), then click ACTIVATE.
fn actor(role) { let ids = get_entities_with_role(role); if array_length(ids) > 0 { return array_get(ids, 0); } return ""; }
fn tint(role, r, g, b) {
let id = actor(role); if id != "" { set_entity_component(id, "sprite_color", array_push(array_push(array_push(array_push(array_new(), r), g), b), 1.0)); }
}
fn lamp(role, key) { if game_get(key, 0) == 1 { tint(role, 1.0, 0.85, 0.25); } else { tint(role, 0.12, 0.2, 0.28); } }
fn flip(key) { game_set(key, 1 - game_get(key, 0)); }
fn board() {
game_set("l0", 0); game_set("l1", 0); game_set("l2", 0); game_set("l3", 0); game_set("l4", 0);
game_set("moves", 0); game_set("selected", 0); game_set("outcome", "playing");
if game_get("stage", 1) == 2 { game_set("l0", 1); game_set("l2", 1); game_set("l4", 1); }
if game_get("stage", 1) == 3 { game_set("l0", 1); game_set("l2", 1); game_set("l3", 1); }
}
fn hud() {
ui_set_text("health", "LIGHT ALL FIVE LAMPS");
ui_set_text("inventory", game_get("moves", 0));
let selected = game_get("selected", 0);
if selected == 0 { ui_set_text("damage", "A: toggle 1 + 3 + 5"); }
if selected == 1 { ui_set_text("damage", "B: toggle 1 + 3 + 4"); }
if selected == 2 { ui_set_text("damage", "C: toggle 1 + 2 + 3"); }
}
fn activate() {
let selected = game_get("selected", 0);
if selected == 0 { flip("l0"); flip("l2"); flip("l4"); }
if selected == 1 { flip("l0"); flip("l2"); flip("l3"); }
if selected == 2 { flip("l0"); flip("l1"); flip("l2"); }
game_set("moves", game_get("moves", 0) + 1); play_sound("hit"); print("SWITCHYARD_TOGGLED");
save_game(0);
}
if script_phase() == "start" {
game_set("stage", 1); board();
spawn_entity("lamp0", 200.0, 200.0, ""); spawn_entity("lamp1", 260.0, 200.0, ""); spawn_entity("lamp2", 320.0, 200.0, ""); spawn_entity("lamp3", 380.0, 200.0, ""); spawn_entity("lamp4", 440.0, 200.0, "");
spawn_entity("switchA", 200.0, 140.0, ""); spawn_entity("switchB", 320.0, 140.0, ""); spawn_entity("switchC", 440.0, 140.0, "");
play_music("theme"); ui_set_text("heading", "SWITCHYARD"); ui_set_text("start", "Repair the circuits");
}
if script_phase() == "ui_event" {
let event = ui_event_name();
if event == "start" { ui_show_screen("hud"); hud(); }
if event == "resume" {
if game_get("outcome", "playing") == "won" {
let stage = game_get("stage", 1) + 1; if stage > 3 { stage = 1; } game_set("stage", stage); board();
}
ui_show_screen("hud"); hud();
}
if event == "damage" { activate(); }
if event == "save" { save_game(0); }
if event == "load" { if has_save(0) { load_game(0); } else { ui_set_text("continue", "Activate a switch to autosave"); } }
if event == "load_complete" { ui_show_screen("hud"); hud(); print("SWITCHYARD_RESUMED"); }
if event == "save_complete" { ui_set_text("save", "Circuit saved"); }
if event == "save_failed" { ui_set_text("save", "Save failed - try again"); }
if event == "sound" { set_volume(0.25); ui_set_text("sound", "Sound saved: 25%"); }
}
if script_phase() == "update" {
let axis = get_input_x();
if state_get("held", false) == false {
let selected = game_get("selected", 0);
if axis > 0.0 { selected = selected + 1; }
if axis < 0.0 { selected = selected - 1; }
if selected > 2 { selected = 0; } if selected < 0 { selected = 2; }
game_set("selected", selected);
}
state_set("held", axis != 0.0);
set_entity_pos("player", 200.0 + game_get("selected", 0) * 120.0, 100.0);
lamp("lamp0", "l0"); lamp("lamp1", "l1"); lamp("lamp2", "l2"); lamp("lamp3", "l3"); lamp("lamp4", "l4");
tint("switchA", 0.2, 0.75, 0.9); tint("switchB", 0.75, 0.3, 0.9); tint("switchC", 0.3, 0.85, 0.5);
hud();
let total = game_get("l0", 0) + game_get("l1", 0) + game_get("l2", 0) + game_get("l3", 0) + game_get("l4", 0);
if total == 5 {
game_set("outcome", "won"); ui_show_screen("game_over"); ui_set_text("heading", "CIRCUIT RESTORED"); ui_set_text("resume", "Next circuit / replay"); save_game(0); print("SWITCHYARD_WON");
}
}
