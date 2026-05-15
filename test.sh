#!/bin/bash

set -e

echo "=== Running cargo test ==="
cargo test

echo ""
echo "=== Running cargo clippy ==="
cargo clippy -- -D warnings

echo ""
echo "=== Running cargo fmt --check ==="
cargo fmt --check

echo ""
echo "All checks passed!"