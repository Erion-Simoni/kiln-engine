#!/usr/bin/env python3
"""Use visible World/Local move axes, snapping, undo/cancel and disk restart."""
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
    output = Path(os.environ.get('KILN_LOCAL_AXES_EVIDENCE', REPO/'target/a02-local-axes')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json', output/'editor/failure.png'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':86')
    # The 640x360 fixture is letterboxed into an 883x497 viewport at this
    # fixed 1440x900 editor size. Pointer pixels are not world units.
    move_x, move_y = 28*640/883, 28*360/497

    def saved():
        return tomllib.loads(manifest.read_text())

    def pose():
        actor = next(e for e in saved()['scenes']['main']['entities'] if e['id'] == 'player')
        return next(c for c in actor['components'] if c['type'] == 'Transform')

    def expect(x, y):
        editor.wait(lambda: math.isclose(pose()['x'], x, abs_tol=0.01)
                    and math.isclose(pose()['y'], y, abs_tol=0.01),
                    description=f'Saved constrained position ({x}, {y})')
        assert math.isclose(pose()['rotation'], math.pi/2, abs_tol=0.001)
        assert pose()['scale'] == 1.0

    def red_axis(x, y):
        width, height, pixels = editor.frame()
        assert (width, height) == (1440, 900)
        for row in range(y-1, y+2):
            for col in range(x-1, x+2):
                r, g, b = pixels[(row*width+col)*3:(row*width+col)*3+3]
                if r > 200 and g < 130 and b < 130:
                    return True
        return False

    def drag(start, finish, cancel=False):
        editor.command('mousemove', '--window', editor.window, *start, 'mousedown', '1')
        for t in (0.3, 0.6, 1.0):
            point = [round(a+(b-a)*t) for a, b in zip(start, finish)]
            editor.command('mousemove', '--window', editor.window, *point)
        if cancel:
            editor.key('Escape')
        editor.command('mouseup', '1'); editor.save()

    def undo():
        editor.key('ctrl+z'); editor.save(); expect(40.0, 60.0)

    try:
        editor.start(); editor.held_click(100, 166)
        editor.value(1245, 230, '90'); editor.save(); expect(40.0, 60.0)
        editor.key('w'); editor.held_click(333, 46)
        editor.wait(lambda: red_axis(304, 550), description='Local X handle visibly points upward')
        editor.shot('local_axes')
        drag((304, 550), (332, 522)); expect(40.0, 60.0+move_y)
        undo(); editor.key('ctrl+y'); editor.save(); expect(40.0, 60.0+move_y); undo()
        # Local Y points left; diagonal pointer motion must not move world Y.
        drag((274, 580), (302, 552)); expect(40.0+move_x, 60.0); undo()
        editor.held_click(380, 46)
        drag((304, 550), (332, 522)); expect(40.0, 92.0); undo()
        editor.held_click(333, 46)
        editor.wait(lambda: red_axis(334, 580), description='World X handle visibly points right')
        editor.shot('world_axes')
        drag((334, 580), (362, 552)); expect(64.0, 60.0); undo()
        editor.held_click(380, 46)
        drag((304, 580), (332, 552)); expect(40.0+move_x, 60.0+move_y); undo()
        editor.held_click(333, 46)
        editor.wait(lambda: red_axis(304, 550), description='Return to local move axes')
        drag((304, 550), (332, 522), cancel=True); expect(40.0, 60.0)
        editor.key('ctrl+y'); editor.save(); expect(40.0+move_x, 60.0+move_y)
        expected = saved(); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':86')
        editor.start(); editor.save()
        assert saved() == expected, 'Restart changed project properties/references'
        editor.shot('reopened')
        result = dict(passed=True, visible_local_world_axes=True, both_axis_constraints=True,
                      local_displacement_snap=True, world_absolute_snap=True,
                      free_center_drag=True, one_gesture_undo=True,
                      cancel_preserves_redo=True, exact_project_restart=True,
                      scope='Linux Xvfb/software graphics; no cross-platform/device claim')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'local-axes', 'widgets', error)
        if editor.process and editor.process.poll() is None:
            editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
