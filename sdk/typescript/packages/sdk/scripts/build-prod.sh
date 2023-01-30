#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true
rm -rf ../../../../dist || true

# run the build
scripts/build.sh
node scripts/buildPackageJson.mjs

# move the output outside of the yarn/npm workspaces
mkdir -p ../../../../dist
mv dist ../../../../dist
mv ../../../../dist/dist ../../../../dist/sdk

echo "Output can be found in:"
realpath ../../../../dist/sdk
