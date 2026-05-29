#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

# Build the worker (which inlines smolmix-wasm bytes via @rollup/plugin-wasm),
# then move the result next to src/index.ts so rollup-plugin-web-worker-loader
# can pick it up and base64-inline it into the main ESM bundle.

rollup -c rollup-worker.config.mjs

rm -f src/worker/*.wasm
mv dist/index.js src/worker/worker.js

mkdir -p dist/worker
mv dist/*.wasm dist/worker 2>/dev/null || true

rollup -c rollup-esm.config.mjs

cp README.md dist/esm 2>/dev/null || true
