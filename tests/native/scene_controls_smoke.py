#!/usr/bin/env python3
"""Real two-scene authoring: viewport selection, alignment, lock/hide and crash recovery."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import tomllib
from task9_editor_games import Editor, REPO, executable


def main():
    output=Path(os.environ.get('KILN_SCENE_CONTROLS_EVIDENCE',REPO/'target/a02-scene-controls')).resolve()
    output.mkdir(parents=True,exist_ok=True)
    run=Path(tempfile.mkdtemp(prefix='run-',dir=output));project=run/'project'
    shutil.copytree(REPO/'samples/ui2d_editor',project)
    manifest=project/'kiln.toml';sidecar=project/'.kiln.toml.recovery.json'
    editor=Editor(REPO/'target/debug',project,output/'editor',':85')
    def saved():return tomllib.loads(manifest.read_text())
    def entity(id):return next(e for e in saved()['scenes']['main']['entities'] if e['id']==id)
    def transform(e):return next(c for c in e['components'] if c['type']=='Transform')
    def flags(e):return next((c for c in e['components'] if c['type']=='EditorFlags'),{})
    def context(row,item):
        editor.command('mousemove','--window',editor.window,100,row,'click','3')
        editor.held_click(145,row+(18 if item=='lock' else 46))
    def blue():
        ppm=subprocess.check_output([executable('import'),'-display',editor.env['DISPLAY'],'-window',editor.window,'-depth','8','ppm:-'],env=editor.env,timeout=10)
        header,pixels=ppm.split(b'\n255\n',1);w,h=map(int,header.splitlines()[-1].split());assert (w,h)==(1440,900)
        return sum(1 for y in range(175,650) for x in range(250,1100) if pixels[(y*w+x)*3+2]>100 and pixels[(y*w+x)*3+2]>pixels[(y*w+x)*3]*1.3 and pixels[(y*w+x)*3+2]>pixels[(y*w+x)*3+1]*1.1)
    try:
        editor.start();editor.key('q');editor.held_click(304,580)
        editor.key('ctrl+c');editor.key('ctrl+v');editor.save()
        editor.wait(lambda:len(saved()['scenes']['main']['entities'])==3,description='Viewport-selected entity copied and pasted')
        copied=saved()['scenes']['main']['entities'][-1];assert copied['name']=='Player Copy';assert transform(copied)['x']==56.0
        editor.held_click(100,166);editor.command('keydown','Control_L');editor.held_click(100,226);editor.command('keyup','Control_L')
        editor.held_click(50,12);editor.shot('alignment_menu')
        editor.held_click(130,171);editor.shot('alignment_choices')
        editor.held_click(240,178);editor.save()
        editor.wait(lambda:transform(entity(copied['id']))['x']==40.0,description='Align selected world positions through menu')
        editor.key('ctrl+z');editor.save();editor.wait(lambda:transform(entity(copied['id']))['x']==56.0,description='Undo alignment')
        editor.key('ctrl+y');editor.save();editor.wait(lambda:transform(entity(copied['id']))['x']==40.0,description='Redo alignment')
        before=blue();context(166,'hide');editor.save()
        editor.wait(lambda:flags(entity('player')).get('hidden') is True,description='Save editor-only hide flag')
        editor.wait(lambda:blue()<before-200,description='Hide removes scene pixels')
        hidden=blue();editor.shot('hidden_in_editor');editor.held_click(677,46)
        editor.wait(lambda:blue()>hidden+200,description='Hidden editor entity remains visible in Play')
        editor.shot('visible_in_play');editor.held_click(677,46)
        context(166,'hide');editor.held_click(100,166);context(166,'lock');editor.save()
        editor.wait(lambda:flags(entity('player')).get('locked') is True,description='Save editor-only lock flag')
        old_transform=transform(entity('player'));editor.key('w')
        editor.command('mousemove','--window',editor.window,304,580,'mousedown','1')
        editor.command('mousemove','--window',editor.window,360,560);editor.command('mouseup','1');editor.save()
        assert transform(entity('player'))==old_transform,'Locked entity moved through viewport'
        editor.shot('locked_inspector')
        editor.key('ctrl+c');editor.held_click(261,145);editor.key('ctrl+v');editor.save()
        editor.wait(lambda:len(saved()['scenes'])==2,description='Create and save a second scene')
        other_id=next(key for key in saved()['scenes'] if key!='main')
        editor.wait(lambda:len(saved()['scenes'][other_id]['entities'])==1,description='Paste locked entity into second scene')
        context(166,'lock');editor.save()
        assert not flags(saved()['scenes'][other_id]['entities'][0]).get('locked',False)
        # Unsaved scene edits must reach the actual recovery timer, not a synthetic sidecar.
        editor.held_click(393,114)
        editor.wait(lambda:sidecar.exists() and len(json.loads(sidecar.read_text())['project']['scenes'][other_id]['entities'])==2,seconds=45,description='Autosave unsaved second-scene entity')
        assert len(saved()['scenes'][other_id]['entities'])==1,'Autosave wrote the manifest'
        editor.shot('unsaved_second_scene')
        editor.process.kill();editor.process.wait(timeout=10);editor.start();editor.shot('scene_recovery_prompt')
        editor.held_click(590,499);editor.save()
        editor.wait(lambda:len(saved()['scenes'][other_id]['entities'])==2 and not sidecar.exists(),description='Restore and save scene recovery')
        expected=saved();assert flags(entity('player'))['locked'] is True
        editor.close();editor=Editor(REPO/'target/debug',project,output/'reopened',':85');editor.start();editor.save();assert saved()==expected
        editor.shot('two_scenes_reopened')
        result={'passed':True,'viewport_selection':True,'alignment_undo_redo':True,'editor_hide_play_visibility':True,'lock_blocks_viewport_edit':True,'two_scene_crash_recovery_and_restart':True,'scope':'Real Linux Xvfb/software graphics widgets; no Windows/device claim'}
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception:
        editor.shot('failure');raise
    finally:editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':main()
