#!/bin/bash

set -e

echo "Running cargo tests..."
cargo test

echo ""
echo "All tests passed!"