# Bundled SDK plan

`sdk-install-plan.json` is the single canonical manifest compiled into the app by `src/sdk/plan.rs`. It was moved byte-for-byte from `docs/reports/sdk-installer/install-plan.json` under ADR 0040. This folder contains required resources, not test output.

To review an update, save official Google repository and system-image XML metadata, then generate a new candidate at a new path:

```bash
python3 scripts/prepare-sdk-manifest.py \
  --repository /path/to/repository2-3.xml \
  --image /path/to/sys-img2-3.xml \
  --output /tmp/sdk-install-plan-candidate.json
```

Run from the workspace root. Review package/revision/architecture, checksums, sizes, URLs and license changes before replacing the canonical JSON. The generator refuses to overwrite its output. Keep acceptance false; user consent occurs in the app for the exact license hashes. Run `bash scripts/check.sh` and rebuild after replacing the resource. Do not embed an accepted license state or add a second live manifest under reports.
