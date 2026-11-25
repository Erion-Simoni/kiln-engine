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
        def pixels():
            nonlocal observed_bounds,blue_pixels
            ppm=subprocess.check_output([executable('import'),'-display',':86','-window',handle,'-depth','8','ppm:-'],env=env)
            header,frame=ppm.split(b'\n255\n',1);width,height=map(int,header.splitlines()[-1].split())
            green=sum(1 for at in range(0,len(frame),3) if ((frame[at]>220 and 30<frame[at+1]<200 and frame[at+2]<180) if expect_warning else (frame[at+1]>220 and frame[at]<30 and frame[at+2]<30)))
            if (expected_bounds or require_motion) and green:
                hits=[at//3 for at in range(0,len(frame),3) if frame[at+1]>220 and frame[at]<30 and frame[at+2]<30]
                observed_bounds=[min(n%width for n in hits),min(n//width for n in hits),max(n%width for n in hits)+1,max(n//width for n in hits)+1]
                seen_bounds.add(tuple(observed_bounds))
                if expected_bounds and observed_bounds!=expected_bounds:return None
            if require_motion and len(seen_bounds)<2:return None
            if require_ready:
                blue_pixels=sum(1 for at in range(0,len(frame),3) if frame[at+2]>220 and frame[at]<30 and frame[at+1]<30)
                if blue_pixels<=20:return None
            return (width,height,green) if green>200 else None
        width,height,green=wait_for(pixels,player,player_log)
        subprocess.run([executable('import'),'-display',':86','-window',handle,str(output/('rendered_scene.png' if scenario else 'generated_clip.png'))],env=env,check=True)
        result=dict(passed=True,relocated_native_export=True,authoring_copy_hidden=True,developer_tools_absent_from_player_path=True,
                    frame=[width,height],scope='Linux Xvfb/software graphics')
        result['rendered_warning_pixels' if expect_warning else ('rendered_green_pixels' if scenario else 'generated_clip_green_pixels')]=green
        if scenario:result['scenario']=scenario
        if expected_bounds:result['observed_green_bounds']=observed_bounds
        if require_motion:result['distinct_green_bounds']=len(seen_bounds)
        if require_ready:result['blue_ready_pixels']=blue_pixels
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    finally:
        stop_process(player);stop_process(server)


if __name__=='__main__':main()
