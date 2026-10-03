#!/usr/bin/env bash
# Development-only presentation scenarios behind app-core's projection effects.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --example projections --target wasm32-unknown-unknown
PROJECTION_TARGET_DIR="$(cargo metadata --locked --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])')"
mkdir -p out/projections
wasm-bindgen --target web --no-typescript --out-dir out/projections/pkg \
  "$PROJECTION_TARGET_DIR/wasm32-unknown-unknown/debug/examples/projections.wasm"
cp crates/web-ui/assets/theme.css crates/web-ui/assets/projections.css out/projections/
cp crates/web-ui/examples/projections.html out/projections/index.html
cat > out/projections/loader.js <<'JS'
import init from './pkg/projections.js';
await init();
JS
printf 'Projection preview built: serve out/projections with an HTTP server.\n'
