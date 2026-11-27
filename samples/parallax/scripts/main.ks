if get_frame_count() == 1 { camera_transition(80.0, 40.0, 1.0, 2.0); }
if get_frame_count() >= 150 {
    set_entity_component("ready", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 0.0), 0.0), 1.0), 1.0));
}
