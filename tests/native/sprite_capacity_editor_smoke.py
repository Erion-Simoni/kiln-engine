#!/usr/bin/env python3
"""Observe the editor's world-queue warning without editing the stress project."""
import json
import os
from pathlib import Path
import re
import subprocess
from task9_editor_games import Editor, REPO, record_failure


def main():
    project=Path(os.environ.get('KILN_STRESS_PROJECT',REPO/'target/a05-overflow-project')).resolve()
    output=Path(os.environ.get('KILN_STRESS_EDITOR_EVIDENCE',REPO/'target/a05-overflow-editor')).resolve()
    output.mkdir(parents=True,exist_ok=True)
    manifest=project/'kiln.toml';before=manifest.read_bytes()
    editor=Editor(REPO/'target/debug',project,output/'editor',':85')
    def visible_warning():
        editor.shot('capacity_warning')
        text=subprocess.check_output(['tesseract',str(output/'editor/capacity_warning.png'),'stdout','--psm','11'],stderr=subprocess.DEVNULL,text=True,timeout=20)
        (output/'ocr.txt').write_text(text)
        match=re.search(r'SPRITES\s*:\s*(\d+)\s+dropp',text,re.I)
        return int(match.group(1)) if match and int(match.group(1))>0 else None
    try:
        editor.start();dropped=editor.wait(visible_warning,seconds=45,description='Observe visible editor sprite-capacity diagnostic')
        assert manifest.read_bytes()==before
        result=dict(passed=True,visible_editor_capacity_warning=True,dropped=dropped,manifest_unchanged=True,scope='Linux Xvfb/software graphics; OCR-observed status bar')
        (output/'result.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result))
    except Exception as error:
        record_failure(output,'sprite-capacity','editor',error);raise
    finally:editor.close(failed=True)


if __name__=='__main__':main()
