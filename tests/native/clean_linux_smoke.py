#!/usr/bin/env python3
"""Native editor/game acceptance inside the isolated runtime-only Linux image."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile


def run(command, **kwargs):
    return subprocess.run(list(map(str, command)), check=True, text=True, **kwargs)


def main():
    evidence = Path('/evidence')
    (evidence/'result.json').unlink(missing_ok=True)
    if os.geteuid() == 0:
        raise RuntimeError('Acceptance must run unprivileged')
    absent_tools = ['cargo', 'rustc', 'gcc', 'cc', 'clang', 'make', 'git']
    if any(shutil.which(name) for name in absent_tools):
        raise RuntimeError('Development tools present in runtime test image')
    absent_paths = ['/workspace', '/acceptance/kiln-core', '/acceptance/Cargo.toml',
                    '/.kiln-tools', '/usr/local/cargo']
    for name in absent_paths:
        if Path(name).exists():
            raise RuntimeError(f'Unexpected source/toolchain path: {name}')
    Path.home().mkdir(parents=True, exist_ok=True)
    extracted = Path('/work/extracted')
    with tarfile.open('/input/editor.tar.gz') as archive:
        archive.extractall(extracted, filter='data')
    roots = list(extracted.iterdir())
    if len(roots) != 1 or not roots[0].is_dir():
        raise RuntimeError('Expected one installation root')
    installation = Path('/work/moved installation')
    roots[0].rename(installation)
    unrelated = Path('/work/unrelated cwd')
    unrelated.mkdir()
    expected_packages = Path('/acceptance/linux-runtime-packages.txt').read_text()
    if (installation/'linux-runtime-packages.txt').read_text() != expected_packages:
        raise RuntimeError('Shipped and tested runtime package requirements differ')
    cli = installation/'kiln-cli'
    versions = run(['dpkg-query', '-W', '-f=${Package}\t${Version}\n'], capture_output=True).stdout
    (evidence/'system-packages.tsv').write_text(versions)
    dependencies = {}
    identities = {}
    for tool in ('kiln-editor', 'kiln-cli', 'kiln-player'):
        output = run(['ldd', installation/tool], capture_output=True)
        if 'not found' in output.stdout or 'not found' in output.stderr:
            raise RuntimeError(f'Missing library for {tool}')
        dependencies[tool] = output.stdout
        identities[tool] = json.loads(run([installation/tool, '--tool-info'], capture_output=True).stdout)
    run([cli, 'verify-editor-bundle', '--input', installation], cwd=unrelated)
    env = os.environ.copy()
    env.update(KILN_EDITOR_PATH=str(installation/'kiln-editor'),
               KILN_EXPORT_PROJECT=str(installation/'examples/ui2d_editor'),
               KILN_EXPORT_CWD=str(unrelated),
               KILN_EXPORT_EVIDENCE_DIR=str(evidence/'editor-export'))
    run([sys.executable, '/acceptance/tests/native/export_smoke.py'], cwd=unrelated, env=env)
    exported = json.loads((evidence/'editor-export/result.json').read_text())
    if not exported['source_project_unavailable']:
        raise RuntimeError('Authoring source must be unavailable before playing')
    game = Path(exported['relocated_export'])/'game'
    env.update(KILN_PLAYER_PATH=str(game), KILN_TASK7_NO_ARCHIVE='1',
               KILN_TASK7_EVIDENCE_DIR=str(evidence/'gameplay'), KILN_TASK7_DISPLAY=':88')
    run([sys.executable, '/acceptance/tests/native/task7_smoke.py'], cwd=unrelated, env=env)
    gameplay = json.loads((evidence/'gameplay/result.json').read_text())
    if not gameplay['passed']:
        raise RuntimeError('Relocated game acceptance failed')
    run([cli, 'verify-editor-bundle', '--input', installation], cwd=unrelated)
    result = {'passed': True, 'scope': 'Clean Ubuntu 24.04 userspace; shared host kernel/software graphics',
              'uid': os.geteuid(), 'absent_development_tools': absent_tools,
              'runtime_package_requirements': Path('/acceptance/linux-runtime-packages.txt').read_text().splitlines(),
              'repository_or_development_sysroot_mounted': False,
              'installed_tools': identities, 'native_library_resolution': dependencies,
              'editor_save_export_cancel_and_failure_preservation': True,
              'relocated_gameplay_save_restart_continue': True,
              'source_project_unavailable': True, 'installation_unchanged_after_use': True,
              'physical_gpu_audio_controller': False, 'browser_tested_in_this_container': False}
    (evidence/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    main()
