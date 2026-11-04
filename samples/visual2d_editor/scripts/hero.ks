if script_phase() == "animation_event" {
state_set("events", state_get("events", 0) + 1);
state_set("last_event", animation_event_name());
state_set("payload", animation_event_value());
}