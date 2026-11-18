# US-041 — Guest connectivity verification

Status: implemented, 2026-11-17. Contract: [ADR 0052](../decisions/0052-guest-connectivity-probe.md).

## Implementation

- Device settings exposes an explicit Check connectivity action, guest name and UTC request time. It stays unavailable until boot readiness is verified. Opening settings alone does not probe.
- Repository-owned Java helper observes an active interface address, resolves www.google.com, and requests https://www.google.com/generate_204 with Android's default TLS/hostname verification. No redirects/body/account data, settings changes or host fallback.
- Fixed protocol distinguishes success, DNS/TLS/I/O/timeout/unexpected HTTP, and unavailable/unknown. Parser rejects extra, malformed, oversized or wrong-boot output. Copyable details contain endpoint/time/codes only.
- PID/AVD/transport/boot checks surround execution. The cancellable desktop subscription is scoped to the settings target and request epoch; closing the panel, changing session or quitting drops it. It does not take the mutation lock or input queue.
- Per-step worker deadlines, a guest watchdog and stdin EOF termination bound helper lifetime. One shell channel owns exact-length upload, SHA-256 verification, read-only DEX execution and EXIT cleanup. No persistent host cache.

## Automated evidence

- `bash scripts/build-connectivity-probe.sh`: Java failure-classification/redaction/deadline harness passed; DEX rebuilt from source using installed SDK 35.0.1. Harness does not contact the network.
- Final `bash scripts/check.sh`: **188 Rust tests** (78 desktop + 4 domain + 106 infrastructure), **26 Python tests**, fmt, clippy and shell syntax passed. 20 existing integration tests remain ignored. Existing block 0.1.6 future-incompatibility warning remains.
- App build and `codesign --verify --deep --strict 'Android Hub.app'` passed.
- New Rust fixtures cover protocol failures, unavailable executable, excess output before EOF, cancellation/reaping of the host transport, complete response and stdin lifetime; desktop covers stopped/no-auto-probe, cancel/retry epochs, late responses, closing settings and changed PID.

## Native evidence

Isolated app/data `/tmp/android-hub-connectivity-qa-20261117`, content size 1000×640. Created QA Connectivity through the Hub form, ID `hub-1794909121551-17`, port 5584, Android 16/API 36 Google Play ARM64. No personal account/game data was copied.

1. Guest reached Android Home and Ready to control. Settings exposed the endpoint and explicit action; there was no report before clicking.
2. A request at 09:53:26 UTC returned Interface/DNS/HTTPS Passed. Details reported completion epoch 1794909208 and all three fixed OK codes. This is a real guest-origin HTTPS 204 through the shipped helper; trust bypasses are absent from the helper source.
3. VI/EN labels, timestamp, endpoint caveat and wrapped technical details were readable. Copy details followed by paste into the QA search field produced the report text (visible trailing service caveat); search was then cleared.
4. A retry visibly entered Checking connectivity with Cancel. It completed before the attempted Cancel click, so that click is **not** claimed as a successful native cancellation. Cancel message semantics and child reaping are covered by automated fixtures.
5. Started another request and immediately closed settings. Reopening showed no old report and did not restart a probe. Read-only guest process/directory checks found neither ConnectivityProbe nor android-hub-connectivity-* entries after normal completion and after panel cancellation. The cancellation can occur during identity checks; this does not establish that a blocked guest DNS call was interrupted on this run.
6. Cmd+Q closed QA and stopped its guest. App inventory and exact owned path/guest-ID process checks showed no QA process remained. The owned Finder window was closed; the pre-existing tmp Finder window was preserved.

No network fault was injected into macOS or the guest. Timeout/TLS/DNS failures use the Java/Rust fixtures. A successful endpoint check is not evidence that YouTube, games, VPN-routed apps or every network service works.

## Cleanup

The QA bundle/AVD data, two owned check/build logs and Cargo target were removed after the final process check. Main Android Hub.app, personal runtime, SDK and existing AVDs were preserved. The checked-in connectivity-probe.dex is a required runtime asset and is retained. No commit was made.
