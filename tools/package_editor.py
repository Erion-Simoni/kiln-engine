#!/usr/bin/env python3
"""Prepare a development/release portable installation from prebuilt tools.

Uses only Python's standard library; Cargo metadata is a preparation input.
No build or download is performed. The shipped editor/exporter needs no Python.
"""
import argparse
import gzip
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import tempfile
import tomllib

REPO = Path(__file__).resolve().parents[1]
TOOLS = ('kiln-editor', 'kiln-cli', 'kiln-player')


def digest(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True)+'\n', encoding='utf-8')


def copy_file(source, destination, executable=False):
    source = Path(source)
    if source.is_symlink() or not source.is_file() or source.stat().st_size > 512*1024*1024:
        raise ValueError(f'Not a regular bounded file: {source}')
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(source, destination)
    destination.chmod(0o755 if executable else 0o644)


def copy_tree(source, destination):
    if source.is_symlink() or not source.is_dir():
        raise ValueError(f'Not a directory: {source}')
    for path in sorted(source.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'Symlink in input tree: {path}')
        if path.is_file():
            copy_file(path, destination/path.relative_to(source))
        elif not path.is_dir():
            raise ValueError(f'Special file in input tree: {path}')


def run_json(command):
    result = subprocess.run(list(map(str, command)), check=True, capture_output=True,
                            text=True, timeout=60)
    if len(result.stdout) > 65536:
        raise ValueError('Oversized tool response')
    return json.loads(result.stdout)


def identities(binary_dir):
    suffix = '.exe' if os.name == 'nt' else ''
    result = {}
    for tool in TOOLS:
        info = run_json([binary_dir/(tool+suffix), '--tool-info'])
        required = {'audio', 'fonts', 'gamepad'} if tool != 'kiln-cli' else {
            'native-export', 'browser-export', 'bundle-verify'}
        if info.get('tool') != tool or info.get('schema') != 1 or not required <= set(info.get('features', [])):
            raise ValueError(f'Invalid/missing installed-tool identity: {tool}')
        result[tool] = info
    first = result[TOOLS[0]]
    for info in result.values():
        if any(info[key] != first[key] for key in ('version', 'platform', 'arch', 'debug_assertions')):
            raise ValueError('Tools must have matching version, host and build profile')
    return result


def reference_notices(destination):
    records = json.loads((REPO/'packaging/license-data.json').read_text())
    for name, record in records.items():
        source = REPO/'packaging/licenses'/f'{name}.txt'
        if digest(source) != record['sha256']:
            raise ValueError(f'Changed pinned license reference: {name}')
        copy_file(source, destination/'reference'/source.name)
    write_json(destination/'reference/sources.json', records)
    return set(records)


def package_notices(metadata_paths, destination):
    reference_ids = reference_notices(destination)
    packages = {}
    for path in metadata_paths:
        metadata = json.loads(path.read_text())
        if metadata.get('version') != 1 or not metadata.get('resolve'):
            raise ValueError('Expected full Cargo metadata format-version 1')
        for package in metadata['packages']:
            packages[package['id']] = package
    lock = tomllib.loads((REPO/'Cargo.lock').read_text())
    checksums = {(p['name'], p['version'], p.get('source')): p.get('checksum') for p in lock['package']}
    inventory = []
    for package in sorted(packages.values(), key=lambda p: (p['name'], p['version'])):
        root = Path(package['manifest_path']).parent
        key = f"{package['name']}-{package['version']}"
        expression = package['license']
        ids = set(re.findall(r'[A-Za-z0-9][A-Za-z0-9.+-]*', expression or '')) - {'AND', 'OR', 'WITH'}
        unknown = ids-reference_ids-{'LicenseRef-UFL-1.0'}
        if unknown:
            raise ValueError(f'Missing license references for {key}: {sorted(unknown)}')
        original = []
        # Preserve original attribution text, including nested font and vendor notices.
        for path in sorted(root.rglob('*')):
            if not path.is_file() or path.is_symlink():
                continue
            relative = path.relative_to(root)
            if any(part.startswith('.') or part in {'target', 'node_modules'} for part in relative.parts):
                continue
            name = path.name.lower()
            if (any(word in name for word in ('license', 'licence', 'copying', 'copyright', 'notice'))
                    or name in {'ofl.txt', 'ufl.txt'} or (len(relative.parts) == 1 and name.startswith('readme'))):
                if path.stat().st_size > 2*1024*1024:
                    raise ValueError(f'Oversized notice: {path}')
                output = Path('packages')/key/relative
                copy_file(path, destination/output)
                original.append(output.as_posix())
        # Package manifests preserve author/repository attribution when crates omit license files.
        output = Path('packages')/key/'Cargo.toml'
        copy_file(root/'Cargo.toml', destination/output)
        original.append(output.as_posix())
        if 'LicenseRef-UFL-1.0' in ids and not any(p.endswith('/UFL.txt') for p in original):
            raise ValueError('Missing Ubuntu Font License original text')
        record = {k: package[k] for k in ('name', 'version', 'license', 'repository', 'source')}
        record['original_notices'] = original
        if package['source']:
            if not package['source'].startswith('registry+'):
                raise ValueError(f'Unsupported source provenance: {key}')
            checksum = checksums.get((package['name'], package['version'], package['source']))
            if not checksum:
                raise ValueError(f'Missing Cargo.lock checksum: {key}')
            record['cargo_checksum'] = checksum
            record['source_download'] = f"https://static.crates.io/crates/{package['name']}/{key}.crate"
            if 'MPL-2.0' in ids:
                archive = root.parents[2]/'cache'/root.parent.name/f'{key}.crate'
                if digest(archive) != checksum:
                    raise ValueError(f'Changed MPL source archive: {key}')
                output = Path('sources')/archive.name
                copy_file(archive, destination/output)
                record['included_source'] = output.as_posix()
        inventory.append(record)
    if not inventory:
        raise ValueError('Empty package inventory')
    write_json(destination/'inventory.json', {
        'scope': 'Conservative union of supplied host and WASM Cargo workspace graphs; includes build/test dependencies.',
        'workspace_license_declaration': 'MIT OR Apache-2.0',
        'packages': inventory,
    })
    (destination/'README.txt').write_text(
        'Dependency notices for this prepared development installation.\n'
        'inventory.json preserves package license declarations, original notices, source URLs and checksums.\n'
        'The inventory is a conservative host/WASM workspace superset, not an exact linked-code report.\n'
        'reference/ contains pinned SPDX reference texts; these do not replace original copyright notices.\n'
        'sources/ contains unmodified MPL dependency crate sources verified against Cargo checksums.\n'
        'Internal manifests with no package license retain null; the workspace declares MIT OR Apache-2.0.\n'
        'rust/ contains the preparing compiler standard-library copyright report; OS drivers are not shipped.\n'
        'Keep this directory and the example font license when redistributing.\n', encoding='utf-8')


def file_entries(root):
    entries = []
    for path in sorted(root.rglob('*')):
        if path.is_file():
            entries.append({'path': path.relative_to(root).as_posix(), 'bytes': path.stat().st_size,
                            'sha256': digest(path), 'executable': bool(path.stat().st_mode & 0o111)})
    if len(entries) > 19999 or sum(e['bytes'] for e in entries) > 2*1024*1024*1024:
        raise ValueError('Installation exceeds verifier limits')
    return entries


def normalized_archive(root, archive):
    with archive.open('xb') as raw, gzip.GzipFile(fileobj=raw, mode='wb', filename='', mtime=0) as compressed:
        with tarfile.open(fileobj=compressed, mode='w') as tar:
            for path in [root, *sorted(root.rglob('*'))]:
                if path.is_symlink() or not (path.is_dir() or path.is_file()):
                    raise ValueError('Archive contains a link or special file')
                entry = tar.gettarinfo(str(path), arcname=str(Path(root.name)/path.relative_to(root)))
                entry.uid = entry.gid = entry.mtime = 0
                entry.uname = entry.gname = ''
                entry.mode = 0o755 if path.is_dir() or path.stat().st_mode & 0o111 else 0o644
                if path.is_file():
                    with path.open('rb') as stream:
                        tar.addfile(entry, stream)
                else:
                    tar.addfile(entry)


def compiler_notices(destination):
    compiler = subprocess.check_output(['rustc', '-vV'], text=True).strip()
    sysroot = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip())
    copy_file(sysroot/'share/doc/rust/COPYRIGHT-library.html', destination/'rust/COPYRIGHT-library.html')
    (destination/'rust/compiler.txt').write_text(compiler+'\n', encoding='utf-8')
    return compiler


def system_observations(binary_dir, info):
    observation = {'system_libraries_bundled': False,
                   'rust_standard_library_copyright_report_included': True}
    if info['platform'] == 'linux':
        for tool in TOOLS:
            binary = binary_dir/tool
            dynamic = subprocess.run(['readelf', '-d', str(binary)], check=True, capture_output=True, text=True).stdout
            versions = subprocess.run(['readelf', '--version-info', str(binary)], check=True, capture_output=True, text=True).stdout
            glibc = re.findall(r'GLIBC_(\d+)\.(\d+)', versions)
            observation[tool] = {'elf_needed': re.findall(r'Shared library: \[(.*?)\]', dynamic),
                                 'max_glibc_symbol': '.'.join(max(glibc, key=lambda pair: tuple(map(int, pair)))) if glibc else None}
    return observation


def prepare(args):
    if args.output.exists() or args.output.is_symlink():
        raise ValueError('Refusing to replace existing output')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    identities_by_tool = identities(args.binaries)
    identity = identities_by_tool[TOOLS[0]]
    with tempfile.TemporaryDirectory(prefix='.kiln-package-', dir=args.output.parent) as temporary:
        root = Path(temporary)/f"kiln-{identity['version']}-{identity['platform']}-{identity['arch']}"
        root.mkdir()
        for tool in TOOLS:
            name = tool+('.exe' if os.name == 'nt' else '')
            copy_file(args.binaries/name, root/name, executable=True)
        for name in ('kiln_web.js', 'kiln_web_bg.wasm', 'runtime.json'):
            copy_file(args.web_runtime/name, root/'web-runtime'/name)
        copy_tree(REPO/'templates', root/'templates')
        copy_tree(REPO/'samples/ui2d_editor', root/'examples/ui2d_editor')
        copy_tree(REPO/'samples/browser_physics2d', root/'examples/browser_physics2d')
        copy_file(REPO/'Cargo.lock', root/'Cargo.lock')
        if identity['platform'] == 'linux':
            copy_file(REPO/'packaging/linux-runtime-packages.txt', root/'linux-runtime-packages.txt')
        package_notices(args.metadata, root/'licenses')
        compiler = compiler_notices(root/'licenses')
        (root/'README.txt').write_text(
            'Kiln portable editor build candidate\n\n'
            'Extract the complete directory somewhere writable; keep its contents together.\n'
            'Run ./kiln-cli verify-editor-bundle --input . before first use.\n'
            'Launch kiln-editor, then open examples/ui2d_editor/kiln.toml or create a project.\n'
            'Save projects outside this installation to preserve installation verification.\n'
            'Native and Browser exports use sibling kiln-cli, kiln-player and web-runtime.\n'
            'No Rust, Cargo, Node, Python or wasm-bindgen is needed to author/export/play.\n'
            'Browser output must be served over HTTP(S); open index.html through that server.\n'
            'Browser physics requires a physics-enabled runtime; material overrides remain unsupported.\n'
            'Open examples/browser_physics2d/kiln.toml for the physics playground.\n'
            'Linux requires compatible glibc and system display, GPU, audio and controller libraries.\n'
            'On Ubuntu 24.04, install the packages listed in linux-runtime-packages.txt.\n'
            'This includes the runtime-loaded libxkbcommon-x11 dependency required for X11.\n'
            'Drivers/system libraries are not bundled; bundle.json records direct ELF/glibc observations.\n'
            'This is development relocation evidence, not a signed release or physical-device compatibility certification.\n'
            'Internal package license declarations and exact binary provenance require release review.\n'
            'Checksums detect changes; this unsigned manifest does not authenticate a download.\n', encoding='utf-8')
        initial = file_entries(root)
        (root/'SHA256SUMS').write_text(''.join(f"{e['sha256']}  {e['path']}\n" for e in initial), encoding='utf-8')
        revision = subprocess.run(['git', '-C', str(REPO), 'rev-parse', 'HEAD'], check=True, capture_output=True, text=True).stdout.strip()
        dirty = bool(subprocess.run(['git', '-C', str(REPO), 'status', '--porcelain'], check=True, capture_output=True, text=True).stdout)
        manifest = {k: identity[k] for k in ('schema', 'version', 'platform', 'arch')}
        manifest.update(tools=identities_by_tool, files=file_entries(root), source_revision=revision,
                        source_dirty=dirty, preparing_compiler=compiler,
                        distribution='development' if identity['debug_assertions'] else 'release-candidate', system_observations=system_observations(args.binaries, identity))
        write_json(root/'bundle.json', manifest)
        subprocess.run([str(root/('kiln-cli.exe' if os.name == 'nt' else 'kiln-cli')),
                        'verify-editor-bundle', '--input', str(root)], check=True, timeout=120)
        archive = Path(temporary)/'bundle.tar.gz'
        normalized_archive(root, archive)
        # Same-filesystem exclusive publication; no existing output can be overwritten.
        os.link(archive, args.output)
    print(json.dumps({'archive': str(args.output), 'sha256': digest(args.output), 'bytes': args.output.stat().st_size}))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binaries', type=Path, required=True)
    parser.add_argument('--web-runtime', type=Path, required=True)
    parser.add_argument('--metadata', type=Path, action='append', required=True, help='Host/WASM filtered Cargo metadata JSON (repeat)')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.binaries = args.binaries.resolve()
    args.web_runtime = args.web_runtime.resolve()
    args.output = args.output.absolute()
    prepare(args)


if __name__ == '__main__':
    main()
