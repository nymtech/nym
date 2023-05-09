#!/usr/bin/env bash

set -o errexit
set -o nounset

rm -rf ./dist || true
rm -rf ../../dist || true


# Bundle application

npm run build

# Bundle types

npm run build:types

# Build package.json for bundle

node ./scripts/buildPackageJson.mjs

# Copy README

cp README.md dist/nym-validator-client

# move the output outside of the npm workspaces

mv ./dist ../../

echo "Output can be found in:"
realpath ../../dist