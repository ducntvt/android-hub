#!/bin/bash
# Build app-only ARM64 assets. Never include SDKs, Android images or user data.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
[[ "$(uname -s)" = Darwin && "$(uname -m)" = arm64 ]] || { echo 'Requires an Apple Silicon Mac' >&2; exit 1; }
[[ $# -eq 0 ]] || { echo 'Usage: bash scripts/package-release.sh' >&2; exit 1; }
version="$(python3 scripts/release.py product-version)"
name="Android-Hub-v$version-macos-arm64"
destination="$root/dist/v$version-macos-arm64"
[[ ! -e "$destination" && ! -L "$destination" ]] || { echo "Output already exists: $destination" >&2; exit 1; }
mkdir -p "$root/dist"
work="$(mktemp -d "$root/dist/.package.XXXXXX")"
mounted=false
cleanup() {
    if [[ "$mounted" = true ]]; then
        if ! hdiutil detach "$work/mount"; then
            echo "Could not detach verification image; retained $work" >&2
            return
        fi
    fi
    rm -rf "$work"
}
trap cleanup EXIT
mkdir "$work/output"
app="$work/output/Android Hub.app"
bash scripts/build-android-hub.sh --release --output "$app"
codesign --verify --deep --strict "$app"
python3 scripts/release.py verify-bundle "$app/Contents/Info.plist"
[[ "$(lipo -archs "$app/Contents/MacOS/android-hub")" = arm64 ]]
# Reject accidental dependencies on the build machine (Homebrew, local dylibs).
otool -L "$app/Contents/MacOS/android-hub" | python3 -c '
import sys
paths = [line.strip().split(" (", 1)[0] for line in sys.stdin.read().splitlines()[1:]]
unexpected = [p for p in paths if not p.startswith(("/System/Library/", "/usr/lib/"))]
if unexpected:
    raise SystemExit("Non-system dynamic dependencies: " + ", ".join(unexpected))
'
cat > "$work/output/INSTALL.txt" <<'INSTALL'
Android Hub — Apple Silicon, macOS 15 or later

Copy Android Hub.app to Applications before opening it.
This personal build is ad-hoc signed, NOT Developer ID signed or notarized.
Gatekeeper may block a downloaded copy. Building from the repository is also
supported; no system-wide security changes are required by this package.

The app contains no Android SDK, engine, images, devices or Google accounts.
Configure a compatible Android SDK and Java in Hub settings before creating a
device. See the repository's README and docs/SETUP.md (or docs/SETUP.vi.md).
The complete fresh-machine SDK installation flow is not yet qualified.

Existing device data stays in ~/Library/Application Support/Android Hub.
Back up stopped devices before updating. No automatic app updates are included.
INSTALL
ditto -c -k --sequesterRsrc --keepParent "$app" "$work/output/$name.app.zip"
mkdir "$work/dmg"
ditto "$app" "$work/dmg/Android Hub.app"
cp "$work/output/INSTALL.txt" "$work/dmg/INSTALL.txt"
ln -s /Applications "$work/dmg/Applications"
hdiutil create -volname "Android Hub $version" -srcfolder "$work/dmg" -format UDZO -fs HFS+ "$work/output/$name.dmg"
hdiutil verify "$work/output/$name.dmg"
mkdir "$work/mount"
hdiutil attach -readonly -nobrowse -mountpoint "$work/mount" "$work/output/$name.dmg"
mounted=true
codesign --verify --deep --strict "$work/mount/Android Hub.app"
cmp "$app/Contents/MacOS/android-hub" "$work/mount/Android Hub.app/Contents/MacOS/android-hub"
[[ "$(readlink "$work/mount/Applications")" = /Applications ]]
hdiutil detach "$work/mount"
mounted=false
ditto -x -k "$work/output/$name.app.zip" "$work/unzip"
codesign --verify --deep --strict "$work/unzip/Android Hub.app"
cmp "$app/Contents/MacOS/android-hub" "$work/unzip/Android Hub.app/Contents/MacOS/android-hub"
(
    cd "$work/output"
    shasum -a 256 "$name.app.zip" "$name.dmg" INSTALL.txt > SHA256SUMS
    shasum -a 256 -c SHA256SUMS
)
# Refuse overwrites; publish only after both containers verify.
[[ ! -e "$destination" && ! -L "$destination" ]]
mv "$work/output" "$destination"
echo "Verified release assets: $destination"
