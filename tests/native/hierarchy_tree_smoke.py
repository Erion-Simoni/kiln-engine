#!/usr/bin/env python3
"""Author a real parent tree, search/fold/edit, detach, undo and recover two scenes."""
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
    output = Path(os.environ.get('KILN_HIERARCHY_TREE_EVIDENCE', REPO/'target/a02-hierarchy-tree')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json', output/'editor/failure.png'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    sidecar = project/'.kiln.toml.recovery.json'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':86')

    def saved():
        return tomllib.loads(manifest.read_text())

    def entity(id, scene='main'):
        return next(e for e in saved()['scenes'][scene]['entities'] if e['id'] == id)

    def component(id, kind, scene='main'):
        return next(c for c in entity(id, scene)['components'] if c['type'] == kind)

    def expect_pose(id, x, y, rotation, scale, scene='main'):
        def matches():
            pose = component(id, 'Transform', scene)
            return all(math.isclose(pose[k], v, abs_tol=0.001)
                       for k, v in [('x', x), ('y', y), ('rotation', rotation), ('scale', scale)])
        editor.wait(matches, description=f'Persist complete {scene}/{id} transform')

    def drop(start, target):
        editor.command('mousemove', '--window', editor.window, *start, 'mousedown', '1')
        for t in (0.3, 0.6, 1.0):
            editor.command('mousemove', '--window', editor.window,
                           *[round(a+(b-a)*t) for a, b in zip(start, target)])
        editor.command('mouseup', '1'); editor.save()

    try:
        editor.start(); editor.held_click(100, 166); editor.held_click(55, 262); editor.save()
        editor.wait(lambda: len(saved()['scenes']['main']['entities']) == 3, description='Duplicate through hierarchy button')
        child = saved()['scenes']['main']['entities'][-1]['id']
        editor.held_click(100, 226)
        editor.value(1268, 202, '80'); editor.value(1340, 202, '100'); editor.save()
        expect_pose(child, 80, 100, 0, 1)
        editor.held_click(100, 166)
        editor.value(1245, 230, '90'); editor.value(1335, 230, '2'); editor.save()
        expect_pose('player', 40, 60, math.pi/2, 2)
        drop((100, 226), (100, 166))
        editor.wait(lambda: any(c.get('parent') == 'player' for c in entity(child)['components']),
                    description='Drop child under rotated and scaled parent')
        expect_pose(child, 20, -20, -math.pi/2, 0.5)
        before_fold = manifest.read_bytes()
        editor.held_click(16, 166); editor.shot('collapsed_parent')
        assert manifest.read_bytes() == before_fold, 'Folding modified the project'
        editor.text(100, 108, 'Player Copy'); editor.shot('search_with_ancestor_context')
        editor.held_click(100, 196); editor.value(1268, 202, '22'); editor.save()
        expect_pose(child, 22, -20, -math.pi/2, 0.5)
        editor.held_click(100, 108); editor.key('ctrl+a'); editor.key('BackSpace')
        editor.held_click(16, 166)
        editor.shot('expanded_after_property_rebuild')
        drop((100, 196), (60, 132))
        editor.wait(lambda: not any(c['type'] == 'Parent' for c in entity(child)['components']),
                    description='Drop onto scene header detaches to root')
        expect_pose(child, 80, 104, 0, 1)
        detached = entity(child)
        editor.key('ctrl+z'); editor.save(); expect_pose(child, 22, -20, -math.pi/2, 0.5)
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: entity(child) == detached, description='Redo exact detached entity')
        editor.held_click(100, 226); editor.held_click(130, 264); editor.save()
        editor.wait(lambda: len(saved()['scenes']['main']['entities']) == 2, description='Delete detached child')
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: entity(child) == detached, description='Undo restores complete entity and stable ID')
        drop((100, 226), (100, 166)); expect_pose(child, 22, -20, -math.pi/2, 0.5)
        main_scene = saved()['scenes']['main']
        editor.held_click(100, 166); editor.key('ctrl+c')
        editor.held_click(261, 145); editor.key('ctrl+v'); editor.save()
        editor.wait(lambda: len(saved()['scenes']) == 2, description='Create second scene')
        other = next(s for s in saved()['scenes'] if s != 'main')
        editor.wait(lambda: len(saved()['scenes'][other]['entities']) == 2, description='Paste actual parent subtree')
        copied = saved()['scenes'][other]['entities']
        copied_child = next(e for e in copied if any(c['type'] == 'Parent' for c in e['components']))
        copied_parent = next(e for e in copied if e['id'] != copied_child['id'])
        assert component(copied_child['id'], 'Parent', other)['parent'] == copied_parent['id']
        assert component(copied_child['id'], 'Transform', other) == component(child, 'Transform')
        editor.held_click(100, 196); editor.value(1340, 202, '-15')
        def recovered():
            if not sidecar.exists(): return False
            entities = json.loads(sidecar.read_text())['project']['scenes'][other]['entities']
            child_data = next(e for e in entities if e['id'] == copied_child['id'])
            return any(c['type'] == 'Transform' and math.isclose(c['y'], -15, abs_tol=0.001)
                       for c in child_data['components'])
        editor.wait(recovered, seconds=45, description='Real autosave of unsaved parented child property')
        expect_pose(copied_child['id'], 22, -20, -math.pi/2, 0.5, other)
        editor.process.kill(); editor.process.wait(timeout=10); editor.start()
        editor.held_click(590, 499); editor.save()
        expect_pose(copied_child['id'], 22, -15, -math.pi/2, 0.5, other)
        assert saved()['scenes']['main'] == main_scene
        assert component(copied_child['id'], 'Parent', other)['parent'] == copied_parent['id']
        expected = saved(); editor.shot('recovered_parent_tree'); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':86')
        editor.start(); editor.save(); assert saved() == expected
        editor.shot('reopened')
        result = dict(passed=True, duplicate_reparent_properties=True, folded_search_context=True,
                      detach_preserves_world_pose=True, delete_undo_identity=True,
                      two_scene_subtree_recovery=True, exact_restart=True,
                      scope='Linux Xvfb/software graphics; no cross-platform/device claim')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'hierarchy-tree', 'widgets/recovery', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
