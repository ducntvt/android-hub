# Empty-registry SDK native smoke — 2026-11-16

Used `/tmp/android-hub-empty-native-20261116/Android Hub Empty QA.app`, with an isolated empty registry and environment overrides unset. No guest created or booted, no main preferences changed.

- Sidebar displayed no devices; SDK setup remained available.
- Panel opened and SDK labels, input, save button and explanatory text were visible without overlap at 2360×1704.
- Missing `/tmp/hub-empty-sdk-missing` was rejected with an OS missing-file error.
- `/Users/ducnt/Library/Android/sdk` saved successfully, with ready/saved notice.
- Cmd+Q and native reopen: panel retained the SDK path and ready state, list remained empty.
- Read-only disk check: saved SDK matched, preference permissions 0600, registry still empty.

Existing populated-registry/running-device checks are in [SDK/Java smoke](../sdk-java-native-20261116.md). This verifies selection of an installed SDK, not downloading a new SDK. Long Java paths scroll within their text field; no claim that the entire Java path fits at once.
