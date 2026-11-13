// SKYBRIDGE: two real physics levels. Arrows/WASD move, Up jumps.
// Bind action_0=S, action_1=L, action_2=R through Input Rebinding.
fn level() { let value = get_property("stage"); if value == 2.0 { return 2.0; } return 1.0; }
if script_phase() == "start" {
game_set("stage", level());
game_set("jumped", false); game_set("won", false);
state_set("jump_held", false); state_set("save_held", false); state_set("load_held", false);
}
if script_phase() == "collision_enter" {
if event_entity() == "trigger" {
if game_get("jumped", false) {
if level() == 1.0 { print("SKYBRIDGE_LEVEL_TWO"); change_scene("scene_1"); }
else { game_set("won", true); print("SKYBRIDGE_WON"); }
}
}
}
if script_phase() == "update" {
let id = self_entity(); let pos = get_entity_pos(id); let vel = get_entity_component(id, "velocity");
let ground = raycast_query(array_get(pos, 0), array_get(pos, 1), 0.0, -1.0, 15.0, 1);
let vy = array_get(vel, 1);
if get_input_y() > 0.0 {
if state_get("jump_held", false) == false {
if ground != false { vy = 360.0; game_set("jumped", true); print("SKYBRIDGE_JUMP"); }
}
}
state_set("jump_held", get_input_y() > 0.0);
let speed = get_input_x() * 170.0;
if game_get("won", false) { speed = 0.0; }
set_entity_vel(id, speed, vy);
play_animation(id, "run");
if speed == 0.0 { set_animation_speed(id, 0.0); } else { set_animation_speed(id, 1.0); }
if level() == 1.0 { game_set("status", "SKYBRIDGE 1/2 - Jump, gate. S save L load R replay"); }
else { game_set("status", "SKYBRIDGE 2/2 - Far gate. S save L load R replay"); }
if game_get("jumped", false) == false { game_set("status", "Jump with Up/W before passing the gate!"); }
if game_get("won", false) { game_set("status", "SKYBRIDGE COMPLETE! R: replay this level"); }
if array_get(pos, 1) < -80.0 { set_entity_pos(id, 80.0, 60.0); set_entity_vel(id, 0.0, 0.0); game_set("jumped", false); }
if get_action("action_0") {
if state_get("save_held", false) == false { save_game(0); print("SKYBRIDGE_SAVE_REQUESTED"); }
}
state_set("save_held", get_action("action_0"));
// Load on release: restoring saved key-edge state cannot reload every frame.
if get_action("action_1") == false {
if state_get("load_held", false) { if has_save(0) { load_game(0); } }
}
state_set("load_held", get_action("action_1"));
if get_action("action_2") { set_entity_pos(id, 80.0, 60.0); set_entity_vel(id, 0.0, 0.0); game_set("won", false); game_set("jumped", false); }
}
if script_phase() == "ui_event" {
if ui_event_name() == "load_complete" { print("SKYBRIDGE_RESUMED"); }
if ui_event_name() == "save_failed" { game_set("status", "Save failed - retry S"); }
if ui_event_name() == "load_failed" { game_set("status", "Load failed - existing game preserved"); }
}
