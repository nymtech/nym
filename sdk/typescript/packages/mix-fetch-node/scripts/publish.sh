#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

rm -rf dist || true
rm -rf ../../../../dist || true

yarn
yarn build
cd ../../../../dist/sdk

cd cjs
echo "Publishing CommonJS package to NPM.."
npm publish --access=public
cd ..
