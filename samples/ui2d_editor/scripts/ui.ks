
if script_phase() == "start" { state_set("starts", state_get("starts", 0) + 1); play_music("theme"); }
if script_phase() == "ui_event" {
    state_set("event", ui_event_name());
    state_set("widget", ui_event_widget());
    state_set("screen", ui_event_screen());
    state_set("clicked", ui_clicked(ui_event_widget()));
    if ui_event_name() == "sound" {
        set_volume(0.25); setting_set("menu_setting", "saved");
        ui_set_text("sound", "Sound saved 25%"); play_sound("hit");
    }
}
if script_phase() == "update" { state_set("ticks", state_get("ticks", 0) + 1); }
