#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

##---------------------------------------------------------------------------
## ✅ Run this script from the root of the repository using `yarn sdk:build`
##---------------------------------------------------------------------------

# use wasm-pack to build packages
yarn build:wasm

# build the Typescript SDK packages
yarn build:ci:sdk

# build documentation
yarn docs:prod:build
