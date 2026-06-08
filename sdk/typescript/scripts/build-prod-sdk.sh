#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

##---------------------------------------------------------------------------
## ✅ Run this script from the root of the repository using `pnpm sdk:build`
##---------------------------------------------------------------------------

# use wasm-pack to build packages
pnpm build:wasm

# enable dev mode and then install dev packages
#
# `--no-frozen-lockfile` is required: dev:on injects the four smolmix-family
# packages (plus wasm/smolmix/pkg) as new workspace importers that the committed
# lockfile does not know about. CI sets CI=true, which makes a bare `pnpm install`
# default to frozen and fail with ERR_PNPM_OUTDATED_LOCKFILE. Use the `--no-`
# form, not `--frozen-lockfile false`: pnpm parses the bare `false` as a separate
# argument and ignores it.
pnpm dev:on
pnpm install --no-frozen-lockfile

# build the Typescript SDK packages
pnpm build:ci:sdk

# build documentation
#pnpm docs:prod:build

# turn dev mode off
pnpm dev:off
