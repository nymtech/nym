#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true
rm -rf ../../../../dist/ts/sdk/contract-clients || true

# run the build
npm run build:tsc
node scripts/buildPackageJson.mjs
cp README.md dist

# move the output outside of the yarn/npm workspaces
mkdir -p ../../../../dist/ts/sdk
mv dist ../../../../dist/ts/sdk
mv ../../../../dist/ts/sdk/dist ../../../../dist/ts/sdk/contract-clients

echo "Output can be found in:"
realpath ../../../../dist/ts/sdk/contract-clients
