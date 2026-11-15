#!/usr/bin/env python3
"""Exercise snapped gizmo gestures, history and two-scene recovery through widgets."""
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
    output = Path(os.environ.get('KILN_TRANSFORM_SNAP_EVIDENCE', REPO/'target/a02-transform-snap')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'result.json').unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    sidecar = project/'.kiln.toml.recovery.json'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':86')

    def saved():
        return tomllib.loads(manifest.read_text())

    def pose(scene='main'):
        entity = next(e for e in saved()['scenes'][scene]['entities'] if e['name'].startswith('Player'))
        return next(c for c in entity['components'] if c['type'] == 'Transform')

    def expect(field, value, scene='main'):
        editor.wait(lambda: math.isclose(pose(scene)[field], value, abs_tol=0.001),
                    description=f'Saved {scene} {field} equals {value}')

    def drag(start, points, cancel=False):
        editor.command('mousemove', '--window', editor.window, *start, 'mousedown', '1')
        for point in points:
            editor.command('mousemove', '--window', editor.window, *point)
        if cancel:
            editor.key('Escape')
        editor.command('mouseup', '1')
        editor.save()

    try:
        editor.start()
        editor.held_click(100, 166)
        editor.held_click(380, 46)  # Snap on, leaving the move grid at 32.
        editor.key('e')
        drag((342, 580), [(340, 570), (337, 563), (334, 557)])
        expect('rotation', math.pi/4)
        editor.key('ctrl+z'); editor.save(); expect('rotation', 0.0)
        editor.key('ctrl+y'); editor.save(); expect('rotation', math.pi/4)
        editor.key('r')
        drag((358, 580), [(358, 576), (358, 571), (358, 567)])
        expect('scale', 1.1)
        editor.key('ctrl+z'); editor.save(); expect('scale', 1.0)
        editor.key('ctrl+y'); editor.save(); expect('scale', 1.1)
        drag((358, 580), [(358, 568), (358, 546)], cancel=True)
        expect('scale', 1.1)
        # Cancel must leave the preceding real scale gesture as the next undo.
        editor.key('ctrl+z'); editor.save(); expect('scale', 1.0)
        editor.key('ctrl+y'); editor.save(); expect('scale', 1.1)
        editor.held_click(380, 46)  # Snap off must retain continuous scaling.
        drag((358, 580), [(358, 567)])
        expect('scale', 1.1*math.exp(0.13))
        editor.key('ctrl+z'); editor.save(); expect('scale', 1.1)
        editor.held_click(380, 46)
        editor.key('w')
        drag((304, 580), [(315, 569), (332, 552)])
        expect('x', 64.0); expect('y', 96.0)
        editor.key('ctrl+z'); editor.save(); expect('x', 40.0); expect('y', 60.0)
        editor.key('ctrl+y'); editor.save(); expect('x', 64.0); expect('y', 96.0)
        editor.shot('snapped_move_rotate_scale')
        main = saved()['scenes']['main']
        editor.key('ctrl+c'); editor.held_click(261, 145); editor.key('ctrl+v'); editor.save()
        editor.wait(lambda: len(saved()['scenes']) == 2, description='Create second scene and paste transformed actor')
        other = next(s for s in saved()['scenes'] if s != 'main')
        editor.wait(lambda: len(saved()['scenes'][other]['entities']) == 1, description='Persist pasted actor')
        expect('rotation', math.pi/4, other); expect('scale', 1.1, other)
        assert saved()['scenes']['main'] == main, 'Second scene changed the first scene'
        # A real unsaved inspector edit must survive the actual autosave timer.
        editor.value(1335, 230, '1.7')
        def recovered_scale():
            if not sidecar.exists():
                return False
            data = json.loads(sidecar.read_text())['project']['scenes'][other]['entities'][0]
            return any(c.get('type') == 'Transform' and math.isclose(c['scale'], 1.7, abs_tol=0.001)
                       for c in data['components'])
        editor.wait(recovered_scale, seconds=45, description='Autosave actual unsaved scale property')
        expect('scale', 1.1, other)
        editor.process.kill(); editor.process.wait(timeout=10); editor.start()
        editor.held_click(590, 499); editor.save()
        expect('scale', 1.7, other)
        assert saved()['scenes']['main'] == main
        expected = saved()
        editor.shot('recovered_scale')
        editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':86')
        editor.start(); editor.save()
        assert saved() == expected, 'Restart changed saved properties or references'
        editor.shot('reopened_two_scenes')
        result = dict(passed=True, rotation_15_degree_snap=True, scale_point_one_snap=True,
                      move_grid_snap=True, continuous_scale_opt_out=True,
                      each_drag_one_undo=True, escape_cancel_preserves_history=True,
                      two_scene_copy_recovery_restart=True, project=str(project),
                      scope='Linux Xvfb/software graphics; no cross-platform/device claim')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'transform-snap', 'widgets/recovery', error)
        if editor.process and editor.process.poll() is None:
            editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
