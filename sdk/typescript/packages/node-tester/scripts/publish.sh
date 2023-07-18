#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

yarn
yarn build
cd ../../../../dist/ts/sdk/node-tester

cd cjs
echo "Publishing CommonJS package to NPM.."
npm publish --access=public
cd ..

cd esm
echo "Publishing ESM package to NPM.."
npm publish --access=public
cd ..
