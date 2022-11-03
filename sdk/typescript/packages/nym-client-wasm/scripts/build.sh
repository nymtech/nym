#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# change to the same directory as the script, then go up one
cd "$(dirname "$0")"
cd ..

# clear out any files and suppress missing file errors
rm nym_client_wasm* package.json || true

# let wasm-pack build the files and put them in the output location rather than `./pkg`
cd ../../../../clients/webassembly
wasm-pack build --scope nymproject --target no-modules --out-dir ../../sdk/typescript/packages/nym-client-wasm

# clean up some files that come with the build
cd ../../sdk/typescript/packages/nym-client-wasm
rm README.md LICENSE_APACHE