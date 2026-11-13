"""Failed GUI replays must not leak an editor or mask the original error."""
import subprocess
import sys
import unittest

from task9_editor_games import stop_process


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
