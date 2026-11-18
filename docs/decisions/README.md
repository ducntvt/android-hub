# Architecture Decision Records (ADRs)

Durable records of architectural choices. Captures **why** we picked X over Y so the next agent (human or AI) doesn't have to re-derive it from `git log`.

## When to write one

Write an ADR when you make a decision that:

- Picks one library/approach over another viable alternative
- Changes a layer boundary, dependency direction, or interface contract
- Affects auth, authorization, data persistence, or external integrations
- Removes a previously documented constraint or pattern

Skip an ADR for: bug fixes, refactors that preserve behavior, dependency bumps with no API change, doc-only edits.

## Workflow

1. Copy `template.md` to `NNNN-short-kebab-title.md` (NNNN = next sequential 4-digit number).
2. Fill in: Context, Decision, Consequences, Alternatives considered.
3. Set status to **Proposed**, open a PR.
4. After review, update status to **Accepted** and merge.
5. If later superseded, set status to **Superseded by NNNN** and link the new ADR.

## Status values

- **Proposed** — under discussion
- **Accepted** — current guidance
- **Superseded** — replaced by a later ADR (link to it)
- **Deprecated** — no longer relevant, kept for history

## Index

- [0051 — Data catalogs and usage presentation](0051-data-catalog-and-usage-presentation.md) — Accepted

<!-- Keep this list in sync. One line per ADR. -->

- [0001 — Record architecture decisions](0001-record-architecture-decisions.md) — Accepted
- [0002 — Xây Android VM cá nhân bằng Rust và kiểm chứng engine trước](0002-build-personal-android-vm-with-rust.md) — Deprecated

- [0003 — Dùng minifb cho cửa sổ kiểm chứng P0](0003-use-minifb-for-p0-viewer.md) — Deprecated

- [0004 — Rust Device Hub manager](0004-rust-device-hub-manager.md) — Accepted

- [0005 — Embedded emulator display](0005-embedded-emulator-display.md) — Accepted

- [0006 — Frame notifications](0006-frame-notifications.md) — Accepted

- [0007 — Explicit APK install](0007-explicit-apk-install.md) — Accepted

- [0008 — Verified offline backups](0008-verified-offline-backups.md) — Accepted

- [0009 — Restore compatibility](0009-restore-compatibility.md) — Accepted

- [0010 — Journaled restore](0010-journaled-restore.md) — Accepted

- [0011 — Copy space preflight](0011-copy-space-preflight.md) — Accepted

- [0012 — Device wheel input](0012-device-wheel-input.md) — Accepted

- [0013 — Release order](0013-release-order.md) — Accepted

- [0014 — Framebuffer orientation](0014-framebuffer-orientation.md) — Accepted

- [0015 — Explicit clipboard](0015-explicit-clipboard.md) — Accepted

- [0016 — Remove device](0016-remove-device.md) — Accepted

- [0017 — Data root fallback](0017-data-root-fallback.md) — Accepted

- [0018 — On-demand disk usage](0018-on-demand-disk-usage.md)

- [0019 — Creation journal](0019-creation-journal.md)

- [0020 — Create space policy](0020-create-space-policy.md)

- [0021 — Engine log retention](0021-engine-log-retention.md)

- [0022 — SDK preferences](0022-sdk-preferences.md)

- [0023 — Managed SDK plan](0023-managed-sdk-plan.md)

- [0024 — SDK archive staging](0024-sdk-archive-staging.md)

- [0025 — SDK extraction](0025-sdk-extraction.md)

- [0026 — SDK tool probe](0026-sdk-tool-probe.md)

- [0027 — Java preferences](0027-java-preferences.md)

- [0028 — Managed SDK assembly](0028-managed-sdk-assembly.md)

- [0029 — Managed SDK discovery](0029-managed-sdk-discovery.md)

- [0030 — SDK plan review](0030-sdk-plan-review.md)

- [0031 — SDK setup workflow](0031-sdk-setup-workflow.md)

- [0032 — SDK cache quarantine](0032-sdk-cache-quarantine.md)

- [0033 — SDK package registration](0033-sdk-package-registration.md)

- [0034 — SDK cache lock release](0034-sdk-cache-lock-release.md)

- [0035 — Combined pointer and touch (rejected after QA)](0035-combined-pointer-touch.md)

- [0036 — Framework scroll bridge](0036-framework-scroll-bridge.md) — Accepted for API 36; native and user trackpad evidence, remaining limits in ADR.

- [0039 — Stop engines on Quit](0039-stop-engines-on-quit.md)

- [0040 — Organize application modules and owned resources](0040-scaffold-organization.md)

- [0041 — Android Hub project name](0041-android-hub-project-name.md)

- [0042 — Retire VM path alias](0042-retire-vm-path-alias.md)

- [0043 — Adaptive device workspace](0043-adaptive-device-workspace.md)

- [0044 — Observe boot and shutdown progress](0044-observe-boot-and-shutdown-progress.md)

- [0045 — Modal feedback and confirmed stop](0045-modal-feedback-and-confirmed-stop.md)

- [0046 — Settings drafts and optimistic save](0046-settings-drafts-and-optimistic-save.md)

- [0047 — Create request and registered result](0047-create-request-and-registered-result.md)

- [0048 — Native APK picker and explicit drop target](0048-native-apk-picker-and-explicit-drop-target.md)

- [0049 — Host appearance and semantic palette](0049-host-appearance-and-semantic-palette.md) — Accepted

- [0050 — Keyboard focus and scroll reveal](0050-keyboard-focus-and-scroll-reveal.md) — Accepted

- [0052 — Guest connectivity probe](0052-guest-connectivity-probe.md) — Accepted

- [0053 — Consistent named Rust modules](0053-rust-module-conventions.md) — Accepted

- [0054 — App-owned data root after authorized reset](0054-app-owned-data-root.md) — Accepted
