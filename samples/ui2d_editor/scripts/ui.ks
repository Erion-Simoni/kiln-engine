if script_phase() == "start" {
    state_set("starts", state_get("starts", 0) + 1);
    if game_get("initialized", false) == false {
        game_set("initialized", true);
        game_set("health", 73);
        game_set("inventory", array_push(array_new(), "brass_key"));
    }
    play_music("theme");
}
if script_phase() == "ui_event" {
    state_set("event", ui_event_name());
    state_set("widget", ui_event_widget());
    state_set("screen", ui_event_screen());
    state_set("clicked", ui_clicked(ui_event_widget()));
    if ui_event_name() == "start" { ui_show_screen("hud"); }
    if ui_event_name() == "resume" { ui_show_screen("hud"); }
    if ui_event_name() == "sound" {
        set_volume(0.25); setting_set("menu_setting", "saved");
        ui_set_text("sound", "Sound saved 25%"); play_sound("hit");
    }
    if ui_event_name() == "damage" {
        game_set("health", game_get("health", 73) - 7);
        play_sound("hit"); play_sound("hit");
    }
    if ui_event_name() == "save" {
        save_game(0); ui_set_text("save", "Saving...");
    }
    if ui_event_name() == "save_complete" { ui_set_text("save", "Game saved"); }
    if ui_event_name() == "load" {
        if has_save(0) { load_game(0); }
        else { ui_set_text("continue", "No saved game"); }
    }
    if ui_event_name() == "load_complete" { ui_close_screen(); ui_show_screen("hud"); }
    if ui_event_name() == "save_failed" { ui_set_text("save", "Save failed"); }
    if ui_event_name() == "load_failed" { ui_set_text("continue", "Load failed"); }
}
if script_phase() == "update" {
    state_set("ticks", state_get("ticks", 0) + 1);
    ui_set_text("health", game_get("health", 73));
    let inventory = game_get("inventory", array_new());
    if array_length(inventory) > 0 { ui_set_text("inventory", array_get(inventory, 0)); }
}
