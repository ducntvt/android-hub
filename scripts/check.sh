#!/bin/bash
# Local checks only: no GUI, guest boot, SDK install or ignored integration tests.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
export PYTHONDONTWRITEBYTECODE=1

cargo fmt --all -- --check
cargo fmt --manifest-path tools/metrics/Cargo.toml -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --manifest-path tools/metrics/Cargo.toml --locked
cargo clippy --manifest-path tools/metrics/Cargo.toml --all-targets --locked -- -D warnings
python3 -m unittest discover -s scripts -p 'test_*.py'
python3 scripts/check-docs.py
for script in scripts/*.sh; do
    bash -n "$script"
done
