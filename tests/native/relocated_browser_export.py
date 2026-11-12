#!/usr/bin/env python3
"""Serve/play only the browser artifact produced by the native editor smoke."""
import json
import os
from pathlib import Path
import subprocess

repo = Path(__file__).resolve().parents[2]
result = json.loads((repo/'target/task8-browser-export-evidence/result.json').read_text())
if not result['source_project_unavailable']:
    raise RuntimeError('Make the source project unavailable before relocated acceptance')
env = os.environ.copy()
env.update(WEB_EXPORT_ROOT=result['relocated_export'],
           WEB_TASK7_EVIDENCE=str(repo/'target/task8-browser-evidence'))
process = subprocess.run(['node', str(repo/'tests/web/task7.cjs')], cwd=repo, env=env)
raise SystemExit(process.returncode)
