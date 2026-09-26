#!/usr/bin/env bash
# Build the local, mock-only gallery. Hosts do not ship this example.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example foundations --target wasm32-unknown-unknown
TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/foundations
wasm-bindgen --target web --no-typescript --out-dir out/foundations/pkg \
  "$TARGET_DIR/wasm32-unknown-unknown/debug/examples/foundations.wasm"
cp crates/web-ui/assets/theme.css crates/web-ui/examples/foundations.css out/foundations/
cp crates/web-ui/examples/foundations.html out/foundations/index.html
cat > out/foundations/loader.js <<'JS'
import init from './pkg/foundations.js';
await init();
JS
printf 'Gallery built: serve out/foundations with an HTTP server.\n'
