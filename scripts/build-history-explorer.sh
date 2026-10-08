#!/usr/bin/env bash
# Development-only views; production hosts supply their app-core state.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example history_explorer --target wasm32-unknown-unknown
EXPLORER_TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/history-explorer
wasm-bindgen --target web --no-typescript --out-dir out/history-explorer/pkg \
  "$EXPLORER_TARGET_DIR/wasm32-unknown-unknown/debug/examples/history_explorer.wasm"
cp crates/web-ui/assets/{theme,history,history-details,history-explorer,navigation}.css out/history-explorer/
cp crates/web-ui/examples/history_explorer.html out/history-explorer/index.html
cat > out/history-explorer/loader.js <<'JS'
import init from './pkg/history_explorer.js';
await init();
JS
