#!/usr/bin/env python3
"""Task 7 native UI authoring and editor save/resume regression on an isolated Xvfb desktop.

Requires a built kiln-editor, Xvfb, xdotool and ImageMagick. All edits occur in a
throwaway copy of samples/ui2d_editor. Retains screenshots, logs and results.
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
    output = Path(os.environ.get('KILN_TASK7_EDITOR_EVIDENCE_DIR') or
                  tempfile.mkdtemp(prefix='kiln-editor-native-')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'result.json').unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='profile-', dir=output))
    project = run/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    manifest = project/'kiln.toml'
    display = os.environ.get('KILN_TASK7_EDITOR_DISPLAY', ':90')
    socket = Path('/tmp/.X11-unix')/f'X{display.removeprefix(":")}'
    if socket.exists():
        raise RuntimeError(f'{display} is occupied; choose KILN_TASK7_EDITOR_DISPLAY')
    env = os.environ.copy()
    env.update(DISPLAY=display, XDG_DATA_HOME=str(run/'data'),
               XDG_CONFIG_HOME=str(run/'config'), XDG_CACHE_HOME=str(run/'cache'),
               XDG_RUNTIME_DIR=str(run/'runtime'), LIBGL_ALWAYS_SOFTWARE='1',
               ALSA_CONFIG_PATH=str(run/'alsa.conf'), RUST_LOG='warn,kiln_editor=info,kiln_runtime::ui=debug')
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

        def manifest_data():
            return tomllib.loads(manifest.read_text())

        def designer():
            click(80, 12)
            click(105, 631)
            # Resize the floating dock using real drag events.
            command('mousemove', '--window', window, 365, 510)
            command('mousedown', 1)
            for point in [(450,550),(650,650),(850,750),(1100,850)]:
                command('mousemove', '--window', window, *point)
            command('mouseup', 1)

        original = manifest_data()['ui']
        designer()
        shot('ui_designer')
        click(145, 375)  # Add Label.
        shot('label_added')
        command('mousemove', '--window', window, 130, 531)
        command('mousedown', 1)
        for point in [(150,551),(180,571),(210,591)]:
            command('mousemove', '--window', window, *point)
        command('mouseup', 1)
        shot('label_dragged')
        # Exercise document undo/redo through the specialist panel.
        click(60, 207)
        shot('label_undo')
        click(124, 207)
        shot('label_redo')
        click(1084, 98)
        key('ctrl+s')
        def new_widget():
            old_ids = {w['id'] for w in original['screens']['game_over']['widgets']}
            return next((w for w in manifest_data()['ui']['screens']['game_over']['widgets'] if w['id'] not in old_ids), None)
        widget = wait_for(new_widget, editor, editor_log)
        assert widget['kind'] == 'Label'
        assert widget['layout']['offset'] == [104.0, 80.0], widget
        saved_ui = manifest_data()['ui']
        editor.terminate()
        editor.wait(timeout=10)
        editor, window = launch()
        assert manifest_data()['ui'] == saved_ui
        shot('reopened')
        click(677, 44)  # Real editor Play uses the saved screens and script attachments.
        shot('play_title')
        # Keyboard routes through editor input into the shared authored UI runtime.
        key('Return')
        shot('play_hud')
        key('Return')  # Focused Damage button: health 73 -> 66.
        key('Escape')
        key('F5')
        slot_path = project/'.kiln-save/slot_0.json'
        wait_for(slot_path.exists, editor, editor_log)
        def saved():
            return json.loads(json.loads(slot_path.read_text())['data'])
        first = saved()
        assert first['game_data']['health']['Integer'] == 66, first['game_data']
        assert first['attached']['ui']['memory']['values']['starts']['Integer'] == 1
        assert first['ui']['active'] == 'pause'
        shot('play_saved_pause')
        click(677, 44)  # Stop, then a fresh Play session.
        click(677, 44)
        shot('play_fresh_title')
        key('F9')
        shot('play_loaded_hud')
        key('Escape')
        key('F5')
        resumed = saved()
        assert resumed['game_data']['health']['Integer'] == 66
        assert resumed['attached']['ui']['memory']['values']['starts']['Integer'] == 1
        assert resumed['attached']['ui']['memory']['values']['ticks']['Integer'] >= first['attached']['ui']['memory']['values']['ticks']['Integer']
        click(677, 44)
        assert manifest_data()['ui'] == saved_ui, 'Play changed the authored UI document'
        log = editor_log.read_text()
        assert not any(e in log for e in ['panicked', 'Validation Error', 'scene script failed']), log[-6000:]
        result = {'passed': True, 'ui_widget': widget['id'], 'drag_offset': widget['layout']['offset'], 'saved_reopened': True, 'editor_save_resume': True, 'health': 66, 'script_starts': 1}
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    finally:
        if editor is not None and editor.poll() is None:
            editor.terminate()
            editor.wait(timeout=10)
        server.terminate()
        server.wait(timeout=10)


if __name__ == '__main__':
    main()
