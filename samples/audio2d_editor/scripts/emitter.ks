
if script_phase() == "start" {
    set_volume(0.5);
    set_bus_volume("Combat", 0.5);
    setting_set("language", "sq");
    play_sound("hit"); play_sound("hit");
}
if script_phase() == "update" {
    state_set("music", music_playing());
    state_set("hit", sound_playing("hit"));
    state_set("language", setting_get("language", "en"));
    state_set("master", get_bus_volume("Master"));
}
