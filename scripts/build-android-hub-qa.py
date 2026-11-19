#!/usr/bin/env python3
"""Package the existing Hub binary for isolated QA; never builds or starts a VM."""
import argparse
import hashlib
from pathlib import Path
import plistlib
import re
import shutil
import subprocess


LAUNCHER = '''#!/bin/sh
set -eu
unset ANDROID_SDK_ROOT ANDROID_HUB_JAVA_HOME P0_JAVA_HOME ANDROID_HUB_WINDOW_SIZE
bundle_bin_dir="$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)"
qa_root="$(CDPATH= cd -- "$bundle_bin_dir/../../.." && pwd)"
export ANDROID_HUB_ROOT="$qa_root/data"
exec "$bundle_bin_dir/android-hub" "$@"
'''


def render_launcher(window_size=None):
    if window_size is None:
        return LAUNCHER
    if not re.fullmatch(r'[0-9]{4}x[0-9]{3,4}', window_size):
        raise ValueError('Expected WIDTHxHEIGHT, e.g. 1000x640')
    width, height = map(int, window_size.split('x'))
    if not (1000 <= width <= 7680 and 640 <= height <= 4320):
        raise ValueError('Window size outside supported bounds')
    return LAUNCHER.replace('exec "$bundle_bin_dir/android-hub"', f'export ANDROID_HUB_WINDOW_SIZE={window_size}\nexec "$bundle_bin_dir/android-hub"')


def package(destination, source, window_size=None):
    launcher_text = render_launcher(window_size)
    destination = Path(destination).resolve()
    source = Path(source).resolve(strict=True)
    if not source.is_file():
        raise ValueError('Expected a built Hub executable')
    # Never replace an existing QA bundle or its guest data.
    destination.mkdir(parents=True, exist_ok=False)
    app = destination / 'Android Hub QA.app'
    binary_dir = app / 'Contents/MacOS'
    binary_dir.mkdir(parents=True)
    (destination / 'data').mkdir()
    shutil.copy2(source, binary_dir / 'android-hub')
    resources = app / 'Contents/Resources'
    resources.mkdir()
    server_assets = Path(__file__).resolve().parents[1] / 'crates/infrastructure/assets/scrcpy'
    for source_name, bundle_name in [('LICENSE', 'SCRCPY-LICENSE.txt'), ('README.md', 'SCRCPY-PROVENANCE.md')]:
        shutil.copy2(server_assets / source_name, resources / bundle_name)
    launcher = binary_dir / 'qa'
    launcher.write_text(launcher_text)
    launcher.chmod(0o755)
    identifier = hashlib.sha256(str(destination).encode()).hexdigest()[:16]
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps({
        'CFBundleIdentifier': f'local.androidhub.qa{identifier}',
        'CFBundleExecutable': 'qa',
        'CFBundleName': 'Android Hub QA',
        'CFBundlePackageType': 'APPL',
        'CFBundleVersion': '1',
        'CFBundleShortVersionString': '0.1.0',
        'NSHighResolutionCapable': True,
    }))
    subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(app)], check=True)
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    return app


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', type=Path, help='New directory for the QA app and data')
    parser.add_argument("--window-size", help="Initial content size, e.g. 1000x640")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[1]
    print(package(args.destination, root / 'Android Hub.app/Contents/MacOS/android-hub', args.window_size))
