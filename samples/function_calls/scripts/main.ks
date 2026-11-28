import "outer.ks";
let result = evaluate(2);
let nested = bump(bump(0));
if result == 7 && nested == 2 {
    set_entity_component("actor", "sprite_color", array_push(array_push(array_push(array_push(array_new(), 0.0), 1.0), 0.0), 1.0));
}
