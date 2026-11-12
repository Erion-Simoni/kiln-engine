#!/usr/bin/env python3
"""Report existing artifact sizes for information; no release-size thresholds."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[1]
paths = [root/'target/wasm32-unknown-unknown/release/kiln_web.wasm']
for name in ('kiln-editor', 'kiln-player'):
    paths.extend(root/'target/release'/f'{name}{suffix}' for suffix in ('', '.exe'))
print(json.dumps({'policy': 'informational; no package-size release goal',
                  'artifacts': [{'path': str(p.relative_to(root)), 'bytes': p.stat().st_size}
                                for p in paths if p.is_file()]}, indent=2))
