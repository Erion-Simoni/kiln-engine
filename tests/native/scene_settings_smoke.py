#!/usr/bin/env python3
"""Exercise scene metadata, camera history, composition preview and crash recovery."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, record_failure


def main():
    output = Path(os.environ.get('KILN_SCENE_SETTINGS_EVIDENCE', REPO/'target/a02-scene-settings')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    recovery = project/'.kiln.toml.recovery.json'
    tools = Path(os.environ.get('KILN_SCENE_SETTINGS_TOOLS', REPO/'target/debug')).resolve()
    editor = Editor(tools, project, output/'editor', ':87')

    def saved():
        return tomllib.loads(manifest.read_text())

    def checkpoint():
        editor.save()
        return saved()

    def undo_redo(before, after):
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved() == before, description='One Undo restores complete scene settings')
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved() == after, description='One Redo restores complete scene settings')

    def panel():
        editor.panel(80, 690)

    def white_bounds():
        width, height, pixels = editor.frame()
        assert (width, height) == (1440, 900)
        points = [(x, y) for y in range(175, 650) for x in range(400, 1130)
                  if min(pixels[(y*width+x)*3:(y*width+x)*3+3]) > 240]
        if not points: return None
        return (min(x for x, _ in points), min(y for _, y in points),
                max(x for x, _ in points), max(y for _, y in points))

    try:
        editor.start(); initial = checkpoint()
        panel(); editor.text(240, 224, 'Authored main')
        renamed = checkpoint()
        assert renamed['scenes']['main']['name'] == 'Authored main'
        undo_redo(initial, renamed)
        # Scene path is also a document edit, without touching source bytes.
        source = (project/'scripts/main.ks').read_bytes()
        editor.text(240, 252, 'scripts/ui.ks')
        script_changed = checkpoint()
        assert script_changed['scenes']['main']['script'] == 'scripts/ui.ks'
        undo_redo(renamed, script_changed)
        assert (project/'scripts/main.ks').read_bytes() == source
        editor.held_click(98, 280)
        editor.value(90, 364, '2.5')
        camera_changed = checkpoint()
        assert camera_changed['cameras']['main']['zoom'] == 2.5
        undo_redo(script_changed, camera_changed)
        # A live camera drag cancels without consuming the prior redo entry.
        editor.key('ctrl+z'); editor.save()
        editor.command('mousemove', '--window', editor.window, 92, 364, 'mousedown', '1')
        editor.command('mousemove', '--window', editor.window, 122, 364)
        editor.key('Escape'); editor.command('mouseup', '1'); editor.save()
        assert saved() == script_changed
        editor.key('ctrl+y'); editor.save()
        editor.wait(lambda: saved() == camera_changed, description='Cancelled camera edit preserves redo')
        editor.shot('camera_history')
        editor.held_click(125, 132)  # Close settings before creating the child scene.
        editor.held_click(261, 145); editor.held_click(393, 114); editor.save()
        editor.wait(lambda: 'scene_1' in saved()['scenes'] and len(saved()['scenes']['scene_1']['entities']) == 1,
                    description='Create second scene and author its entity')
        child = saved()['scenes']['scene_1']
        editor.held_click(470, 145)
        before_nested = checkpoint()
        panel(); editor.held_click(235, 196); editor.held_click(215, 223)
        nested = checkpoint()
        assert nested['scenes']['main']['nested_scenes'][0]['scene_id'] == 'scene_1'
        assert nested['scenes']['scene_1'] == child
        undo_redo(before_nested, nested)
        # Camera expansion persists in panel view; collapse it to expose links.
        editor.held_click(95, 280); editor.shot('nested_link')
        # Offset controls and Additive must preserve the linked scene itself.
        editor.value(180, 352, '120'); x_offset = checkpoint()
        editor.value(230, 352, '80'); configured = checkpoint()
        undo_redo(x_offset, configured)
        assert configured['scenes']['main']['nested_scenes'][0]['offset'] == {'x': 120.0, 'y': 80.0}
        assert configured['scenes']['scene_1'] == child
        editor.held_click(125, 132); editor.shot('composition_preview')
        bounds = editor.wait(white_bounds, description='Render nested entity after offset edit')
        assert all(abs(a-b) <= 3 for a, b in zip(bounds, (834, 282, 878, 325))), bounds
        panel(); editor.held_click(70, 383)  # Remove wraps onto the next row in a narrow panel.
        removed = checkpoint()
        assert removed['scenes']['main']['nested_scenes'] == []
        assert removed['scenes']['scene_1'] == child
        editor.held_click(125, 132)
        editor.wait(lambda: white_bounds() is None, description='Remove immediately clears nested preview')
        editor.key('ctrl+z'); editor.save()
        editor.wait(lambda: saved() == configured and white_bounds() == bounds,
                    description='Undo restores nested document and exact preview')
        # Camera-only unsaved data must use the real recovery timer.
        panel(); editor.held_click(95, 280); editor.value(90, 364, '3.5')
        def autosaved():
            return recovery.exists() and json.loads(recovery.read_text())['project'].get('cameras', {}).get('main', {}).get('zoom') == 3.5
        editor.wait(autosaved, seconds=45, description='Autosave camera-only unsaved settings')
        assert saved() == configured
        editor.process.kill(); editor.process.wait(timeout=10); editor.start()
        editor.held_click(590, 499); editor.save()
        restored = saved()
        expected = configured
        expected['cameras']['main']['zoom'] = 3.5
        assert restored == expected
        editor.shot('recovered_camera_composition'); editor.close()
        editor = Editor(tools, project, output/'reopened', ':87')
        editor.start(); editor.save(); assert saved() == restored
        editor.shot('reopened')
        result = dict(passed=True, scene_name_script_undo=True, camera_presence_undo_redo=True,
                      camera_cancel_preserves_redo=True, explicit_scene_composition=True,
                      nested_offsets_preserve_child=True, nested_remove_undo_preview=True, camera_only_crash_recovery=True,
                      exact_complete_project_restart=True, scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'scene-settings', 'widgets/recovery', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
