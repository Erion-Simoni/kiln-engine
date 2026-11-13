// EMBERWATCH: defend the relay. Move with arrows/WASD, click FIRE.
// Project gameplay only: no engine-specific sample code.
fn clamp(v, low, high) { if v < low { return low; } if v > high { return high; } return v; }
fn abs(v) { if v < 0.0 { return -v; } return v; }
fn actor(role) { let ids = get_entities_with_role(role); if array_length(ids) > 0 { return array_get(ids, 0); } return ""; }
fn tint(role, r, g, b) {
let id = actor(role);
if id != "" { set_entity_component(id, "sprite_color", array_push(array_push(array_push(array_push(array_new(), r), g), b), 1.0)); }
}
fn place(role, x, y) { let id = actor(role); if id == "" { spawn_entity(role, x, y, ""); } else { set_entity_pos(id, x, y); } }
fn reset_round() {
game_set("health", 5); game_set("cleared", 0); game_set("outcome", "playing");
game_set("scout", 1); game_set("guard", 2); game_set("brute", 3);
state_set("hurt", 0.0);
set_entity_pos("player", 80.0, 180.0);
place("scout", 460.0, 180.0); place("guard", 550.0, 260.0); place("brute", 560.0, 100.0);
let bolt = actor("bolt"); if bolt != "" { despawn_entity(bolt); }
place("relay", 570.0, 180.0);
}
fn fire() {
if actor("bolt") == "" {
let target = actor("scout"); if target == "" { target = actor("guard"); } if target == "" { target = actor("brute"); }
if target != "" {
let p = get_entity_pos("player"); let e = get_entity_pos(target);
let dx = array_get(e, 0) - array_get(p, 0); let dy = array_get(e, 1) - array_get(p, 1);
let length = abs(dx) + abs(dy); if length < 1.0 { length = 1.0; }
game_set("bolt_vx", dx / length * 460.0); game_set("bolt_vy", dy / length * 460.0);
spawn_entity("bolt", array_get(p, 0), array_get(p, 1), "");
play_sound("hit"); print("EMBERWATCH_FIRE");
}
}
}
fn enemy(role, speed) {
let id = actor(role); if id != "" {
let pos = get_entity_pos(id); let player = get_entity_pos("player");
let dx = array_get(player, 0) - array_get(pos, 0); let dy = array_get(player, 1) - array_get(pos, 1);
let length = abs(dx) + abs(dy); if length < 1.0 { length = 1.0; }
set_entity_pos(id, array_get(pos, 0) + dx / length * speed * get_delta_time(), array_get(pos, 1) + dy / length * speed * get_delta_time());
if dx * dx + dy * dy < 1000.0 {
if state_get("hurt", 0.0) <= 0.0 {
game_set("health", game_get("health", 5) - 1); state_set("hurt", 1.2); play_sound("hit"); print("EMBERWATCH_DAMAGE");
}
}
let bolt = actor("bolt"); if bolt != "" {
let b = get_entity_pos(bolt); let bx = array_get(b, 0) - array_get(pos, 0); let by = array_get(b, 1) - array_get(pos, 1);
if bx * bx + by * by < 1300.0 {
despawn_entity(bolt); game_set(role, game_get(role, 1) - 1); print("EMBERWATCH_HIT");
if game_get(role, 1) <= 0 { despawn_entity(id); game_set("cleared", game_get("cleared", 0) + 1); }
}
}
}
}
fn hud() {
ui_set_text("health", "RELAY DEFENCE - Hull:"); ui_set_text("inventory", game_get("health", 5)); ui_set_text("damage", "FIRE / auto-aim");
}
if script_phase() == "start" {
reset_round(); play_music("theme"); ui_set_text("heading", "EMBERWATCH"); ui_set_text("start", "Defend the relay");
}
if script_phase() == "ui_event" {
let event = ui_event_name();
if event == "start" { ui_show_screen("hud"); hud(); }
if event == "resume" {
if game_get("outcome", "playing") != "playing" { reset_round(); }
ui_show_screen("hud"); hud();
}
if event == "damage" { fire(); }
if event == "save" { save_game(0); }
if event == "load" { if has_save(0) { load_game(0); } else { ui_set_text("continue", "No saved defence yet"); } }
if event == "load_complete" { ui_show_screen("hud"); hud(); print("EMBERWATCH_RESUMED"); }
if event == "save_complete" { ui_set_text("save", "Defence saved"); }
if event == "save_failed" { ui_set_text("save", "Save failed - try again"); }
if event == "sound" { set_volume(0.25); setting_set("quiet", true); ui_set_text("sound", "Sound saved: 25%"); }
}
if script_phase() == "update" {
let dt = get_delta_time(); let p = get_entity_pos("player");
set_entity_pos("player", clamp(array_get(p, 0) + get_input_x() * dt * 180.0, 24.0, 615.0), clamp(array_get(p, 1) + get_input_y() * dt * 180.0, 25.0, 285.0));
state_set("hurt", state_get("hurt", 0.0) - dt);
tint("scout", 1.0, 0.25, 0.2); tint("guard", 0.9, 0.45, 0.12); tint("brute", 0.8, 0.15, 0.6); tint("bolt", 1.0, 0.9, 0.3);
let bolt = actor("bolt"); if bolt != "" {
let b = get_entity_pos(bolt);
let x = array_get(b, 0) + game_get("bolt_vx", 0.0) * dt; let y = array_get(b, 1) + game_get("bolt_vy", 0.0) * dt;
set_entity_pos(bolt, x, y);
if x < 0.0 { despawn_entity(bolt); } if x > 640.0 { despawn_entity(bolt); }
if y < 0.0 { despawn_entity(bolt); } if y > 360.0 { despawn_entity(bolt); }
}
enemy("scout", 32.0); enemy("guard", 22.0); enemy("brute", 15.0);
if game_get("health", 5) <= 0 {
game_set("outcome", "lost"); ui_show_screen("game_over"); ui_set_text("heading", "Relay lost - try again"); ui_set_text("resume", "Restart defence"); print("EMBERWATCH_LOST");
}
if game_get("cleared", 0) == 3 {
tint("relay", 0.25, 1.0, 0.5);
let player = get_entity_pos("player"); let dx = array_get(player, 0) - 570.0; let dy = array_get(player, 1) - 180.0;
ui_set_text("health", "All hostiles down - reach the green relay!");
if dx * dx + dy * dy < 1500.0 {
game_set("outcome", "won"); ui_show_screen("game_over"); ui_set_text("heading", "RELAY SECURED"); ui_set_text("resume", "Play another defence"); save_game(0); print("EMBERWATCH_WON");
}
} else { hud(); tint("relay", 0.2, 0.4, 0.6); }
}
