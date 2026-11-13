#!/usr/bin/env python3
"""Native editor export transaction regression on an isolated Xvfb desktop.

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
import hashlib

sys.dont_write_bytecode = True
from ui_smoke import REPO, TOOLS, executable, wait_for  # noqa: E402


def main():
    output = Path(os.environ.get('KILN_EXPORT_EVIDENCE_DIR') or
                  tempfile.mkdtemp(prefix='kiln-editor-native-')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'result.json').unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='profile-', dir=output))
    project = run/'project'
    shutil.copytree(Path(os.environ.get('KILN_EXPORT_PROJECT', REPO/'samples/ui2d_editor')), project)
    manifest = project/'kiln.toml'
    display = os.environ.get('KILN_EXPORT_DISPLAY', ':87')
    socket = Path('/tmp/.X11-unix')/f'X{display.removeprefix(":")}'
    if socket.exists():
        raise RuntimeError(f'{display} is occupied; choose KILN_EXPORT_DISPLAY')
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
    no_rust = run/'no-rust-tools'
    no_rust.mkdir()
    env['PATH'] = str(no_rust)
    config = run/'config/kiln'
    config.mkdir(parents=True)
    (config/'project_hub.json').write_text(json.dumps([
        {'path': str(manifest), 'name': 'Editor Design Showcase', 'last_opened': 0}]))
    server_log, editor_log = output/'xvfb.log', output/'editor.log'
    editor_log.write_text('')
    server = subprocess.Popen([executable('Xvfb'), display, '-screen', '0',
                               '1440x900x24', '-nolisten', 'tcp', '-ac'], env=env,
                              stdout=server_log.open('w'), stderr=subprocess.STDOUT)
    editor = None
    try:
        # A socket can exist before Xvfb is accepting X11 connections.
        wait_for(lambda: subprocess.run([executable('xdotool'), 'getdisplaygeometry'],
                                       env=env, stdout=subprocess.DEVNULL,
                                       stderr=subprocess.DEVNULL).returncode == 0,
                 server, server_log)
        binary = os.environ.get('KILN_EDITOR_PATH', str(REPO/'target/debug/kiln-editor'))
        xdo = executable('xdotool')

        def launch():
            process = subprocess.Popen([binary], cwd=os.environ.get('KILN_EXPORT_CWD', REPO), env=env,
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
            actor = next(e for e in data['scenes']['main']['entities'] if e['id'] == 'player')
            return next(c['x'] for c in actor['components'] if c['type'] == 'Transform')

        click(90, 161)
        command('mousemove', '--window', window, 1262, 195, 'click', '--repeat', 2, '--delay', 80, 1)
        key('ctrl+a')
        command('type', '--clearmodifiers', '88')
        key('Return')
        assert authored_x() == 40, 'The inspector edit should remain unsaved until export'
        click(80, 12)
        click(105, 242)
        command('mousemove', '--window', window, 365, 510)
        command('mousedown', 1)
        for point in [(450,550),(650,650),(850,750),(1100,850)]:
            command('mousemove', '--window', window, *point)
        command('mouseup', 1)
        destination = run/'native game'
        # This edits the actual output field; the old panel discarded these edits.
        click(250, 273)
        key('ctrl+a')
        command('type', '--clearmodifiers', str(destination))
        key('Return')
        shot('ready_unsaved')
        click(75, 346)
        # Installed exports also hash the complete runtime/notice inventory.
        wait_for(lambda: (destination/'export.json').exists(), editor, editor_log, attempts=300)
        wait_for(lambda: 'Export result: Success' in editor_log.read_text(), editor, editor_log, attempts=300)
        shot('export_verified')
        assert authored_x() == 88, 'Save & Export missed the actual scene edit'
        receipt = json.loads((destination/'export.json').read_text())
        assert receipt['project_id'] == 'saved-ui-2d'
        assert receipt['runtime']['audio'] and receipt['runtime']['fonts'] and receipt['runtime']['gamepad']
        for file in receipt['files']:
            assert (destination/file['path']).stat().st_size == file['bytes']
        before = hashlib.sha256((destination/'assets.kar').read_bytes()).hexdigest()
        click(75, 346)  # Existing output must fail, never overwrite a previous good export.
        wait_for(lambda: 'Export result: Failed' in editor_log.read_text(), editor, editor_log)
        shot('existing_output_failed')
        assert hashlib.sha256((destination/'assets.kar').read_bytes()).hexdigest() == before
        # A slow runtime handshake keeps a real export active long enough to cancel.
        cancel_destination = run/'cancelled game'
        slow_runtime = run/'slow-runtime'
        slow_runtime.write_text('#!/bin/sh\nexec /bin/sleep 20\n')
        slow_runtime.chmod(0o755)
        click(250, 273)
        key('ctrl+a')
        command('type', '--clearmodifiers', str(cancel_destination))
        key('Return')
        click(80, 301)
        click(260, 352)
        key('ctrl+a')
        command('type', '--clearmodifiers', str(slow_runtime))
        key('Return')
        click(75, 416)
        shot('active_export')
        click(240, 416)
        wait_for(lambda: 'Export result: Cancelled' in editor_log.read_text(), editor, editor_log)
        shot('cancelled')
        assert not cancel_destination.exists()
        assert not list(run.glob('.kiln-export-*')), 'Cancelled job left staging files'
        # Move the complete artifact outside the repository and the source project.
        relocated = Path(tempfile.mkdtemp(prefix='kiln-relocated-'))/'game folder'
        shutil.move(str(destination), relocated)
        project.rename(run/'source-not-available')
        result = {'passed': True, 'authored_x': 88, 'saved_by_export': True,
                  'failure_preserves_previous_output': True, 'runtime_files': receipt['files'],
                  'relocated_export': str(relocated), 'source_project_unavailable': True, 'rust_required_at_export': False, 'cancelled_from_ui': True}
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
