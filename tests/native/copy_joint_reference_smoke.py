#!/usr/bin/env python3
"""Copy joint/script data into a composed scene, preserve history, recover and restart."""
from copy import deepcopy
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, project_error_visible, record_failure


def main():
    output = Path(os.environ.get('KILN_COPY_JOINT_EVIDENCE', REPO/'target/a02-copy-joint')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    recovery = project/'.kiln.toml.recovery.json'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':88')

    def saved():
        return tomllib.loads(manifest.read_text())

    def checkpoint():
        editor.save()
        return saved()

    def assert_joint(entity):
        joint = next(c['joint'] for c in entity['components'] if c['type'] == 'Joint')
        assert joint['entity_a'] == entity['id'] and joint['entity_b'] == 'player'
        assert joint['length'] == 32
        assert any(c['type'] == 'Script' and c['path'] == 'scripts/ui.ks' for c in entity['components'])

    try:
        editor.start(); editor.held_click(100, 196)
        editor.held_click(1200, 218); editor.held_click(1190, 244)
        source = checkpoint()['scenes']['main']
        editor.key('ctrl+c'); editor.held_click(261, 145)
        empty = checkpoint()
        assert empty['scenes']['scene_1']['entities'] == []
        # Independent scenes cannot resolve the clipboard's external endpoint.
        editor.key('ctrl+v')
        editor.wait(lambda: project_error_visible(*editor.frame()), description='Refuse clipboard endpoint absent from target scene')
        editor.shot('unresolved_paste_refused'); editor.key('Escape'); editor.save()
        assert saved() == empty, 'Rejected paste partially changed the target scene'
        editor.panel(80, 690); editor.held_click(235, 196); editor.held_click(215, 223)
        composed = checkpoint()
        assert composed['scenes']['scene_1']['nested_scenes'][0]['scene_id'] == 'main'
        editor.held_click(125, 132)
        editor.key('ctrl+v'); editor.save()
        editor.wait(lambda: len(saved()['scenes']['scene_1']['entities']) == 1,
                    description='Paste joint with unique endpoint in nested scene')
        pasted = saved()
        assert pasted['scenes']['main'] == source
        assert_joint(pasted['scenes']['scene_1']['entities'][0])
        editor.shot('nested_endpoint_pasted')
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved() == composed, description='One Undo removes entire joint/script paste')
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved() == pasted, description='Redo restores exact identity and joint/script data')
        # A second unsaved paste must reach the actual autosave timer.
        editor.key('ctrl+v')
        def autosaved():
            if not recovery.exists(): return False
            data = json.loads(recovery.read_text())['project']
            return len(data['scenes']['scene_1']['entities']) == 2
        editor.wait(autosaved, seconds=45, description='Autosave unsaved copied joint references')
        draft = json.loads(recovery.read_text())['project']
        expected_restored = deepcopy(pasted)
        expected_restored['scenes']['scene_1']['entities'] = draft['scenes']['scene_1']['entities']
        for entity in draft['scenes']['scene_1']['entities']: assert_joint(entity)
        assert saved() == pasted, 'Autosave wrote the source manifest'
        editor.process.kill(); editor.process.wait(timeout=10); editor.start()
        editor.held_click(590, 499); editor.save()
        restored = saved()
        assert restored == expected_restored, 'Recovery changed unrelated project properties or references'
        entities = restored['scenes']['scene_1']['entities']
        assert len(entities) == 2 and entities[0]['id'] != entities[1]['id']
        for entity in entities: assert_joint(entity)
        assert restored['scenes']['main'] == source
        assert restored['scenes']['scene_1']['entities'] == draft['scenes']['scene_1']['entities']
        editor.shot('recovered_joint_copies'); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':88')
        editor.start(); editor.save(); assert saved() == restored
        editor.shot('reopened')
        result = dict(passed=True, unresolved_paste_atomic=True, composed_endpoint_paste=True,
                      joint_owner_remapped=True, source_scene_unchanged=True, one_paste_undo_redo=True,
                      real_timer_crash_recovery=True, script_and_joint_properties_preserved=True,
                      exact_restart=True, scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'copy-joint-references', 'widgets/recovery', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
