#! /bin/bash

set -e

if ! [[ "$RELEASE_TAG" =~ ^nym-connect-s-v.*? ]]; then
  echo -e " ✗ Invalid release tag $RELEASE_TAG"
  exit 1
fi

version="${RELEASE_TAG#nym-connect-s-v}"

sed -i "s/^name = \".*\"/name = \"nym-connect-s\"/" src-tauri/Cargo.toml
sed -i "s/^version = \".*\"/version = \"$version\"/" src-tauri/Cargo.toml
sed -i 's/"productName": "nym-connect"/"productName": "NymConnect S"/' src-tauri/tauri.conf.json
sed -i "s/\"version\": \".*\"/\"version\": \"$version\"/" src-tauri/tauri.conf.json

echo -e " ✓ bump version $version"
