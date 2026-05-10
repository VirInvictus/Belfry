#!/usr/bin/env bash
# Belfry regression gate — fmt + clippy + tests.
# Run before pushing: `scripts/regression.sh`.
set -euo pipefail

cd "$(dirname "$0")/.."

echo "::group::cargo fmt --all --check"
cargo fmt --all --check
echo "::endgroup::"

echo "::group::cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings
echo "::endgroup::"

echo "::group::cargo test --workspace"
cargo test --workspace
echo "::endgroup::"

echo "regression: passed"
