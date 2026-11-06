#!/usr/bin/env python3
"""Native saved-UI smoke on Xvfb; requires a default-feature player build.

Uses Python's standard library, Xvfb, xdotool and ImageMagick import. Linux only.
Writes screenshots/logs to KILN_UI_EVIDENCE_DIR or a fresh temporary directory.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time

REPO = Path(__file__).resolve().parents[2]
TOOLS = REPO.parent / '.kiln-tools/sysroot'


def executable(name):
    path = shutil.which(name) or str(TOOLS / 'usr/bin' / name)
    if not Path(path).is_file():
        raise RuntimeError(f'Install {name}, or add its directory to PATH')
    return path


def wait_for(check, process, log, attempts=100):
    for _ in range(attempts):
        value = check()
        if value:
            return value
        if process.poll() is not None:
            raise RuntimeError(f'Process exited: {log.read_text()[-6000:]}')
        time.sleep(0.1)
    raise RuntimeError(f'Timed out: {log.read_text()[-6000:]}')


def main():
    output = Path(os.environ.get('KILN_UI_EVIDENCE_DIR') or tempfile.mkdtemp(prefix='kiln-ui-native-'))
    output.mkdir(parents=True, exist_ok=True)
    output = output.resolve()
    (output/'result.json').unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='profile-', dir=output))
    display = os.environ.get('KILN_UI_DISPLAY', ':97')
    socket = Path('/tmp/.X11-unix') / f'X{display.removeprefix(":")}'
    if socket.exists():
        raise RuntimeError(f'{display} is occupied; choose KILN_UI_DISPLAY')
    env = os.environ.copy()
    env.update(DISPLAY=display, XDG_DATA_HOME=str(run/'data'), XDG_CONFIG_HOME=str(run/'config'),
               XDG_CACHE_HOME=str(run/'cache'), XDG_RUNTIME_DIR=str(run/'runtime'),
               LIBGL_ALWAYS_SOFTWARE='1', WGPU_BACKEND='gl', ALSA_CONFIG_PATH=str(run/'alsa.conf'),
               RUST_LOG='warn,kiln_player=info,kiln_runtime::ui=debug')
    if TOOLS.is_dir():
        env['LD_LIBRARY_PATH'] = str(TOOLS/'usr/lib/x86_64-linux-gnu') + ':' + env.get('LD_LIBRARY_PATH', '')
    swiftshader = Path('/usr/lib/chromium/vk_swiftshader_icd.json')
    if swiftshader.exists():
        env.setdefault('VK_ICD_FILENAMES', str(swiftshader))
    (run/'runtime').mkdir(mode=0o700)
    (run/'alsa.conf').write_text('pcm.!default { type null }\n')
    server_log, player_log = output/'xvfb.log', output/'player.log'
    server = subprocess.Popen([executable('Xvfb'), display, '-screen', '0', '1280x720x24',
                               '-nolisten', 'tcp', '-ac'], env=env,
                              stdout=server_log.open('w'), stderr=subprocess.STDOUT)
    player = None
    try:
        wait_for(socket.exists, server, server_log)
        binary = os.environ.get('KILN_PLAYER_PATH', str(REPO/'target/debug/kiln-player'))
        player = subprocess.Popen([binary, '--archive', str(REPO/'samples/ui2d_editor'),
                                   '--game-name', 'ui2d-native-check'], env=env,
                                  stdout=player_log.open('w'), stderr=subprocess.STDOUT)
        xdo = executable('xdotool')

        def search():
            result = subprocess.run([xdo, 'search', '--name', 'Kiln Game'], env=env,
                                    capture_output=True, text=True)
            return result.stdout.strip().splitlines()[0] if result.returncode == 0 else None

        window = wait_for(search, player, player_log)
        subprocess.run([xdo, 'windowfocus', '--sync', window], env=env, check=True)

        def shot(name):
            time.sleep(0.6)
            if player.poll() is not None:
                raise RuntimeError(player_log.read_text()[-6000:])
            subprocess.run([executable('import'), '-display', display, '-window', window,
                            str(output/f'{name}.png')], env=env, check=True)

        def key(value):
            subprocess.run([xdo, 'key', '--clearmodifiers', value], env=env, check=True)
            time.sleep(0.15)

        def click(x, y):
            subprocess.run([xdo, 'mousemove', '--window', window, str(x), str(y), 'click', '1'],
                           env=env, check=True)
            time.sleep(0.15)

        shot('title')
        click(320, 228)
        shot('settings')
        click(320, 180)
        shot('settings_changed')
        key('Escape')
        key('Up')
        key('Return')
        shot('gameplay')
        key('Escape')
        shot('pause')
        key('Down')
        key('Down')
        key('Return')
        shot('game_over')
        log = player_log.read_text()
        for event in ['screen=title widget=settings event=settings', 'screen=settings widget=sound event=sound',
                      'screen=title widget=start event=start', 'screen=pause widget=gameover event=gameover']:
            if event not in log:
                raise AssertionError(f'{event} missing:\n{log[-6000:]}')
        if log.count('Loaded gameplay for scene') != 1:
            raise AssertionError('Escape restarted the game session')
        if any(error in log for error in ['Validation Error', 'panicked', 'scene script failed']):
            raise AssertionError(log[-6000:])
        preferences = json.loads((run/'data/ui2d-native-check/preferences.json').read_text())
        if preferences['audio']['master_volume'] != 0.25 or preferences['values']['menu_setting'] != 'saved':
            raise AssertionError('Settings callback did not persist the expected values')
        result = {'passed': True, 'evidence': str(output), 'screenshots': [str(p) for p in sorted(output.glob('*.png'))]}
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    finally:
        if player is not None:
            player.terminate()
            player.wait(timeout=10)
        server.terminate()
        server.wait(timeout=10)


if __name__ == '__main__':
    main()
