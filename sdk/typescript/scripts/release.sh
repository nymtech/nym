#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

# Build WASM packages
make sdk-wasm-build

# Build Typescript packages
pnpm i
pnpm build:sdk

# Publish to NPM
./sdk/typescript/scripts/publish.sh
