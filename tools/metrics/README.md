# Engine metrics

Standalone Rust sampler for a running Android Emulator process. It accepts
`qemu-system-aarch64` and `qemu-system-aarch64-headless`, verifies process identity
on each sample and never starts/stops a guest. No third-party dependencies.

```bash
cargo build --locked --manifest-path tools/metrics/Cargo.toml
# PID, sample count, interval seconds, new output file:
tools/metrics/target/debug/engine-metrics PID 361 10 /path/to/new-run.jsonl
```

Use [the concurrent collector](../../docs/METRICS.md) for one/two engines and
validated overlap. CPU is macOS `ps %cpu` (100% = one core); RSS is KiB for the
selected process, not total VM/GPU memory. This tool does not measure game FPS.

`scripts/check.sh` includes formatting, tests and Clippy for this package.
`target/` is disposable; preserve Cargo.lock. Historical evidence may refer to
its former path `experiments/p0/control`.
