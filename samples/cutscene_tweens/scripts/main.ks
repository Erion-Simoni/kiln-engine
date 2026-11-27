if script_phase()=="update" {
 let pause=get_action("pause");let restart=get_action("restart");let seek=get_action("seek");let cancel=get_action("cancel");
 let pause_edge=pause && !state_get("held_pause",false);
 let restart_edge=restart && !state_get("held_restart",false);
 let seek_edge=seek && !state_get("held_seek",false);
 let cancel_edge=cancel && !state_get("held_cancel",false);
 state_set("held_pause",pause);state_set("held_restart",restart);state_set("held_seek",seek);state_set("held_cancel",cancel);
 if pause_edge || restart_edge || seek_edge || cancel_edge {state_set("manual",true);}
 let phase=state_get("phase",0);
 if phase==0 || restart_edge {
  set_entity_pos("actor",80.0,120.0);
  set_entity_pos("camera_rig",160.0,120.0);
  set_entity_component("actor","sprite_color",array_push(array_push(array_push(array_push(array_new(),1.0),0.0),0.0),1.0));
  set_entity_component("ready","sprite_color",array_push(array_push(array_push(array_push(array_new(),1.0),0.0),0.0),1.0));
  tween_position("actor",256.0,120.0,3.0,"smooth");
  tween_position("camera_rig",192.0,120.0,3.0,"smooth");
  tween_color("actor",0.0,1.0,0.0,1.0,3.0,"linear");
  state_set("paused",false);state_set("phase",1);
 }
 if pause_edge && tween_active("actor","position") {
  let paused=!state_get("paused",false);state_set("paused",paused);
  tween_pause("actor","position",paused);tween_pause("actor","color",paused);tween_pause("camera_rig","position",paused);
 }
 if seek_edge {tween_seek("actor","position",1.5);tween_seek("actor","color",1.5);tween_seek("camera_rig","position",1.5);}
 if cancel_edge {
  tween_cancel("actor","position");tween_cancel("actor","color");tween_cancel("camera_rig","position");state_set("phase",3);
 }
 if !state_get("manual",false) {
  let frame=get_frame_count();
  if frame==15 {
   state_set("paused",true);
   tween_pause("actor","position",true);tween_pause("actor","color",true);tween_pause("camera_rig","position",true);
   state_set("hold",array_get(get_entity_pos("actor"),0));state_set("hold_rig",array_get(get_entity_pos("camera_rig"),0));state_set("remaining",tween_remaining("actor","position"));
  }
  if frame==29 {
   state_set("held",array_get(get_entity_pos("actor"),0)==state_get("hold",0.0) && tween_remaining("actor","position")==state_get("remaining",0.0) && array_get(get_entity_pos("camera_rig"),0)==state_get("hold_rig",0.0));
  }
  if frame==30 {tween_seek("actor","position",1.5);tween_seek("actor","color",1.5);tween_seek("camera_rig","position",1.5);}
  if frame==31 {
   state_set("paused",false);
   let color=get_entity_component("actor","sprite_color");
   state_set("seeked",array_get(get_entity_pos("actor"),0)==168.0 && array_get(color,0)==0.5 && array_get(color,1)==0.5 && array_get(get_entity_pos("camera_rig"),0)==176.0);
   tween_pause("actor","position",false);tween_pause("actor","color",false);tween_pause("camera_rig","position",false);
  }
 }
 if phase==1 && !tween_active("actor","position") && !tween_active("actor","color") && !tween_active("camera_rig","position") {
  let position=get_entity_pos("actor");let color=get_entity_component("actor","sprite_color");
  let endpoint=array_get(position,0)==256.0 && array_get(position,1)==120.0 && array_get(color,0)==0.0 && array_get(color,1)==1.0 && array_get(color,2)==0.0 && array_get(color,3)==1.0 && array_get(get_entity_pos("camera_rig"),0)==192.0;
  if endpoint && (state_get("manual",false) || (state_get("held",false) && state_get("seeked",false))) {
   set_entity_component("ready","sprite_color",array_push(array_push(array_push(array_push(array_new(),0.0),0.0),1.0),1.0));
  }
  state_set("phase",2);
 }
}
