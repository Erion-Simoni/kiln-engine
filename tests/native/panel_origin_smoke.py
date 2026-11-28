#!/usr/bin/env python3
"""Target a small animation panel over an enlarged source panel without editing source."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, record_failure


def main():
    output = Path(os.environ.get('KILN_PANEL_ORIGIN_EVIDENCE', REPO/'target/a02-panel-origin')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json', output/'editor/failure.png'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    shutil.copyfile(REPO/'games/skybridge/art/hero.png', project/'hero.png')
    sources = {path.relative_to(project): path.read_bytes() for path in project.rglob('*.ks')}
    editor = Editor(REPO/'target/debug', project, output/'editor', ':87')
    try:
        editor.start(); editor.panel(118, 68); editor.enlarge()
        editor.shot('enlarged_source')
        # Keep the source panel open: it deliberately exposes a longer title
        # segment behind the newly opened animation panel.
        editor.panel(80, 214); editor.shot('foreground_panel_positioned')
        editor.held_click(173, 216)
        editor.text(180, 244, 'origin_probe'); editor.text(225, 272, 'hero.png')
        editor.held_click(59, 300)
        # Source Editor intentionally remains open; choose project Save explicitly
        # so its own Ctrl+S handling cannot consume this verification step.
        editor.panel(18, 102)
        def clip():
            return tomllib.loads((project/'kiln.toml').read_text()).get('animation_clips', {}).get('origin_probe')
        editor.wait(clip, description='Create clip through foreground animation widgets')
        assert clip()['texture'] == 'hero.png' and len(clip()['frames']) == 1
        assert {path.relative_to(project): path.read_bytes() for path in project.rglob('*.ks')} == sources
        editor.shot('clip_created_without_source_edits')
        result = dict(passed=True, overlapping_panels=True, correct_foreground_origin=True,
                      real_clip_created=True, source_files_unchanged=True,
                      scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'panel-origin', 'widgets', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
