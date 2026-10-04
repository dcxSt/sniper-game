#!/usr/bin/env bash
# Build the wasm module and generate the JS glue into www/pkg.
set -euo pipefail
cd "$(dirname "$0")"
cargo build --target wasm32-unknown-unknown --release
rm -rf www/pkg
wasm-bindgen --target web --no-typescript --out-dir www/pkg target/wasm32-unknown-unknown/release/sniper_game.wasm
if command -v wasm-opt >/dev/null 2>&1; then
  wasm-opt -O3 -o www/pkg/sniper_game_bg.wasm www/pkg/sniper_game_bg.wasm || true
fi
date '+%Y-%m-%d %H:%M:%S' > www/pkg/BUILD   # shown on the title screen so you can tell which build you're running
ls -la www/pkg
