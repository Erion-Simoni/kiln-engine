#!/usr/bin/env python3
"""Save/reopen real per-target import settings, live preview, asset move and diagnostics."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib
from task9_editor_games import Editor, REPO, record_failure


def main():
    output=Path(os.environ.get('KILN_ASSET_IMPORT_EVIDENCE',REPO/'target/a03-asset-import')).resolve()
    output.mkdir(parents=True,exist_ok=True);run=Path(tempfile.mkdtemp(prefix='run-',dir=output));project=run/'project'
    shutil.copytree(REPO/'samples/vertical_slice',project);manifest=project/'kiln.toml'
    editor=Editor(REPO/'target/debug',project,output/'editor',':85')
    def saved():return tomllib.loads(manifest.read_text())
    try:
        editor.start();editor.held_click(65,688);editor.panel(80,96);editor.enlarge()
        editor.held_click(1055,166);editor.save()
        editor.wait(lambda:'art/hero.png' in saved().get('asset_ids',{}),description='Import allocates stable asset identity')
        identity=saved()['asset_ids']['art/hero.png'];default=saved()['asset_import_settings'][identity]
        editor.held_click(715,220);editor.held_click(220,380);editor.shot('preview_alive')
        editor.held_click(70,451);editor.held_click(70,594);editor.held_click(112,480);editor.held_click(231,480);editor.save()
        editor.wait(lambda:saved().get('asset_platform_import_settings',{}).get(identity,{}).get('web',{}).get('nearest_filter') is True,description='Save Web import override')
        web=saved()['asset_platform_import_settings'][identity]['web'];assert web['generate_mipmaps'] is False
        assert saved()['asset_import_settings'][identity]==default,'Web override changed native/default settings'
        editor.shot('target_settings_saved');editor.held_click(960,166)
        editor.text(980,177,'art/hero-renamed.png');editor.held_click(872,203);editor.save()
        editor.wait(lambda:(project/'art/hero-renamed.png').exists() and not (project/'art/hero.png').exists(),description='Move art through asset dialog')
        editor.wait(lambda:saved()['asset_ids'].get('art/hero-renamed.png')==identity,description='Move retains stable asset ID')
        assert saved()['asset_platform_import_settings'][identity]['web']==web
        sprites=[c for scene in saved()['scenes'].values() for entity in scene['entities'] for c in entity['components'] if c['type']=='Sprite']
        assert any(c.get('texture')=='art/hero-renamed.png' for c in sprites);assert all(c.get('texture')!='art/hero.png' for c in sprites)
        editor.held_click(180,380);editor.held_click(180,408)
        editor.wait(lambda:'no missing references or duplicates' in (output/'editor/editor.log').read_text(),description='Background asset dependency diagnostics')
        editor.shot('renamed_dependencies_checked');expected=saved();editor.close()
        editor=Editor(REPO/'target/debug',project,output/'reopened',':85');editor.start();editor.save()
        assert saved()==expected,'Restart changed per-target settings or references';editor.shot('reopened')
        result={'passed':True,'gpu_preview_survives_frames':True,'per_target_import_settings':True,'rename_preserves_identity_and_overrides':True,'dependency_check':True,'save_restart_equal':True}
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception as error:
        record_failure(output,'asset-import','widgets',error)
        if editor.process and editor.process.poll() is None:editor.shot('failure')
        raise
    finally:editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':main()
