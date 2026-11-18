# Offline input QA fixture

Build with `bash scripts/build-input-qa-apk.sh /tmp/new-input-qa-directory`.
The output folder must not exist. This uses installed Android SDK 35/build tools
35.0.1 and Android Studio Java; no Gradle download, permissions or network code.
Install only on an identified disposable QA guest, then launch **Hub Input QA**
from the Android app drawer. Installation via adb is fixture preparation and does
not count as verification of Hub's APK drag/drop interface.

The three colored zones independently accumulate horizontal/vertical ACTION_SCROLL
axis values. The header displays the most recent local view coordinates and axes.
Rows move according to the respective zone's accumulated axes. Touch down has a
separate counter; wheel must not increment it. This custom View is an input probe,
not proof of every Android widget's scrolling behavior.

Alphabetic keys A–Z update the event label with the Android key code and separate
Down/Up counts. The view requests keyboard focus on launch and touch; other keys
retain normal Android handling. This probe can distinguish a visible Hub focus
indicator from actual keyboard delivery to the intended guest.

Native checks:

1. Scroll over zone 3; only zone 3 changes, with matching local pointer coordinates.
2. Scroll over zone 1; zone 2/3 retain their values.
3. Scroll left/right; only the target zone's H value changes.
4. Scroll outside the Android canvas; no guest counters change.
5. Repeat in two simultaneously visible guests; the other guest stays unchanged.
6. Tap/drag then wheel; touch and wheel remain distinct.
7. Reconnect the display session; no old wheel event replays.
8. Type a letter in each guest; only the focused guest's key counts change.
9. Open a Hub confirmation, click/type behind it, then cancel with Escape. No
   touch/key events should reach the guest while the modal is open. Type a
   different letter without clicking the canvas again; the previously focused
   guest receives it with balanced Down/Up counts.

Use native UI tooling for test events. Do not inject directly through adb/shell,
which would bypass the host event routing being tested. Coordinates exclude the
Android status bar inset; derive expectations from the actual displayed canvas.
