#!/usr/bin/env python3
"""Force-close/restart the real editor and recover/discard unsaved source text."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile

from task9_editor_games import Editor, REPO


def main():
    output = Path(os.environ.get('KILN_SCRIPT_RECOVERY_EVIDENCE_DIR',
                                  REPO/'target/a02-script-recovery')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    (output/'result.json').unlink(missing_ok=True)
    run = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    project = run/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    source = project/'scripts/main.ks'
    original = source.read_bytes()
    sidecar = project/'.kiln.toml.recovery.json'
    editor = Editor(REPO/'target/debug', project, output/'editor', ':83')

    def edit_and_autosave(text, name):
        draft = run/(name+'.ks')
        draft.write_bytes(text)
        editor.panel(118, 68)
        editor.enlarge()
        editor.text(225, 166, 'scripts/main.ks')
        editor.click(392, 166)
        editor.paste(draft)
        editor.shot(name+'_unsaved')
        editor.wait(lambda: sidecar.exists() and
                    json.loads(sidecar.read_text()).get('script', {}).get('source') == text.decode(),
                    seconds=45, description='Autosave actual unsaved source')

    def crash_and_reopen(name):
        shutil.copyfile(output/'editor/editor.log', output/(name+'_before_crash.log'))
        editor.process.kill()
        editor.process.wait(timeout=10)
        editor.large = False
        editor.start()
        editor.shot(name+'_prompt')
        before = sidecar.read_bytes()
        editor.key('Escape')
        assert sidecar.read_bytes() == before, 'Escape discarded recovery data'

    try:
        editor.start()
        recovered = original+b'\n// RECOVERY_RESTORE_MARKER\n'
        edit_and_autosave(recovered, 'restore')
        assert source.read_bytes() == original, 'Autosave wrote the source file'
        crash_and_reopen('restore')
        editor.click(590, 499)  # Restore in the visible centered recovery dialog.
        editor.panel(118, 68)
        editor.enlarge()
        editor.shot('restored_buffer')
        assert source.read_bytes() == original, 'Restore wrote source before Save'
        editor.save()
        editor.wait(lambda: source.read_bytes() == recovered and not sidecar.exists(),
                    description='Save recovered buffer and clear recovery')
        editor.shot('restored_saved')

        discarded = recovered+b'// RECOVERY_DISCARD_MARKER\n'
        edit_and_autosave(discarded, 'discard')
        assert source.read_bytes() == recovered
        crash_and_reopen('discard')
        editor.click(685, 499)  # Explicit Discard Recovery, after Escape preserved it.
        editor.wait(lambda: not sidecar.exists(), description='Explicit discard recovery')
        assert source.read_bytes() == recovered, 'Discard overwrote saved source'
        editor.shot('discarded')
        edit_and_autosave(recovered+b'// EXIT_DISCARD_MARKER\n', 'exit_discard')
        editor.panel(18, 262)
        editor.shot('exit_discard_prompt')
        editor.key('Escape')  # Cancel normal exit; keep both the draft and recovery.
        assert editor.process.poll() is None and sidecar.exists()
        editor.panel(18, 262)
        editor.click(658, 489)  # Discard in the normal unsaved-project exit prompt.
        editor.wait(lambda: editor.process.poll() is not None, description='Exit after discard')
        assert not sidecar.exists(), 'Explicit exit discard left a recoverable draft'
        assert source.read_bytes() == recovered

        # Synthetic invalid sidecar exercises error handling, not normal authoring.
        editor.process.kill()
        editor.process.wait(timeout=10)
        unreadable = b'{"recovery_version":999,"project":{},"script":null}'
        sidecar.write_bytes(unreadable)
        editor.large = False
        editor.start()
        editor.key('Escape')
        editor.shot('unsupported_recovery_preserved')
        assert sidecar.read_bytes() == unreadable, 'Unreadable recovery was erased automatically'
        assert source.read_bytes() == recovered
        editor.process.kill()
        editor.process.wait(timeout=10)
        result = {'passed': True, 'real_autosave_interval_seconds': 30,
                  'force_kill_restart_restore_save': True, 'source_unchanged_until_save': True,
                  'escape_preserves_recovery': True, 'unsupported_recovery_preserved': True,
                  'exit_cancel_preserves_draft': True, 'exit_discard_clears_recovery': True, 'explicit_discard_preserves_saved_source': True,
                  'scope': 'Isolated Xvfb/software graphics; no physical-device claim'}
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
