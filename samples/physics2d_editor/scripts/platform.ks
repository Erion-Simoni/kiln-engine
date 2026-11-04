if script_phase() == "update" {
    let pos = get_entity_pos(self_entity());
    let direction = state_get("direction", 1.0);
    if array_get(pos, 0) > 300.0 { direction = -1.0; }
    if array_get(pos, 0) < 120.0 { direction = 1.0; }
    state_set("direction", direction);
    set_entity_vel(self_entity(), direction * 40.0, 0.0);
}
