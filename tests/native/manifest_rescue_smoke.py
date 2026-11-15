#!/usr/bin/env python3
"""Refuse normal Save after external manifest changes, then rescue the draft via Save As."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, record_failure


def check_case(output, state):
    output.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    source = run/'source'
    shutil.copytree(REPO/'samples/ui2d_editor', source)
    manifest = source/'kiln.toml'
    original = manifest.read_bytes()
    original_project = tomllib.loads(original.decode())
    destination = run/'rescued project'
    editor = Editor(REPO/'target/debug', source, output/'editor', ':86')

    def resources(root):
        return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob('*')
                if p.is_file() and p.name != 'kiln.toml'
                and not any(part.startswith('.') for part in p.relative_to(root).parts)}

    original_resources = resources(source)

    def disk_state():
        return manifest.read_bytes() if manifest.exists() else None

    def error_visible():
        width, height, pixels = editor.frame()
        assert (width, height) == (1440, 900)
        count = 0
        for y in range(120, 400):
            for x in range(20, 700):
                r, g, b = pixels[(y*width+x)*3:(y*width+x)*3+3]
                count += r > 140 and g < 110 and b < 130
        return count > 20

    def rescued():
        return tomllib.loads((destination/'kiln.toml').read_text())

    def pose():
        actor = next(e for e in rescued()['scenes']['main']['entities'] if e['id'] == 'player')
        return next(c for c in actor['components'] if c['type'] == 'Transform')

    try:
        editor.start(); editor.held_click(100, 166); editor.value(1268, 202, '88')
        if state == 'deleted':
            manifest.unlink()
        elif state == 'corrupt':
            manifest.write_bytes(b'[not valid TOML\n')
        else:
            manifest.write_bytes(original.replace(b'name = "Saved UI demo"', b'name = "External edit"', 1))
        external = disk_state()
        editor.save()
        editor.wait(error_visible, description=f'Save visibly refuses {state} source manifest')
        assert disk_state() == external
        assert resources(source) == original_resources
        editor.shot('save_refused')
        editor.key('Escape')
        editor.wait(lambda: not error_visible(), description='Dismiss Save error without discarding scene edits')
        editor.panel(18, 130); editor.text(160, 158, str(destination/'kiln.toml'))
        editor.held_click(106, 184)
        editor.wait(lambda: (destination/'kiln.toml').exists(), description=f'Rescue {state} manifest draft into a new directory')
        assert pose()['x'] == 88.0
        assert rescued()['name'] == original_project['name'], 'Imported the external manifest instead of the authoring draft'
        assert disk_state() == external
        assert resources(source) == original_resources
        assert resources(destination) == original_resources
        assert not list(run.glob('.kiln-save-as-*'))
        editor.project = destination
        editor.held_click(100, 166); editor.value(1340, 202, '72'); editor.save()
        editor.wait(lambda: pose()['y'] == 72.0, description='Subsequent Save targets only the rescued copy')
        assert disk_state() == external
        editor.shot('rescued_copy_editable')
        expected = rescued(); editor.close()
        editor = Editor(REPO/'target/debug', destination, output/'reopened', ':86')
        editor.start(); editor.save(); assert rescued() == expected
        assert disk_state() == external
        assert resources(source) == original_resources
        return dict(state=state, normal_save_refused=True, original_external_state_preserved=True,
                    draft_and_resources_rescued=True, copy_owns_later_edits=True, exact_restart=True)
    except Exception as error:
        record_failure(output, 'manifest-rescue', state, error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


def main():
    output = Path(os.environ.get('KILN_MANIFEST_RESCUE_EVIDENCE', REPO/'target/a02-manifest-rescue')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'result.json').unlink(missing_ok=True)
    results = [check_case(output/state, state) for state in ('deleted', 'corrupt', 'changed')]
    result = dict(passed=True, cases=results, scope='Linux Xvfb/software graphics; no cross-platform/device claim')
    (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))


if __name__ == '__main__':
    main()
