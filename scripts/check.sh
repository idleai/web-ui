#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/released-dependencies.sh
bash scripts/lint.sh
cargo build --workspace --locked
cargo build --workspace --locked --target wasm32-unknown-unknown
