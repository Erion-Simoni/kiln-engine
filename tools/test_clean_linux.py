#!/usr/bin/env python3
"""Test a prepared editor in a runtime-only, offline Ubuntu userspace.

Only the archive and output directory are mounted; the repository/sysroot are
never mounted. Docker's managed local socket and proxy defaults are preserved.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import uuid

REPO = Path(__file__).resolve().parents[1]


def docker(command, **kwargs):
    env = os.environ.copy()
    for key in ('DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_TLS', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH'):
        env.pop(key, None)
    return subprocess.run(['docker', '--host=unix:///var/run/docker.sock', *map(str, command)],
                          env=env, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('archive', type=Path)
    parser.add_argument('--evidence', type=Path, default=REPO/'target/task8-clean-linux-evidence')
    args = parser.parse_args()
    archive, evidence = args.archive.resolve(strict=True), args.evidence.resolve()
    if any(',' in str(path) for path in (archive, evidence)):
        raise ValueError('Docker bind paths cannot contain commas')
    evidence.mkdir(parents=True, exist_ok=True)
    (evidence/'result.json').unlink(missing_ok=True)
    docker(['info', '--format', '{{.ServerVersion}}'], capture_output=True, text=True)
    with tempfile.TemporaryDirectory(prefix='kiln-clean-linux-build-') as temporary:
        context = Path(temporary)
        shutil.copyfile(REPO/'tests/native/clean-linux/Dockerfile', context/'Dockerfile')
        shutil.copyfile(REPO/'packaging/linux-runtime-packages.txt', context/'linux-runtime-packages.txt')
        scripts = context/'tests/native'
        scripts.mkdir(parents=True)
        for name in ('clean_linux_smoke.py', 'export_smoke.py', 'ui_smoke.py', 'task7_smoke.py'):
            shutil.copyfile(REPO/'tests/native'/name, scripts/name)
        iid = context/'image-id'
        with (evidence/'build.log').open('w') as log:
            docker(['build', '--progress=plain', '--iidfile', iid, context], stdout=log, stderr=subprocess.STDOUT)
        image = iid.read_text().strip()
        name = 'kiln-clean-linux-'+uuid.uuid4().hex[:12]
        with archive.open('rb') as stream:
            checksum = hashlib.file_digest(stream, 'sha256').hexdigest()
        info = {'image_id': image, 'dockerfile_base': (context/'Dockerfile').read_text().split('FROM ')[1].splitlines()[0],
                'archive_sha256': checksum, 'archive_bytes': archive.stat().st_size,
                'user': f'{os.getuid()}:{os.getgid()}', 'network': 'none', 'read_only_root': True,
                'mounts': ['archive (read-only)', 'evidence (writable)'],
                'shared_host_kernel': True, 'physical_hardware_acceptance': False}
        (evidence/'container-info.json').write_text(json.dumps(info, indent=2)+'\n')
        try:
            with (evidence/'container.log').open('w') as log:
                docker(['run', '--rm', '--name', name, '--user', info['user'],
                        '--network=none', '--read-only', '--cap-drop=ALL', '--security-opt=no-new-privileges',
                        '--shm-size=256m', '--tmpfs', '/tmp:rw,exec,mode=1777',
                        '--tmpfs', '/work:rw,exec,mode=1777', '-e', 'HOME=/tmp/kiln-home',
                        '-e', 'PYTHONDONTWRITEBYTECODE=1',
                        '--mount', f'type=bind,src={archive},dst=/input/editor.tar.gz,readonly',
                        '--mount', f'type=bind,src={evidence},dst=/evidence', image],
                       stdout=log, stderr=subprocess.STDOUT, timeout=900)
        finally:
            # Remove only this test's named container if an interrupted CLI left it alive.
            try:
                docker(['rm', '-f', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            except subprocess.CalledProcessError:
                pass
    result = json.loads((evidence/'result.json').read_text())
    if not result.get('passed'):
        raise RuntimeError('Acceptance did not complete')
    print(json.dumps({'passed': True, 'evidence': str(evidence), 'archive_sha256': checksum}, indent=2))


if __name__ == '__main__':
    main()
