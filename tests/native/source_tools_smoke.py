#!/usr/bin/env python3
"""Exercise source formatting, saved-project navigation and call insertion via widgets."""
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
from task9_editor_games import Editor, REPO, record_failure


def main():
    output = Path(os.environ.get('KILN_SOURCE_TOOLS_EVIDENCE', REPO/'target/a04-source-tools')).resolve()
    output.mkdir(parents=True, exist_ok=True)
    run = Path(tempfile.mkdtemp(prefix='run-', dir=output))
    project = run/'project'
    shutil.copytree(REPO/'samples/vertical_slice', project)
    editor = Editor(REPO/'target/debug', project, output/'editor', ':85')
    helper = 'fn imported_helper() {\nreturn 43;\n}\n'
    main_code = 'fn helper() {\nreturn 42;\n}\nif true {\nlet value=helper();\n}\n'
    main_formatted = main_code.replace('return 42;', '    return 42;').replace('let value=', '    let value=')
    helper_file = project/'scripts/helper.ks'
    try:
        editor.start(); editor.panel(118, 68); editor.enlarge()
        draft = run/'draft.ks'
        draft.write_text(helper); editor.source('scripts/helper.ks', draft)
        draft.write_text(main_code); editor.source('scripts/main.ks', draft)
        editor.held_click(405, 194); editor.held_click(65, 194)
        editor.wait(lambda: (project/'scripts/main.ks').read_text() == main_formatted,
                    description='Format indentation and save main source')
        editor.shot('formatted')
        editor.held_click(510, 194); editor.text(950, 175, 'fn imported_helper(')
        editor.held_click(1170, 175); editor.shot('search_results')
        editor.held_click(1010, 207); editor.held_click(405, 194); editor.held_click(65, 194)
        editor.wait(lambda: helper_file.read_text() == helper.replace('return 43;', '    return 43;'),
                    description='Navigate to saved project search result, format and save the correct file')
        assert (project/'scripts/main.ks').read_text() == main_formatted
        editor.shot('navigation_saved_correct_file')
        before_completion = helper_file.read_bytes()
        draft.write_text('get_el'); editor.paste(draft)
        assert helper_file.read_bytes() == before_completion, 'Partial identifier was saved before completion'
        editor.held_click(330, 194); editor.text(960, 161, 'get_elapsed')
        editor.shot('completion_reference'); editor.held_click(1007, 207); editor.held_click(65, 194)
        editor.wait(lambda: helper_file.read_text() == 'get_elapsed()',
                    description='Shared registry completion replaces prefix and saves a valid call')
        editor.shot('inserted_call')
        editor.save(); expected = (project/'kiln.toml').read_bytes(); editor.close()
        editor = Editor(REPO/'target/debug', project, output/'reopened', ':85'); editor.start()
        assert (project/'kiln.toml').read_bytes() == expected
        assert helper_file.read_text() == 'get_elapsed()'
        assert (project/'scripts/main.ks').read_text() == main_formatted
        editor.shot('reopened')
        result = dict(passed=True, formatting=True, search_navigates_correct_file=True,
                      completion_replaces_identifier=True, unsaved_partial_preserved=True, restart_equal=True)
        (output/'result.json').write_text(json.dumps(result, indent=2)+'\n')
        print(json.dumps(result))
    except Exception as error:
        record_failure(output, 'source-tools', 'widgets', error)
        if editor.process and editor.process.poll() is None:
            editor.shot('failure')
        raise
    finally:
        editor.close(failed=sys.exc_info()[0] is not None)


if __name__ == '__main__':
    main()
