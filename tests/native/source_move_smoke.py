#!/usr/bin/env python3
"""Move source modules through asset widgets, preserving imports and open buffers."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import tomllib

from task9_editor_games import Editor, REPO, project_error_visible, record_failure


def main():
    output = Path(os.environ.get('KILN_SOURCE_MOVE_EVIDENCE', REPO/'target/a03-source-move')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    for stale in (output/'result.json', output/'failure.json'):
        stale.unlink(missing_ok=True)
    project = Path(tempfile.mkdtemp(prefix='run-', dir=output))/'project'
    shutil.copytree(REPO/'samples/ui2d_editor', project)
    (project/'scripts/lib').mkdir()
    main_source = 'import "lib/helper.ks";\nlet answer_value = answer();\n'
    helper_source = 'import "../base.ks";\nfn answer() { return base(); }\n'
    base_source = 'fn base() { return 42; }\n'
    for path, source in [('main.ks', main_source), ('lib/helper.ks', helper_source), ('base.ks', base_source)]:
        (project/'scripts'/path).write_text(source)
    tools = Path(os.environ.get('KILN_SOURCE_MOVE_TOOLS', REPO/'target/debug')).resolve()
    editor = Editor(tools, project, output/'editor', ':86')
    manifest = project/'kiln.toml'

    def saved():
        return tomllib.loads(manifest.read_text())

    def source_panel():
        editor.panel(118, 68); editor.large = False; editor.enlarge()

    def close_source():
        editor.held_click(125, 132)

    def assets():
        editor.panel(80, 96); editor.large = False; editor.enlarge()
        editor.held_click(110, 268)

    def move_card(x, destination):
        editor.held_click(x, 220); editor.held_click(960, 166)
        editor.text(980, 177, destination); editor.held_click(872, 203)

    try:
        editor.start(); source_panel()
        editor.text(225, 166, 'scripts/lib/helper.ks'); editor.held_click(392, 166)
        editor.shot('opened_module'); close_source()
        editor.held_click(65, 688)  # Close the initial docked Assets tab.
        assets(); editor.held_click(1055, 166); editor.panel(18, 102)
        editor.wait(lambda: 'scripts/lib/helper.ks' in saved().get('asset_ids', {}),
                    description='Import source stable identities')
        helper_id = saved()['asset_ids']['scripts/lib/helper.ks']
        main_id = saved()['asset_ids']['scripts/main.ks']
        move_card(340, 'scripts/renamed.ks')
        editor.wait(lambda: (project/'scripts/renamed.ks').exists() and not (project/'scripts/lib/helper.ks').exists(),
                    description='Move selected source through asset dialog')
        expected_helper = helper_source.replace('../base.ks', 'base.ks')
        assert (project/'scripts/renamed.ks').read_text() == expected_helper
        assert (project/'scripts/main.ks').read_text() == main_source.replace('lib/helper.ks', 'renamed.ks')
        assert (project/'scripts/base.ks').read_text() == base_source
        editor.panel(18, 102)
        assert saved()['asset_ids']['scripts/renamed.ks'] == helper_id
        assert 'scripts/lib/helper.ks' not in saved()['asset_ids']
        editor.shot('module_and_importers_moved')
        # main.ks is now the second script card. Its scene reference must follow.
        move_card(340, 'scripts/start.ks'); editor.panel(18, 102)
        editor.wait(lambda: saved()['scenes']['main']['script'] == 'scripts/start.ks',
                    description='Save scene entrypoint after source move')
        assert saved()['asset_ids']['scripts/start.ks'] == main_id
        assert 'scripts/main.ks' not in saved()['asset_ids']
        assert not (project/'scripts/main.ks').exists()
        assert (project/'scripts/start.ks').read_text() == main_source.replace('lib/helper.ks', 'renamed.ks')
        editor.held_click(90, 132); source_panel()
        editor.held_click(65, 194); editor.held_click(208, 194)
        assert not (project/'scripts/lib/helper.ks').exists(), 'Open source buffer retained its old path'
        assert (project/'scripts/renamed.ks').read_text() == expected_helper
        editor.shot('open_buffer_retargeted')
        # Dirty source must block another source move without writing its draft.
        draft = output/'dirty.ks'; draft.write_text(expected_helper+'// unsaved draft\n')
        editor.paste(draft); close_source(); assets()
        # Sorted cards are base.ks, renamed.ks, start.ks, ui.ks.
        before = {p.relative_to(project): p.read_bytes() for p in project.rglob('*.ks')}
        move_card(340, 'scripts/blocked.ks')
        editor.wait(lambda: project_error_visible(*editor.frame()), description='Refuse source move while buffer is dirty')
        assert {p.relative_to(project): p.read_bytes() for p in project.rglob('*.ks')} == before
        assert not (project/'scripts/blocked.ks').exists()
        editor.shot('dirty_source_move_refused'); editor.key('Escape')
        editor.held_click(90, 132); source_panel(); editor.held_click(144, 194); close_source()
        editor.panel(18, 102); expected = saved(); editor.close()
        editor = Editor(tools, project, output/'reopened', ':86')
        editor.start(); editor.panel(18, 102)
        assert saved() == expected
        assert {p.relative_to(project): p.read_bytes() for p in project.rglob('*.ks')} == before
        editor.shot('reopened')
        result = dict(passed=True, real_source_selection_move=True, incoming_and_outgoing_imports=True,
                      scene_entrypoint_rewritten=True, stable_source_identities=True, open_buffer_retargeted=True, dirty_buffer_move_refused=True,
                      exact_source_project_restart=True, scope='Linux Xvfb/software graphics')
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n'); print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'source-move', 'widgets', error)
        if editor.process and editor.process.poll() is None: editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
