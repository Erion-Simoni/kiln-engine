"""Producer regressions: publication inputs, deterministic archives and notices."""
import importlib.util
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('package_editor', Path(__file__).resolve().parents[2]/'tools/package_editor.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PortablePackageTests(unittest.TestCase):
    def test_normalized_archive_preserves_permissions_and_is_repeatable(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            root = directory/'kiln'
            root.mkdir()
            (root/'run').write_text('tool')
            (root/'run').chmod(0o755)
            (root/'notice').write_text('license')
            first, second = directory/'one.tgz', directory/'two.tgz'
            package.normalized_archive(root, first)
            os.utime(root/'run', (123456789, 123456789))
            package.normalized_archive(root, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with tarfile.open(first) as archive:
                self.assertEqual(archive.getmember('kiln/run').mode, 0o755)
                self.assertTrue(all(m.uid == m.gid == m.mtime == 0 for m in archive))
            (root/'escape').symlink_to(directory)
            with self.assertRaisesRegex(ValueError, 'link'):
                package.normalized_archive(root, directory/'bad.tgz')
            with self.assertRaisesRegex(ValueError, 'Symlink'):
                package.copy_tree(root, directory/'copy')

    def test_mixed_tool_versions_or_disabled_features_fail(self):
        def info(command):
            name = command[0].name
            return {'tool': name, 'schema': 1, 'version': '1' if name != 'kiln-player' else '2',
                    'platform': 'linux', 'arch': 'x86_64', 'debug_assertions': True,
                    'features': ['audio', 'fonts', 'gamepad', 'native-export', 'browser-export', 'bundle-verify']}
        with patch.object(package, 'run_json', side_effect=info):
            with self.assertRaisesRegex(ValueError, 'matching'):
                package.identities(Path('/tools'))
        def release_info(command):
            result = info(command)
            result.update(version='1', debug_assertions=False)
            return result
        with patch.object(package, 'run_json', side_effect=release_info):
            self.assertFalse(package.identities(Path('/tools'))['kiln-editor']['debug_assertions'])
        with patch.object(package, 'run_json', return_value={'tool': 'kiln-editor', 'schema': 1, 'features': []}):
            with self.assertRaisesRegex(ValueError, 'identity'):
                package.identities(Path('/tools'))

    def test_missing_license_reference_and_missing_font_notice_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'Cargo.toml').write_text('[package]\nname="fixture"\n')
            record = {'id': 'fixture', 'name': 'fixture', 'version': '1', 'manifest_path': str(root/'Cargo.toml'),
                      'license': 'LicenseRef-Unknown', 'repository': None, 'source': None}
            metadata = root/'metadata.json'
            def run():
                metadata.write_text(json.dumps({'version': 1, 'resolve': {'nodes': []}, 'packages': [record]}))
                package.package_notices([metadata], root/'notices')
            with self.assertRaisesRegex(ValueError, 'Missing license references'):
                run()
            record['license'] = 'LicenseRef-UFL-1.0'
            with self.assertRaisesRegex(ValueError, 'Ubuntu Font License'):
                run()

    def test_existing_archive_is_preserved_before_running_tools(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = Path(temporary)/'existing.tgz'
            output.write_bytes(b'keep')
            from argparse import Namespace
            with self.assertRaisesRegex(ValueError, 'replace'):
                package.prepare(Namespace(output=output))
            self.assertEqual(output.read_bytes(), b'keep')


if __name__ == '__main__':
    unittest.main()
