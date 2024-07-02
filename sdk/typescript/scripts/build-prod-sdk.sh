#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

##---------------------------------------------------------------------------
## ✅ Run this script from the root of the repository using `yarn sdk:build`
##---------------------------------------------------------------------------

# use wasm-pack to build packages
pnpm build:wasm

# enable dev mode and then install dev packages
pnpm dev:on
pnpm

# build the Typescript SDK packages
pnpm build:ci:sdk

# build documentation
pnpm docs:prod:build

# turn dev mode off
pnpm dev:off
