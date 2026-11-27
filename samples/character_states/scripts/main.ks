if script_phase() == "update" {
 let state=get_animation_state("actor");
 if state=="idle" {state_set("idle_seen",true);}
 if state=="run" && state_get("idle_seen",false) {state_set("run_seen",true);}
 if state=="jump" && state_get("run_seen",false) {state_set("jump_seen",true);}
 if state=="idle" && state_get("jump_seen",false) {state_set("returned",true);}
 let axis=get_input_x();let held=get_action("jump");
 if axis!=0.0 || held {state_set("manual",true);}
 if state_get("manual",false) {
  set_animation_parameter("actor","moving",axis!=0.0);
  set_animation_parameter("actor","jump",held && !state_get("held",false));
  state_set("held",held);
  let position=get_entity_pos("actor");
  let x=array_get(position,0)+axis*80.0*get_delta_time();
  if x<32.0 {x=32.0;}if x>288.0 {x=288.0;}
  set_entity_pos("actor",x,120.0);
 } else {
  let frame=get_frame_count();
  if frame==30 {set_animation_parameter("actor","moving",true);}
  if frame==60 {set_animation_parameter("actor","moving",false);set_animation_parameter("actor","jump",true);}
  if frame==61 {set_animation_parameter("actor","jump",false);}
  if frame==90 {set_animation_parameter("actor","moving",true);}
  if frame>=120 && state=="run" && state_get("returned",false) {
   set_entity_component("ready","sprite_color",array_push(array_push(array_push(array_push(array_new(),0.0),0.0),1.0),1.0));
  }
 }
}
