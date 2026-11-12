"""Windows ZIP extraction must reject aliases/links/traversal before writing."""
import importlib.util
from pathlib import Path
import stat
import tempfile
import unittest
import zipfile

spec = importlib.util.spec_from_file_location('windows_smoke', Path(__file__).resolve().parents[1]/'windows/portable_editor_smoke.py')
smoke = importlib.util.module_from_spec(spec)
spec.loader.exec_module(smoke)


class WindowsArchiveTests(unittest.TestCase):
    def test_good_zip_extracts_into_new_directory_and_never_merges_existing_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root/'installation.zip'
            with zipfile.ZipFile(archive, 'w') as stream:
                stream.writestr('kiln/kiln-cli.exe', b'actual payload')
            installed = smoke.extract(archive, root/'new')
            self.assertEqual((installed/'kiln-cli.exe').read_bytes(), b'actual payload')
            with self.assertRaises(FileExistsError):
                smoke.extract(archive, root/'new')
            self.assertEqual((installed/'kiln-cli.exe').read_bytes(), b'actual payload')

    def test_invalid_paths_fail_before_creating_destination(self):
        for name in ('../escape', '/absolute', 'C:/escape', 'kiln/../escape',
                     'kiln\\escape', 'kiln/./file', 'kiln//file', 'kiln/file.'):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                archive = root/'bad.zip'
                with zipfile.ZipFile(archive, 'w') as stream:
                    stream.writestr(name, b'bad')
                with self.assertRaises(ValueError):
                    smoke.extract(archive, root/'destination')
                self.assertFalse((root/'destination').exists())

    def test_windows_case_aliases_and_unix_links_are_rejected(self):
        for linked in (False, True):
            with self.subTest(linked=linked), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                archive = root/'bad.zip'
                with zipfile.ZipFile(archive, 'w') as stream:
                    if linked:
                        info = zipfile.ZipInfo('kiln/link')
                        info.create_system = 3
                        info.external_attr = (stat.S_IFLNK | 0o777) << 16
                        stream.writestr(info, '../outside')
                    else:
                        stream.writestr('kiln/Tool.exe', b'one')
                        stream.writestr('kiln/tool.exe', b'two')
                with self.assertRaises(ValueError):
                    smoke.extract(archive, root/'destination')
                self.assertFalse((root/'destination').exists())


if __name__ == '__main__':
    unittest.main()
