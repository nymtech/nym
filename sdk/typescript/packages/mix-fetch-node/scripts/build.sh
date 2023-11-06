#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

#-------------------------------------------------------
# WEB WORKER (mix-fetch WASM)
#-------------------------------------------------------
# The web worker needs to be bundled because the WASM bundle needs to be loaded synchronously and all dependencies
# must be included in the worker script (because it is not loaded as an ES Module)

# build the worker
rollup -c rollup-worker.config.mjs

# move it next to the Typescript `src/index.ts` so it can be inlined by rollup
rm -f src/worker/*.js
rm -f src/worker/*.wasm
mv dist/cjs/index.js src/worker/worker.js

# move WASM files out of build area
mkdir -p dist/worker
mv dist/cjs/*.wasm dist/worker

#-------------------------------------------------------
# COMMON JS
#-------------------------------------------------------
# Some old build systems cannot fully handle ESM or ES2021, so build
# a CommonJS bundle targeting ES5

# build the SDK as a CommonJS bundle
rollup -c rollup-cjs.config.mjs

# move WASM files into place
cp dist/worker/*.wasm dist/cjs

#-------------------------------------------------------
# CLEAN UP
#-------------------------------------------------------

rm -rf dist/cjs/worker

# copy README
cp README.md dist/cjs/README.md


