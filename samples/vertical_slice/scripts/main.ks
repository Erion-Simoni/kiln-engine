// Kiln vertical slice: collect the coins.
//
// Runs once per frame. The engine gives the script the input axis, the
// frame length and the state of the world; the script decides what happens.

let speed = 120.0;
let dt = get_delta_time();
let dx = get_input_x() * speed * dt;
let dy = get_input_y() * speed * dt;
move_player(dx, dy);

if collect_coin() {
    add_score(10);
    set_status("PICKED UP A COIN");
}

if get_coin_count() == 0 {
    set_status("ALL COINS COLLECTED");
}
