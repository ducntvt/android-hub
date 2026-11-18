# Native removal recovery — 2026-11-16

Used the isolated `/tmp/android-hub-recovery-native-20261116/data` fixture. Pending states were prepared while Hub was closed, using the existing fixture directory identities and receipt schema. These are synthetic post-registry interruption states, not real process-kill/power-loss trials. No Android guest was booted.

1. Pending remove with registry empty and data/registration in archive: native app reopen restored the profile, marker and registration. UI showed “gỡ máy bị gián đoạn; đã đưa máy về trạng thái trước đó”. Disk audit found a rolled-back receipt and no pending journal.
2. Pending undo with profile/data already live: native reopen returned to empty list and showed “đưa máy trở lại bị gián đoạn; dữ liệu vẫn trong archive”. The archive remained selectable through the new wrapped selection list.
3. Retried undo using native selection → request → confirmation. Device returned selected/stopped with success notice and undo history; did not auto-boot.
4. Final disk audit: original marker and registration present, correct profile in registry, no pending removal journal, removed/rolled-back/undo-rolled-back/restored receipts retained.

Public API stopped/busy-port recovery test also passed: [log](final-recovery-20261116.log). Native remove/cancel/last-device checks and label fix are recorded in [previous report](last-device-native-20261116.md). Guest package persistence after remove/undo was verified separately by the [134.12-second guest test](../restore/final-guest-20261116.log).
