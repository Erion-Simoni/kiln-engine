#!/usr/bin/env python3
"""Real native player/editor material pixels on Xvfb, with relocated export."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
sys.dont_write_bytecode=True
from ui_smoke import REPO,TOOLS,executable,wait_for


def main():
    output=Path(os.environ.get('KILN_MATERIAL_EVIDENCE',REPO/'target/task8-material-native-evidence')).resolve()
    output.mkdir(parents=True,exist_ok=True);(output/'result.json').unlink(missing_ok=True)
    run=Path(tempfile.mkdtemp(prefix='profile-',dir=output));project=run/'project'
    shutil.copytree(REPO/'samples/materials2d',project)
    env=os.environ.copy();env.update(DISPLAY=':84',XDG_DATA_HOME=str(run/'data'),XDG_CONFIG_HOME=str(run/'config'),XDG_CACHE_HOME=str(run/'cache'),XDG_RUNTIME_DIR=str(run/'runtime'),LIBGL_ALWAYS_SOFTWARE='1',ALSA_CONFIG_PATH=str(run/'alsa.conf'),RUST_LOG='warn,kiln_player=info,kiln_editor=info')
    if TOOLS.is_dir():env['LD_LIBRARY_PATH']=str(TOOLS/'usr/lib/x86_64-linux-gnu')+':'+env.get('LD_LIBRARY_PATH','')
    if Path('/usr/lib/chromium/vk_swiftshader_icd.json').exists():env['VK_ICD_FILENAMES']='/usr/lib/chromium/vk_swiftshader_icd.json'
    (run/'runtime').mkdir(mode=0o700);(run/'alsa.conf').write_text('pcm.!default { type null }\n')
    if Path('/tmp/.X11-unix/X84').exists():raise RuntimeError('Display :84 occupied')
    server_log=output/'xvfb.log';server=subprocess.Popen([executable('Xvfb'),':84','-screen','0','1440x900x24','-nolisten','tcp','-ac'],env=env,stdout=server_log.open('w'),stderr=subprocess.STDOUT)
    process=None
    try:
        xdo=executable('xdotool')
        wait_for(lambda:subprocess.run([xdo,'getdisplaygeometry'],env=env,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL).returncode==0,server,server_log)
        destination=run/'export';export_env=env.copy();export_env['PATH']=''
        subprocess.run([str(REPO/'target/debug/kiln-cli'),'export','--project',str(project/'kiln.toml'),'--output',str(destination)],env=export_env,check=True,stdout=(output/'export.log').open('w'))
        moved=Path(tempfile.mkdtemp(prefix='kiln-native-materials-'))/'game folder';shutil.move(destination,moved)
        receipt=json.loads((moved/'export.json').read_text());assert receipt['runtime']['materials']
        log=output/'player.log'
        def launch(binary,title):
            proc=subprocess.Popen([str(binary)],cwd=run,env=env,stdout=log.open('w'),stderr=subprocess.STDOUT)
            def search():
                r=subprocess.run([xdo,'search','--name',title],env=env,capture_output=True,text=True)
                return r.stdout.strip().splitlines()[0] if r.returncode==0 else None
            window=wait_for(search,proc,log);subprocess.run([xdo,'windowfocus','--sync',window],env=env,check=True);time.sleep(1.5)
            return proc,window
        def key(k):
            subprocess.run([xdo,'keydown',k],env=env,check=True);time.sleep(.08)
            subprocess.run([xdo,'keyup',k],env=env,check=True);time.sleep(.2)
        def shot(window,name):
            time.sleep(.4);path=output/(name+'.png')
            subprocess.run([executable('import'),'-display',':84','-window',window,str(path)],env=env,check=True)
            return path
        def pixels(path,points):
            return [list(subprocess.check_output([executable('convert'),str(path),'-crop',f'1x1+{x}+{y}','-depth','8','rgba:-'])) for x,y in points]
        process,window=launch(moved/'game','Kiln Game')
        points=[[80,100],[180,100],[280,100],[380,100],[480,100],[80,190],[180,190],[100,280],[364,280],[396,280],[480,264],[480,296]]
        colors=pixels(shot(window,'materials'),points)
        blue=lambda c:c[2]>200 and c[0]<40 and c[1]<40
        red=lambda c:c[0]>200 and c[1]<40 and c[2]<40
        assert blue(colors[0]) and blue(colors[4]),colors
        assert colors[1][1]>200 and colors[1][0]<40,colors
        assert red(colors[2]) and 90<colors[3][0]<190,colors
        assert red(colors[5]) and blue(colors[6]),colors
        assert colors[7][1]>colors[7][0] and abs(colors[7][0]-colors[7][2])<10,colors
        assert blue(colors[8]) and red(colors[9]) and blue(colors[10]) and red(colors[11]),colors
        key('s');key('n');assert blue(pixels(shot(window,'next-scene'),[[80,100]])[0])
        key('l');assert pixels(shot(window,'restored'),points)==colors
        assert not any(error in log.read_text() for error in ['Validation Error','panicked','Material error']),log.read_text()
        process.terminate();process.wait(timeout=10);process=None
        # Editor uses a disposable working copy; verify the same project is previewed.
        config=run/'config/kiln';config.mkdir(parents=True,exist_ok=True)
        (config/'project_hub.json').write_text(json.dumps([{'path':str(project/'kiln.toml'),'name':'Materials','last_opened':0}]))
        log=output/'editor.log';process,window=launch(REPO/'target/debug/kiln-editor','Kiln Editor -')
        editor_shot=shot(window,'editor-materials')
        # Count unmistakable saturated red/green pixels inside the actual scene panel.
        crop=subprocess.check_output([executable('convert'),str(editor_shot),'-crop','850x570+245+85','-depth','8','rgba:-'])
        red_count=sum(crop[i]>240 and crop[i+1]<20 and crop[i+2]<20 for i in range(0,len(crop),4))
        green_count=sum(crop[i+1]>240 and crop[i]<20 and crop[i+2]<20 for i in range(0,len(crop),4))
        assert red_count>100 and green_count>100,(red_count,green_count,log.read_text())
        subprocess.run([xdo,'mousemove','--window',window,'80','12','click','1'],env=env,check=True)
        shot(window,'editor-view-menu')
        result={'passed':True,'runtime_materials':True,'native_pixels':colors,'points':points,'scene_save_restore':True,'editor_preview_red_pixels':red_count,'editor_preview_green_pixels':green_count,'relocated_export':str(moved)}
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    finally:
        if process is not None and process.poll() is None:process.terminate();process.wait(timeout=10)
        server.terminate();server.wait(timeout=10)


if __name__=='__main__':main()
