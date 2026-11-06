
if script_phase() == "update" {
    move_player(120.0 * get_delta_time(), 0.0);
    state_set("root_updates", state_get("root_updates", 0) + 1);
}
if script_phase() == "ui_event" {
    state_set("root_event", ui_event_name());
    state_set("event_screen", ui_event_screen());
}
