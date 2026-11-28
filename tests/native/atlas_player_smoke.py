#!/usr/bin/env python3
"""Relocated native green-pixel replay: atlas fixture or an explicitly supplied project."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
from task9_editor_games import REPO, stop_process
from ui_smoke import executable, wait_for


def main():
    output=Path(os.environ.get('KILN_ATLAS_PLAYER_EVIDENCE',REPO/'target/a03-atlas-player')).resolve()
    output.mkdir(parents=True,exist_ok=True);(output/'result.json').unlink(missing_ok=True)
    supplied=os.environ.get('KILN_ATLAS_PROJECT')
    original=Path(supplied) if supplied else Path(json.loads((REPO/'target/a03-atlas-recipe/result.json').read_text())['project'])
    scenario=os.environ.get('KILN_PIXEL_SCENARIO')
    pointer_test=os.environ.get('KILN_PIXEL_TEST_POINTER')=='1'
    animation_test=os.environ.get('KILN_PIXEL_TEST_ANIMATION_STATES')=='1'
    expect_warning=os.environ.get('KILN_EXPECT_SPRITE_DROPS')=='1'
    expected_bounds=[int(n) for n in os.environ['KILN_PIXEL_EXPECT_GREEN_BOUNDS'].split(',')] if os.environ.get('KILN_PIXEL_EXPECT_GREEN_BOUNDS') else None
    if expected_bounds:assert len(expected_bounds)==4
    observed_bounds=None
    require_motion=os.environ.get('KILN_PIXEL_REQUIRE_MOTION')=='1'
    require_ready=os.environ.get('KILN_PIXEL_REQUIRE_BLUE_READY')=='1'
    seen_bounds=set();blue_pixels=0
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));project=run/'project';shutil.copytree(original,project)
    env=os.environ.copy();env.update(DISPLAY=':86',XDG_RUNTIME_DIR=str(run/'runtime'),XDG_DATA_HOME=str(run/'data'),
        XDG_CONFIG_HOME=str(run/'config'),XDG_CACHE_HOME=str(run/'cache'),ALSA_CONFIG_PATH=str(run/'alsa.conf'),LIBGL_ALWAYS_SOFTWARE='1')
    (run/'runtime').mkdir(mode=0o700);(run/'alsa.conf').write_text('pcm.!default { type null }\n')
    if Path('/tmp/.X11-unix/X86').exists():raise RuntimeError('Display :86 occupied')
    server_log=output/'xvfb.log';player_log=output/'player.log';server=player=None
    try:
        server=subprocess.Popen([executable('Xvfb'),':86','-screen','0','1440x900x24','-nolisten','tcp','-ac'],env=env,
            stdout=server_log.open('w'),stderr=subprocess.STDOUT)
        xdo=executable('xdotool')
        wait_for(lambda:subprocess.run([xdo,'getdisplaygeometry'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0,server,server_log)
        game=run/'relocated game'
        if os.environ.get('KILN_ATLAS_NATIVE_EXPORT'):
            shutil.copytree(Path(os.environ['KILN_ATLAS_NATIVE_EXPORT']),game)
        else:
            cli=Path(os.environ.get('KILN_CLI_PATH',REPO/'target/debug/kiln-cli')).resolve()
            child=env.copy();child['PATH']=''
            subprocess.run([str(cli),'export','--project',str(project/'kiln.toml'),'--output',str(game)],env=child,
                check=True,stdout=(output/'export.log').open('w'),stderr=subprocess.STDOUT)
        project.rename(run/'source-hidden')
        unrelated=run/'unrelated';unrelated.mkdir();child=env.copy();child['PATH']=''
        player=subprocess.Popen([str(game/'game')],cwd=unrelated,env=child,stdout=player_log.open('w'),stderr=subprocess.STDOUT)
        def window():
            result=subprocess.run([xdo,'search','--name','Kiln Game'],env=env,capture_output=True,text=True)
            return result.stdout.strip().splitlines()[0] if result.returncode==0 else None
        handle=wait_for(window,player,player_log)
        subprocess.run([xdo,'windowfocus','--sync',handle],env=env,check=True,timeout=10)
        subprocess.run([xdo,'key','--clearmodifiers','Return'],env=env,check=True,timeout=10)
        require_blend=os.environ.get('KILN_PIXEL_REQUIRE_BLEND')=='1'; mixed_seen=set()
        def pixels():
            nonlocal observed_bounds,blue_pixels
            ppm=subprocess.check_output([executable('import'),'-display',':86','-window',handle,'-depth','8','ppm:-'],env=env)
            header,frame=ppm.split(b'\n255\n',1);width,height=map(int,header.splitlines()[-1].split())
            green=sum(1 for at in range(0,len(frame),3) if ((frame[at]>220 and 30<frame[at+1]<200 and frame[at+2]<180) if expect_warning else (frame[at+1]>220 and frame[at]<30 and frame[at+2]<30)))
            if require_blend:
                for label,x0,x1 in [('fade',96,128),('tree',208,240)]:
                    count=sum(1 for y in range(104,136) for x in range(x0,x1) if frame[(y*width+x)*3]>40 and frame[(y*width+x)*3+1]>40 and frame[(y*width+x)*3+2]<30)
                    if count>200:mixed_seen.add(label)
            if (expected_bounds or require_motion) and green:
                hits=[at//3 for at in range(0,len(frame),3) if frame[at+1]>220 and frame[at]<30 and frame[at+2]<30]
                observed_bounds=[min(n%width for n in hits),min(n//width for n in hits),max(n%width for n in hits)+1,max(n//width for n in hits)+1]
                seen_bounds.add(tuple(observed_bounds))
                if expected_bounds and observed_bounds!=expected_bounds:return None
            if require_motion and len(seen_bounds)<2:return None
            if require_ready:
                blue_pixels=sum(1 for at in range(0,len(frame),3) if frame[at+2]>220 and frame[at]<30 and frame[at+1]<30)
                if blue_pixels<=20:return None
            if require_blend and len(mixed_seen)<2:return None
            return (width,height,green) if green>200 else None
        if pointer_test:
            subprocess.run([xdo,'windowsize','--sync',handle,'640','240'],env=env,check=True,timeout=10)
            subprocess.run([xdo,'mousemove','--sync','--window',handle,'320','120'],env=env,check=True,timeout=10)
            wait_for(pixels,player,player_log)
            subprocess.run([executable('import'),'-display',':86','-window',handle,str(output/'pointer-valid.png')],env=env,check=True)
            subprocess.run([xdo,'mousemove','--sync','--window',handle,'40','120'],env=env,check=True,timeout=10)
            def invalid_pointer():
                ppm=subprocess.check_output([executable('import'),'-display',':86','-window',handle,'-depth','8','ppm:-'],env=env)
                frame=ppm.split(b'\n255\n',1)[1]
                green=sum(1 for at in range(0,len(frame),3) if frame[at+1]>220 and frame[at]<30 and frame[at+2]<30)
                red=sum(1 for at in range(0,len(frame),3) if frame[at]>220 and frame[at+1]<30 and frame[at+2]<30)
                return green==0 and red>200
            wait_for(invalid_pointer,player,player_log)
            subprocess.run([executable('import'),'-display',':86','-window',handle,str(output/'pointer-invalid.png')],env=env,check=True)
            subprocess.run([xdo,'mousemove','--sync','--window',handle,'320','120'],env=env,check=True,timeout=10)
            wait_for(pixels,player,player_log)
            subprocess.run([xdo,'mousemove','--sync','--window',handle,'700','120'],env=env,check=True,timeout=10)
            wait_for(invalid_pointer,player,player_log)
            subprocess.run([executable('import'),'-display',':86','-window',handle,str(output/'pointer-outside.png')],env=env,check=True)
            subprocess.run([xdo,'mousemove','--sync','--window',handle,'320','120'],env=env,check=True,timeout=10)
        width,height,green=wait_for(pixels,player,player_log)
        if animation_test:
            def phase(kind):
                ppm=subprocess.check_output([executable('import'),'-display',':86','-window',handle,'-depth','8','ppm:-'],env=env)
                header,frame=ppm.split(b'\n255\n',1);w=int(header.splitlines()[-1].split()[0]);red=blue=0;green=[]
                for at in range(0,len(frame),3):
                    r,g,b=frame[at:at+3]
                    if g>220 and r<30 and b<30:green.append(at//3%w)
                    if r>220 and g<30 and b<30:red+=1
                    if b>220 and r<30 and g<30:blue+=1
                if kind=='moving':return len(green)>1000 and min(green)>150
                if kind=='idle':return red>1000 and not green and blue<200
                return blue>1000 and not green
            subprocess.run([xdo,'keydown','Right'],env=env,check=True,timeout=10)
            wait_for(lambda:phase('moving'),player,player_log)
            subprocess.run([xdo,'keyup','Right'],env=env,check=True,timeout=10)
            wait_for(lambda:phase('idle'),player,player_log)
            subprocess.run([xdo,'keydown','space'],env=env,check=True,timeout=10)
            wait_for(lambda:phase('action'),player,player_log)
            subprocess.run([xdo,'keyup','space'],env=env,check=True,timeout=10)
            wait_for(lambda:phase('idle'),player,player_log)
            subprocess.run([xdo,'keydown','Right'],env=env,check=True,timeout=10)
            expected_bounds=[256,88,320,152]
            width,height,green=wait_for(pixels,player,player_log)

        subprocess.run([executable('import'),'-display',':86','-window',handle,str(output/('rendered_scene.png' if scenario else 'generated_clip.png'))],env=env,check=True)
        result=dict(passed=True,relocated_native_export=True,authoring_copy_hidden=True,developer_tools_absent_from_player_path=True,
                    frame=[width,height],scope='Linux Xvfb/software graphics')
        result['rendered_warning_pixels' if expect_warning else ('rendered_green_pixels' if scenario else 'generated_clip_green_pixels')]=green
        if require_blend:result['mixed_texture_phases']=sorted(mixed_seen)
        if animation_test:result['animation_input_phases']=['run-moves','idle-on-release','one-shot-action','idle-on-finish','run-at-boundary']
        if pointer_test:result['pointer_phases']=['inside','letterbox-invalid','inside','outside-invalid','inside']
        if scenario:result['scenario']=scenario
        if expected_bounds:result['observed_green_bounds']=observed_bounds
        if require_motion:result['distinct_green_bounds']=len(seen_bounds)
        if require_ready:result['blue_ready_pixels']=blue_pixels
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    finally:
        stop_process(player);stop_process(server)


if __name__=='__main__':main()
