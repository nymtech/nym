#!/usr/bin/env bash

set -o errexit
set -o nounset
set -o pipefail

yarn
yarn build
cd ../../../../dist/sdk/sdk

cd cjs
echo "Publishing CommonJS package to NPM.."
npm publish --access=public
cd ..

cd esm
echo "Publishing ESM package to NPM.."
npm publish --access=public
cd ..

cd cjs-full-fat
echo "Publishing CommonJS (Full Fat) package to NPM.."
npm publish --access=public
cd ..

cd esm-full-fat
echo "Publishing ESM (Full Fat) package to NPM.."
npm publish --access=public
cd ..
