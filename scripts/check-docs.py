#!/usr/bin/env python3
"""Check maintained docs' local links, script references and bash example syntax.

Never execute examples, call the network, or validate historical runtime artifacts.
Run from any directory; exit nonzero on broken maintained documentation.
"""
import re
import subprocess
from pathlib import Path
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[1]
DOCS = [
    *ROOT.glob('*.md'),
    *ROOT.joinpath('docs').glob('*.md'),
    ROOT / 'docs/stories/README.md',
    ROOT / 'docs/decisions/README.md',
    ROOT / 'docs/stories/US-046-private-repository-documentation.md',
    ROOT / 'docs/stories/US-047-ci-and-source-releases.md',
    ROOT / 'docs/decisions/0055-source-release-automation.md',
    ROOT / 'docs/decisions/0056-immutable-android-image-per-device.md',
    ROOT / 'docs/decisions/0057-ad-hoc-app-distribution.md',
    ROOT / 'docs/decisions/0058-optional-avd-target.md',
    ROOT / 'docs/stories/US-054-app-and-dmg-release.md',
    *ROOT.joinpath('docs/stories/epics').rglob('*.md'),
    ROOT / 'tools/metrics/README.md',
    ROOT / 'crates/desktop/assets/README.md',
    ROOT / 'crates/infrastructure/resources/README.md',
]


def main():
    errors = []
    for path in sorted(set(DOCS)):
        label = str(path.relative_to(ROOT))
        source = path.read_text()
        prose = re.sub(r'```[^\n]*\n.*?```', '', source, flags=re.S)
        links = re.findall(r'\]\(([^)]+)\)', prose)
        links += re.findall(r'<img\b[^>]*\bsrc=[\'"]([^\'"]+)', prose)
        for target in links:
            target = target.strip().strip('<>')
            parsed = urlsplit(target)
            if parsed.scheme or parsed.netloc or not parsed.path:
                continue
            local = path.parent / unquote(parsed.path)
            if not local.exists():
                errors.append(f'{label}: missing local link {target}')
        for script in re.findall(r'\b(?:bash|python3)\s+(scripts/[\w.-]+)', source):
            if not (ROOT / script).is_file():
                errors.append(f'{label}: missing script {script}')
        for index, example in enumerate(re.findall(r'```(?:bash|sh)\n(.*?)```', source, re.S), 1):
            result = subprocess.run(['bash', '-n'], input=example, text=True, capture_output=True)
            if result.returncode:
                errors.append(f'{label}: shell example {index}: {result.stderr.strip()}')
    if errors:
        print('\n'.join(errors))
        return 1
    print(f'Documentation checks passed ({len(set(DOCS))} files; no examples executed).')
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
