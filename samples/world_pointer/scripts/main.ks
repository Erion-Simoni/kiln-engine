let valid = mouse_world_valid() && get_mouse_world_x() > 150.0 && get_mouse_world_x() < 170.0 && get_mouse_world_y() > 110.0 && get_mouse_world_y() < 130.0;
if valid {
    set_entity_component("actor", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 0.0), 1.0), 0.0), 1.0));
} else {
    set_entity_component("actor", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 1.0), 0.0), 0.0), 1.0));
}
