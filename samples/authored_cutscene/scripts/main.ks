// kiln-cutscene-v1 {"version":1,"tracks":[{"entity":"actor","property":"position","keys":[{"time":"0","value":["80","120"],"ease":"linear"},{"time":"1","value":["144","88"],"ease":"smooth"},{"time":"3","value":["256","120"],"ease":"smooth"}]},{"entity":"actor","property":"rotation","keys":[{"time":"0","value":["0"],"ease":"linear"},{"time":"3","value":["7.853981633974483"],"ease":"linear"}]},{"entity":"actor","property":"scale","keys":[{"time":"0","value":["1","1"],"ease":"linear"},{"time":"1","value":["0.75","1.5"],"ease":"smooth"},{"time":"3","value":["2","1"],"ease":"smooth"}]},{"entity":"actor","property":"color","keys":[{"time":"0","value":["1","0","0","1"],"ease":"linear"},{"time":"3","value":["0","1","0","1"],"ease":"linear"}]},{"entity":"camera_rig","property":"position","keys":[{"time":"0","value":["160","120"],"ease":"linear"},{"time":"3","value":["192","120"],"ease":"smooth"}]},{"entity":"ready","property":"color","keys":[{"time":"0","value":["1","0","0","1"],"ease":"linear"},{"time":"3","value":["1","0","0","1"],"ease":"linear"},{"time":"3.05","value":["0","0","1","1"],"ease":"linear"}]}],"actions":["pause","restart","seek","cancel"]}
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
 if apply {
  if true {
   let v0=80.0;
   let v1=120.0;
   if time>=0.0 && time<1.0 {let t=(time-0.0)/1.0;
    let f=t*t*(3.0-2.0*t);
    v0=80.0+(144.0-80.0)*f;
    v1=120.0+(88.0-120.0)*f;
   }
   if time>=1.0 && time<3.0 {let t=(time-1.0)/2.0;
    let f=t*t*(3.0-2.0*t);
    v0=144.0+(256.0-144.0)*f;
    v1=88.0+(120.0-88.0)*f;
   }
   if time>=3.0 {
    v0=256.0;
    v1=120.0;
   }
   tween_position("actor",v0,v1,0.0,"linear");
  }
  if true {
   let v0=0.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t;
    v0=0.0+(7.853981633974483-0.0)*f;
   }
   if time>=3.0 {
    v0=7.853981633974483;
   }
   tween_rotation("actor",v0,0.0,"linear");
  }
  if true {
   let v0=1.0;
   let v1=1.0;
   if time>=0.0 && time<1.0 {let t=(time-0.0)/1.0;
    let f=t*t*(3.0-2.0*t);
    v0=1.0+(0.75-1.0)*f;
    v1=1.0+(1.5-1.0)*f;
   }
   if time>=1.0 && time<3.0 {let t=(time-1.0)/2.0;
    let f=t*t*(3.0-2.0*t);
    v0=0.75+(2.0-0.75)*f;
    v1=1.5+(1.0-1.5)*f;
   }
   if time>=3.0 {
    v0=2.0;
    v1=1.0;
   }
   tween_scale("actor",v0,v1,0.0,"linear");
  }
  if true {
   let v0=1.0;
   let v1=0.0;
   let v2=0.0;
   let v3=1.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t;
    v0=1.0+(0.0-1.0)*f;
    v1=0.0+(1.0-0.0)*f;
    v2=0.0+(0.0-0.0)*f;
    v3=1.0+(1.0-1.0)*f;
   }
   if time>=3.0 {
    v0=0.0;
    v1=1.0;
    v2=0.0;
    v3=1.0;
   }
   tween_color("actor",v0,v1,v2,v3,0.0,"linear");
  }
  if true {
   let v0=160.0;
   let v1=120.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t*t*(3.0-2.0*t);
    v0=160.0+(192.0-160.0)*f;
    v1=120.0+(120.0-120.0)*f;
   }
   if time>=3.0 {
    v0=192.0;
    v1=120.0;
   }
   tween_position("camera_rig",v0,v1,0.0,"linear");
  }
  if true {
   let v0=1.0;
   let v1=0.0;
   let v2=0.0;
   let v3=1.0;
   if time>=0.0 && time<3.0 {let t=(time-0.0)/3.0;
    let f=t;
    v0=1.0+(1.0-1.0)*f;
    v1=0.0+(0.0-0.0)*f;
    v2=0.0+(0.0-0.0)*f;
    v3=1.0+(1.0-1.0)*f;
   }
   if time>=3.0 && time<3.05 {let t=(time-3.0)/0.04999999999999982;
    let f=t;
    v0=1.0+(0.0-1.0)*f;
    v1=0.0+(0.0-0.0)*f;
    v2=0.0+(1.0-0.0)*f;
    v3=1.0+(1.0-1.0)*f;
   }
   if time>=3.05 {
    v0=0.0;
    v1=0.0;
    v2=1.0;
    v3=1.0;
   }
   tween_color("ready",v0,v1,v2,v3,0.0,"linear");
  }
 }
 if time>=3.05 {running=false;}
 state_set("cutscene_time",time);state_set("cutscene_started",true);state_set("cutscene_paused",paused);state_set("cutscene_running",running);
}
