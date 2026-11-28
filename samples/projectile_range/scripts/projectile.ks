import "motion.ks";
if script_phase() == "update" {
    let position = get_entity_pos(self_entity());
    let speed = 100.0 + get_input_x() * 100.0;
    set_entity_pos(self_entity(), advance(array_get(position, 0), speed, get_delta_time()), 80.0);
}
