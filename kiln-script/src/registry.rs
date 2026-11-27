//! Shared gameplay host function names, stable IDs and argument counts.
pub const BUILTINS: &[(&str, u32, u32)] = &[
    // Core builtins (0-16)
    ("print", 0, 1),
    ("get_input_x", 1, 0),
    ("get_input_y", 2, 0),
    ("get_elapsed", 3, 0),
    ("get_score", 4, 0),
    ("add_score", 5, 1),
    ("get_coin_count", 6, 0),
    ("collect_coin", 7, 0),
    ("move_player", 8, 2),
    ("set_status", 9, 1),
    ("get_status", 10, 0),
    ("get_delta_time", 11, 0),
    ("play_sound", 12, 1),
    ("stop_sound", 13, 1),
    ("play_music", 14, 1),
    ("stop_music", 15, 0),
    ("set_volume", 16, 1),
    // Audio extended (17-25)
    ("set_sfx_volume", 17, 1),   // volume 0-1
    ("set_music_volume", 18, 1), // volume 0-1
    ("pause_sound", 19, 1),      // sound_name -> void
    ("resume_sound", 20, 1),     // sound_name -> void
    ("pause_music", 21, 0),      // -> void
    ("resume_music", 22, 0),     // -> void
    ("stop_all_sounds", 23, 0),  // -> void
    ("sound_playing", 24, 1),    // sound_name -> bool
    ("music_playing", 25, 0),    // -> bool
    // Entity management (26-31)
    ("spawn_entity", 26, 4),   // name, x, y, texture_path?
    ("despawn_entity", 27, 1), // source_id
    ("get_entity_pos", 28, 1), // source_id -> (x, y)
    ("set_entity_pos", 29, 3), // source_id, x, y
    // Query builtins (30-34)
    ("get_entities_with_role", 30, 1), // role -> array of source_ids
    ("get_entity_component", 31, 2),   // source_id, component_name -> value
    ("set_entity_component", 32, 3),   // source_id, component_name, value
    // World queries (33-34)
    ("find_nearest", 33, 3), // x, y, role -> source_id
    ("raycast", 34, 4),      // x, y, dx, dy -> hit info
    // Time/frame (35-36)
    ("get_frame_count", 35, 0), // -> frame number
    ("get_time_scale", 36, 0),  // -> time scale
    // Extended entity API (37-41)
    ("set_entity_vel", 37, 3),           // source_id, vx, vy
    ("get_entities_at_position", 38, 3), // x, y, radius -> array of source_ids
    ("get_entity_role", 39, 1),          // source_id -> role string
    ("add_component", 40, 3),            // source_id, component_type, value_json
    ("remove_component", 41, 2),         // source_id, component_type
    // Animation API (42-44)
    ("play_animation", 42, 2),      // source_id, clip_name
    ("stop_animation", 43, 1),      // source_id
    ("set_animation_speed", 44, 2), // source_id, speed
    // Collision events (45)
    ("get_collision_events", 45, 0), // -> array of [entity_a, entity_b, started]
    // UI widgets (46-49)
    ("ui_button", 46, 3),  // id, x, y -> clicked bool
    ("ui_label", 47, 4),   // id, x, y, text -> void
    ("ui_panel", 48, 5),   // id, x, y, w, h -> void
    ("ui_clicked", 49, 1), // id -> clicked bool
    // Input actions (50-51)
    ("get_action", 50, 1),      // action_name -> pressed bool
    ("get_action_axis", 51, 1), // action_name -> -1 to 1 value
    // Localization (52)
    ("localize", 52, 1), // key -> localized string
    // Save/load (53-54)
    ("save_game", 53, 1), // slot_id -> bool
    ("load_game", 54, 1), // slot_id -> bool
    // Animation clip creation (55)
    ("create_animation_clip", 55, 3), // source_id, clip_name, texture_path -> void
    ("self_entity", 56, 0),
    ("state_get", 57, 2),
    ("state_set", 58, 2),
    ("get_property", 59, 1),
    ("script_phase", 60, 0),
    ("array_length", 61, 1),
    ("array_get", 62, 2),
    ("array_push", 63, 2),
    ("emit_signal", 64, 2),
    ("signal_name", 65, 0),
    ("signal_value", 66, 0),
    ("event_entity", 67, 0),
    ("change_scene", 68, 1),
    ("dict_new", 110, 0),
    ("dict_get", 111, 3),
    ("dict_set", 112, 3),
    ("dict_has", 113, 2),
    ("dict_remove", 114, 2),
    ("dict_keys", 115, 1),
    ("task_schedule", 116, 3),
    ("task_ready", 117, 1),
    ("task_take", 118, 2),
    ("task_cancel", 119, 1),
    ("task_exists", 120, 1),
    ("task_remaining", 121, 1),
    ("timer_start", 122, 2),
    ("timer_finished", 123, 1),
    ("array_new", 69, 0),
    ("overlap_circle", 70, 4),
    ("raycast_query", 71, 6),
    ("get_mouse_world_x", 72, 0),
    ("get_mouse_world_y", 73, 0),
    ("animation_event_name", 74, 0),
    ("animation_event_value", 75, 0),
    ("set_bus_volume", 76, 2),
    ("get_bus_volume", 77, 1),
    ("setting_get", 78, 2),
    ("setting_set", 79, 2),
    ("ui_show_screen", 80, 1),
    ("ui_close_screen", 81, 0),
    ("ui_screen", 82, 0),
    ("ui_event_name", 83, 0),
    ("ui_event_widget", 84, 0),
    ("ui_set_text", 85, 2),
    ("ui_set_visible", 86, 2),
    ("ui_set_enabled", 87, 2),
    ("ui_event_screen", 88, 0),
    ("game_get", 89, 2),
    ("game_set", 90, 2),
    ("save_status", 91, 0),
    ("save_error", 92, 0),
    ("has_save", 93, 1),
    ("camera_shake", 124, 5),
    ("camera_shake_stop", 125, 0),
    ("camera_transition", 126, 4),
    ("camera_transition_stop", 127, 0),
];
