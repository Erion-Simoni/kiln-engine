#!/usr/bin/env python3
"""Author joint/camera references, reject broken edits, resolve them and delete safely."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, color_matches, record_failure


def main():
    output = Path(os.environ.get('KILN_ENTITY_REFERENCE_EVIDENCE', REPO/'target/a02-entity-references')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':88')

    def saved():
        return tomllib.loads(manifest.read_text())

    def joint():
        ui = next(e for e in saved()['scenes']['main']['entities'] if e['id'] == 'ui')
        return next(c['joint'] for c in ui['components'] if c['type'] == 'Joint')

    def checkpoint():
        editor.save()
        return saved()

    def follow(target):
        editor.held_click(90, 308)
        def painted():
            width, height, pixels = editor.frame()
            assert (width, height) == (1440, 900)
            return color_matches(pixels[(394*width+135)*3:(394*width+135)*3+3], (35, 38, 44))
        editor.wait(painted, description='Render camera follow-target choices')
        editor.held_click(85, 366 if target == 'player' else 338)

    def refuse_delete(before, screenshot, reason):
        editor.held_click(100, 166); editor.held_click(130, 234)
        editor.wait(lambda: reason in editor.diagnostic(), description='Explain blocked reference-safe deletion')
        def visible_error():
            width, height, pixels = editor.frame()
            assert (width, height) == (1440, 900)
            red = 0
            for y in range(300, 600):
                for x in range(400, 1040):
                    r, g, b = pixels[(y*width+x)*3:(y*width+x)*3+3]
                    red += r > 180 and r > g*1.4 and r > b*1.3
            return red > 120
        editor.wait(visible_error, description='Show deletion error above editor panels')
        assert saved() == before, 'Referenced entity deletion changed authored data'
        editor.shot(screenshot); editor.key('Escape'); editor.save()
        assert saved() == before

    try:
        editor.start(); checkpoint()
        editor.held_click(100, 196); editor.held_click(1200, 218); editor.held_click(1190, 244)
        added = checkpoint()
        assert joint()['entity_a'] == 'ui' and joint()['entity_b'] == 'player'
        editor.text(1300, 272, 'missing'); editor.held_click(1190, 609); editor.save()
        assert saved() == added, 'Invalid joint draft was published'
        editor.shot('invalid_endpoint_retained')
        # Correct a rejected form; both fields are an explicit single edit.
        editor.text(1300, 272, 'player'); editor.value(1200, 382, '48')
        editor.save(); assert saved() == added
        editor.held_click(1190, 609); changed = checkpoint()
        assert joint()['entity_b'] == 'player' and joint()['length'] == 48
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved() == added, description='One Undo restores whole applied joint draft')
        editor.value(1200, 382, '55'); editor.held_click(1290, 609); editor.save()
        assert saved() == added, 'Cancel changed persisted joint properties'
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved() == changed, description='Cancelled joint draft preserves prior redo')
        editor.shot('joint_redo_after_cancel')
        refuse_delete(changed, 'joint_endpoint_delete_refused', 'is used by a joint on `ui`')
        editor.held_click(100, 196); editor.held_click(1190, 638)
        no_joint = checkpoint()
        controller = next(e for e in no_joint['scenes']['main']['entities'] if e['id'] == 'ui')
        assert not any(c['type'] == 'Joint' for c in controller['components'])
        editor.panel(80, 690); editor.held_click(98, 280)
        follow('player')
        camera = checkpoint()
        assert camera['cameras']['main']['target'] == 'player'
        editor.held_click(125, 132)
        refuse_delete(camera, 'camera_target_delete_refused', 'is the follow target of camera `main`')
        # Remove the final dependency through camera widgets, then deletion works.
        editor.panel(80, 690); follow(None)
        before_delete = checkpoint()
        assert before_delete['cameras']['main'].get('target') is None
        editor.held_click(125, 132); editor.held_click(100, 166); editor.held_click(130, 234)
        deleted = checkpoint()
        assert [e['id'] for e in deleted['scenes']['main']['entities']] == ['ui']
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved() == before_delete, description='Undo restores full entity after references resolved')
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved() == deleted, description='Redo exact reference-safe deletion')
        expected = saved(); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':88')
        editor.start(); editor.save(); assert saved() == expected
        editor.shot('reopened')
        result = dict(passed=True, invalid_joint_draft_retained=True, corrected_joint_apply_one_undo=True,
                      joint_cancel_preserves_redo=True, camera_and_joint_delete_guards=True,
                      resolved_delete_undo_redo=True, exact_complete_restart=True,
                      scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'entity-references', 'widgets', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
