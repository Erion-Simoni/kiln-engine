"""Producer regressions: publication inputs, deterministic archives and notices."""
import importlib.util
import json
import os
from pathlib import Path
import tarfile
import tempfile
import unittest
import zipfile
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('package_editor', Path(__file__).resolve().parents[2]/'tools/package_editor.py')
package = importlib.util.module_from_spec(spec)
spec.loader.exec_module(package)


class PortablePackageTests(unittest.TestCase):
    def test_zip_is_repeatable_and_preserves_exact_payload_and_root(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            root = directory/'kiln windows'
            (root/'templates').mkdir(parents=True)
            (root/'kiln-editor.exe').write_bytes(b'compiled tool')
            (root/'templates/starter.ron').write_bytes(b'project')
            first, second = directory/'one.zip', directory/'two.zip'
            package.normalized_zip(root, first)
            os.utime(root/'kiln-editor.exe', (123456789, 123456789))
            package.normalized_zip(root, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with zipfile.ZipFile(first) as archive:
                self.assertEqual(archive.read('kiln windows/kiln-editor.exe'), b'compiled tool')
                self.assertEqual(archive.read('kiln windows/templates/starter.ron'), b'project')
                self.assertTrue(all(info.date_time == (1980, 1, 1, 0, 0, 0) for info in archive.infolist()))
                archive.extractall(directory/'extracted')
            self.assertEqual((directory/'extracted/kiln windows/kiln-editor.exe').read_bytes(), b'compiled tool')
            with self.assertRaises(FileExistsError):
                package.normalized_zip(root, first)
            self.assertEqual(first.read_bytes(), second.read_bytes())

    @unittest.skipIf(os.name == 'nt', 'Symlink creation requires platform privileges')
    def test_zip_rejects_links_instead_of_following_outside_files(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            root = directory/'kiln'
            root.mkdir()
            (directory/'private').write_text('outside')
            (root/'escape').symlink_to(directory/'private')
            with self.assertRaisesRegex(ValueError, 'link'):
                package.normalized_zip(root, directory/'bad.zip')

    def test_windows_readme_has_native_commands_and_no_linux_dependency_instructions(self):
        text = package.installation_readme('windows')
        self.assertIn(r'.\kiln-cli.exe verify-editor-bundle', text)
        self.assertNotIn('Ubuntu', text)
        self.assertNotIn('glibc', text)
        self.assertIn('libxkbcommon-x11', package.installation_readme('linux'))

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
            record['license'] = None
            with self.assertRaisesRegex(ValueError, 'Missing project package license'):
                run()

    def test_project_grants_are_copied_verbatim_and_missing_grants_fail(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ('LICENSE-MIT', 'LICENSE-APACHE'):
                (root/name).write_text(f'Complete original {name}\n')
            with patch.object(package, 'REPO', root):
                paths = package.project_notices(root/'notices')
                self.assertEqual(set(paths), {'project/LICENSE-MIT', 'project/LICENSE-APACHE'})
                for name in ('LICENSE-MIT', 'LICENSE-APACHE'):
                    self.assertEqual((root/'notices/project'/name).read_bytes(), (root/name).read_bytes())
                (root/'LICENSE-MIT').unlink()
                with self.assertRaisesRegex(ValueError, 'regular bounded file'):
                    package.project_notices(root/'incomplete')

    def test_cargo_metadata_unicode_survives_windows_legacy_default_encoding(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root/'Cargo.toml').write_text('[package]\nname="fixture"\n', encoding='utf-8')
            record = {'id': 'fixture', 'name': 'fixture', 'version': '1',
                      'manifest_path': str(root/'Cargo.toml'), 'license': 'MIT',
                      'repository': 'https://example.invalid/Łukasz', 'source': None}
            metadata = root/'metadata.json'
            metadata.write_text(json.dumps({'version': 1, 'resolve': {'nodes': []},
                                            'packages': [record]}, ensure_ascii=False), encoding='utf-8')
            original = Path.read_text
            def read(path, *args, **kwargs):
                if path == metadata and kwargs.get('encoding') is None:
                    kwargs['encoding'] = 'cp1252'
                return original(path, *args, **kwargs)
            with patch.object(Path, 'read_text', read):
                package.package_notices([metadata], root/'notices')
            inventory = json.loads((root/'notices/inventory.json').read_text(encoding='utf-8'))
            self.assertEqual(inventory['packages'][0]['repository'], record['repository'])

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
