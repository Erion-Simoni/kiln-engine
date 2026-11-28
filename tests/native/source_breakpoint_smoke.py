#!/usr/bin/env python3
"""Observe real imported source stops, statement continuation and Restart through widgets."""
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
    output=Path(os.environ.get('KILN_SOURCE_DEBUG_EVIDENCE',REPO/'target/a04-source-widgets')).resolve()
    output.mkdir(parents=True,exist_ok=True)
    project=Path(tempfile.mkdtemp(prefix='run-',dir=output))/'project'
    shutil.copytree(REPO/'samples/vertical_slice',project)
    manifest=project/'kiln.toml';baseline=manifest.read_bytes()
    script=project/tomllib.loads(manifest.read_text())['scenes']['main']['script']
    script.write_text('import "logic.ks";\nlet result=compute(3);\nstate_set("result",result);\nadd_score(1);\n')
    (script.parent/'logic.ks').write_text('fn compute(value){\nlet increment=value+1;\nreturn increment*2;\n}\n')
    editor=Editor(REPO/'target/debug',project,output/'editor',':89')
    def observe():
        editor.shot('observation')
        raw=subprocess.check_output(['tesseract',str(output/'editor/observation.png'),'stdout','--psm','11','tsv'],stderr=subprocess.DEVNULL,text=True)
        words=[r for r in csv.DictReader(io.StringIO(raw),delimiter='\t') if r['text'].strip()]
        text=' '.join(r['text'] for r in words);(output/'ocr-latest.txt').write_text(text+'\n')
        return text,words
    def point(token):
        matches=[r for r in observe()[1] if r['text'].strip().strip("‘’'\"`")==token]
        if token=='Pause' and matches:matches=[min(matches,key=lambda r:int(r['top']))]
        assert len(matches)==1,f'{token}: {[(r["text"],r["top"]) for r in matches]}'
        r=matches[0];return int(r['left'])+int(r['width'])//2,int(r['top'])+int(r['height'])//2
    def click(token):editor.held_click(*point(token))
    def expect(pattern,description):editor.wait(lambda:re.search(pattern,observe()[0],re.I),seconds=20,description=description)
    def add_watch(selector):
        x,y=point('Add');editor.value(180,y-24,selector);editor.held_click(x,y)
    try:
        editor.start();editor.panel(18,102);baseline=manifest.read_bytes();editor.held_click(678,46)
        editor.held_click(80,12);_,row=point('Debugger');editor.key('Escape');editor.panel(80,row);editor.enlarge()
        click('Pause');click('breakpoints')
        x,y=point('Add');editor.value(180,y-48,'scripts/logic.ks');editor.value(180,y-24,'3');editor.held_click(x,y)
        expect(r'Source\s+breakpoint\s+added','Install an executable imported line')
        click('breakpoints');click('Restart')
        expect(r'Source\s+stop:.*logic.*3','Restart retains and hits the imported source breakpoint')
        click('watches');add_watch('expr:scope("increment")')
        expect(r'[I1l]64\s*\(\s*4\s*\)','Read the active imported local before Return')
        editor.shot('imported_source_stop')
        click('Statement')
        expect(r'Source\s+stop:.*main.*3','Step through the real Return into the caller statement')
        add_watch('active:result')
        expect(r'[I1l]64\s*\(\s*8\s*\)','Caller result is initialized after statement continuation')
        editor.shot('caller_statement_stop')
        click('Frame')
        expect(r'Active\s+lexical\s+binding\s+was\s+not\s+found','Completing the pending frame leaves no active locals')
        editor.wait(lambda:not re.search(r'Source\s+stop:',observe()[0],re.I),seconds=20,description='Finish the suspended tick without starting another tick')
        assert manifest.read_bytes()==baseline
        editor.shot('completed_pending_tick')
        editor.held_click(678,46)
        result=dict(passed=True,real_imported_source_breakpoint=True,restart_retains_breakpoint=True,
                    real_statement_continuation=True,caller_result=8,completed_pending_tick=True,manifest_unchanged=True,
                    scope='Linux Xvfb/software graphics; OCR-observed widgets')
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception as error:
        record_failure(output,'source-breakpoints','widgets',error)
        if editor.process and editor.process.poll() is None:editor.shot('failure')
        raise
    finally:editor.close(failed=sys.exc_info()[0] is not None)


if __name__=='__main__':main()
