if script_phase() == "start" {
    state_set("trigger", (0 == 1));
}
if script_phase() == "collision_enter" {
    if event_entity() == "trigger" { state_set("trigger", (1 == 1)); }
}
if script_phase() == "update" {
    let pos = get_entity_pos(self_entity());
    let vel = get_entity_component(self_entity(), "velocity");
    let ground = raycast_query(array_get(pos, 0), array_get(pos, 1), 0.0, -1.0, 14.0, 1);
    let vy = array_get(vel, 1);
    if get_input_y() > 0.0 {
        if state_get("jump_held", (0 == 1)) == (0 == 1) {
            if ground != (0 == 1) { vy = 260.0; }
        }
    }
    state_set("jump_held", get_input_y() > 0.0);
    set_entity_vel(self_entity(), get_input_x() * 150.0, vy);
    state_set("mouse_world", array_push(array_push(array_new(), get_mouse_world_x()), get_mouse_world_y()));
    if state_get("trigger", (0 == 1)) { set_status("Trigger reached"); }
}
