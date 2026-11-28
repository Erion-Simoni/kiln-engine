// kiln-cutscene-v1 {"version":1,"tracks":[{"entity":"target","property":"position","keys":[{"time":"0","value":["140","80"],"ease":"smooth"},{"time":"3","value":["240","120"],"ease":"smooth"}],"blend":{"keys":[{"time":"0","value":["180","160"],"ease":"smooth"},{"time":"3","value":["240","120"],"ease":"smooth"}],"weight":"0.5","parameter":"reach_blend"}}],"actions":["pause","restart","seek","cancel"],"cues":[{"time":"0","kind":"signal","name":"marker","value":"1"},{"time":"0.5","kind":"sound","name":"chime","value":"0"},{"time":"1","kind":"signal","name":"marker","value":"1"},{"time":"3.05","kind":"signal","name":"done","value":"1"}],"ik":[{"bones":["upper","lower","tip"],"target":"target","bend":"1","weight":"1"}]}
// Generated cutscene. Reopen in Animation Editor to change keys.
if script_phase()=="update" {
 let time=state_get("cutscene_time",0.0);
 let started=state_get("cutscene_started",false);
 let paused=state_get("cutscene_paused",false);
 let running=state_get("cutscene_running",true);
 let apply=!started;
 let pressed0=get_action("pause");let edge0=pressed0 && !state_get("cutscene_held0",false);state_set("cutscene_held0",pressed0);
 let pressed1=get_action("restart");let edge1=pressed1 && !state_get("cutscene_held1",false);state_set("cutscene_held1",pressed1);
 let pressed2=get_action("seek");let edge2=pressed2 && !state_get("cutscene_held2",false);state_set("cutscene_held2",pressed2);
 let pressed3=get_action("cancel");let edge3=pressed3 && !state_get("cutscene_held3",false);state_set("cutscene_held3",pressed3);
 if edge0 {paused=!paused;}
 if started && running && !paused && !edge1 && !edge2 && !edge3 {let dt=get_delta_time();if dt>0.0 {time=time+dt;}if time>3.05 {time=3.05;}apply=true;}
 if edge1 {time=0.0;paused=false;running=true;apply=true;}
 if edge2 {time=1.525;running=true;apply=true;}
 if edge3 {running=false;apply=false;}
 let previous=state_get("cutscene_time",0.0);
 if apply {
  if true {
   let v0=140.0;
   let v1=80.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t*t*(3.0-2.0*t);
    v0=140.0+(240.0-140.0)*f;
    v1=80.0+(120.0-80.0)*f;
   }
   if time>=3.0 {
    v0=240.0;
    v1=120.0;
   }
   let b0=180.0;
   let b1=160.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t*t*(3.0-2.0*t);
    b0=180.0+(240.0-180.0)*f;
    b1=160.0+(120.0-160.0)*f;
   }
   if time>=3.0 {
    b0=240.0;
    b1=120.0;
   }
   let w=game_get("reach_blend",0.5);if w<0.0 {w=0.0;}if w>1.0 {w=1.0;}
   if w==1.0 {v0=b0;}else {if w>0.0 {v0=v0+(b0-v0)*w;}}
   if w==1.0 {v1=b1;}else {if w>0.0 {v1=v1+(b1-v1)*w;}}
   tween_position("target",v0,v1,0.0,"linear");
  }
  if true {
   let target0=140.0;
   let target1=80.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t*t*(3.0-2.0*t);
    target0=140.0+(240.0-140.0)*f;
    target1=80.0+(120.0-80.0)*f;
   }
   if time>=3.0 {
    target0=240.0;
    target1=120.0;
   }
   let other0=180.0;
   let other1=160.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t*t*(3.0-2.0*t);
    other0=180.0+(240.0-180.0)*f;
    other1=160.0+(120.0-160.0)*f;
   }
   if time>=3.0 {
    other0=240.0;
    other1=120.0;
   }
   let w=game_get("reach_blend",0.5);if w<0.0 {w=0.0;}if w>1.0 {w=1.0;}
   target0=target0+(other0-target0)*w;
   target1=target1+(other1-target1)*w;
   solve_ik_2d("upper","lower","tip",target0,target1,1.0,1.0);
  }
 }
 if !edge3 && !edge2 {
  if edge1 {state_set("cutscene_cue_0",false);}
  if !state_get("cutscene_cue_0",false) && (!started || edge1) {
   emit_signal("marker",1.0);
   state_set("cutscene_cue_0",true);
  }
  if edge1 {state_set("cutscene_cue_1",false);}
  if !state_get("cutscene_cue_1",false) && (started && running && !paused && !edge1 && previous<0.5 && time>=0.5) {
   queue_sound("chime");
   state_set("cutscene_cue_1",true);
  }
  if edge1 {state_set("cutscene_cue_2",false);}
  if !state_get("cutscene_cue_2",false) && (started && running && !paused && !edge1 && previous<1.0 && time>=1.0) {
   emit_signal("marker",1.0);
   state_set("cutscene_cue_2",true);
  }
  if edge1 {state_set("cutscene_cue_3",false);}
  if !state_get("cutscene_cue_3",false) && (started && running && !paused && !edge1 && previous<3.05 && time>=3.05) {
   emit_signal("done",1.0);
   state_set("cutscene_cue_3",true);
  }
 }
 if time>=3.05 {running=false;}
 state_set("cutscene_time",time);state_set("cutscene_started",true);state_set("cutscene_paused",paused);state_set("cutscene_running",running);
}
