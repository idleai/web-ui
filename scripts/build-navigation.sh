#!/usr/bin/env bash
# Development-only workspace overview using app-core fixtures.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example navigation --target wasm32-unknown-unknown
NAVIGATION_TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/navigation
wasm-bindgen --target web --no-typescript --out-dir out/navigation/pkg \
  "$NAVIGATION_TARGET_DIR/wasm32-unknown-unknown/debug/examples/navigation.wasm"
cp crates/web-ui/assets/theme.css crates/web-ui/assets/history.css crates/web-ui/assets/history-explorer.css crates/web-ui/assets/navigation.css out/navigation/
cp crates/web-ui/examples/navigation.html out/navigation/index.html
cat > out/navigation/loader.js <<'JS'
import init from './pkg/navigation.js';
await init();
JS
printf 'Navigation preview built: serve out/navigation with an HTTP server.\n'
