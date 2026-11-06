#!/usr/bin/env python3
"""Native Task 7 save/resume acceptance across fresh player processes.

Uses Python's standard library, Xvfb, xdotool and ImageMagick import. Linux only.
Writes screenshots/logs to KILN_TASK7_EVIDENCE_DIR or a fresh temporary directory.
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
    output = Path(os.environ.get('KILN_TASK7_EVIDENCE_DIR') or tempfile.mkdtemp(prefix='kiln-task7-native-'))
    output.mkdir(parents=True, exist_ok=True)
    output = output.resolve()
    (output/'result.json').unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='profile-', dir=output))
    display = os.environ.get('KILN_TASK7_DISPLAY', ':92')
    socket = Path('/tmp/.X11-unix') / f'X{display.removeprefix(":")}'
    if socket.exists():
        raise RuntimeError(f'{display} is occupied; choose KILN_TASK7_DISPLAY')
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
        archive = os.environ.get('KILN_TASK7_ARCHIVE', str(REPO/'samples/ui2d_editor'))
        binary = str(Path(binary).resolve())
        archive = str(Path(archive).resolve())
        args = [binary] + ([] if os.environ.get('KILN_TASK7_NO_ARCHIVE') else ['--archive', archive]) + ['--game-name', 'task7-native-check']
        player = subprocess.Popen(args, cwd=tempfile.gettempdir(), env=env,
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
        click(320, 276)  # Damage changes generic game data and starts overlapping SFX.
        key('Escape')
        shot('pause')
        click(320, 304)  # Save from authored UI while gameplay is paused.
        slot_path = run/'data/task7-native-check/slot_0.json'
        wait_for(slot_path.exists, player, player_log)
        def saved():
            return json.loads(json.loads(slot_path.read_text())['data'])
        first = saved()
        assert first['game_data']['health']['Integer'] == 66
        assert first['game_data']['inventory']['Array'][0]['Text'] == 'brass_key'
        assert first['ui']['active'] == 'pause'
        assert first['attached']['ui']['memory']['values']['starts']['Integer'] == 1
        def actor_x(snapshot):
            entity = next(e for e in snapshot['scene']['entities'] if e['id'] == 'player')
            return next(c['x'] for c in entity['components'] if c['type'] == 'Transform')
        assert actor_x(first) > 40
        player.terminate()
        player.wait(timeout=10)
        # Relaunch with the same profile, then Continue from the actual title menu.
        player = subprocess.Popen(args, cwd=tempfile.gettempdir(), env=env,
                                  stdout=player_log.open('a'), stderr=subprocess.STDOUT)
        window = wait_for(search, player, player_log)
        subprocess.run([xdo, 'windowfocus', '--sync', window], env=env, check=True)
        shot('restarted_title')
        click(320, 276)
        time.sleep(0.15)
        key('Escape')
        key('F5')
        shot('resumed_pause')
        resumed = saved()
        assert resumed['game_data']['health']['Integer'] == 66
        assert resumed['attached']['ui']['memory']['values']['starts']['Integer'] == 1
        assert resumed['attached']['ui']['memory']['values']['ticks']['Integer'] >= first['attached']['ui']['memory']['values']['ticks']['Integer']
        assert actor_x(resumed) >= actor_x(first)
        preferences = json.loads((run/'data/task7-native-check/preferences.json').read_text())
        assert preferences['audio']['master_volume'] == .25
        assert preferences['values']['menu_setting'] == 'saved'
        # Corruption is surfaced through the real F9 route; neither file nor live state changes.
        slot_path.write_text('broken')
        key('F9')
        shot('load_error')
        assert slot_path.read_text() == 'broken'
        log = player_log.read_text()
        assert 'could not load game:' in log, log[-4000:]
        if any(error in log for error in ['Validation Error', 'panicked', 'scene script failed']):
            raise AssertionError(log[-6000:])
        result = {'passed': True, 'health': 66, 'script_starts': 1, 'saved_x': actor_x(first), 'resumed_x': actor_x(resumed), 'fresh_process_resume': True, 'corrupt_slot_preserved': True, 'evidence': str(output), 'screenshots': [str(p) for p in sorted(output.glob('*.png'))]}
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
