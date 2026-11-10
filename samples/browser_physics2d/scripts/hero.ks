if script_phase() == "start" {
    game_set("hero_starts", game_get("hero_starts", 0) + 1);
}
if script_phase() == "collision_enter" {
    if event_entity() == "sensor" { game_set("sensor_enters", game_get("sensor_enters", 0) + 1); }
}
if script_phase() == "collision_exit" {
    if event_entity() == "sensor" { game_set("sensor_exits", game_get("sensor_exits", 0) + 1); }
}
if script_phase() == "update" {
    if get_action("jump") {
        if state_get("jump_held", (0 == 1)) == (0 == 1) { set_entity_vel(self_entity(), 0.0, 220.0); }
    }
    state_set("jump_held", get_action("jump"));
}
