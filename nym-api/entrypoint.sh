#!/bin/sh

set -e

# Optional: delete existing config and force reinit
if [ "$NYM_API_RESET_CONFIG" = "true" ]; then
    echo "RESET_CONFIG enabled - removing existing configuration..."
    rm -rf ~/.nym/nym-api/default/*
fi

# Init can fail if the mounted volume already has a config
/usr/src/nym/target/release/nym-api init --mnemonic "$MNEMONIC" || true && /usr/src/nym/target/release/nym-api run --mnemonic "$MNEMONIC"
