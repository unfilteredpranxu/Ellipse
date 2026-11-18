#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
if ! command -v cargo >/dev/null; then
  curl --proto '=https' --tlsv1.2 -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal
  export PATH="$HOME/.cargo/bin:$PATH"
fi
rustup target add wasm32-unknown-unknown
if ! command -v trunk >/dev/null; then cargo install trunk --locked; fi
mkdir -p web/assets
cp -R site/assets/. web/assets/
(cd web && trunk build --release --locked --dist ../dist)
cp site/favicon.svg site/manifest.webmanifest site/sw.js site/_headers dist/
if commit=$(git rev-parse --verify HEAD 2>/dev/null); then
  printf '{"commit":"%s"}\n' "$commit" > dist/version.json
fi
echo "Ellipse Leptos/Rust WebAssembly output: dist/"
