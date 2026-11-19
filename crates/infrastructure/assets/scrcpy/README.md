# scrcpy Android server

- Version: **5.0.1**, unmodified official release binary.
- Source: https://github.com/Genymobile/scrcpy/tree/v5.0.1/server
- Download: https://github.com/Genymobile/scrcpy/releases/download/v5.0.1/scrcpy-server-v5.0.1
- SHA-256: `764eb6f79811d5211fe9df341120882ba9994c7a61b897d7bf3fb662e53bc536`
- Copyright: Genymobile and scrcpy contributors. License: Apache-2.0, see LICENSE.

The Rust client is pinned to the same version. The checksum is checked before
upload. Only a user-started authorized USB session uploads a unique temporary JAR
and starts its server; no Android SDK/image is bundled. No audio, control,
clipboard autosync, wake lock or display-power change is requested by US-059.
The unmodified server includes its normal self-cleanup; Hub only removes its own
random JAR path on cleanup, never shared files or another viewer's resources.

This binary is a runtime dependency, not disposable build output or a cache.
Keep its license/provenance in app bundles when distributing an embedded copy.
