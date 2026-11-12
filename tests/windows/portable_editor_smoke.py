#!/usr/bin/env python3
"""Accept a moved Windows installation using installed tools and visible windows.

The build runner supplies a Windows desktop. This does not certify physical
hardware, audible output, full GUI authoring/export or complete game playthrough.
"""
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import stat
import subprocess
import sys
import tempfile
import time
import zipfile

WINDOWS_RESERVED_NAMES = {'CON', 'PRN', 'AUX', 'NUL'} | {
    f'{prefix}{number}' for prefix in ('COM', 'LPT') for number in range(1, 10)
}


def extract(archive, destination):
    """Validate all ZIP names/types/bounds before writing into a new directory."""
    with zipfile.ZipFile(archive) as stream:
        entries = stream.infolist()
        if len(entries) > 20001 or sum(entry.file_size for entry in entries) > 2*1024**3:
            raise ValueError('Oversized installation ZIP')
        seen = set()
        for entry in entries:
            # Windows ZipInfo normalizes backslashes and strips NUL suffixes.
            # Validate the original central-directory name before those changes.
            name = entry.orig_filename.rstrip('/')
            parts = name.split('/')
            kind = stat.S_IFMT(entry.external_attr >> 16)
            if (not name or PurePosixPath(name).is_absolute() or '\\' in name or ':' in name
                    or any(ord(char) < 32 for char in name)
                    or any(part in ('', '.', '..') or part.rstrip(' .') != part for part in parts)
                    or any(part.split('.')[0].upper() in WINDOWS_RESERVED_NAMES for part in parts)
                    or name.casefold() in seen or entry.file_size > 512*1024**2
                    or kind not in (0, stat.S_IFREG, stat.S_IFDIR)):
                raise ValueError('Invalid, duplicate or linked installation ZIP member')
            seen.add(name.casefold())
        destination.mkdir()  # Refuse to merge into an existing destination.
        stream.extractall(destination)
    roots = list(destination.iterdir())
    if len(roots) != 1 or not roots[0].is_dir():
        raise ValueError('Expected one installation directory')
    return roots[0]


class Windows:
    def __init__(self):
        self.api = ctypes.WinDLL('user32', use_last_error=True)
        self.callback = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
        self.api.EnumWindows.argtypes = [self.callback, wintypes.LPARAM]
        self.api.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
        self.api.IsWindowVisible.argtypes = [wintypes.HWND]
        self.api.GetClientRect.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.RECT)]
        self.api.ClientToScreen.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.POINT)]
        self.api.SetForegroundWindow.argtypes = [wintypes.HWND]
        self.api.GetForegroundWindow.restype = wintypes.HWND
        self.api.PostMessageW.argtypes = [wintypes.HWND, wintypes.UINT, wintypes.WPARAM, wintypes.LPARAM]

    def visible(self, pid):
        found = []
        @self.callback
        def visit(window, _):
            owner = wintypes.DWORD()
            self.api.GetWindowThreadProcessId(window, ctypes.byref(owner))
            if owner.value == pid and self.api.IsWindowVisible(window):
                found.append(window)
            return True
        self.api.EnumWindows(visit, 0)
        return found[0] if found else None

    def screenshot(self, window, output, env):
        self.api.SetForegroundWindow(window)
        for _ in range(20):
            if self.api.GetForegroundWindow() == window:
                break
            time.sleep(0.1)
        if self.api.GetForegroundWindow() != window:
            raise RuntimeError('Test window did not become foreground for its capture')
        rectangle, origin = wintypes.RECT(), wintypes.POINT()
        if not self.api.GetClientRect(window, ctypes.byref(rectangle)):
            raise RuntimeError('Cannot inspect window client area')
        if not self.api.ClientToScreen(window, ctypes.byref(origin)):
            raise RuntimeError('Cannot locate window client area')
        powershell = Path(env['SYSTEMROOT'])/'System32/WindowsPowerShell/v1.0/powershell.exe'
        result = subprocess.run([str(powershell), '-NoProfile', '-NonInteractive', '-File',
                        str(Path(__file__).with_name('capture_window.ps1')),
                        '-Left', str(origin.x), '-Top', str(origin.y),
                        '-Width', str(rectangle.right), '-Height', str(rectangle.bottom),
                        '-Output', str(output)], env=env, timeout=30,
                       creationflags=subprocess.CREATE_NO_WINDOW, capture_output=True,
                       text=True, encoding='utf-8', errors='replace')
        if result.returncode != 0:
            raise RuntimeError('Window capture failed: '+(result.stdout+result.stderr)[-3000:])
        print(result.stdout, flush=True)


def accept_window(command, cwd, env, evidence, name):
    windows = Windows()
    with (evidence/f'{name}.log').open('wb') as log:
        child = subprocess.Popen(list(map(str, command)), cwd=cwd, env=env,
                                 stdout=log, stderr=subprocess.STDOUT)
        try:
            deadline = time.monotonic()+60
            window = None
            while child.poll() is None and time.monotonic() < deadline:
                window = windows.visible(child.pid)
                if window:
                    break
                time.sleep(0.1)
            if not window:
                raise RuntimeError(f'{name} failed to open a visible window; see its log')
            # Give the renderer time to present; a visible empty title bar is insufficient.
            for _ in range(30):
                if child.poll() is not None:
                    raise RuntimeError(f'{name} exited during startup: {child.returncode}')
                time.sleep(0.1)
            windows.screenshot(window, evidence/f'{name}.png', env)
            if not windows.api.PostMessageW(window, 0x0010, 0, 0):  # WM_CLOSE
                raise RuntimeError(f'Cannot close {name} normally')
            if child.wait(timeout=30) != 0:
                raise RuntimeError(f'{name} exited with {child.returncode}')
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=15)
    text = (evidence/f'{name}.log').read_text(errors='replace')
    if any(error in text for error in ('panicked at', 'Validation Error', 'Failed to find GPU adapter')):
        raise RuntimeError(f'{name} reported a panic/GPU failure')


def accept(archive, evidence):
    if os.name != 'nt':
        raise RuntimeError('Run Windows acceptance on an actual Windows desktop')
    outside = Path(tempfile.mkdtemp(prefix='kiln-windows-acceptance-'))
    installed = extract(archive, outside/'extracted')
    installed = Path(shutil.move(str(installed), outside/'moved installation'))
    cwd = outside/'unrelated working directory'
    cwd.mkdir()
    env = dict(os.environ, PATH=str(Path(os.environ['SYSTEMROOT'])/'System32'),
               APPDATA=str(outside/'profile/roaming'), LOCALAPPDATA=str(outside/'profile/local'),
               RUST_LOG='warn', WGPU_BACKEND='dx12')
    for name in ('cargo', 'rustc', 'node', 'wasm-bindgen'):
        if shutil.which(name, path=env['PATH']):
            raise RuntimeError(f'Build tool remains on acceptance PATH: {name}')
    cli = installed/'kiln-cli.exe'
    def run(*args, success=True):
        result = subprocess.run(list(map(str, args)), cwd=cwd, env=env, capture_output=True, timeout=120)
        with (evidence/'installed-cli.log').open('ab') as log:
            log.write(result.stdout+result.stderr)
        if (result.returncode == 0) != success:
            output = (result.stdout+result.stderr).decode('utf-8', errors='replace')[-3000:]
            raise RuntimeError(f'Unexpected installed command status {result.returncode}: {args}\n{output}')
        return result.stdout
    run(cli, 'verify-editor-bundle', '--input', installed)
    for tool in ('kiln-editor', 'kiln-cli', 'kiln-player'):
        info = json.loads(run(installed/f'{tool}.exe', '--tool-info'))
        if info['tool'] != tool or info['platform'] != 'windows' or info['debug_assertions']:
            raise RuntimeError('Wrong tool, platform or release profile')
    player = installed/'kiln-player.exe'
    with player.open('ab') as stream:
        stream.write(b'tamper')
    run(cli, 'verify-editor-bundle', '--input', installed, success=False)
    with player.open('r+b') as stream:
        stream.truncate(player.stat().st_size-len(b'tamper'))
    run(cli, 'verify-editor-bundle', '--input', installed)
    for template in ('empty_2d', 'platformer', 'topdown'):
        project = outside/f'new {template}'
        run(cli, 'init', '--name', 'Installed starter', '--template', template, '--output', project)
        run(cli, 'init', '--name', 'Do not replace', '--output', project, success=False)
        for target in ('windows', 'web'):
            output = outside/f'{template} {target}'
            run(cli, 'export', '--platform', target, '--project', project/'kiln.toml', '--output', output)
            receipt = json.loads((output/'export.json').read_text())
            if receipt['runtime']['platform'] != target:
                raise RuntimeError('Wrong export runtime platform')
    project = outside/'game project'
    shutil.copytree(installed/'examples/ui2d_editor', project)
    exported = outside/'original export'
    run(cli, 'export', '--platform', 'windows', '--project', project/'kiln.toml', '--output', exported)
    moved = Path(shutil.move(str(exported), outside/'moved standalone game'))
    project.rename(outside/'unavailable game source')
    run(moved/'game.exe', '--runtime-info')
    accept_window([installed/'kiln-editor.exe'], cwd, env, evidence, 'editor-startup')
    accept_window([moved/'game.exe', '--game-name', 'kiln-windows-acceptance'], cwd, env, evidence, 'player-startup')
    run(cli, 'verify-editor-bundle', '--input', installed)
    with archive.open('rb') as stream:
        checksum = hashlib.file_digest(stream, 'sha256').hexdigest()
    return {'passed': True, 'platform': sys.platform, 'windows_version': sys.getwindowsversion().build,
            'archive_sha256': checksum, 'source_revision': json.loads((installed/'bundle.json').read_text())['source_revision'],
            'relocated_installation_and_corruption_rejection': True, 'build_tools_absent_from_path': True,
            'all_templates_export_windows_and_web': True, 'source_project_unavailable': True,
            'visible_editor_and_exported_player_startup_and_normal_close': True,
            'limits': 'Hosted Windows/DX12 startup and CLI exports, not full GUI authoring/gameplay, audible devices or physical hardware.'}


def main():
    evidence = Path(sys.argv[2]).resolve()
    evidence.mkdir(parents=True, exist_ok=False)
    report = {'passed': False}
    try:
        report = accept(Path(sys.argv[1]).resolve(), evidence)
    except Exception as error:
        diagnostics = []
        for name in ('editor-startup.log', 'player-startup.log'):
            path = evidence/name
            if path.is_file():
                diagnostics.append(name+':\n'+path.read_text(encoding='utf-8', errors='replace')[-3000:])
        report['error'] = '\n'.join([str(error), *diagnostics])
        if os.environ.get('GITHUB_ACTIONS') == 'true':
            message = report['error'].replace('%', '%25').replace('\r', '%0D').replace('\n', '%0A')
            print('::error title=Windows installed acceptance::'+message, flush=True)
        raise
    finally:
        (evidence/'result.json').write_text(json.dumps(report, indent=2)+'\n', encoding='utf-8')


if __name__ == '__main__':
    main()
