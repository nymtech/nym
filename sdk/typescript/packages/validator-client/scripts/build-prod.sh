#!/usr/bin/env bash

set -o errexit
set -o nounset

rm -rf ./dist || true
rm -rf ../../dist || true


# Bundle application

pnpm build

# Bundle types

pnpm build:types

# Build package.json for bundle

node ./scripts/buildPackageJson.mjs

# Copy README

cp README.md dist/

# move the output outside of the yarn/npm workspaces

mv ./dist ../../

echo "Output can be found in:"
realpath ../../dist