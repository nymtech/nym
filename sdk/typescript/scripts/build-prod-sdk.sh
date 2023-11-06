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

# enable dev mode and then install dev packages
yarn dev:on
yarn

# build the Typescript SDK packages
yarn build:ci:sdk

# build documentation
yarn docs:prod:build

# turn dev mode off
yarn dev:off
