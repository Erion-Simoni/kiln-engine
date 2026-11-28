if script_phase() == "collision_enter" {
    if game_get("hits", 0) == 1 {
        state_set("activated", true);
        set_status("Range cleared");
        set_entity_component("actor", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 0.0), 1.0), 0.0), 1.0));
    }
}
