#!/usr/bin/env python3
"""Force atlas publication crashes before autosave and after a real recovery snapshot."""
import json
import os
from pathlib import Path
import shutil
import sys
import struct
import tempfile
import tomllib
from task9_editor_games import Editor, REPO, record_failure


def canonical(value):
    if isinstance(value,dict):return {k:canonical(v) for k,v in value.items() if v is not None}
    if isinstance(value,list):return [canonical(v) for v in value]
    return value


def main():
    output=Path(os.environ.get('KILN_ATLAS_RECOVERY_EVIDENCE',REPO/'target/a03-atlas-recovery')).resolve()
    output.mkdir(parents=True,exist_ok=True)
    for stale in (output/'result.json',output/'failure.json',output/'editor/failure.png'):stale.unlink(missing_ok=True)
    fixture=json.loads((REPO/'target/a03-atlas-recipe/result.json').read_text())
    project=Path(tempfile.mkdtemp(prefix='run-',dir=output))/'project';shutil.copytree(Path(fixture['project']),project)
    manifest=project/'kiln.toml';atlas=project/'art/atlas.png';recovery=project/'.kiln.toml.recovery.json'
    original_manifest=manifest.read_bytes();original_atlas=atlas.read_bytes()
    original_sources={p.relative_to(project):p.read_bytes() for p in project.rglob('*')
                      if p.is_file() and (p.suffix=='.ks' or p.parent==project/'art' and p.name!='atlas.png')}
    editor=Editor(REPO/'target/debug',project,output/'editor',':86')

    def saved():return tomllib.loads(manifest.read_text())
    def field(x,y,value):
        editor.held_click(x,y);editor.key('ctrl+a');editor.command('type','--clearmodifiers','--delay','2',value)
    def open_atlas():
        editor.panel(80,578);editor.enlarge();editor.held_click(170,166)
        editor.held_click(520,238);editor.held_click(490,266)
    def publish_new_source():
        open_atlas();field(650,408,'aaa');field(650,436,'art/coin.png')
        editor.held_click(500,464);editor.held_click(525,530)
        editor.wait(lambda:atlas.read_bytes()!=original_atlas,description='Publish unsaved atlas with shifted hero region')
        assert manifest.read_bytes()==original_manifest
    def crash():
        editor.process.kill();editor.process.wait(timeout=10);editor.start()
    def green_pixels():
        width,height,pixels=editor.frame();assert (width,height)==(1440,900)
        return sum(1 for y in range(175,650) for x in range(400,1100)
                   if pixels[(y*width+x)*3+1]>220 and pixels[(y*width+x)*3]<30 and pixels[(y*width+x)*3+2]<30)

    try:
        initial=saved();editor.start();editor.panel(18,102)
        assert saved()==initial
        original_manifest=manifest.read_bytes();assert not recovery.exists()
        publish_new_source();assert not recovery.exists(),'First crash must precede the real autosave'
        editor.shot('uncommitted_atlas_published');crash()
        editor.wait(lambda:atlas.read_bytes()==original_atlas,description='Startup reconstructs saved atlas before autosave exists')
        assert manifest.read_bytes()==original_manifest
        editor.wait(lambda:green_pixels()>200,description='Saved hero renders correctly after interrupted atlas rollback')
        editor.shot('saved_atlas_reconstructed');editor.panel(18,102)
        original_manifest=manifest.read_bytes()
        publish_new_source();new_atlas=atlas.read_bytes()
        def autosaved():
            return recovery.exists() and 'aaa' in json.loads(recovery.read_text())['project']['atlases']['atlas']['sources']
        editor.wait(autosaved,seconds=45,description='Real autosave includes changed atlas recipe and generated clips')
        snapshot=canonical(json.loads(recovery.read_text())['project'])
        (output/'snapshot-before-crash.json').write_text(json.dumps(snapshot,indent=2)+'\n')
        # JSON emits shortest f32 decimals; TOML represents f32 as an f64 value.
        # Normalize the typed frame duration, retaining exact equality elsewhere.
        for clip in snapshot['animation_clips'].values():
            for frame in clip['frames']:
                frame['duration']=struct.unpack('f',struct.pack('f',frame['duration']))[0]
        assert snapshot['animation_clips']['atlas:atlas:hero']['frames'][0]['rect'][0]>2
        crash()
        editor.wait(lambda:atlas.read_bytes()==original_atlas,description='Saved generation is selected before recovery choice')
        editor.held_click(590,499);editor.panel(18,102)
        editor.wait(lambda:'aaa' in saved()['atlases']['atlas']['sources'],description='Restore and save the autosaved atlas recipe')
        (output/'saved-after-restore.json').write_text(json.dumps(canonical(saved()),indent=2)+'\n')
        assert canonical(saved())==snapshot,'Restoring the atlas changed unrelated project properties'
        assert atlas.read_bytes()==new_atlas,'Recovery draft did not reconstruct its matching PNG generation'
        editor.wait(lambda:green_pixels()>200,description='Restored generated hero samples its recovered region')
        assert not (project/'.kiln-atlas-generations.json').exists(),'Successful Save retained pending provenance'
        assert {p.relative_to(project):p.read_bytes() for p in project.rglob('*')
                if p.is_file() and p.relative_to(project) in original_sources}==original_sources
        editor.shot('draft_atlas_reconstructed');expected=saved();editor.close()
        editor=Editor(REPO/'target/debug',project,output/'reopened',':86');editor.start();editor.panel(18,102)
        assert saved()==expected and atlas.read_bytes()==new_atlas
        editor.wait(lambda:green_pixels()>200,description='Restart retains recovered atlas pixels and clip regions')
        editor.shot('reopened')
        result=dict(passed=True,pre_autosave_crash_repaired=True,real_timer_snapshot_restore=True,
                    selected_recipe_reconstructs_png=True,complete_project_equality=True,source_art_and_scripts_unchanged=True,
                    saved_provenance_compacted=True,scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception as error:
        record_failure(output,'atlas-recovery','widgets/crash',error)
        if editor.process and editor.process.poll() is None:editor.shot('failure')
        raise
    finally:editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':main()
