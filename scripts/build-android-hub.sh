#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
profile=debug
app="$root/Android Hub.app"
while [[ $# -gt 0 ]]; do
    case "$1" in
        --release) profile=release; shift ;;
        --output)
            [[ $# -ge 2 && "$2" = /* && "$2" = *.app ]] || { echo '--output requires an absolute .app path' >&2; exit 1; }
            app="$2"; shift 2 ;;
        *) echo "Unknown argument: $1" >&2; exit 1 ;;
    esac
done
version="$(python3 scripts/release.py bundle-version)"
if [[ "$profile" = release ]]; then
    export MACOSX_DEPLOYMENT_TARGET=15.0
    cargo build --release --locked -p android-hub
else
    cargo build --locked -p android-hub
fi
target_dir="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# Replace the executable atomically: the user's running Hub keeps its old inode.
staged_binary="$(mktemp "$app/Contents/MacOS/.android-hub.XXXXXX")"
icon_work="$(mktemp -d "${TMPDIR:-/tmp}/android-hub-icon.XXXXXX")"
trap 'rm -f "$staged_binary"; rm -rf "$icon_work"' EXIT
mkdir "$icon_work/AndroidHub.iconset"
for size in 16 32 128 256 512; do
    sips -z "$size" "$size" "$root/crates/desktop/assets/android-hub.png" \
        --out "$icon_work/AndroidHub.iconset/icon_${size}x${size}.png" >/dev/null
    double=$((size * 2))
    sips -z "$double" "$double" "$root/crates/desktop/assets/android-hub.png" \
        --out "$icon_work/AndroidHub.iconset/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$icon_work/AndroidHub.iconset" -o "$app/Contents/Resources/AndroidHub.icns"
cp "$target_dir/$profile/android-hub" "$staged_binary"
chmod 755 "$staged_binary"
mv -f "$staged_binary" "$app/Contents/MacOS/android-hub"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0"><dict>
<key>CFBundleIdentifier</key><string>local.androidhub.desktop</string>
<key>CFBundleName</key><string>Android Hub</string>
<key>CFBundleDisplayName</key><string>Android Hub</string>
<key>CFBundleExecutable</key><string>android-hub</string>
<key>CFBundleIconFile</key><string>AndroidHub</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>CFBundleShortVersionString</key><string>$version</string>
<key>CFBundleVersion</key><string>$version</string>
<key>NSHighResolutionCapable</key><true/>
</dict></plist>
PLIST
if [[ "$profile" = release ]]; then
    python3 - "$root" "$app" <<'NOTICES'
import hashlib, sys
from pathlib import Path
root, app = map(Path, sys.argv[1:])
notices = root / 'crates/desktop/assets/THIRD-PARTY-NOTICES.txt'
expected = 'Cargo.lock SHA-256: ' + hashlib.sha256((root / 'Cargo.lock').read_bytes()).hexdigest()
if expected not in notices.read_text().splitlines()[:3]:
    raise SystemExit('Review third-party notices for the current Cargo.lock before release packaging')
(app / 'Contents/Resources/THIRD-PARTY-NOTICES.txt').write_bytes(notices.read_bytes())
for source_name, bundle_name in [('LICENSE', 'SCRCPY-LICENSE.txt'), ('README.md', 'SCRCPY-PROVENANCE.md')]:
    (app / 'Contents/Resources' / bundle_name).write_bytes((root / 'crates/infrastructure/assets/scrcpy' / source_name).read_bytes())
NOTICES
    /usr/libexec/PlistBuddy -c 'Add :LSMinimumSystemVersion string 15.0' "$app/Contents/Info.plist"
fi
codesign --force --deep --sign - "$app"
# The bundle directory's mtime otherwise stays unchanged across in-place builds.
# Refresh this bundle's LaunchServices record without restarting the user's Dock.
touch "$app"
registrar="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
if [[ "$app" = "$root/Android Hub.app" && -z "${CI:-}" && -x "$registrar" ]]; then
    "$registrar" -f "$app" || echo 'Warning: macOS app registration refresh failed' >&2
fi
echo "$app"
