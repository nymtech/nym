#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

#-------------------------------------------------------
# WEB WORKER (WASM client)
#-------------------------------------------------------
# The web worker needs to be bundled because the WASM bundle needs to be loaded synchronously and all dependencies
# must be included in the worker script (because it is not loaded as an ES Module)

# build the worker
rollup -c rollup-worker.config.mjs

# move it next to the Typescript `mixnet/wasm/index.ts` so it can be inlined by rollup
rm -f src/mixnet/wasm/worker.js
mv dist/worker.js src/mixnet/wasm/worker.js

#-------------------------------------------------------
# WEB WORKER (Node tester)
#-------------------------------------------------------
# The web worker needs to be bundled because the WASM bundle needs to be loaded synchronously and all dependencies
# must be included in the worker script (because it is not loaded as an ES Module)

# build the worker
rollup -c rollup-node-tester-worker.config.mjs

# move it next to the Typescript `mixnet/node-tester/index.ts` so it can be inlined by rollup
rm -f src/mixnet/node-tester/worker.js
mv dist/worker.js src/mixnet/node-tester/worker.js

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
# FULL FAT
#-------------------------------------------------------

# build the SDK as a ESM bundle
rollup -c rollup-full-fat.config.mjs

#-------------------------------------------------------
# CLEAN UP
#-------------------------------------------------------

# copy README
cp README.md dist/esm
cp README-CommonJS.md dist/cjs/README.md


