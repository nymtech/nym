#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

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
mv dist/index.js src/worker/worker.js

# move WASM files out of build area
mkdir -p dist/worker
mv dist/*.wasm dist/worker

#-------------------------------------------------------
# ESM
#-------------------------------------------------------

# build the SDK as a ESM bundle
rollup -c rollup-esm.config.mjs

# move WASM files into place
cp dist/worker/*.wasm dist/esm
node scripts/postBuildReplace.mjs dist

#-------------------------------------------------------
# CLEAN UP
#-------------------------------------------------------

# remove typings that aren't needed
rm -rf dist/esm/worker

# clear staging area
rm -rf dist/worker


