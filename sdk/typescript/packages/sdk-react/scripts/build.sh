#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true

# build
yarn build:tsc

# add README and prod package.json
node scripts/buildPackageJson.mjs
cp README.md dist

# move the output outside of the yarn/npm workspaces
mkdir -p ../../../../dist/ts/sdk-react
mv dist ../../../../dist/ts/sdk-react
mv ../../../../dist/ts/sdk-react/dist ../../../../dist/ts/sdk/sdk-react

echo "Output can be found in:"
realpath ../../../../dist/ts/sdk/sdk-react

