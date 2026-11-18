# Controlled reconnect after acknowledged touch

## Scope

QA-only checkout `/tmp/android-hub-reconnect-qa-20261116/checkout`, copied from current production sources including handshake timeouts. The only behavior substitution is a one-shot atomic fault after the first successful down RPC: return Err from the input future. The ordinary Session run/error/gate suspension/queue drain/retry/initial release paths handle the fault unchanged. No extra release is injected by the QA hook.

Bundle `/tmp/android-hub-reconnect-qa-20261116/bundle/Android Hub QA.app` uses `/tmp/android-hub-ui-qa-20261116/data`, with the previous QA window closed first. Target Android 3 is `hub-1794811158106`. No personal guest was manipulated or restarted.

## Observations

1. Selected running Android 3 through native UI. Offline fixture baseline: Down 4, Up 4, Cancel 0, Held false, Moves 0.
2. Native CUA click requested 1500 ms hold in zone 2. Hub visibly replaced the canvas with `QA: injected transport failure after acknowledged DOWN`.
3. Next observation showed the canvas restored automatically: Down 5, Up 5, Cancel 0, Held false, Last UP, Moves 0.
4. Fresh native click in zone 2 produced Down 6, Up 6, Held false. No duplicate down appeared from replaying the old input.
5. QA stderr marker confirms the post-ack fault hook executed. Only the marker is retained in `reconnect-native-marker-20261116.log`; no auth data or guest pixels are logged.

This establishes balanced touch and fresh input after a same-process reconnect through the actual error/retry path. It does not establish recovery from a hung RPC, server crash, lost reply, queue saturation, or interruption of an in-flight scroll. Precise latency was not measured. The fixture's existing wheel counters were already nonzero at baseline; this test makes no wheel-delivery claim.

## Build

QA build log: `reconnect-native-build-20261116.log`. Normal main bundle rebuilt afterward and strict signature verification passed: `reconnect-native-final-build-20261116.log`. Fault injection exists only in the temporary QA checkout/bundle. Production source is unchanged in this experiment; its latest full checks remain the handshake-timeout logs (113 passed, 20 opt-in ignored).
