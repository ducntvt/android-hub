#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="$(python3 scripts/release.py bundle-version)"
cargo build --locked -p android-hub
app="$root/Android Hub.app"
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
cp "$root/target/debug/android-hub" "$staged_binary"
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
codesign --force --deep --sign - "$app"
# The bundle directory's mtime otherwise stays unchanged across in-place builds.
# Refresh this bundle's LaunchServices record without restarting the user's Dock.
touch "$app"
registrar="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
if [[ -z "${CI:-}" && -x "$registrar" ]]; then
    "$registrar" -f "$app" || echo 'Warning: macOS app registration refresh failed' >&2
fi
echo "$app"
