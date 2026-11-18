#!/usr/bin/env python3
"""Collect non-secret P0 host/toolchain evidence; does not start a VM."""
import json
import platform
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def capture(args):
    try:
        p = subprocess.run(args, capture_output=True, text=True, timeout=30)
        return {'exit_code': p.returncode, 'stdout': p.stdout.strip(), 'stderr': p.stderr.strip()}
    except (OSError, subprocess.TimeoutExpired) as e:
        return {'error': str(e)}

result = {
    'platform': platform.system(), 'architecture': platform.machine(),
    'macos': capture(['sw_vers', '-productVersion']),
    'memory_bytes': capture(['sysctl', '-n', 'hw.memsize']),
    'rustc': capture(['rustc', '--version']), 'cargo': capture(['cargo', '--version']),
    'xcode': capture(['xcodebuild', '-version']),
    'cmake': capture(['cmake', '--version']), 'ninja': capture(['ninja', '--version']),
}
out = ROOT/'docs/reports/p0/environment.json'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_text(json.dumps(result, indent=2) + '\n')
print(out.relative_to(ROOT))
