"""Check actual exported notice/runtime bytes against an installed editor tree."""
import hashlib
import json
import os
from pathlib import Path


def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def verify_export_notices(installed, output, web):
    installed, output = Path(installed), Path(output)
    receipt = json.loads((output/'export.json').read_text(encoding='utf-8'))
    manifest_path = output/'licenses/runtime-notices.json'
    if receipt.get('runtime_notice_manifest_sha256') != digest(manifest_path):
        raise RuntimeError('Export receipt does not bind its actual notice manifest')
    manifest = json.loads(manifest_path.read_text(encoding='utf-8'))
    source = {path.relative_to(installed).as_posix(): path for path in (installed/'licenses').rglob('*') if path.is_file()}
    expected = {name: {'bytes': path.stat().st_size, 'sha256': digest(path)} for name, path in source.items()}
    listed = {file['path']: {'bytes': file['bytes'], 'sha256': file['sha256']} for file in manifest['files']}
    if listed != expected or len(listed) != len(manifest['files']):
        raise RuntimeError('Export does not preserve the complete installation notice inventory')
    actual = {path.relative_to(output).as_posix() for path in (output/'licenses').rglob('*') if path.is_file()}
    expected_paths = set(expected) | {'licenses/runtime-notices.json'}
    receipted = {file['path'] for file in receipt['files'] if file['path'].startswith('licenses/')}
    if actual != expected_paths or receipted != expected_paths:
        raise RuntimeError('Notice tree/receipt has missing or unlisted files')
    for name, identity in expected.items():
        path = output/name
        if path.is_symlink() or path.stat().st_size != identity['bytes'] or digest(path) != identity['sha256']:
            raise RuntimeError('Exported original notice bytes changed: '+name)
    suffix = '.exe' if os.name == 'nt' else ''
    runtimes = ({f'runtime/{name}': installed/'web-runtime'/name for name in
                 ('kiln_web.js', 'kiln_web_bg.wasm', 'runtime.json')} if web else
                {f'game{suffix}': installed/f'kiln-player{suffix}'})
    listed_runtime = {file['path']: file for file in manifest['runtime_files']}
    if set(listed_runtime) != set(runtimes) or len(listed_runtime) != len(manifest['runtime_files']):
        raise RuntimeError('Wrong runtime notice binding')
    for name, original in runtimes.items():
        file = output/name
        if (digest(file) != digest(original) or digest(file) != listed_runtime[name]['sha256']
                or file.stat().st_size != listed_runtime[name]['bytes']):
            raise RuntimeError('Notices refer to a different runtime payload')
