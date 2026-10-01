#!/usr/bin/env bash
# Development-only fixture. Production hosts supply app-core views and effects.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example history --target wasm32-unknown-unknown
TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/history
wasm-bindgen --target web --no-typescript --out-dir out/history/pkg \
  "$TARGET_DIR/wasm32-unknown-unknown/debug/examples/history.wasm"
cp crates/web-ui/assets/theme.css crates/web-ui/assets/history.css out/history/
cp crates/web-ui/examples/history.html out/history/index.html
cat > out/history/loader.js <<'JS'
import init from './pkg/history.js';
await init();
JS
printf 'History fixture built: serve out/history with an HTTP server.\n'
