# Physical input QA — US-060

Offline fixture derived from android-input without modifying the frozen US-056
benchmark source. No network or runtime permissions. Build with:

```bash
bash scripts/build-input-qa-apk.sh /tmp/hub-control-qa tests/android-control
```

It uses `local.androidhub.inputqa`; check the target has no pre-existing copy before
installing. Inspect the synthetic counters via its accessibility description:
touch down/up/cancel, held state, motion, horizontal/vertical wheel, key down/up
and last key. The activity consumes keys (including Back) so navigation should
also be checked outside it. Start with `--ez landscape true` to test a capture
orientation change; otherwise it requests portrait. Force-stop between launches.
Add `--ez rotateOnTouch true` to rotate this activity 500 ms after its first
touch down. Hold the pointer through the rotation to verify cancellation and
capture changes. Counters survive configuration changes; this does not modify
the phone's system auto-rotation setting.

Use an explicitly verified USB transport, never an implicit device. Cover Fit,
110% zoom, resize, focus loss/modal, stop and reconnect. Delete only the fixture
installed by this QA session when done. Do not read personal clipboard content:
the optional legacy paste handler is not part of US-060.
