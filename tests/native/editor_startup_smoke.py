#!/usr/bin/env python3
"""Delay the actual editor's first render; verify default tabs close and reopen."""
import json
import os
from pathlib import Path
import shutil
import signal
import sys
import tempfile
import threading

from task9_editor_games import Editor, REPO, record_failure


def main():
    output = REPO/'target/a01-editor-startup'
    output.mkdir(parents=True, exist_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/vertical_slice', project)
    editor = Editor(REPO/'target/debug', project, output/'editor', ':86')
    command = editor.command
    resume = None

    def delayed_focus(*args):
        nonlocal resume
        if args[0] == 'windowfocus' and resume is None:
            # The X11 window already exists, but startup must not assume that
            # the renderer is ready. Pause only this owned process, then resume
            # after the former fixed startup delay and tab clicks would finish.
            os.kill(editor.process.pid, signal.SIGSTOP)
            resume = threading.Timer(2.5, os.kill, (editor.process.pid, signal.SIGCONT))
            resume.start()
        command(*args)

    editor.command = delayed_focus
    try:
        editor.start()
        assert resume is not None, 'Startup delay was not exercised'
        editor.panel(118, 68)
        editor.shot('floating-source-after-delayed-start')
        result = {'passed': True, 'delayed_render_seconds': 2.5,
                  'source_reopened_as_floating_panel': True}
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'editor-startup', 'delayed-render', error)
        if editor.process and editor.process.poll() is None:
            editor.shot('failure')
        raise
    finally:
        if resume is not None:
            resume.join()
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
