if script_phase() == "update" {
    if get_action("save") { if state_get("save_held", (0 == 1)) == (0 == 1) { save_game(0); } }
    if get_action("load") { if state_get("load_held", (0 == 1)) == (0 == 1) { load_game(0); } }
    if get_action("next") { if state_get("next_held", (0 == 1)) == (0 == 1) { change_scene("next"); } }
    state_set("save_held", get_action("save"));
    state_set("load_held", get_action("load"));
    state_set("next_held", get_action("next"));
}
