if script_phase()=="update" {
 let frame=get_frame_count();
 if frame==15 {set_animation_parameter("fade","go",1.0);}
 if frame==20 {set_animation_parameter("tree","speed",0.5);tween_rotation("lower",1.5707963267948966,1.0,"smooth");}
 if frame==90 {set_animation_parameter("tree","speed",1.0);tween_rotation("lower",0.0,1.0,"smooth");tween_position("lower",112.0,0.0,1.0,"smooth");}
 if frame>180 && !tween_active("lower","rotation") && !tween_active("lower","position") && get_animation_state("fade")=="green" {
  set_entity_component("ready","sprite_color",array_push(array_push(array_push(array_push(array_new(),0.0),0.0),1.0),1.0));
 }
}
