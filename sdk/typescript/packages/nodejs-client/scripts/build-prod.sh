#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true
rm -rf ../../../../dist/ts/sdk/nodejs-client || true

# run the build
scripts/build.sh
node scripts/buildPackageJson.mjs

# move the output outside of the yarn/npm workspaces
mkdir -p ../../../../dist/ts/sdk
mv dist ../../../../dist/ts/sdk
mv ../../../../dist/ts/sdk/dist ../../../../dist/ts/sdk/nodejs-client

echo "Output can be found in:"
realpath ../../../../dist/ts/sdk/nodejs-client
