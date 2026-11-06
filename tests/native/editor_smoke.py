#!/usr/bin/env python3
"""Native editor design/authoring regression on an isolated Xvfb desktop.

Requires a built kiln-editor, Xvfb, xdotool and ImageMagick. All edits occur in a
throwaway copy of samples/editor_design. Retains screenshots, logs and results.
"""
from pathlib import Path
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib

sys.dont_write_bytecode = True
from ui_smoke import REPO, TOOLS, executable, wait_for  # noqa: E402


def main():
    output = Path(os.environ.get('KILN_EDITOR_EVIDENCE_DIR') or
                  tempfile.mkdtemp(prefix='kiln-editor-native-')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'result.json').unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='profile-', dir=output))
    project = run/'project'
    shutil.copytree(REPO/'samples/editor_design', project)
    manifest = project/'kiln.toml'
    display = os.environ.get('KILN_EDITOR_DISPLAY', ':96')
    socket = Path('/tmp/.X11-unix')/f'X{display.removeprefix(":")}'
    if socket.exists():
        raise RuntimeError(f'{display} is occupied; choose KILN_EDITOR_DISPLAY')
    env = os.environ.copy()
    env.update(DISPLAY=display, XDG_DATA_HOME=str(run/'data'),
               XDG_CONFIG_HOME=str(run/'config'), XDG_CACHE_HOME=str(run/'cache'),
               XDG_RUNTIME_DIR=str(run/'runtime'), LIBGL_ALWAYS_SOFTWARE='1',
               ALSA_CONFIG_PATH=str(run/'alsa.conf'), RUST_LOG='warn,kiln_editor=info')
    if TOOLS.is_dir():
        env['LD_LIBRARY_PATH'] = str(TOOLS/'usr/lib/x86_64-linux-gnu')+':'+env.get('LD_LIBRARY_PATH', '')
    swiftshader = Path('/usr/lib/chromium/vk_swiftshader_icd.json')
    if swiftshader.exists():
        env.setdefault('VK_ICD_FILENAMES', str(swiftshader))
    (run/'runtime').mkdir(mode=0o700)
    (run/'alsa.conf').write_text('pcm.!default { type null }\n')
    config = run/'config/kiln'
    config.mkdir(parents=True)
    (config/'project_hub.json').write_text(json.dumps([
        {'path': str(manifest), 'name': 'Editor Design Showcase', 'last_opened': 0}]))
    server_log, editor_log = output/'xvfb.log', output/'editor.log'
    server = subprocess.Popen([executable('Xvfb'), display, '-screen', '0',
                               '1440x900x24', '-nolisten', 'tcp', '-ac'], env=env,
                              stdout=server_log.open('w'), stderr=subprocess.STDOUT)
    editor = None
    try:
        wait_for(socket.exists, server, server_log)
        binary = os.environ.get('KILN_EDITOR_PATH', str(REPO/'target/debug/kiln-editor'))
        xdo = executable('xdotool')

        def launch():
            process = subprocess.Popen([binary], cwd=REPO, env=env,
                                       stdout=editor_log.open('a'), stderr=subprocess.STDOUT)

            def search():
                result = subprocess.run([xdo, 'search', '--name', 'Kiln Editor -'], env=env,
                                        capture_output=True, text=True)
                return result.stdout.strip().splitlines()[0] if result.returncode == 0 else None

            window = wait_for(search, process, editor_log)
            subprocess.run([xdo, 'windowfocus', '--sync', window], env=env, check=True)
            time.sleep(1.5)
            return process, window

        editor, window = launch()

        def command(*args):
            subprocess.run([xdo, *map(str, args)], env=env, check=True, timeout=10)
            time.sleep(0.15)

        def click(x, y):
            command('mousemove', '--window', window, x, y)
            command('mousedown', '1')
            command('mouseup', '1')

        def key(value):
            command('key', '--clearmodifiers', value)

        def shot(name):
            time.sleep(0.3)
            if editor.poll() is not None:
                raise RuntimeError(editor_log.read_text()[-6000:])
            path = output/f'{name}.png'
            subprocess.run([executable('import'), '-display', display, '-window', window,
                            str(path)], env=env, check=True, timeout=10)
            return path

        def authored_x():
            data = tomllib.loads(manifest.read_text())
            player = next(e for e in data['scenes']['main']['entities'] if e['id'] == 'player')
            return next(c['x'] for c in player['components'] if c['type'] == 'Transform')

        def save_x(expected):
            key('ctrl+s')
            wait_for(lambda: authored_x() == expected, editor, editor_log)

        def blue_x(path):
            # Read the real framebuffer's scene pixels, excluding editor chrome.
            result = subprocess.run([executable('convert'), str(path), '-crop', '890x510+248+156',
                                     '-depth', '8', 'RGB:-'], capture_output=True, check=True)
            pixels = result.stdout
            points = [i//3 % 890 for i in range(0, len(pixels), 3)
                      if pixels[i+2] > pixels[i]+40 and pixels[i+2] > pixels[i+1]+20]
            if len(points) < 50:
                raise RuntimeError('The actual blue player is missing from the framebuffer')
            return sum(points)/len(points)

        def scene_mean(path):
            result = subprocess.run([executable('convert'), str(path), '-crop', '890x510+248+156',
                                     '-format', '%[fx:mean]', 'info:'], capture_output=True,
                                    text=True, check=True)
            return float(result.stdout)

        click(90, 161)  # Select Player in the actual hierarchy.
        initial = shot('2d_editor')
        initial_x = blue_x(initial)
        # Edit Transform X using its real DragValue text input.
        command('mousemove', '--window', window, 1262, 195, 'click', '--repeat', '2', '--delay', '80', '1')
        key('ctrl+a')
        command('type', '--clearmodifiers', '215')
        key('Return')
        save_x(215.0)
        edited_x = blue_x(shot('2d_edited'))
        assert edited_x > initial_x+30, (initial_x, edited_x)
        key('ctrl+z')
        save_x(165.0)
        key('ctrl+y')
        save_x(215.0)
        before_preview = manifest.read_bytes()

        click(340, 78)  # Game tab: 3D must switch back to the Scene workspace.
        click(1316, 44)
        preview = shot('3d_preview')
        command('mousemove', '--window', window, 700, 420, 'mousedown', '1')
        command('mousemove', '--window', window, 810, 470)
        command('mouseup', '1')
        orbit = shot('3d_orbit')
        assert abs(scene_mean(preview)-scene_mean(orbit)) > 0.0005
        # Temporary lighting properties must visibly affect the demonstration.
        command('mousemove', '--window', window, 1204, 363, 'mousedown', '1')
        command('mousemove', '--window', window, 1153, 363)
        command('mouseup', '1')
        dark = shot('3d_lighting')
        assert scene_mean(dark) < scene_mean(orbit)-0.001
        key('ctrl+s')
        assert manifest.read_bytes() == before_preview, '3D preview modified the 2D project'
        click(1278, 44)
        returned_x = blue_x(shot('2d_returned'))
        assert abs(returned_x-edited_x) < 1.0

        # Reopen the saved project in a fresh editor process.
        editor.terminate()
        editor.wait(timeout=10)
        editor, window = launch()
        assert authored_x() == 215.0
        click(90, 161)
        reopened_x = blue_x(shot('2d_reopened'))
        assert abs(reopened_x-edited_x) < 1.0
        click(677, 44)  # Play active 2D scene.
        start_x = blue_x(shot('play_started'))
        command('keydown', 'Right')
        time.sleep(0.3)
        command('keyup', 'Right')
        moved_x = blue_x(shot('play_moved'))
        assert moved_x > start_x+20, (start_x, moved_x)
        click(715, 44)  # Pause.
        paused_x = blue_x(shot('play_paused'))
        command('keydown', 'Right')
        time.sleep(0.3)
        held_x = blue_x(shot('play_paused_held'))
        assert abs(held_x-paused_x) < 1.0
        click(755, 44)  # Exactly one step while paused, with input held.
        step_x = blue_x(shot('play_stepped'))
        command('keyup', 'Right')
        assert 0.5 < step_x-held_x < 5.0, (held_x, step_x)
        click(677, 44)  # Stop restores the authored world.
        stopped_x = blue_x(shot('play_stopped'))
        assert abs(stopped_x-reopened_x) < 1.0
        assert manifest.read_bytes() == before_preview
        log = editor_log.read_text()
        for marker in ['panicked', 'Validation Error', 'Could not start Play', '[ERROR]']:
            assert marker not in log, log[-6000:]
        assert '3D visual preview' in log and '2D scene' in log
        result = {'passed': True, 'saved_x': authored_x(),
                  'player_pixels': {'initial': initial_x, 'edited': edited_x,
                                    'reopened': reopened_x, 'playing': moved_x,
                                    'paused': paused_x, 'stepped': step_x, 'stopped': stopped_x},
                  'preview_does_not_modify_project': True}
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result, indent=2))
    finally:
        if editor is not None and editor.poll() is None:
            editor.terminate()
            editor.wait(timeout=10)
        server.terminate()
        server.wait(timeout=10)


if __name__ == '__main__':
    main()
