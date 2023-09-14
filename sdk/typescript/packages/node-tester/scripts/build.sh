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
rm -f src/worker.js
mv dist/worker.js src/worker.js

#-------------------------------------------------------
# ESM
#-------------------------------------------------------

# build the SDK as a ESM bundle
rollup -c rollup-esm.config.mjs

#-------------------------------------------------------
# COMMON JS
#-------------------------------------------------------
# Some old build systems cannot fully handle ESM or ES2021, so build
# a CommonJS bundle targeting ES5

# build the SDK as a CommonJS bundle
rollup -c rollup-cjs.config.mjs

#-------------------------------------------------------
# ESM (full-fat)
#-------------------------------------------------------

# build the SDK as a ESM bundle (with worker inlined as a blob)
rollup -c rollup-esm-full-fat.config.mjs

#-------------------------------------------------------
# COMMON JS (full-fat)
#-------------------------------------------------------
# Some old build systems cannot fully handle ESM or ES2021, so build
# a CommonJS bundle targeting ES5

# build the SDK as a CommonJS bundle (with worker inlined as a blob)
rollup -c rollup-cjs-full-fat.config.mjs

#-------------------------------------------------------
# CLEAN UP
#-------------------------------------------------------

rm -rf dist/cjs/worker
rm -rf dist/esm/worker
rm -rf dist/cjs-full-fat/worker
rm -rf dist/esm-full-fat/worker

# copy README
cp README.md dist/esm
cp README-CommonJS.md dist/cjs/README.md
cp README-CommonJS-full-fat.md dist/cjs-full-fat/README.md
cp README-full-fat.md dist/esm-full-fat/README.md


