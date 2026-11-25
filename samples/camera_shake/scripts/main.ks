if get_frame_count() == 1 { camera_shake(24.0, 16.0, 2.0, 9.0, 17); }
if get_frame_count() >= 150 {
    set_entity_component("ready", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 0.0), 0.0), 1.0), 1.0));
}
