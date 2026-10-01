#!/usr/bin/env bash
# Development-only responses behind app-core's session effect boundary.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example sessions --target wasm32-unknown-unknown
TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/sessions
wasm-bindgen --target web --no-typescript --out-dir out/sessions/pkg \
  "$TARGET_DIR/wasm32-unknown-unknown/debug/examples/sessions.wasm"
cp crates/web-ui/assets/theme.css crates/web-ui/assets/history-details.css crates/web-ui/assets/sessions.css out/sessions/
cp crates/web-ui/examples/sessions.html out/sessions/index.html
cat > out/sessions/loader.js <<'JS'
import init from './pkg/sessions.js';
await init();
JS
printf 'Session fixture built: serve out/sessions with an HTTP server.\n'
