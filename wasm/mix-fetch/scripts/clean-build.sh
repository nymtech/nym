#!/usr/bin/env bash
#
# Clean build of mix-fetch: removes all build artifacts, rebuilds WASM (debug),
# installs internal-dev dependencies, and starts the webpack dev server.
#
# Usage (from wasm/mix-fetch):
#   scripts/clean-build.sh
#
# Or from anywhere:
#   ./wasm/mix-fetch/scripts/clean-build.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WASM_MIX_FETCH="${SCRIPT_DIR}/.."
REPO_ROOT="${WASM_MIX_FETCH}/../.."
INTERNAL_DEV="${WASM_MIX_FETCH}/internal-dev"

echo "==> Cleaning build artifacts..."
rm -rf "${WASM_MIX_FETCH}/pkg"
rm -rf "${WASM_MIX_FETCH}/go-mix-conn/build"
rm -rf "${REPO_ROOT}/dist/wasm/mix-fetch"
rm -rf "${INTERNAL_DEV}/node_modules"
rm -rf "${INTERNAL_DEV}/dist"

echo "==> Building WASM (Go + Rust debug)..."
make -C "${WASM_MIX_FETCH}/go-mix-conn" build-debug-dev
# Touch lib.rs to force Cargo to recompile and pick up a fresh build timestamp
touch "${WASM_MIX_FETCH}/src/lib.rs"
cd "${WASM_MIX_FETCH}"
make build-rust-debug

echo "==> Installing internal-dev dependencies..."
cd "${INTERNAL_DEV}"
npm install

echo "==> Starting internal-dev webpack dev server on port 3000..."
npm start
