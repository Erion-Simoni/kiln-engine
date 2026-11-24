#!/usr/bin/env python3
"""Optional native widget replay; requires existing Xvfb/xdotool/ImageMagick and tesseract."""
import csv
import io
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib
from task9_editor_games import Editor, REPO, record_failure


def main():
    output=Path(os.environ.get('KILN_SHARED_BUDGET_EVIDENCE',REPO/'target/a04-shared-budget-widgets')).resolve()
    output.mkdir(parents=True,exist_ok=True)
    for stale in [output/'result.json',output/'failure.json']:stale.unlink(missing_ok=True)
    project=Path(tempfile.mkdtemp(prefix='run-',dir=output))/'project'
    shutil.copytree(REPO/'samples/vertical_slice',project)
    manifest=project/'kiln.toml'
    manifest.write_text('script_instruction_budget = 1000\n'+manifest.read_text())
    data=tomllib.loads(manifest.read_text());script=project/data['scenes']['main']['script']
    inspect_frames=os.environ.get('KILN_DEBUG_FUNCTION_FRAMES')=='1'
    active_scopes=os.environ.get('KILN_DEBUG_ACTIVE_SCOPES')=='1'
    expression_watch=os.environ.get('KILN_DEBUG_EXPRESSION_WATCHES')=='1' or active_scopes
    if inspect_frames:
        script.write_text('import "outer.ks";\nouter(9);\n')
        (script.parent/'outer.ks').write_text('import "inner.ks";\nfn outer(value) { return inner(value); }\n')
        (script.parent/'inner.ks').write_text('fn inner(value) {\nlet debug_counter = value;\nwhile true { continue; }\n}\n')
    else:
        script.write_text('let debug_counter = 9;\nwhile true { continue; }\n')
    editor=Editor(REPO/'target/debug',project,output/'editor',':89')

    def observe():
        editor.shot('observation')
        raw=subprocess.check_output(['tesseract',str(output/'editor/observation.png'),'stdout','--psm','11','tsv'],stderr=subprocess.DEVNULL,text=True)
        words=[row for row in csv.DictReader(io.StringIO(raw),delimiter='\t') if row['text'].strip()]
        lines={}
        for row in words:
            key=tuple(row[k] for k in ['page_num','block_num','par_num','line_num'])
            lines.setdefault(key,[]).append(row)
        text='\n'.join(' '.join(row['text'] for row in group) for group in lines.values())
        (output/'ocr-latest.txt').write_text(text+'\n')
        return text,words
    def point(words,token):
        matches=[row for row in words if row['text'].strip()==token]
        if token == 'Pause' and matches:
            matches=[min(matches,key=lambda row:int(row['top']))]
        assert len(matches)==1,f'Expected one painted {token}: {[r["text"] for r in words]}'
        row=matches[0];return int(row['left'])+int(row['width'])//2,int(row['top'])+int(row['height'])//2
    def click_word(token):
        _,words=observe();editor.held_click(*point(words,token))
    def expect(pattern,description):
        return editor.wait(lambda:re.search(pattern,observe()[0],re.I),seconds=20,description=description)

    try:
        editor.start();editor.panel(18,102);baseline=manifest.read_bytes()
        editor.held_click(678,46)
        editor.held_click(80,12);_,words=observe();_,row=point(words,'Debugger');editor.key('Escape')
        editor.panel(80,row);editor.enlarge()
        expect(r'1000\s*/\s*1000','Observe aggregate instruction counter exhausted')
        expect(r'shared\s+gameplay\s+tick\s+limit','Observe source-located shared-budget failure')
        editor.shot('budget_error')
        # Open the real watch controls; target the visible selector placeholder.
        click_word('watches')
        # Field positions follow the observed, normalized enlarged debugger panel.
        # OCR confuses the faint placeholder and repeats 'scene' in help text.
        editor.held_click(180,340)
        editor.key('ctrl+a');editor.command('type','--clearmodifiers','expr:debug_counter*2+1' if expression_watch else 'local:debug_counter')
        editor.held_click(80,366)
        expect(r'[I1l]64\s*\(\s*19\s*\)' if expression_watch else r'[I1l]64\s*\(\s*9\s*\)','Read real halted VM value through live watch')
        if active_scopes:
            editor.held_click(180,340)
            editor.command('type','--clearmodifiers','expr:scope("debug_counter")*2+1')
            editor.held_click(80,366)
            expect(r'scope.*[I1l]64\s*\(\s*19\s*\)','Read the active imported lexical binding')
            click_word('Active')
            expect(r'Current\s+function\s+frame','Enable the active lexical local view')
        editor.shot('halted_vm_watch')
        if inspect_frames:
            click_word('<scene>')
            expect(r'outer\s*@','Read the real imported caller frame')
            expect(r'inner\s*@','Read the real imported current function frame')
            editor.shot('imported_call_frames')
        assert manifest.read_bytes()==baseline
        script.write_text('let debug_counter = 7;\nadd_score(7);\n')
        click_word('Restart')
        expect(r'Pause','Restart corrected saved source into running Play')
        click_word('Pause')
        expect(r'\bResume\b','Pause the restarted game through the live debugger')
        expect(r'[I1l]64\s*\(\s*15\s*\)' if expression_watch else r'[I1l]64\s*\(\s*7\s*\)','Inspect corrected source after successful restart')
        text,_=observe();assert not re.search(r'shared\s+gameplay\s+tick\s+limit',text,re.I)
        if active_scopes:expect(r'Active\s+lexical\s+binding\s+was\s+not\s+found','Completed VM has no active lexical binding')
        editor.shot('restarted_and_paused')
        assert manifest.read_bytes()==baseline
        result=dict(passed=True,actual_budget_failure=True,real_vm_watch=True,saved_source_restart=True,
                    replacement_watch_value=True,manifest_unchanged=True,real_imported_frames=inspect_frames,read_only_expression_watch=expression_watch,active_lexical_scopes=active_scopes,scope='Linux Xvfb/software graphics; OCR-observed widgets')
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception as error:
        record_failure(output,'shared-script-budget','widgets',error)
        if editor.process and editor.process.poll() is None:editor.shot('failure')
        raise
    finally:editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':main()
