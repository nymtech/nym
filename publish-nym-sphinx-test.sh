#!/bin/bash
set -e

# nym-sphinx and its sub-crates
PACKAGES=(
  "nym-sphinx-acknowledgements"
  "nym-sphinx-addressing"
  "nym-sphinx-anonymous-replies"
  "nym-sphinx-chunking"
  "nym-sphinx-cover"
  "nym-sphinx-forwarding"
  "nym-sphinx-framing"
  "nym-sphinx-params"
  "nym-sphinx-routing"
  "nym-sphinx-types"
  "nym-sphinx"
)

PACKAGE_FLAGS=""
for pkg in "${PACKAGES[@]}"; do
  PACKAGE_FLAGS="$PACKAGE_FLAGS -p $pkg"
done

cargo release \
  $PACKAGE_FLAGS \
  --prev-tag-name "" \
  --no-push \
  --no-tag \
  --allow-branch '*' \
  -v \
  "$@"
