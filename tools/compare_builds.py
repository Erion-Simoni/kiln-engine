#!/usr/bin/env python3
"""Rehash two captured builds and compare all native/prepared-browser payload bytes."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import sys

sys.dont_write_bytecode = True
import build_editor as build
import package_editor as package


def bounded_hash(path, limit=512*1024*1024):
    if path.is_symlink() or not path.is_file() or path.stat().st_size > limit:
        raise ValueError(f'Not a regular bounded comparison input: {path.name}')
    return package.digest(path)


def read_json(path):
    bounded_hash(path, 8*1024*1024)
    return json.loads(path.read_text())


def member(root, name):
    if (not isinstance(name, str) or not name or '\\' in name
            or PurePosixPath(name).is_absolute() or '..' in PurePosixPath(name).parts):
        raise ValueError('Invalid recorded log path')
    path = root/name
    if any(parent.is_symlink() for parent in [path, *path.parents] if parent.is_relative_to(root)):
        raise ValueError('Linked comparison input')
    return path


def verify_build(root):
    """Integrity-check real files before trusting any recorded equality claim."""
    record = read_json(root/'build-record.json')
    result = read_json(root/'result.json')
    if (not isinstance(record, dict) or not isinstance(result, dict)
            or record.get('schema') != 1 or record.get('kind') != 'unsigned-local-build'
            or record.get('host') != build.HOST or record.get('profile') not in ('release', 'dev')
            or result.get('passed') is not True or result.get('profile') != record['profile']):
        raise ValueError('Incompatible/incomplete build receipt')
    if bounded_hash(root/'build-record.json', 8*1024*1024) != result['build_record_sha256']:
        raise ValueError('Build record changed')
    if bounded_hash(root/'kiln-portable.tar.gz', 2*1024*1024*1024) != result['archive_sha256']:
        raise ValueError('Completed archive changed')
    source = record['source']
    for path in [root/'source', *(root/'source').rglob('*')]:
        if path.is_symlink() or not (path.is_file() or path.is_dir()):
            raise ValueError('Linked/special captured source input')
    if (bounded_hash(root/'source.tar.gz', 2*1024*1024*1024) != source['archive_sha256']
            or result['source_archive_sha256'] != source['archive_sha256']
            or (root/'source').is_symlink() or package.file_entries(root/'source') != source['files']
            or bounded_hash(root/'source/Cargo.lock') != source['cargo_lock_sha256']
            or hashlib.sha256(json.dumps(source['files'], sort_keys=True).encode()).hexdigest() != source['files_sha256']):
        raise ValueError('Captured source/archive/lock changed')
    driver = root/'build-driver.py'
    if not driver.exists():
        driver = root/'source/tools/build_editor.py'  # Initial records predate the standalone copy.
    if bounded_hash(driver) != record['builder_sha256']:
        raise ValueError('Executing build driver changed or is unavailable')
    directory = 'release' if record['profile'] == 'release' else 'debug'
    binaries, runtime = root/'target'/build.HOST/directory, root/'web-runtime'
    for path in [*(binaries/tool for tool in package.TOOLS),
                 *(runtime/name for name in ('kiln_web.js', 'kiln_web_bg.wasm', 'runtime.json'))]:
        bounded_hash(member(root, path.relative_to(root).as_posix()))
    if build.artifacts(binaries, runtime) != record['artifacts']:
        raise ValueError('Actual compiled/prepared payload changed')
    for command in record['commands']:
        if not isinstance(command['argv'], list) or any(not isinstance(arg, str) for arg in command['argv']):
            raise ValueError('Invalid command argument record')
        log = member(root, command['log'])
        if (bounded_hash(log) != command['log_sha256']
                or bounded_hash(log.with_suffix('.stderr.log')) != command['stderr_sha256']):
            raise ValueError('Recorded build diagnostics changed')
    return record, result


def input_contract(record, root):
    prefix = str(root)+'/'
    commands = [[('{build}/'+arg[len(prefix):]) if arg.startswith(prefix) else arg
                 for arg in command['argv']] for command in record['commands']]
    return {'source_files': record['source']['files_sha256'],
            'source_archive': record['source']['archive_sha256'],
            'lockfile': record['source']['cargo_lock_sha256'],
            'profile': record['profile'], 'host': record['host'], 'toolchain': record['toolchain'],
            'build_settings': record['build_settings'], 'configuration_hashes': record['configuration_hashes'],
            'commands': commands}


def compare(first, second):
    report = {'passed': False, 'scope': 'Same-host/toolchain native and prepared-browser payload bytes'}
    try:
        left, left_result = verify_build(first)
        right, right_result = verify_build(second)
        a, b = input_contract(left, first), input_contract(right, second)
        differences = [key for key in a if a[key] != b[key]]
        before = {e['path']: e['sha256'] for e in left['artifacts']}
        after = {e['path']: e['sha256'] for e in right['artifacts']}
        changed = [name for name in before if before[name] != after[name]]
        report.update(passed=not differences and not changed, inputs_match=not differences,
                      input_differences=differences, different_artifacts=changed,
                      matching_artifacts=[name for name in before if name not in changed],
                      archive_bytes_equal=left_result['archive_sha256'] == right_result['archive_sha256'],
                      first=left_result, second=right_result)
    except (OSError, ValueError, KeyError, TypeError) as error:
        report['error'] = str(error)
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('first', type=Path)
    parser.add_argument('second', type=Path)
    parser.add_argument('--output', type=Path, required=True, help='New comparison report; never replaced')
    args = parser.parse_args()
    if args.output.exists() or args.output.is_symlink():
        parser.error('Comparison output already exists')
    report = compare(args.first.resolve(), args.second.resolve())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with args.output.open('x') as stream:
        json.dump(report, stream, indent=2, sort_keys=True)
        stream.write('\n')
    print(json.dumps(report))
    raise SystemExit(0 if report['passed'] else 1)


if __name__ == '__main__':
    main()
