#!/bin/bash
set -e

# nym-sphinx and its sub-crates
# batches for rate limit of 5 new crates per burst
BATCH1=(
  "nym-sphinx-acknowledgements"
  "nym-sphinx-addressing"
  "nym-sphinx-anonymous-replies"
  "nym-sphinx-chunking"
  "nym-sphinx-cover"
)

BATCH2=(
  "nym-sphinx-forwarding"
  "nym-sphinx-framing"
  "nym-sphinx-params"
  "nym-sphinx-routing"
  # "nym-sphinx-types"
)

BATCH3=(
  "nym-sphinx"
)

publish_batch() {
  local -n batch_ref=$1
  shift

  PACKAGE_FLAGS=""
  for pkg in "${batch_ref[@]}"; do
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
}

echo "Publishing batch 1 "
publish_batch BATCH1 "$@"

if [[ "$*" == *"--execute"* ]]; then
  echo "Waiting 600 seconds (10 minutes) before next batch..."
  sleep 600
fi

echo "Publishing batch 2 "
publish_batch BATCH2 "$@"

if [[ "$*" == *"--execute"* ]]; then
  echo "Waiting 600 seconds (10 minutes) before final batch..."
  sleep 600
fi

echo "Publishing batch 3 (1 crate)..."
publish_batch BATCH3 "$@"

echo "All packages published successfully!"
