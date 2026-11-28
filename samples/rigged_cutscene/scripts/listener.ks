if script_phase()=="signal" {
 if signal_name()=="marker" {state_set("markers",state_get("markers",0)+1);}
 if signal_name()=="done" && state_get("markers",0)==2 {
  set_entity_component("ready","sprite_color",array_push(array_push(array_push(array_push(array_new(),0.0),0.0),1.0),1.0));
 }
}
