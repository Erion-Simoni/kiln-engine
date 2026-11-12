if script_phase() == "start" {
    game_set("starts", game_get("starts", 0) + 1);
}
if script_phase() == "update" {
    game_set("ground_hit", raycast_query(100.0, 60.0, 0.0, -1.0, 100.0, 1));
    game_set("tile_hit", raycast_query(210.0, 90.0, 0.0, -1.0, 100.0, 1));
    if get_action("capture") {
        if state_get("capture_held", (0 == 1)) == (0 == 1) { save_game(0); }
    }
    if get_action("checkpoint") {
        if state_get("checkpoint_held", (0 == 1)) == (0 == 1) { save_game(1); }
    }
    if get_action("restore") {
        if state_get("restore_held", (0 == 1)) == (0 == 1) { load_game(1); }
    }
    if get_action("next") {
        if state_get("next_held", (0 == 1)) == (0 == 1) { change_scene("next"); }
    }
    state_set("capture_held", get_action("capture"));
    state_set("checkpoint_held", get_action("checkpoint"));
    state_set("restore_held", get_action("restore"));
    state_set("next_held", get_action("next"));
}
