#!/bin/sh

set -e

# Init can fail if the mounted volume already has a config
/usr/src/nym/target/release/nym-api init --mnemonic "$MNEMONIC" || true && /usr/src/nym/target/release/nym-api run --mnemonic "$MNEMONIC"
