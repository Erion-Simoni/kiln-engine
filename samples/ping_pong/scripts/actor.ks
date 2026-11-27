if script_phase() == "animation_event" {
 let n = state_get("n",0);
 let expected = "zero";
 if n == 1 || n == 3 { expected = "one"; }
 if n == 2 { expected = "two"; }
 if n >= 5 || animation_event_name() != expected {state_set("bad",true);}
 state_set("n",n+1);
}
if script_phase() == "update" {
 if get_frame_count() == 15 {state_set("fast",state_get("n",0)>=4);}
 if get_frame_count() >= 90 && state_get("n",0)==5 && state_get("fast",false) && !state_get("bad",false) {
  set_entity_component("actor","sprite_color",array_push(array_push(array_push(array_push(array_new(),0.0),1.0),0.0),1.0));
 }
}
