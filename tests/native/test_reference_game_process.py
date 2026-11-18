"""Failed GUI replays must not leak an editor or mask the original error."""
import subprocess
import sys
import contextlib
import io
import json
import os
from pathlib import Path
import tempfile
from unittest.mock import Mock, patch
import unittest

from task9_editor_games import Editor, stop_process, record_failure, floating_title_bar, menu_frame_ready, project_error_visible


def panel_frame(panels=((15, 373),)):
    """Render the observed dock-header geometry, including arrow/close glyphs."""
    width, height = 1440, 900
    pixels = bytearray(bytes((43, 43, 43))*width*height)
    def pixel(x, y, color):
        offset=(y*width+x)*3; pixels[offset:offset+3]=bytes(color)
    for left, right in panels:
        for y in range(82, 120):
            offset=(y*width+left)*3
            pixels[offset:offset+(right-left)*3]=bytes((35, 38, 44))*(right-left)
            pixel(left,y,(20,20,20));pixel(right-1,y,(20,20,20))
        arrow=left+15
        for y, offsets in [(99,range(-2,4)),(100,range(-1,3)),(101,range(-1,3)),(102,range(0,2)),(103,range(0,2))]:
            for x in offsets:pixel(arrow+x,y,(180,180,180))
        for x in [-1,2]:pixel(arrow+x,102,(59,60,63))
        close=right-16
        for y,distance in [(99,3),(100,2),(101,1),(102,0),(103,1),(104,2),(105,3)]:
            for x in {-distance,distance}:pixel(close+x,y,(180,180,180))
        for x in [-1,1]:pixel(close+x,102,(59,60,63))
    return width,height,pixels


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
        width,height,pixels=panel_frame()
        for x in range(33,356):
            d=x%3-1;offset=(102*width+x)*3
            pixels[offset:offset+3]=bytes((35+d,38+d,44+d))
        self.assertEqual(floating_title_bar(width,height,pixels),(33,356))

    def test_blank_short_matches_and_wrong_colors_are_not_panels(self):
        self.assertIsNone(floating_title_bar(*panel_frame(())))
        width,height,pixels=panel_frame()
        for color in [(43,43,43),(35,44,38)]:
            copy=bytearray(pixels)
            copy[102*width*3:103*width*3]=bytes(color)*width
            self.assertIsNone(floating_title_bar(width,height,copy))
        self.assertIsNone(floating_title_bar(1440,900,b''))

    def test_partial_background_header_cannot_hide_foreground_on_either_side(self):
        self.assertEqual(floating_title_bar(*panel_frame(((15,1101),(839,1197)))),(857,1180))
        self.assertEqual(floating_title_bar(*panel_frame(((15,1101),(15,373)))),(33,356))

    def test_hovered_header_triangle_keeps_the_same_origin(self):
        width,height,pixels=panel_frame()
        for x in (29,32):
            offset=(102*width+x)*3;pixels[offset:offset+3]=bytes((142,142,142))
        self.assertEqual(floating_title_bar(width,height,pixels),(33,356))

    def test_two_disjoint_headers_are_ambiguous(self):
        self.assertIsNone(floating_title_bar(*panel_frame(((15,373),(839,1197)))))


class FloatingPanelSettlementTests(unittest.TestCase):
    @staticmethod
    def frame(start):
        return panel_frame(((start-18,start+340),))

    def editor(self, starts):
        editor = Editor.__new__(Editor)
        editor.process = None
        editor.held_click = Mock()
        editor.command = Mock()
        editor.window = 'test-window'
        editor.actions = []
        editor.frame = Mock(side_effect=[self.frame(start) for start in starts])
        return editor

    def test_transient_title_segment_cannot_become_a_drag_origin(self):
        # The first painted frame can expose a different title segment while
        # the reopened dock window adopts the enlarged previous panel size.
        editor = self.editor([354, 33, 33, 33])
        editor.panel(80, 214)
        editor.command.assert_not_called()
        self.assertEqual(editor.frame.call_count, 4)

    def test_width_must_settle_even_when_the_left_origin_is_unchanged(self):
        editor=self.editor([])
        editor.frame=Mock(side_effect=[panel_frame(((15,373),))]+[panel_frame(((15,435),))]*3)
        editor.panel(80,214)
        self.assertEqual(editor.frame.call_count,4)

    def test_overlapping_panel_uses_its_complete_foreground_header(self):
        editor = self.editor([])
        editor.frame = Mock(side_effect=[panel_frame(((15,1101),(839,1197)))]*3 +
                                       [panel_frame(((15,1101),(15,373)))]*3)
        editor.panel(80,214)
        editor.command.assert_any_call('mousemove','--window','test-window',887,102)
        self.assertEqual(editor.command.call_args_list[1].args,('mousedown','1'))

    def test_shifted_panel_must_render_at_destination_before_widget_clicks(self):
        editor = self.editor([120, 120, 120, 120, 33, 33, 33])
        editor.panel(80, 214)
        self.assertEqual(editor.command.call_args_list[0].args,
                         ('mousemove', '--window', 'test-window', 150, 102))
        self.assertEqual(editor.command.call_args_list[1].args, ('mousedown', '1'))
        self.assertEqual(editor.frame.call_count, 7)


class ProjectErrorObservationTests(unittest.TestCase):
    @staticmethod
    def frame(rectangles=()):
        width, height = 1440, 900
        pixels = bytearray(bytes((26, 26, 26))*width*height)
        for x, y, w, h, color in rectangles:
            for row in range(y, y+h):
                offset = (row*width+x)*3
                pixels[offset:offset+w*3] = bytes(color)*w
        return width, height, pixels

    def test_centered_error_is_visible_with_srgb_color_variation(self):
        for color in [(255, 0, 0), (242, 78, 78), (244, 80, 80)]:
            self.assertTrue(project_error_visible(*self.frame([(555, 440, 80, 2, color)])))

    def test_old_error_location_inspector_and_gizmo_do_not_count(self):
        frame = self.frame([(50, 200, 300, 3, (255, 0, 0)),
                            (1160, 450, 200, 3, (255, 0, 0)),
                            (370, 580, 43, 2, (255, 0, 0))])
        self.assertFalse(project_error_visible(*frame))

    def test_unpainted_incomplete_or_wrong_size_frame_is_rejected(self):
        self.assertFalse(project_error_visible(*self.frame()))
        self.assertFalse(project_error_visible(1440, 900, b''))
        self.assertFalse(project_error_visible(900, 1440, self.frame()[2]))

    def test_sparse_pixels_are_not_a_visible_error(self):
        self.assertFalse(project_error_visible(*self.frame([(555, 440, 60, 2, (255, 0, 0))])))
        self.assertTrue(project_error_visible(*self.frame([(555, 440, 61, 2, (255, 0, 0))])))
