# Android Hub UI style

Use a restrained macOS-inspired desktop style with a neutral canvas, modest
rounding, a single blue accent and compact controls. Rust is the implementation
language; the visual system lives in Iced's shared styles and widgets. This is
an app-specific treatment, not a claim that Iced controls are native AppKit.

| Element | Shared treatment |
| --- | --- |
| Buttons, text inputs, pickers | 10 pt corner radius |
| Input border | 1 pt muted border; 1.5 pt accent on keyboard focus |
| Input surface | Base surface, panel surface when disabled |
| Input placeholder | Readable secondary text in both themes |
| Body / button / caption | 14 / 13 / 12 pt |
| Icon | 20 pt SVG viewport, 1.6 pt rounded strokes; filled Android navigation glyphs |
| Icon hover | Subtle neutral fill; accent tint on press; keyboard focus ring retained |
| Tooltip | 12 pt text, 260 pt maximum content width, 6 pt padding, 8 pt gap |
| Tooltip surface | Opaque theme background, 8 pt corners, thin border, small shadow |

Keep labels and hints short. Preserve field labels and validation messages:
placeholder text disappears when people type. Tooltips explain an icon, while
essential information stays visible. Guidance: [Apple text fields](https://developer.apple.com/design/human-interface-guidelines/text-fields)
and [offering help](https://developer.apple.com/design/human-interface-guidelines/offering-help).
The numerical tokens above are Android Hub's choices, not Apple's prescribed values.

Implementation lives in `crates/desktop/src/app/ui/style.rs`, the shared `hint`
helper in `ui.rs`, and the existing keyboard-aware wrappers in `ui/focus.rs`.
All current tooltip call sites use `hint`; text inputs and pickers receive the
shared styling through their wrappers.

`ui/icons.rs` owns thirteen small SVGs. Handles are initialized once, and Iced caches
their rasterization by size and color. This replaces the aliased canvas icon
paths without enabling full-window MSAA on the Android video canvas. Rotation
uses one clear arrow around the device, with all strokes inside its viewport.
Normal, hovered and pressed icons share the same geometry.

Device cards use one navigation bar: Back, Home, Recents, then Rotate, Clipboard
and Install APK. Icons share compact tooltips and shrink their horizontal padding
in narrow split views. APK remains accessible when the screen stream reconnects.
Single-device names and Android versions live in the main header; split cards
retain their picker and compact version. API and configured-image details live
in device settings. Normal readiness has no extra workspace label; the keyboard
focus caption and screen border indicate the actual input target. Boot, reconnect
and failure states remain visible, with the existing retry action.

Inspectors share `ui/panels.rs`: a fixed 18pt heading with a right-aligned 36pt
close control, a separator, and an independently scrolling body. Keep actions at
13pt and explanatory text at 12–13pt. SDK and Java use distinct cards; common
save restrictions appear once, while technical prerequisites live in Details.
Path fields use 12pt monospace text, 10×12pt padding, and a wrapping full-path
tooltip. Inputs keep their existing edit, validation and save behavior. Settings,
storage, archived devices and history use the same inspector shell. Information
dialogs put Close in the heading; destructive confirmations retain explicit Cancel.

Localize retained Hub diagnostics when rendering (`Language::diagnostic`), not
when storing errors. Switching En/Vi must also update existing stream errors,
retry tooltips and dialogs. Only known messages and complete template wrappers
are translated: preserve device names, paths and external tool output. History
uses its own action translator so device names never pass through translation.

Scroll containers share `ui/scroll.rs`: a neutral 5pt rounded thumb inside a 10pt
pointer target. Hover/drag increases contrast without switching to the accent
color. Vertical lists reserve a stable 14pt content gutter so device menus and
form actions cannot sit under the rail. Rails have no painted background and
only appear on overflow; do not use Iced 0.13 `Scrollbar::spacing`, which forces
empty rails. Workspace scrollbars share the style while retaining the existing
zoom, stage padding and guest pointer coordinate mapping.
