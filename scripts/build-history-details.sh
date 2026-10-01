#!/usr/bin/env bash
# Development-only scripted responses; production hosts execute app-core effects.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example history_details --target wasm32-unknown-unknown
TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/history-details
wasm-bindgen --target web --no-typescript --out-dir out/history-details/pkg \
  "$TARGET_DIR/wasm32-unknown-unknown/debug/examples/history_details.wasm"
cp crates/web-ui/assets/theme.css crates/web-ui/assets/history.css crates/web-ui/assets/history-details.css out/history-details/
cp crates/web-ui/examples/history_details.html out/history-details/index.html
cat > out/history-details/loader.js <<'JS'
import init from './pkg/history_details.js';
await init();
JS
printf 'History details fixture built: serve out/history-details with an HTTP server.\n'
