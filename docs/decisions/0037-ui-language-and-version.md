# ADR 0037 — Desktop language and configured Android version

- Status: Accepted
- Date: 2026-11-16

Language is a desktop preference, persisted independently in runtime/ui-language with atomic replacement, without touching SDK settings or guest locale. UI labels use an explicit Vietnamese/English catalog; formatted labels keep compile-time format checking. License text, user names/paths and original diagnostic details are not machine-translated. No network translation service or background task is added.

Version is read from each AVD config target in the existing background snapshot, without starting a guest or issuing per-frame ADB calls. Snapshot carries a separate per-device version map (the runtime map still contains only running engines); known API levels map to releases, unknown API levels display only the numeric API. UI explicitly says configured image: this is not an assertion about a modified guest's live ro.build.version. No profile schema migration is necessary.
