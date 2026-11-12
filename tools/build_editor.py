#!/usr/bin/env python3
"""Build a captured Linux portable candidate offline, with an unsigned local record."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
import package_editor as package

REPO = Path(__file__).resolve().parents[1]
HOST = 'x86_64-unknown-linux-gnu'
WASM = 'wasm32-unknown-unknown'
OVERRIDE_PREFIXES = ('CARGO_PROFILE_', 'CARGO_TARGET_', 'CARGO_BUILD_', 'RUSTC_', 'RUSTDOC')
OVERRIDES = {'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC', 'CARGO_INCREMENTAL'}
BUILD_SETTINGS = ('PKG_CONFIG_PATH', 'PKG_CONFIG_SYSROOT_DIR', 'CC', 'CXX', 'AR',
                  'CFLAGS', 'CXXFLAGS', 'LDFLAGS', 'LIBRARY_PATH', 'LD_LIBRARY_PATH')


def git(repo, *arguments):
    return subprocess.check_output(['git', '-C', str(repo), *arguments])


def build_environment(output, jobs):
    env = {key: value for key, value in os.environ.items()
           if key not in OVERRIDES and not key.startswith(OVERRIDE_PREFIXES)}
    env.update(CARGO_TARGET_DIR=str(output/'target'), CARGO_INCREMENTAL='0',
               CARGO_BUILD_JOBS=str(jobs))
    return env


def snapshot(repo, destination):
    revision = git(repo, 'rev-parse', 'HEAD').decode().strip()
    dirty = bool(git(repo, 'status', '--porcelain'))
    names = set(git(repo, 'ls-files', '--cached', '--others', '--exclude-standard', '-z').split(b'\0'))
    names.discard(b'')
    names.add(b'Cargo.lock')  # Required in current tracked and legacy ignored snapshots.
    for name in sorted(names):
        relative = Path(os.fsdecode(name))
        if relative.is_absolute() or '..' in relative.parts:
            raise ValueError('Invalid source path')
        source = repo/relative
        if any(parent.is_symlink() for parent in [source, *source.parents] if parent != repo.parent):
            raise ValueError(f'Source links are unsupported: {relative}')
        package.copy_file(source, destination/relative, executable=bool(source.stat().st_mode & 0o111))
    entries = package.file_entries(destination)
    return {'revision': revision, 'dirty': dirty, 'files': entries,
            'files_sha256': hashlib.sha256(json.dumps(entries, sort_keys=True).encode()).hexdigest(),
            'cargo_lock_sha256': package.digest(destination/'Cargo.lock')}


def config_observations(env, source):
    # Only hashes, never contents: Cargo configuration can contain private registries.
    cargo_home = Path(env.get('CARGO_HOME', str(Path.home()/'.cargo')))
    candidates = [cargo_home/'config', cargo_home/'config.toml',
                  source/'.cargo/config', source/'.cargo/config.toml']
    return [{'location': 'cargo-home' if path.parent == cargo_home else 'source',
             'name': path.name, 'sha256': package.digest(path)} for path in candidates if path.is_file()]


def run(command, source, output, env, commands, name, json_output=False):
    log = output/'logs'/f'{name}.log'
    log.parent.mkdir(exist_ok=True)
    print(f'Building: {name} (log: {log})', flush=True)
    error_log = log.with_suffix('.stderr.log')
    with log.open('wb') as stream, error_log.open('wb') as errors:
        subprocess.run(list(map(str, command)), cwd=source, env=env,
                       stdout=stream, stderr=errors if json_output else subprocess.STDOUT, check=True)
    commands.append({'argv': list(map(str, command)), 'log': log.relative_to(output).as_posix(),
                     'log_sha256': package.digest(log), 'stderr_sha256': package.digest(error_log)})
    return log


def artifacts(binary_dir, runtime):
    paths = {tool: binary_dir/tool for tool in package.TOOLS}
    paths.update({f'web-runtime/{name}': runtime/name for name in
                  ('kiln_web.js', 'kiln_web_bg.wasm', 'runtime.json')})
    return [{'path': name, 'sha256': package.digest(path), 'bytes': path.stat().st_size}
            for name, path in sorted(paths.items())]


def replay_source(previous, destination, profile):
    """Validate/copy frozen inputs; command strings in a record are never executed."""
    path = previous/'build-record.json'
    if path.is_symlink() or path.stat().st_size > 8*1024*1024:
        raise ValueError('Invalid replay record')
    record = json.loads(path.read_text())
    result = json.loads((previous/'result.json').read_text())
    if (record.get('schema') != 1 or record.get('kind') != 'unsigned-local-build'
            or record.get('profile') != profile or record.get('host') != HOST
            or result.get('passed') is not True
            or package.digest(path) != result['build_record_sha256']):
        raise ValueError('Incompatible/incomplete replay build')
    captured = record['source']
    if ((previous/'source').is_symlink()
            or hashlib.sha256(json.dumps(captured['files'], sort_keys=True).encode()).hexdigest() != captured['files_sha256']
            or package.digest(previous/'source.tar.gz') != captured['archive_sha256']
            or package.file_entries(previous/'source') != captured['files']
            or package.digest(previous/'source/Cargo.lock') != captured['cargo_lock_sha256']):
        raise ValueError('Replay source/archive/lock changed')
    for entry in captured['files']:
        package.copy_file(previous/'source'/entry['path'], destination/entry['path'], executable=entry['executable'])
    if package.file_entries(destination) != captured['files']:
        raise ValueError('Replay copy changed')
    return dict(captured), record


def build(args):
    if args.output.exists() or args.output.is_symlink():
        raise FileExistsError('Build output already exists')
    output = args.output.resolve()
    if output.is_relative_to(REPO) and not output.is_relative_to(REPO/'target'):
        raise ValueError('In-repository build output must be under ignored target/')
    output.mkdir(parents=True, exist_ok=False)  # Never reuse build products or replace evidence.
    builder_hash = package.digest(Path(__file__))
    source = output/'source'
    source.mkdir()
    previous = getattr(args, 'source_from', None)
    captured, prior = replay_source(previous.resolve(), source, args.profile) if previous else (snapshot(REPO, source), None)
    package.normalized_archive(source, output/'source.tar.gz')
    if prior is not None and package.digest(output/'source.tar.gz') != captured['archive_sha256']:
        raise ValueError('Replayed source archive is not identical')
    captured['archive_sha256'] = package.digest(output/'source.tar.gz')
    env = build_environment(output, args.jobs)
    versions = {name: subprocess.check_output(command, cwd=source, env=env, text=True).strip()
                for name, command in [('rustc', ['rustc', '-vV']), ('cargo', ['cargo', '-V']),
                                      ('wasm_bindgen', ['wasm-bindgen', '--version'])]}
    if f'host: {HOST}' not in versions['rustc']:
        raise ValueError('Captured portable builds currently require Linux x86_64')
    settings = {key: env[key] for key in BUILD_SETTINGS if key in env}
    configurations = config_observations(env, source)
    if prior is not None and (versions != prior['toolchain'] or settings != prior['build_settings']
                              or configurations != prior['configuration_hashes']):
        raise ValueError('Replay compiler/SDK/configuration differs from the original build')
    commands = []
    flags = ['--release'] if args.profile == 'release' else []
    run(['cargo', 'build', '--offline', '--locked', '--target', HOST, *flags,
         '-p', 'kiln-editor', '-p', 'kiln-cli', '-p', 'kiln-player', '--bins'],
        source, output, env, commands, 'native')
    run(['cargo', 'build', '--offline', '--locked', '--target', WASM, *flags, '-p', 'kiln-web'],
        source, output, env, commands, 'wasm')
    directory = 'release' if args.profile == 'release' else 'debug'
    binaries = output/'target'/HOST/directory
    runtime = output/'web-runtime'
    run(['wasm-bindgen', output/'target'/WASM/directory/'kiln_web.wasm',
         '--target', 'web', '--out-dir', runtime], source, output, env, commands, 'bindgen')
    run([binaries/'kiln-cli', 'web-runtime', '--input', runtime],
        source, output, env, commands, 'runtime-prepare')
    metadata = []
    for name, platform in [('native', HOST), ('wasm', WASM)]:
        log = run(['cargo', 'metadata', '--offline', '--locked', '--format-version', '1',
                   '--filter-platform', platform], source, output, env, commands, f'metadata-{name}', json_output=True)
        json.loads(log.read_text())
        metadata.append(log)
    if package.file_entries(source) != captured['files']:
        raise ValueError('Source snapshot changed during build')
    if package.digest(Path(__file__)) != builder_hash:
        raise ValueError('Build driver changed during execution')
    record = {'schema': 1, 'kind': 'unsigned-local-build', 'profile': args.profile,
              'host': HOST, 'source': captured, 'toolchain': versions,
              'builder_sha256': builder_hash,
              'build_settings': settings,
              'configuration_hashes': configurations, 'commands': commands,
              'artifacts': artifacts(binaries, runtime),
              'limitations': 'Unsigned local execution record; not hermetic, signed or byte-reproducibility proof.'}
    record_path = output/'build-record.json'
    if prior is not None:
        record['rebuild_of'] = package.digest(previous.resolve()/'build-record.json')
    package.copy_file(Path(__file__), output/'build-driver.py')
    package.write_json(record_path, record)
    package.REPO = source
    package.prepare(argparse.Namespace(output=output/'kiln-portable.tar.gz', binaries=binaries,
                                      web_runtime=runtime, metadata=metadata, build_record=record_path))
    package.write_json(output/'result.json', {'passed': True, 'profile': args.profile,
                       'archive_sha256': package.digest(output/'kiln-portable.tar.gz'),
                       'source_archive_sha256': captured['archive_sha256'],
                       'build_record_sha256': package.digest(record_path)})


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True, help='New evidence/build directory, never reused')
    parser.add_argument('--profile', choices=('release', 'dev'), default='release')
    parser.add_argument('--jobs', type=int, choices=range(1, 17), default=2)
    parser.add_argument('--source-from', type=Path, help='Replay a completed build directory, with identical captured inputs/toolchain')
    build(parser.parse_args())


if __name__ == '__main__':
    main()
