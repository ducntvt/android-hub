# Remaining acceptance audit — 2026-11-16

Current story files: 20 implemented, 6 in progress. This count is not a percentage of full product readiness. Latest source verification: 113 passed, 20 opt-in ignored; APK pending identity and desktop input epoch fixes included. No new product code change in this audit.

| Story | Evidence still required | Current dependency / next concrete step |
| --- | --- | --- |
| US-007 APK | Native drop → cancel → confirm → package on correct guest, peer unchanged | QA updated and Android 3 started; package baseline absent recorded in `apk/native-baseline-20261116.json`. Finder contains signed test APK. Requested user-assisted cross-window drop; no response yet. Do not install or switch target until the pending drop can be observed. |
| US-011 wheel | Native gesture/interruption cases and host scrolling-direction coverage | Basic both-axis, correct-target/outside and reconnect smoke exist; simultaneous hold/wheel and interruption are not proved by ordinary sequential native calls. Need controlled native fault/gesture scenario or manual assistance. |
| US-012 release | Native change of selection/view or connection during active hold | Queue/generation/desktop-epoch regression exists. Native single/split drag exists. Tool exposes complete click/drag actions, not separate down/up; no claimed active-hold fault result. |
| US-015 clipboard | Native selection/session change while asynchronous read is pending | Two-guest Unicode paste passed. Callback guards have regression coverage. Sequential click/change cannot prove the callback was still pending; need a controlled QA delay or manual reproducible timing. |
| US-025 SDK | Real candidate download, publish, select, create and boot through UI | License review UI tested without accepting. User has not supplied action-time acceptance for candidate terms. Existing SDK and fixture tests are not substitute evidence. |
| US-001 engine gate | Two actual games, audio/network/input, controlled performance and at least 60-minute run | User postponed game measurement. Missing measurements/acceptance remains open; idle QA is not a substitute. |

The current QA window is reserved for the pending APK handoff. Avoid changing selection, stopping its guest or repurposing its clipboard until that handoff is resolved. No process wait is claimed: user assistance is not a live tool job.

Other full-plan requirements remain tracked in PLAN/TASKS, including conditional key mapping and lifecycle/performance coverage. Do not close the overall goal from story count alone. This audit records dependencies; it does not waive acceptance requirements or declare all independent engineering work exhausted.
