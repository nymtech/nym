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
cp dist/index.js src/worker/worker.js
rm dist/index.js

#-------------------------------------------------------
# ESM
#-------------------------------------------------------

# build the SDK as a ESM bundle
rollup -c rollup-esm.config.mjs

#-------------------------------------------------------
# CLEAN UP
#-------------------------------------------------------

rm -rf dist/esm/worker


