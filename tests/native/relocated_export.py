#!/usr/bin/env python3
"""Run full native gameplay acceptance on the artifact produced by export_smoke."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

repo = Path(__file__).resolve().parents[2]
result = json.loads((repo/'target/task8-export-evidence/result.json').read_text())
if not result.get('source_project_unavailable'):
    raise RuntimeError('The source project must be unavailable for relocated acceptance')
game = Path(result['relocated_export'])/'game'
if not game.is_file():
    raise RuntimeError('Run export_smoke.py first')
env = os.environ.copy()
# Test/tool executables use system tools; game and child process use no Rust tools.
env.update(PATH='/usr/bin:/bin', KILN_PLAYER_PATH=str(game), KILN_TASK7_NO_ARCHIVE='1',
           KILN_TASK7_DISPLAY=':88', KILN_TASK7_EVIDENCE_DIR=str(repo/'target/task8-relocated-evidence'))
if shutil.which('cargo', path=env['PATH']) or shutil.which('rustc', path=env['PATH']):
    raise RuntimeError('The relocated acceptance PATH must exclude Rust')
process = subprocess.run([sys.executable, str(repo/'tests/native/task7_smoke.py')],
                         cwd=repo, env=env, check=False)
raise SystemExit(process.returncode)
