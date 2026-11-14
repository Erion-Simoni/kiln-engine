"""Windows ZIP extraction must reject aliases/links/traversal before writing."""
import importlib.util
from pathlib import Path
import stat
import tempfile
from types import SimpleNamespace
import unittest
from unittest.mock import patch
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
                     'kiln\\escape', 'kiln/./file', 'kiln//file', 'kiln/file.', 'kiln/NUL.txt'):
            with self.subTest(name=name), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                archive = root/'bad.zip'
                with zipfile.ZipFile(archive, 'w') as stream:
                    info = zipfile.ZipInfo(name)
                    info.filename = name  # Preserve malformed raw names even on Windows.
                    stream.writestr(info, b'bad')
                with self.assertRaises(ValueError):
                    smoke.extract(archive, root/'destination')
                self.assertFalse((root/'destination').exists())

    def test_original_names_are_checked_under_windows_zip_normalization(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root/'bad.zip'
            with zipfile.ZipFile(archive, 'w') as stream:
                info = zipfile.ZipInfo('kiln\\escape')
                info.filename = 'kiln\\escape'
                stream.writestr(info, b'bad')
            with patch.object(zipfile.os, 'sep', '\\'):
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


class WindowsProcessTests(unittest.TestCase):
    def test_capture_uses_canonical_uppercase_windows_environment_keys(self):
        def rectangle(_, pointer):
            pointer._obj.right, pointer._obj.bottom = 640, 480
            return True
        def origin(_, pointer):
            pointer._obj.x, pointer._obj.y = 10, 20
            return True
        window = smoke.Windows.__new__(smoke.Windows)
        window.api = SimpleNamespace(SetForegroundWindow=lambda _: True,
                                     GetForegroundWindow=lambda: 123,
                                     GetClientRect=rectangle, ClientToScreen=origin)
        result = SimpleNamespace(returncode=0, stdout='captured', stderr='')
        with patch.object(smoke.subprocess, 'run', return_value=result) as run:
            with patch.object(smoke.subprocess, 'CREATE_NO_WINDOW', 0, create=True):
                window.screenshot(123, Path('capture.png'), {'SYSTEMROOT': 'C:/Windows'})
        command = run.call_args.args[0]
        self.assertEqual(command[0], str(Path('C:/Windows')/'System32/WindowsPowerShell/v1.0/powershell.exe'))
        self.assertEqual(run.call_args.kwargs['env'], {'SYSTEMROOT': 'C:/Windows'})


if __name__ == '__main__':
    unittest.main()


class WindowsWindowReadinessTests(unittest.TestCase):
    def test_visible_helper_window_is_skipped_until_main_client_is_ready(self):
        import ctypes
        from ctypes import wintypes
        dimensions = {1: (1,1), 2: (1440,900), 3: (640,480)}
        def owner(window, pointer):
            ctypes.cast(pointer, ctypes.POINTER(wintypes.DWORD)).contents.value = 42
        def rectangle(window, pointer):
            rect = ctypes.cast(pointer, ctypes.POINTER(wintypes.RECT)).contents
            rect.right, rect.bottom = dimensions[window]
            return True
        observer = smoke.Windows.__new__(smoke.Windows)
        observer.callback = lambda fn: fn
        observer.api = SimpleNamespace(
            GetWindowThreadProcessId=owner, IsWindowVisible=lambda _: True,
            GetClientRect=rectangle,
            EnumWindows=lambda callback,_: [callback(window,0) for window in dimensions])
        self.assertEqual(observer.visible(42), 2)
        dimensions.clear();dimensions[1] = (1,1)
        self.assertIsNone(observer.visible(42))
        self.assertIsNone(observer.visible(99))
