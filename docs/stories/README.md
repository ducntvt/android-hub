# Stories

A **story packet** is a bounded unit of work. It declares:

- The user-visible behavior to implement
- The verification commands that prove it works
- The files / areas expected to change

Stories make work reviewable, resumable, and AI-friendly: an agent can read one file and know what "done" looks like.

## When to write a story

- **Yes**: Normal-sized features, multi-file changes, anything taking more than ~30 minutes of focused work.
- **No**: Tiny fixes (typos, dependency bumps, one-line bug fixes), pure refactors with no behavior change.
- **Split**: Work that spans multiple risk areas (e.g., auth + UI + data) — write separate stories per area.

## Naming

```
docs/stories/US-NNN-short-kebab-title.md
```

`NNN` is the next sequential 3-digit number. `US-` stands for "user story" — keep it even for internal stories so the prefix is uniform.

For larger initiatives, group under an epic folder:

```
docs/stories/epics/E01-domain-name/US-001-...md
```

## Status lifecycle

```
planned  →  in_progress  →  implemented
                         ↘  changed     (contract modified after impl)
                         ↘  retired     (removed from scope)
```

Update the `Status:` field at the top of the story file as work moves.

## Verify-all convention

Every story has a `## Verify` section containing a numbered list of shell commands. To "verify-all", run each command in order; all must exit 0.

Agents (human or AI) can:
- Read the story file
- Execute each command under `## Verify`
- Report which passed/failed

If you want automation, a 10-line bash script can parse and run them — but the convention works fine manually.

## Index

<!-- Optional: list active stories here if useful. Otherwise rely on directory listing. -->

- [US-001 — Kiểm chứng engine Android và hai game trên Mac M2](US-001-validate-android-engine.md) — in_progress, P0 của [kế hoạch tổng thể](../PLAN.md).

- [US-002 — Rust Device Hub](US-002-rust-device-hub.md) — implemented

- [US-003 — Android nằm trong Hub](US-003-embedded-android.md) — implemented

- [US-004 — Tối ưu Hub](US-004-hub-performance.md) — implemented

- [US-005 — Device workspace UI](US-005-device-workspace-ui.md) — implemented

- [US-006 — Ignore stale polls](US-006-ignore-stale-polls.md) — implemented

- [US-007 — Install APK](US-007-install-apk.md) — implemented

- [US-008 — Offline backups](US-008-offline-backups.md) — implemented

- [US-009 — Restore/recovery](US-009-restore-recovery.md) — implemented

- [US-010 — Copy space preflight](US-010-copy-space-preflight.md) — implemented

- [US-011 — Device scroll](US-011-device-scroll.md) — in_progress

- [US-012 — Input release order](US-012-input-release-order.md) — in_progress

- [US-013 — SDK preflight](US-013-sdk-preflight.md) — implemented

- [US-014 — Display orientation](US-014-display-orientation.md) — implemented

- [US-015 — Android clipboard](US-015-android-clipboard.md) — implemented

- [US-016 — Remove device](US-016-remove-device.md) — implemented

- [US-017 — Data root fallback](US-017-data-root.md) — implemented

- [US-018 — Disk usage](US-018-disk-usage.md) — implemented

- [US-019 — Creation recovery](US-019-creation-recovery.md) — implemented

- [US-020 — Creation history UI](US-020-creation-history-ui.md) — implemented

- [US-021 — Create space preflight](US-021-create-space-preflight.md) — implemented

- [US-022 — Concurrent metrics](US-022-concurrent-metrics.md) — implemented

- [US-023 — Engine log rotation](US-023-engine-log-rotation.md) — implemented

- [US-024 — SDK selection](US-024-sdk-selection.md) — implemented

- [US-025 — Managed SDK installer](US-025-managed-sdk-installer.md) — in_progress

- [US-026 — Java selection](US-026-java-selection.md) — implemented

## App lifecycle and scaffold

- [US-027 — EN/VI, button styling and Android image version](US-027-ui-language-polish.md) — implemented
- [US-028 — Hide per-device engine launcher from Dock](US-028-background-engine-launcher.md) — implemented
- [US-029 — Stop Android devices when closing Hub](US-029-stop-on-quit.md) — implemented
- [US-030 — Organize the existing Rust scaffold](US-030-scaffold-organization.md) — implemented
- [US-031 — Rename the project to Android Hub](US-031-project-name.md) — implemented
- [US-032 — Migrate runtime paths and remove VM alias](US-032-retire-vm-alias.md) — implemented

## UX improvements — 2026-11-17

[Roadmap and review mapping](../UX-ROADMAP.md).

- [US-033 — Adaptive Android workspace](US-033-adaptive-workspace.md) — implemented
- [US-034 — Explicit device pair and visible input target](US-034-explicit-device-pair.md) — implemented
- [US-035 — Trạng thái khởi động và đóng Hub theo từng máy](US-035-device-lifecycle-feedback.md) — implemented
- [US-036 — Thông báo và xác nhận không làm co vùng Android](US-036-feedback-and-confirmations.md) — implemented
- [US-037 — Cấu hình có trạng thái chưa lưu và nhóm nội dung rõ ràng](US-037-device-settings-drafts.md) — implemented
- [US-038 — Tạo máy qua biểu mẫu ngắn trước khi ghi dữ liệu](US-038-create-device-form.md) — implemented
- [US-039 — APK và clipboard có đích rõ ràng](US-039-apk-and-clipboard-actions.md) — implemented
- [US-040 — Ngôn ngữ thị giác, bàn phím và VI/EN nhất quán](US-040-desktop-visual-language-accessibility.md) — implemented; native accessibility gap ở US-043
- [US-041 — Kiểm tra kết nối theo yêu cầu, không tự sửa Android](US-041-on-demand-connectivity-check.md) — implemented
- [US-042 — Sao lưu, lưu trữ máy và dung lượng dễ hiểu](US-042-device-data-management-presentation.md) — implemented

## Follow-up từ accessibility audit

- [US-043 — Native accessibility cho control Hub](US-043-native-hub-accessibility.md) — planned; gap framework tách từ US-040, ngoài đợt US-033–042.

## Source maintenance — 2026-11-18

- [US-044 — Rust source consistency](US-044-rust-source-consistency.md) — implemented

- [US-045 — App-owned data and experiments cleanup](US-045-retire-experiments-runtime.md) — implemented

- [US-046 — Private source repository documentation](US-046-private-repository-documentation.md) — implemented

- [US-047 — CI and source releases](US-047-ci-and-source-releases.md) — implemented

## Android 12–16 — implemented

[Epic E01: phạm vi, luồng sử dụng và thứ tự triển khai](epics/E01-multiple-android-versions/README.md).

- [US-048 — Kiểm chứng Android 12 và 16 trên cùng engine](epics/E01-multiple-android-versions/US-048-android-version-compatibility-spike.md) — implemented
- [US-049 — Lưu cố định image Android theo từng máy](epics/E01-multiple-android-versions/US-049-immutable-device-image-identity.md) — implemented
- [US-050 — Cài image Android theo nhu cầu](epics/E01-multiple-android-versions/US-050-on-demand-android-image-installation.md) — implemented
- [US-051 — Chọn phiên bản Android khi tạo máy mới](epics/E01-multiple-android-versions/US-051-choose-android-version-at-creation.md) — implemented
- [US-052 — Quản lý các phiên bản Android đã cài](epics/E01-multiple-android-versions/US-052-manage-installed-android-images.md) — implemented
- [US-053 — Hoàn thiện tương thích và nghiệm thu Android 12–16](epics/E01-multiple-android-versions/US-053-qualify-five-android-versions.md) — implemented

## Distribution

- [US-054 — App ZIP and DMG releases](US-054-app-and-dmg-release.md) — implemented

- [US-055 — Uninstall Android Hub and owned data](US-055-uninstall-android-hub.md) — implemented
