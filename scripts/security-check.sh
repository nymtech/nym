#!/bin/bash

set -e

echo "starting security checks..."

if [ ! -f "package.json" ]; then
    echo "error: package.json not found, please run this script from the project root."
    exit 1 
fi

echo "checking Node.js version..."
if [ -f ".nvmrc" ]; then
    REQUIRED_NODE_VERSION=$(cat .nvmrc)
    CURRENT_NODE_VERSION=$(node --version | sed 's/v//')
    echo "required Node.js version: $REQUIRED_NODE_VERSION"
    echo "current Node.js version: $CURRENT_NODE_VERSION"
    
    if [ "$CURRENT_NODE_VERSION" != "$REQUIRED_NODE_VERSION" ]; then
        echo "warning: Node.js version mismatch, consider using nvm to switch to the required version."
    fi
fi

echo "checking .npmrc configuration..."
if [ ! -f ".npmrc" ]; then
    echo "Error: .npmrc file not found, security configurations are missing."
    exit 1
fi

echo "checking yarn.lock..."
if [ ! -f "yarn.lock" ]; then
    echo "error: yarn.lock not found, run 'yarn install' to generate it."
    exit 1
fi

echo "running yarn audit..."
yarn audit --level moderate

echo "checking for outdated packages..."
yarn outdated || true

echo "verifying package integrity..."
yarn list --depth=0

echo "checking for known vulnerable packages..."
yarn audit --level high

echo "checking package sources..."
yarn list --depth=0 --json | jq -r '.data.trees[] | select(.children) | .children[] | select(.name | test("^https?://(?!registry\\.npmjs\\.org)")) | .name' || true

echo "checks completed successfully!"
echo ""
echo "always use 'yarn install --frozen-lockfile' in production environments"

