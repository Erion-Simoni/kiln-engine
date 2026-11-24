import "motion.ks";
if script_phase() == "update" {
    let position = get_entity_pos(self_entity());
    set_entity_pos(self_entity(), advance(array_get(position, 0), -12.0, get_delta_time()), 80.0);
}
if script_phase() == "collision_enter" {
    game_set("hits", 1);
    despawn_entity(self_entity());
}
