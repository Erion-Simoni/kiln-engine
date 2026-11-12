#!/usr/bin/env python3
"""Extract/move a prepared development editor; exercise only installed tools.

Requires the native/Xvfb and browser dependencies used by the existing smoke tests.
Archives and mutable installations stay outside Git. This is relocation evidence,
not a different-machine dependency or physical GPU/audio acceptance test.
"""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
from export_notices import verify_export_notices

REPO = Path(__file__).resolve().parents[2]


def main():
    archive = Path(sys.argv[1]).resolve()
    evidence = Path(os.environ.get('KILN_PORTABLE_EVIDENCE_DIR', REPO/'target/task8-portable-evidence')).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence/'result.json').unlink(missing_ok=True)
    outside = Path(tempfile.mkdtemp(prefix='kiln-portable-acceptance-'))
    extracted = outside/'extracted'
    with tarfile.open(archive) as stream:
        stream.extractall(extracted, filter='data')
    roots = list(extracted.iterdir())
    if len(roots) != 1 or not roots[0].is_dir():
        raise RuntimeError('Expected a single installation root')
    installed = outside/'moved installation'
    roots[0].rename(installed)
    unrelated = outside/'unrelated working directory'
    unrelated.mkdir()
    cli = installed/'kiln-cli'
    env = os.environ.copy()
    env['PATH'] = ''
    if shutil.which('cargo', path=env['PATH']) or shutil.which('rustc', path=env['PATH']):
        raise RuntimeError('Rust must be unavailable')
    log = evidence/'installed-cli.log'
    log.write_text('')

    def run(*args, success=True):
        process = subprocess.run(list(map(str, args)), cwd=unrelated, env=env,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
        with log.open('ab') as stream:
            stream.write(process.stdout)
        if (process.returncode == 0) != success:
            raise RuntimeError(log.read_text()[-5000:])
        return process.stdout

    def verify(success=True):
        return run(cli, 'verify-editor-bundle', '--input', installed, success=success)

    verify()
    root_link = outside/'installation-link'
    root_link.symlink_to(installed, target_is_directory=True)
    run(cli, 'verify-editor-bundle', '--input', root_link, success=False)
    root_link.unlink()
    for tool in ('kiln-editor', 'kiln-cli', 'kiln-player'):
        identity = json.loads(run(installed/tool, '--tool-info'))
        if identity['tool'] != tool:
            raise RuntimeError('Wrong installed tool')
    native = installed/'kiln-player'
    original = native.read_bytes()
    with native.open('ab') as stream:
        stream.write(b'tamper')
    verify(success=False)
    corrupt_output = outside/'corrupt-installation-export'
    run(cli, 'export', '--platform', 'linux', '--project', installed/'examples/ui2d_editor/kiln.toml',
        '--output', corrupt_output, '--job-id', 'notice-corrupt', success=False)
    if corrupt_output.exists() or (outside/'.kiln-export-notice-corrupt').exists():
        raise RuntimeError('Corrupt installation export published or left staging')
    native.write_bytes(original)
    original_mode = native.stat().st_mode
    native.chmod(0o644)
    verify(success=False)
    native.chmod(original_mode)
    manifest = installed/'bundle.json'
    original_manifest = manifest.read_text()
    modified = json.loads(original_manifest)
    modified['tools']['kiln-editor']['version'] = 'wrong'
    manifest.write_text(json.dumps(modified))
    verify(success=False)
    manifest.write_text(original_manifest)
    sample = installed/'examples/ui2d_editor/kiln.toml'
    saved = sample.read_bytes()
    sample.unlink()
    verify(success=False)
    sample.write_bytes(saved)
    (installed/'unexpected-link').symlink_to(unrelated, target_is_directory=True)
    verify(success=False)
    (installed/'unexpected-link').unlink()
    verify()
    initialized = []
    for template in ('empty_2d', 'platformer', 'topdown'):
        project = outside/f'new {template}'
        run(cli, 'init', '--name', 'Installed starter', '--template', template, '--output', project)
        run(cli, 'init', '--name', 'Do not replace', '--output', project, success=False)
        for platform in ('linux', 'web'):
            output = outside/f'{template}-{platform}-export'
            run(cli, 'export', '--platform', platform, '--project', project/'kiln.toml', '--output', output)
            verify_export_notices(installed, output, platform == 'web')
        initialized.append(template)
    env = os.environ.copy()
    env.update(KILN_EDITOR_PATH=str(installed/'kiln-editor'),
               KILN_EXPORT_PROJECT=str(installed/'examples/ui2d_editor'),
               KILN_EXPORT_CWD=str(unrelated),
               KILN_EXPORT_EVIDENCE_DIR=str(evidence/'native-export'),
               KILN_BROWSER_EXPORT_EVIDENCE_DIR=str(evidence/'browser-export'))
    for script in ('export_smoke.py', 'browser_export_smoke.py'):
        subprocess.run([sys.executable, str(REPO/'tests/native'/script)], cwd=unrelated, env=env, check=True)
    native_result = json.loads((evidence/'native-export/result.json').read_text())
    browser_result = json.loads((evidence/'browser-export/result.json').read_text())
    if not all(r['source_project_unavailable'] for r in (native_result, browser_result)):
        raise RuntimeError('Exported source must be unavailable for final gameplay')
    verify_export_notices(installed, native_result['relocated_export'], False)
    verify_export_notices(installed, browser_result['relocated_export'], True)
    env.update(PATH='/usr/bin:/bin', KILN_PLAYER_PATH=str(Path(native_result['relocated_export'])/'game'),
               KILN_TASK7_NO_ARCHIVE='1', KILN_TASK7_DISPLAY=':88',
               KILN_TASK7_EVIDENCE_DIR=str(evidence/'native-gameplay'),
               WEB_EXPORT_ROOT=browser_result['relocated_export'],
               WEB_TASK7_EVIDENCE=str(evidence/'browser-gameplay'))
    subprocess.run([sys.executable, str(REPO/'tests/native/task7_smoke.py')], cwd=unrelated, env=env, check=True)
    # Node is a test driver only, not an editor/export/game dependency.
    node = shutil.which('node')
    subprocess.run([node, str(REPO/'tests/web/task7.cjs')], cwd=REPO, env=env, check=True)
    verify()
    result = {'installation': str(installed), 'working_directory': str(unrelated),
              'archive': str(archive), 'installed_tools_verified': True,
              'tamper_missing_permissions_mixed_version_symlink_rejected': True,
              'templates_initialized_and_exported_native_web': initialized,
              'native_browser_gui_export_and_relocated_gameplay': True,
              'installation_unchanged_after_use': True, 'clean_machine': False,
              'runtime_notices_preserved_bound_and_receipted': True,
              'physical_gpu_audio_controller': False}
    (evidence/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
