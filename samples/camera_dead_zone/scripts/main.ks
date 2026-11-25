set_entity_pos("hero", 180.0, 120.0);
if get_frame_count() >= 3 {
    set_entity_component("actor", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 0.0), 1.0), 0.0), 1.0));
}
