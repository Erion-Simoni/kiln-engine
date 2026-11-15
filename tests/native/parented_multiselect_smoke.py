#!/usr/bin/env python3
"""Verify empty-selection safety and real parent/child group gizmo transactions."""
import json
import math
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, record_failure


def main():
    output = Path(os.environ.get('KILN_PARENTED_SELECTION_EVIDENCE', REPO/'target/a02-parented-selection')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json', output/'editor/failure.png'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':86')

    def saved():
        return tomllib.loads(manifest.read_text())

    def entity(id):
        return next(e for e in saved()['scenes']['main']['entities'] if e['id'] == id)

    def pose(id):
        return next(c for c in entity(id)['components'] if c['type'] == 'Transform')

    def drag(start, finish, cancel=False):
        editor.command('mousemove', '--window', editor.window, *start, 'mousedown', '1')
        for t in (0.25, 0.5, 0.75, 1.0):
            editor.command('mousemove', '--window', editor.window,
                           *[round(a+(b-a)*t) for a, b in zip(start, finish)])
        if cancel: editor.key('Escape')
        editor.command('mouseup', '1'); editor.save()

    def additive(row):
        editor.command('keydown', 'Control_L'); editor.held_click(100, row); editor.command('keyup', 'Control_L')

    def expect_parent(x, y, rotation, scale):
        editor.wait(lambda: all(math.isclose(pose('player')[k], v, abs_tol=0.001)
                                for k, v in [('x', x), ('y', y), ('rotation', rotation), ('scale', scale)]),
                    description='Persist complete group pivot transform')

    try:
        editor.start(); editor.held_click(100, 166); editor.save()
        baseline = saved()['scenes']['main']
        additive(166); editor.key('w')
        drag((304, 580), (332, 552))
        assert saved()['scenes']['main'] == baseline, 'Empty hierarchy selection still edited the old gizmo target'
        editor.shot('empty_selection_no_gizmo_target')
        editor.held_click(100, 166); editor.held_click(55, 262); editor.save()
        editor.wait(lambda: len(saved()['scenes']['main']['entities']) == 3, description='Duplicate a child candidate')
        child = saved()['scenes']['main']['entities'][-1]['id']
        editor.held_click(100, 226); editor.value(1268, 202, '80'); editor.value(1340, 202, '100'); editor.save()
        editor.held_click(100, 166); editor.value(1245, 230, '90'); editor.value(1335, 230, '2'); editor.save()
        drag((100, 226), (100, 166))
        editor.wait(lambda: any(c.get('parent') == 'player' for c in entity(child)['components']), description='Parent through hierarchy drag')
        original_child = entity(child)
        before_group = saved()['scenes']['main']
        # Select descendant first, ancestor last: stored selection order must
        # not cause the child to receive the parent's movement twice.
        editor.held_click(100, 196); additive(166)
        editor.held_click(380, 46); editor.key('w')
        drag((304, 580), (332, 552)); expect_parent(64, 96, math.pi/2, 2)
        assert entity(child) == original_child, 'Moving a selected parent and child changed child-local data'
        moved = saved()['scenes']['main']
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved()['scenes']['main'] == before_group, description='One Undo restores whole multiframe group move')
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved()['scenes']['main'] == moved, description='One Redo restores complete parent/child group')
        editor.key('ctrl+z'); editor.save()
        editor.key('e')  # The same selection must survive Undo/Redo without reselection.
        drag((342, 580), (334, 557)); expect_parent(40, 60, 3*math.pi/4, 2)
        assert entity(child) == original_child, 'Group rotation changed the selected child-local transform'
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved()['scenes']['main'] == before_group, description='Undo complete group rotation')
        editor.key('r')
        drag((358, 580), (358, 567)); expect_parent(40, 60, math.pi/2, 2.3)
        assert entity(child) == original_child, 'Group scale changed selected child-local properties'
        scaled = saved()['scenes']['main']
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved()['scenes']['main'] == before_group, description='Undo complete group scale')
        editor.key('r')
        drag((358, 580), (358, 546), cancel=True)
        assert saved()['scenes']['main'] == before_group
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved()['scenes']['main'] == scaled, description='Cancelled group drag preserves previous redo')
        editor.shot('parented_group_saved')
        expected = saved(); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':86')
        editor.start(); editor.save(); assert saved() == expected
        editor.shot('reopened')
        result = dict(passed=True, empty_selection_cannot_edit=True, descendant_first_selection=True,
                      group_move_rotate_scale=True, child_local_properties_preserved=True,
                      each_gesture_one_undo=True, cancel_preserves_redo=True, exact_restart=True,
                      selection_survives_history=True,
                      scope='Linux Xvfb/software graphics; no cross-platform/device claim')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'parented-selection', 'widgets', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
