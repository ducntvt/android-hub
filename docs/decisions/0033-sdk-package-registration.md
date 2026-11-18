# 0033 — Register assembled emulator packages before publication

- **Status**: Accepted
- **Date**: 2026-11-13
- **Supersedes**: none

## Context

Installed avdmanager 23.0 recognizes the API 36 Play image from source.properties, but refuses AVD creation when the emulator has only source.properties. A disposable SDK experiment also found that creation requires emulator/lib/hardware-properties.ini and image data/empty_data_disk (or legacy userdata.img).

## Decision

Before probing and publishing a private assembled SDK, preserve any existing regular, nonempty emulator/package.xml. If absent, create a minimal Android generic local-package descriptor using the already verified numeric emulator revision. Write and sync it only inside unpublished staging. Do not write SDK license acceptance files or modify the selected SDK.

Package validation requires hardware-properties.ini and a userdata source. Prefer the empty_data_disk marker when present; reject an invalid marker rather than silently falling back. Creation preflight checks these files as well.

## Consequences

The assembled SDK can be recognized by avdmanager without running sdkmanager installation or modifying upstream archives. Existing upstream XML is preserved, not semantically validated by this change. Real candidate archive installation and guest boot remain required before declaring the installer complete.

## Alternatives considered

Using source.properties alone failed with the installed avdmanager. Copying metadata from the user's SDK risks describing a different revision. Running sdkmanager introduces a separate installation and license workflow.

## Verification

The ignored real-avdmanager regression clones installed tool libraries into a temporary SDK, first reproduces missing-emulator rejection, then creates disposable AVD metadata with production-generated XML and the empty-data marker. Fake image payloads and absence of an emulator executable ensure this is not a boot test. Original SDK metadata fingerprints remain unchanged. No SDK archive download or license acceptance occurs.

Evidence: [real tool run](../reports/sdk-registration/real-avdmanager.log), [assembly crash boundaries](../reports/sdk-registration/crash.log).
