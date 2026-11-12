"""Comparison fails on changed files, unequal binaries or incompatible build inputs."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'tools'))
import build_editor as build
import compare_builds as comparison
import package_editor as package


def fixture(root, archive=b'first archive'):
    source = root/'source'
    (source/'tools').mkdir(parents=True)
    (source/'Cargo.lock').write_text('frozen dependencies')
    (source/'main.rs').write_text('fn main() {}')
    (source/'tools/build_editor.py').write_text('captured driver')
    (source/'main.rs').chmod(0o755)
    package.normalized_archive(source, root/'source.tar.gz')
    files = package.file_entries(source)
    binaries, runtime = root/'target'/build.HOST/'release', root/'web-runtime'
    binaries.mkdir(parents=True)
    runtime.mkdir()
    for tool in package.TOOLS:
        (binaries/tool).write_text('compiled '+tool)
    for name in ('kiln_web.js', 'kiln_web_bg.wasm', 'runtime.json'):
        (runtime/name).write_text('prepared '+name)
    log = root/'logs/native.log'
    log.parent.mkdir()
    log.write_text('execution at '+str(root))
    log.with_suffix('.stderr.log').write_text('')
    record = {'schema': 1, 'kind': 'unsigned-local-build', 'profile': 'release', 'host': build.HOST,
              'source': {'revision': 'test', 'dirty': False, 'files': files,
                         'files_sha256': hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest(),
                         'cargo_lock_sha256': package.digest(source/'Cargo.lock'),
                         'archive_sha256': package.digest(root/'source.tar.gz')},
              'builder_sha256': package.digest(source/'tools/build_editor.py'),
              'toolchain': {'rustc': 'compiler', 'cargo': 'cargo', 'wasm_bindgen': 'bindgen'},
              'build_settings': {}, 'configuration_hashes': [],
              'commands': [{'argv': ['cargo', 'build', str(source)], 'log': 'logs/native.log',
                            'log_sha256': package.digest(log),
                            'stderr_sha256': package.digest(log.with_suffix('.stderr.log'))}],
              'artifacts': build.artifacts(binaries, runtime)}
    (root/'kiln-portable.tar.gz').write_bytes(archive)
    save(root, record)
    return record


def save(root, record):
    package.write_json(root/'build-record.json', record)
    package.write_json(root/'result.json', {'passed': True, 'profile': record['profile'],
                       'build_record_sha256': package.digest(root/'build-record.json'),
                       'archive_sha256': package.digest(root/'kiln-portable.tar.gz'),
                       'source_archive_sha256': record['source']['archive_sha256']})


class BuildComparisonTests(unittest.TestCase):
    def test_actual_equal_payloads_pass_when_execution_archives_differ(self):
        with tempfile.TemporaryDirectory() as temporary:
            left, right = Path(temporary)/'one', Path(temporary)/'two'
            fixture(left)
            fixture(right, b'different provenance archive')
            report = comparison.compare(left, right)
            self.assertTrue(report['passed'], report)
            self.assertEqual(len(report['matching_artifacts']), 6)
            self.assertFalse(report['archive_bytes_equal'])

    def test_consistent_but_unequal_native_or_wasm_payloads_fail(self):
        for name in ('kiln-player', 'kiln_web_bg.wasm'):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                left, right = Path(temporary)/'one', Path(temporary)/'two'
                fixture(left)
                record = fixture(right)
                binaries = right/'target'/build.HOST/'release'
                target = binaries/name if name.startswith('kiln-') else right/'web-runtime'/name
                target.write_text('different compiler output')
                record['artifacts'] = build.artifacts(binaries, right/'web-runtime')
                save(right, record)
                report = comparison.compare(left, right)
                self.assertFalse(report['passed'])
                self.assertTrue(report['inputs_match'])
                self.assertEqual(len(report['different_artifacts']), 1)

    def test_same_payload_with_incompatible_recorded_toolchain_fails(self):
        with tempfile.TemporaryDirectory() as temporary:
            left, right = Path(temporary)/'one', Path(temporary)/'two'
            fixture(left)
            record = fixture(right)
            record['toolchain']['rustc'] = 'other compiler'
            save(right, record)
            report = comparison.compare(left, right)
            self.assertFalse(report['passed'])
            self.assertIn('toolchain', report['input_differences'])
            self.assertEqual(report['different_artifacts'], [])

    def test_tampered_or_missing_inputs_fail_instead_of_trusting_saved_hashes(self):
        for relative in ('target/'+build.HOST+'/release/kiln-cli', 'source/main.rs',
                         'source.tar.gz', 'kiln-portable.tar.gz', 'logs/native.log', 'build-record.json'):
            with self.subTest(path=relative), tempfile.TemporaryDirectory() as temporary:
                left, right = Path(temporary)/'one', Path(temporary)/'two'
                fixture(left)
                fixture(right)
                (right/relative).unlink()
                report = comparison.compare(left, right)
                self.assertFalse(report['passed'])
                self.assertIn('error', report)

    def test_replay_preserves_source_modes_and_rejects_mutated_sources(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous = root/'previous'
            record = fixture(previous)
            captured, prior = build.replay_source(previous, root/'copy', 'release')
            self.assertEqual(prior, record)
            self.assertEqual(captured['files'], package.file_entries(root/'copy'))
            self.assertTrue((root/'copy/main.rs').stat().st_mode & 0o111)
            (previous/'source/main.rs').write_text('mutated')
            with self.assertRaisesRegex(ValueError, 'changed'):
                build.replay_source(previous, root/'bad', 'release')

    def test_same_content_symlinks_do_not_count_as_unchanged_regular_inputs(self):
        for kind in ('source', 'payload'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as temporary:
                left, right = Path(temporary)/'one', Path(temporary)/'two'
                fixture(left)
                fixture(right)
                path = right/'source/main.rs' if kind == 'source' else right/'target'/build.HOST/'release'
                saved = path.with_name(path.name+'-original')
                path.rename(saved)
                path.symlink_to(saved)
                report = comparison.compare(left, right)
                self.assertFalse(report['passed'])
                self.assertIn('error', report)

    def test_replay_rejects_incompatible_profile_and_never_executes_recorded_commands(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            previous = root/'previous'
            record = fixture(previous)
            marker = root/'must-not-exist'
            record['commands'][0]['argv'] = [sys.executable, '-c', f'open({str(marker)!r}, "w").close()']
            save(previous, record)
            build.replay_source(previous, root/'copy', 'release')
            self.assertFalse(marker.exists())
            with self.assertRaisesRegex(ValueError, 'Incompatible'):
                build.replay_source(previous, root/'bad', 'dev')

    def test_comparison_command_preserves_an_existing_report(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            report = root/'report.json'
            report.write_text('previous evidence')
            result = subprocess.run([sys.executable, str(Path(comparison.__file__)), str(root), str(root),
                                     '--output', str(report)], capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(report.read_text(), 'previous evidence')

    def test_comparison_cli_writes_a_failed_report_and_returns_failure_for_unequal_builds(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            left, right = root/'one', root/'two'
            fixture(left)
            record = fixture(right)
            binaries = right/'target'/build.HOST/'release'
            (binaries/'kiln-cli').write_text('unequal real payload')
            record['artifacts'] = build.artifacts(binaries, right/'web-runtime')
            save(right, record)
            output = root/'comparison.json'
            result = subprocess.run([sys.executable, str(Path(comparison.__file__)), str(left), str(right),
                                     '--output', str(output)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 1)
            report = json.loads(output.read_text())
            self.assertFalse(report['passed'])
            self.assertEqual(report['different_artifacts'], ['kiln-cli'])


if __name__ == '__main__':
    unittest.main()
