#!/usr/bin/env bash

#-------------------------------------------------------
# (WASM client)
#-------------------------------------------------------
# Copy package placeholders to the WASM client package

cp scripts/build/yarn/wasm-placeholder/package.json sdk/typescript/packages/nym-client-wasm

#-------------------------------------------------------
# (Browser Extension client)
#-------------------------------------------------------
# Copy package placeholders to the Browser Extension client package

mkdir -p nym-browser-extension/storage/pkg
cp scripts/build/yarn/storage-placeholder/package.json nym-browser-extension/storage/pkg

#-------------------------------------------------------
# (Node Tester client)
#-------------------------------------------------------
# Copy package placeholders to the Browser Extension client package

cp scripts/build/yarn/node-tester-wasm-placeholder/package.json sdk/typescript/packages/nym-node-tester-wasm

#-------------------------------------------------------