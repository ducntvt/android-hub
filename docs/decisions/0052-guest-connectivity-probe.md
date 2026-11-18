# 0052 — Run a bounded connectivity probe inside the selected guest

- **Status**: Accepted
- **Date**: 2026-11-17
- **Deciders**: Android Hub
- **Related**: US-041, ADR 0044

## Context

An application HTTP 400 does not establish that Android is offline. Host network checks do not test the guest path. Stock images do not guarantee curl/wget; installing a troubleshooting app or changing DNS would expand the user's request.

## Decision

Ship a repository-owned Java DEX helper compiled with the installed official SDK. On explicit request, run it as the guest shell UID through one pinned ADB transport. Its only network target is `https://www.google.com/generate_204`; DNS resolves `www.google.com` with the guest resolver. No redirects, body reads, cookies, account access, settings repair, background polling or host network fallback. Android's default HttpsURLConnection trust/hostname checks remain in place. A 204 proves this endpoint was reachable from this helper at that time, not that every app/service works; apps can have different proxy/VPN policies.

Read interface state as an up, non-loopback interface with a non-loopback/non-link-local address. This does not assert a default route, Internet validation or Wi-Fi identity. Do not emit interface names or IP addresses. DNS and HTTPS use bounded daemon workers (4 s and 9 s); interface enumeration has a 1 s limit. HTTPS additionally sets 4 s connect/read timeouts. Independent 20 s process watchdog and stdin-EOF monitor terminate even if native DNS ignores cancellation. Output is five fixed protocol lines with status codes only; exceptions and server response bodies are never copied to the host.

Verify profile/engine PID, AVD identity, transport and boot completion before the probe, compare the expected boot UUID inside the launch channel and Java helper, then reverify PID/AVD/boot after it. The desktop subscription is explicit and scoped to the selected settings view, request epoch and boot/PID identity. Changing target/view/session or quitting removes it; late results are rejected. The probe does not hold the mutation lock or share the input queue.

Upload embedded DEX bytes into a unique mode-0700 guest directory over the same boot-checked shell channel using exact-length dd. Verify SHA-256, set DEX read-only and execute. Stdin remains open for cancellation monitoring. Shell EXIT cleanup removes only this invocation's file/directory. No shared host cache or package installation is needed. Cancellation drops/kills the local adb child; helper EOF or watchdog terminates the guest worker. Guest/host crash can leave a small incomplete temporary directory; there is no broad automatic cleanup of unknown files.

## Consequences

- Results distinguish a failed test (DNS/TLS/I/O/deadline/unexpected HTTP) from an unavailable probe or changed session. The parser accepts only fixed codes and rejects extra/oversized output.
- Technical copy contains endpoint, timestamp and fixed codes, without boot IDs, serials, addresses, headers, credentials or raw adb output.
- Helper execution has a 25 s host timeout; identity checks add bounded startup/finalization time. No network probe occurs merely because the panel is opened.
- Checked-in DEX is a required runtime asset, not disposable cache. Rebuild with `bash scripts/build-connectivity-probe.sh`; this also runs a network-free Java harness for failure classification and deadline enforcement.
- Native guest evidence must still prove the execution path, successful certificate-verified HTTPS, cancel/cleanup and UI. Unit TLS classification tests alone do not prove platform trust behavior.

## Alternatives considered

- Host HTTP checks: wrong network namespace and UID.
- Guest ping: ICMP is often blocked and does not test DNS/TLS/application endpoints.
- Rely on curl/wget: absent on supported stock images; returning unknown for all HTTPS checks would not satisfy the feature.
- Install an APK: unnecessary persistent app/permission surface for three small probes.

## References

- [Android HttpsURLConnection](https://developer.android.com/reference/javax/net/ssl/HttpsURLConnection)
- [Android URLConnection timeouts](https://developer.android.com/reference/java/net/URLConnection)
