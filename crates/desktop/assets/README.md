# Android Hub logo

`android-hub.svg` is the canonical vector logo. Two connected device silhouettes
form an H on a blue/teal tile. On 2026-11-18 the original generated bitmap was
reconstructed as geometric paths to remove pitted alpha edges and detached pixels.

`android-hub.png` is its checked-in 1024 × 1024 RGBA export. The desktop embeds it
once for the sidebar; `scripts/build-android-hub.sh` derives macOS icon sizes and
`AndroidHub.icns` inside the app bundle. Its temporary iconset is removed on exit.
Both source files are intentional assets, not caches. Normal builds do not need
an SVG renderer. When editing the logo, regenerate the PNG with librsvg:

```sh
rsvg-convert -w 1024 -h 1024 -o crates/desktop/assets/android-hub.png crates/desktop/assets/android-hub.svg
```

Original concept created using the built-in imagegen tool on 2026-11-17. Prompt:

> Use case: logo-brand. Asset type: final macOS application icon for Android Hub, a native desktop application managing multiple Android virtual devices. Create ONE polished, minimalist app icon, not a mockup or a sheet. A strong geometric white H monogram subtly formed by two vertical rounded phone/device silhouettes joined at their centers. Solid substantial shapes, very legible at tiny Dock sizes. Centered on a rich blue rounded-square tile, subtle restrained blue-to-teal depth, modern macOS feel with clean edges, not glassy. Generous optical padding inside the tile. Tile fills about 88% of square canvas, fully visible, transparent outside tile. No text, no lettering other than the implied H symbol, no android robot, no tiny details, no additional symbols, no watermark, no perspective. Square 1024x1024 composition. Output production-ready icon with alpha transparency.

The original 1254 × 1254 generated bitmap and a later imagegen edge-repair
attempt both contained transparency artifacts and are not used in the app.

Validated the replacement PNG's continuous alpha boundary and fully opaque
interior, and visually checked the packaged ICNS exports at 32 px and 128 px.
The rebuilt app passed strict deep codesign verification on 2026-11-18.

After signing, the build script touches the app bundle and refreshes only its
LaunchServices registration so repeated in-place builds expose the current icon.
It does not restart Dock or clear global caches. On 2026-11-18, bundle metadata and
the ICNS were valid while Dock showed a generic icon; the running app's icon was
read back as the H logo. Dock was restarted once to clear its stale presentation.
User confirmed the Dock icon displayed correctly after the refresh. The build
script passed shell syntax validation and the existing app retained a valid strict
deep code signature. Temporary diagnostic images and Swift cache were removed.
