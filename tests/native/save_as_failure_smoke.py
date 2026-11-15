#!/usr/bin/env python3
"""Verify visible Save As failure, cancel/retry, safe copying and destination ownership."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, record_failure


def main():
    output = Path(os.environ.get('KILN_SAVE_AS_EVIDENCE', REPO/'target/a02-save-as-failure')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json', output/'editor/failure.png'):
        stale.unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    source = run/'source'
    shutil.copytree(REPO/'samples/ui2d_editor', source)
    original = (source/'kiln.toml').read_bytes()
    blocked = run/'occupied destination'
    blocked.mkdir()
    foreign = original.replace(b'name = "Saved UI demo"', b'name = "Foreign project"', 1)
    (blocked/'kiln.toml').write_bytes(foreign)
    (blocked/'keep.txt').write_bytes(b'preserve this destination\n')
    destination = run/'copied project'
    editor = Editor(REPO/'target/debug', source, output/'editor', ':86')

    def resources(root):
        return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob('*')
                if p.is_file() and p.name != 'kiln.toml'
                and not any(part.startswith('.') for part in p.relative_to(root).parts)}

    original_resources = resources(source)

    def preserved():
        assert (source/'kiln.toml').read_bytes() == original
        assert resources(source) == original_resources
        assert (blocked/'kiln.toml').read_bytes() == foreign
        assert (blocked/'keep.txt').read_bytes() == b'preserve this destination\n'

    def red_error():
        width, height, pixels = editor.frame()
        assert (width, height) == (1440, 900)
        mask = bytearray()
        for y in range(200, 400):
            for x in range(20, 700):
                r, g, b = pixels[(y*width+x)*3:(y*width+x)*3+3]
                mask.append(r > 140 and g < 110 and b < 130)
        return bytes(mask) if sum(mask) > 20 else None

    def open_blocked():
        editor.panel(18, 130)
        editor.text(160, 158, str(blocked/'kiln.toml')); editor.held_click(106, 184)
        return editor.wait(red_error, description='Show Save As error inline while retaining the form')

    def saved_pose():
        project = tomllib.loads((destination/'kiln.toml').read_text())
        actor = next(e for e in project['scenes']['main']['entities'] if e['id'] == 'player')
        return next(c for c in actor['components'] if c['type'] == 'Transform')

    try:
        editor.start(); editor.held_click(100, 166); editor.value(1268, 202, '88')
        open_blocked(); preserved(); assert not destination.exists()
        editor.shot('occupied_destination_error')
        editor.key('Escape')
        editor.wait(lambda: red_error() is None, description='Escape dismisses failed Save As without losing the draft')
        preserved()
        old_error = open_blocked()
        # Fail during staged copying, after initial destination checks succeed.
        link = source/'bad-link.txt'
        link.symlink_to(blocked/'keep.txt')
        editor.text(160, 158, str(destination/'kiln.toml')); editor.held_click(106, 184)
        editor.wait(lambda: (mask := red_error()) is not None and mask != old_error,
                    description='Render the new staged-copy failure in the same form')
        assert link.is_symlink()
        assert not destination.exists(), 'Failed copy published a partial project'
        assert not list(run.glob('.kiln-save-as-*')), 'Failed copy leaked private staging'
        editor.shot('copy_failure_keeps_form')
        link.unlink(); preserved()
        editor.held_click(106, 184)
        editor.wait(lambda: (destination/'kiln.toml').exists(), description='Retry Save As without reopening the form')
        assert saved_pose()['x'] == 88.0, 'Cancel or failed Save As discarded scene edits'
        assert resources(destination) == original_resources
        preserved()
        editor.project = destination
        # Successful Save As must rebuild the hierarchy for normal continued editing.
        editor.held_click(100, 166); editor.value(1340, 202, '72'); editor.save()
        editor.wait(lambda: saved_pose()['y'] == 72.0, description='Edit and save through the rebuilt copied-project hierarchy')
        preserved()
        editor.shot('copied_project_editable')
        # Save As must not implicitly write an unsaved source buffer first.
        script = destination/'scripts/main.ks'
        script_before = script.read_bytes()
        draft = run/'unsaved.ks'
        draft.write_bytes(script_before+b'\n// SAVE_AS_UNSAVED_DRAFT\n')
        editor.panel(118, 68); editor.enlarge()
        editor.text(225, 166, 'scripts/main.ks'); editor.held_click(392, 166); editor.paste(draft)
        editor.panel(18, 130)
        forbidden = run/'unsaved-source-copy'
        editor.text(160, 158, str(forbidden/'kiln.toml')); editor.held_click(106, 184)
        editor.wait(red_error, description='Visible Save/Revert requirement for an unsaved script')
        editor.shot('dirty_source_blocked_before_write')
        assert script.read_bytes() == script_before, 'Save As implicitly saved the source buffer'
        assert not forbidden.exists(), 'Save As published a copy with unsaved source'
        preserved()
        editor.key('Escape'); editor.held_click(65, 194)
        editor.wait(lambda: script.read_bytes() == draft.read_bytes(),
                    description='Explicit Save Script retains and saves the cancelled draft in the copy')
        editor.held_click(125, 132); editor.save()
        preserved()
        expected = tomllib.loads((destination/'kiln.toml').read_text())
        editor.close()
        editor = Editor(REPO/'target/debug', destination, output/'reopened', ':86')
        editor.start(); editor.save()
        assert tomllib.loads((destination/'kiln.toml').read_text()) == expected
        assert script.read_bytes() == draft.read_bytes()
        preserved()
        result = dict(passed=True, inline_error_and_retry=True, escape_preserves_draft=True,
                      occupied_destination_unchanged=True, failed_copy_not_published=True,
                      staging_cleaned=True, original_manifest_and_assets_unchanged=True,
                      destination_owns_subsequent_edits=True, copied_hierarchy_editable=True,
                      exact_restart=True, dirty_source_requires_explicit_save=True,
                      scope='Linux Xvfb/software graphics; scene-copy failures and guarded dirty source')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'save-as-failure', 'widgets/copy', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
