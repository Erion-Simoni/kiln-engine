// A real 2D scene for comparing the editor to its supplied design.
// Play uses the actual runtime; arrows/WASD move the blue player.
let dt = get_delta_time();
move_player(get_input_x() * 120.0 * dt, get_input_y() * 120.0 * dt);
