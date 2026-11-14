"""Failed GUI replays must not leak an editor or mask the original error."""
import subprocess
import sys
import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
from unittest.mock import patch
import unittest

from task9_editor_games import stop_process, record_failure, floating_title_start, menu_frame_ready


class ProcessCleanupTests(unittest.TestCase):
    def test_child_ignoring_termination_is_reaped(self):
        child = subprocess.Popen([sys.executable, '-u', '-c',
            'import signal,time; signal.signal(signal.SIGTERM, signal.SIG_IGN); '
            'print("ready",flush=True); time.sleep(60)'], stdout=subprocess.PIPE, text=True)
        try:
            self.assertEqual(child.stdout.readline().strip(), 'ready')
            stop_process(child, timeout=0.2)
            self.assertIsNotNone(child.poll())
        finally:
            if child.poll() is None:
                child.kill()
                child.wait(timeout=2)
            child.stdout.close()

    def test_already_exited_child_keeps_its_result(self):
        child = subprocess.Popen([sys.executable, '-c', 'raise SystemExit(7)'])
        child.wait(timeout=5)
        stop_process(child)
        self.assertEqual(child.returncode, 7)


class FailureReportingTests(unittest.TestCase):
    def test_hosted_failure_retains_context_and_cannot_inject_a_second_command(self):
        with tempfile.TemporaryDirectory() as temporary:
            output = io.StringIO()
            message = 'bad 50%\r\n::notice::forged'
            with patch.dict(os.environ, {'GITHUB_ACTIONS': 'true'}), contextlib.redirect_stdout(output):
                record_failure(temporary, 'skybridge', 'authoring', ValueError(message))
            lines = output.getvalue().splitlines()
            self.assertEqual(len(lines), 1)
            self.assertTrue(lines[0].startswith('::error title=Reference-game replay::skybridge authoring: ValueError: '))
            self.assertIn('50%25%0D%0A::notice::forged', lines[0])
            saved = json.loads((Path(temporary)/'failure.json').read_text())
            self.assertEqual(saved, {'game': 'skybridge', 'phase': 'authoring',
                                    'error_type': 'ValueError', 'message': message})

    def test_local_failure_uses_stderr_without_workflow_commands(self):
        with tempfile.TemporaryDirectory() as temporary:
            output, errors = io.StringIO(), io.StringIO()
            with patch.dict(os.environ, {'GITHUB_ACTIONS': 'false'}), contextlib.redirect_stdout(output), contextlib.redirect_stderr(errors):
                record_failure(temporary, 'emberwatch', 'export', RuntimeError('disk full'))
            self.assertEqual(output.getvalue(), '')
            self.assertIn('emberwatch export: RuntimeError: disk full', errors.getvalue())


class FramebufferObservationTests(unittest.TestCase):
    def test_startup_rejects_unpainted_and_solid_windows(self):
        for color in [(0, 0, 0), (255, 255, 255), (35, 38, 44)]:
            self.assertFalse(menu_frame_ready(1440, 900, bytes(color)*1440*900))
        self.assertFalse(menu_frame_ready(1440, 900, b''))

    def test_startup_recognizes_painted_menu_with_srgb_dithering(self):
        frame = bytearray(bytes((43, 43, 43))*1440*900)
        for y in range(26):
            for x in range(200):
                d = (x+y)%3-1
                pixel = (194, 194, 194) if y == 12 and 10 <= x < 100 else (35+d, 38+d, 44+d)
                offset = (y*1440+x)*3
                frame[offset:offset+3] = bytes(pixel)
        self.assertTrue(menu_frame_ready(1440, 900, frame))
        self.assertFalse(menu_frame_ready(900, 1440, frame))

    def test_dithered_srgb_title_is_found_without_matching_canvas_gray(self):
        row=bytes((43,43,43))*33
        row+=b''.join(bytes((35+d,38+d,44+d)) for d in [-1,0,1]*101)
        row+=bytes((43,43,43))*1104
        self.assertEqual(floating_title_start(row),33)

    def test_backgrounds_short_matches_and_wrong_colors_are_not_panels(self):
        for row in [bytes((43,43,43))*1440,bytes((35,38,44))*200+bytes((20,20,20))*1240,bytes((35,44,38))*1440]:
            self.assertIsNone(floating_title_start(row))
