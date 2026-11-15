#!/usr/bin/env python3
"""Reject ambiguous nested identities without changing either authored scene."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, record_failure


def main():
    output = Path(os.environ.get('KILN_NESTED_IDENTITY_EVIDENCE', REPO/'target/a02-nested-identity')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/"result.json", output/"failure.json"):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':87')

    def saved():
        return tomllib.loads(manifest.read_text())

    try:
        editor.start(); editor.save()
        original = saved()['scenes']['main']
        editor.held_click(395, 145); editor.save()
        editor.wait(lambda: len(saved()['scenes']) == 2, description='Duplicate scene through toolbar')
        before = saved()
        assert before['scenes']['main'] == original
        other = next(id for id in before['scenes'] if id != 'main')
        assert [e['id'] for e in before['scenes'][other]['entities']] == [e['id'] for e in original['entities']]
        editor.panel(80, 690); editor.held_click(235, 196); editor.held_click(215, 223)
        editor.save()
        assert saved() == before, 'Nested composition accepted ambiguous SourceIds and changed the project'
        editor.wait(lambda: 'Entity ID `player` is shared by scenes' in editor.diagnostic(),
                    description='Explain conflicting identity without applying a link')
        editor.shot('ambiguous_composition_refused')
        editor.held_click(125, 132); editor.key('ctrl+z'); editor.save()
        assert saved() == before, 'Rejected composition changed scene history'
        editor.key('ctrl+y'); editor.save(); assert saved() == before
        editor.close()
        # Simulate a legacy/external composition containing IDs reused across
        # scenes. This malformed input cannot be authored through the new guard.
        text = manifest.read_text()
        section = f'[scenes.{other}]'
        start = text.index(section)
        empty = text.index('nested_scenes = []', start)
        text = text[:empty] + text[empty:].replace('nested_scenes = []',
            'nested_scenes = [{ scene_id = "main", additive = true, offset = { x = 320.0, y = 40.0 } }]', 1)
        manifest.write_text(text)
        legacy = saved()
        editor = Editor(REPO/'target/debug', project, output/'legacy', ':87')
        editor.start(); editor.held_click(540, 145); editor.held_click(100, 166)
        editor.wait(lambda: 'inspector/gizmo target is disabled' in editor.diagnostic(),
                    description='Reject ambiguous loaded entity targets')
        editor.key('w')
        editor.command('mousemove', '--window', editor.window, 304, 580, 'mousedown', '1')
        editor.command('mousemove', '--window', editor.window, 332, 552)
        editor.command('mouseup', '1'); editor.save()
        assert saved() == legacy, 'Saving an ambiguous imported composition rewrote authored data'
        editor.shot('legacy_ambiguous_targets_disabled'); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':87')
        editor.start(); editor.save(); assert saved() == legacy
        result = dict(passed=True, duplicated_scene_ids_retained=True,
                      ambiguous_nesting_rejected=True, project_and_history_unchanged=True,
                      legacy_authored_data_preserved=True, ambiguous_targets_disabled=True,
                      exact_restart=True, scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'nested-identity', 'widgets', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
