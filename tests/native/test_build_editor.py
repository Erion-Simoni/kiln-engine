"""Captured builds cannot silently reuse binaries or drop source/payload identity."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
import subprocess
from unittest.mock import patch

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parents[2]/'tools'))
import build_editor as build
import package_editor as package


class CapturedBuildTests(unittest.TestCase):
    def test_source_snapshot_preserves_ignored_lock_untracked_inputs_and_modes(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)/'repo'
            repo.mkdir()
            (repo/'run').write_text('tracked')
            (repo/'run').chmod(0o755)
            (repo/'draft').write_text('untracked')
            (repo/'Cargo.lock').write_text('exact lock')
            source = Path(temporary)/'source'
            def git(_, *args):
                return {'rev-parse': b'abc\n', 'status': b'?? draft',
                        'ls-files': b'run\0draft\0'}[args[0]]
            with patch.object(build, 'git', side_effect=git):
                result = build.snapshot(repo, source)
            self.assertTrue(result['dirty'])
            self.assertEqual(result['revision'], 'abc')
            self.assertEqual((source/'Cargo.lock').read_text(), 'exact lock')
            self.assertEqual({e['path'] for e in result['files']}, {'Cargo.lock', 'run', 'draft'})
            self.assertTrue((source/'run').stat().st_mode & 0o111)
            (repo/'draft').unlink()
            (repo/'draft').symlink_to(repo/'run')
            with patch.object(build, 'git', side_effect=git), self.assertRaisesRegex(ValueError, 'links'):
                build.snapshot(repo, Path(temporary)/'bad')

    def test_build_clears_ambient_overrides_without_dumping_credentials(self):
        ambient = {'CARGO_HOME': '/cache', 'CARGO_PROFILE_RELEASE_OPT_LEVEL': '0',
                   'CARGO_TARGET_DIR': '/stale', 'RUSTFLAGS': '-Copt-level=0',
                   'RUSTC_WRAPPER': '/wrapper', 'CARGO_BUILD_RUSTC_WRAPPER': '/other',
                   'HTTPS_PROXY': 'private-value', 'PKG_CONFIG_PATH': '/sdk'}
        with patch.dict(os.environ, ambient, clear=True):
            env = build.build_environment(Path('/fresh'), 2)
        for key in ('CARGO_PROFILE_RELEASE_OPT_LEVEL', 'RUSTFLAGS', 'RUSTC_WRAPPER', 'CARGO_BUILD_RUSTC_WRAPPER'):
            self.assertNotIn(key, env)
        self.assertEqual(env['CARGO_TARGET_DIR'], '/fresh/target')
        self.assertEqual(env['CARGO_HOME'], '/cache')
        self.assertEqual(env['HTTPS_PROXY'], 'private-value')
        self.assertNotIn('HTTPS_PROXY', build.BUILD_SETTINGS)

    def test_existing_output_is_preserved_before_snapshot_or_build(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)/'existing'
            output.mkdir()
            (output/'keep').write_text('original')
            with self.assertRaises(FileExistsError):
                build.build(argparse.Namespace(output=output, profile='release', jobs=2))
            self.assertEqual((output/'keep').read_text(), 'original')

    def test_command_failure_retains_diagnostics_without_success_record(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            commands = []
            with self.assertRaises(subprocess.CalledProcessError):
                build.run([sys.executable, '-c', 'import sys; print("failed build"); sys.exit(7)'],
                          root, root, os.environ.copy(), commands, 'failed')
            self.assertIn('failed build', (root/'logs/failed.log').read_text())
            self.assertEqual(commands, [])
            log = build.run([sys.executable, '-c',
                             'import sys; print("{}"); print("metadata warning", file=sys.stderr)'],
                            root, root, os.environ.copy(), commands, 'metadata', json_output=True)
            self.assertEqual(json.loads(log.read_text()), {})
            self.assertIn('metadata warning', (root/'logs/metadata.stderr.log').read_text())

    def test_parent_traversal_cannot_put_capture_in_unignored_repo_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            output = repo/'target/../unignored'
            with patch.object(build, 'REPO', repo), self.assertRaisesRegex(ValueError, 'ignored target'):
                build.build(argparse.Namespace(output=output, profile='release', jobs=2))
            self.assertFalse((repo/'unignored').exists())

    def test_record_rejects_changed_payload_source_profile_or_missing_coverage(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, payload = root/'source', root/'payload'
            source.mkdir()
            (source/'Cargo.lock').write_text('lock')
            (source/'main.rs').write_text('source')
            paths = [*package.TOOLS, 'web-runtime/kiln_web.js',
                     'web-runtime/kiln_web_bg.wasm', 'web-runtime/runtime.json']
            for name in paths:
                path = payload/name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(name)
            files = package.file_entries(source)
            record = {'schema': 1, 'kind': 'unsigned-local-build', 'profile': 'release',
                      'artifacts': build.artifacts(payload, payload/'web-runtime'),
                      'source': {'files': files,
                                 'files_sha256': hashlib.sha256(json.dumps(files, sort_keys=True).encode()).hexdigest(),
                                 'cargo_lock_sha256': package.digest(source/'Cargo.lock')}}
            path = root/'record.json'
            package.write_json(path, record)
            def verify():
                with patch.object(package, 'REPO', source):
                    return package.verify_build_record(path, payload, {'debug_assertions': False})
            self.assertEqual(verify()['profile'], 'release')
            (payload/'kiln-player').write_text('changed')
            with self.assertRaisesRegex(ValueError, 'artifact mismatch'):
                verify()
            (payload/'kiln-player').write_text('kiln-player')
            (source/'main.rs').write_text('changed')
            with self.assertRaisesRegex(ValueError, 'source snapshot mismatch'):
                verify()
            (source/'main.rs').write_text('source')
            record['profile'] = 'dev'
            package.write_json(path, record)
            with self.assertRaisesRegex(ValueError, 'profile'):
                verify()
            record['profile'] = 'release'
            record['artifacts'].pop()
            package.write_json(path, record)
            with self.assertRaisesRegex(ValueError, 'exactly'):
                verify()


if __name__ == '__main__':
    unittest.main()
