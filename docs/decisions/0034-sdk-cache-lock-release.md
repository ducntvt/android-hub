# 0034 — Release SDK cache locks explicitly after owned workers finish

- **Status**: Accepted
- **Date**: 2026-11-13
- **Supersedes**: normal-exit close-only release in ADR 0024

## Context

The parallel infrastructure suite reproduced a false busy error in setup_workflow_uses_verified_cache_and_never_selects_result on the second repetition. Each test owns a separate cache. Close-on-exec does not prevent a concurrent fork from temporarily sharing a locked open file description before exec. Closing the parent's descriptor can therefore leave the lock held temporarily.

A deterministic regression uses dup (the same shared-open-file-description lock semantics) to demonstrate that close-only release remains locked while another reference exists. This establishes the mechanism; the original failing process was not traced at syscall level.

## Decision

Use SdkCacheLock for downloader, extraction and quarantine/recovery. Acquire nonblocking; report contention only for WouldBlock and preserve other OS errors. On normal scope exit explicitly unlock before closing. The downloader must finish or kill/reap its owned curl before dropping this guard. TransferChild already provides that ordering on return and unwinding.

SIGKILL does not execute Drop. Curl still inherits the locked descriptor and retains protection until it exits if the Hub process dies. The inherited descriptor is intentionally retained across exec only for curl. An unrelated transient fork reference can no longer prolong normal cache ownership.

## Consequences

Sequential cache operations no longer depend on all unrelated children reaching exec before reacquisition. Explicit unlock affects every shared reference, so moving or dropping the guard while a writer is alive is forbidden. The guard is not Clone. SDK cache contention stays fail-fast; no retry loop hides competing writers.

Other Hub locks are outside this change. No SDK/AVD user data or license settings are modified.

## Verification

- Deterministic regression proves close-only retention, explicit release, rejection while owned, and that dropping a stale reference cannot release a subsequent owner's lock.
- Infrastructure suite: 20 consecutive parallel runs, each 76 passed / 19 ignored; previous failure retained.
- Workspace: 100 passed / 19 ignored. Clippy, build and codesign pass; existing block future-compat warning remains.
- Real curl localhost fixtures pass: resume/ETag, cancel/reap, and parent SIGKILL retaining exclusion through curl exit. Four test outputs include one helper, only three independent scenarios.

See [evidence](../reports/sdk-cache-lock/parallel-repeats.log) and [crash/download checks](../reports/sdk-cache-lock/http.log). Real candidate installation and native workflow remain pending under US-025.

## Extension — 2026-11-17

The parallel workspace suite reproduced the same false-busy symptom in
`failed_probe_does_not_publish_and_retry_uses_cached_archives` at the immediate
retry after an invalid Java path. SDK assembly used a close-only file guard.
Assembly now reuses SdkCacheLock for its install.lock; all owned probes finish
before scope exit. Contention remains fail-fast. The shared-description regression
above covers the release mechanism; no competing SDK installation is bypassed.
