#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# change to the same directory as the script, then go up one
cd "$(dirname "$0")"
cd ..

# clear out any files and suppress missing file errors
rm -f nym_node_tester_wasm* package.json || true

# let wasm-pack build the files and put them in the output location rather than `./pkg`
cd ../../../../wasm/node-tester
wasm-pack build --scope nymproject --target web --out-dir ../../sdk/typescript/packages/nym-node-tester-wasm

# run wasm-opt manually to circumvent wasm-pack issues with Apple Silicon
cd ../../sdk/typescript/packages/nym-client-wasm
wasm-opt -O4 nym_node_tester_wasm_bg.wasm -o nym_node_tester_wasm_bg.wasm

# clean up some files that come with the build
rm README.md LICENSE_APACHE
