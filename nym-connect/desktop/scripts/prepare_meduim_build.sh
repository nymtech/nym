#! /bin/bash

set -e

if ! [[ "$RELEASE_TAG" =~ ^nym-connect-medium-v.*? ]]; then
  echo -e " ✗ Invalid release tag $RELEASE_TAG"
  exit 1
fi

version="${RELEASE_TAG#nym-connect-medium-v}"

sed -i 's/"productName": "nym-connect"/"productName": "NymConnect S"/' src-tauri/tauri.conf.json
sed -i "s/\"version\": \".*\"/\"version\": \"$version\"/" src-tauri/tauri.conf.json
sed -i "s/^version = \".*\"/version = \"$version\"/" src-tauri/Cargo.toml

echo -e " ✓ bump version version"

