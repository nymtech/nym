#!/bin/bash
# Probe localnet gateways for LP two-hop testing
# Usage: ./scripts/probe-localnet.sh [mode]
# Modes: two-hop (default), single-hop, lp-only

set -e

MODE="${1:-two-hop}"

# Gateway API (localhost mapped ports)
ENTRY_API="127.0.0.1:30004"
EXIT_API="127.0.0.1:30005"

# Get gateway identities from API
ENTRY_ID=$(curl -s "http://${ENTRY_API}/api/v1/host-information" | jq -r '.data.keys.ed25519_identity')
EXIT_ID=$(curl -s "http://${EXIT_API}/api/v1/host-information" | jq -r '.data.keys.ed25519_identity')

if [ -z "$ENTRY_ID" ] || [ "$ENTRY_ID" = "null" ] || [ -z "$EXIT_ID" ] || [ "$EXIT_ID" = "null" ]; then
    echo "Error: Could not get gateway identities from API"
    echo "Make sure localnet is running: container list"
    exit 1
fi

echo "Entry gateway: $ENTRY_ID"
echo "Exit gateway:  $EXIT_ID"
echo "Mode: $MODE"
echo "---"

cargo run -p nym-gateway-probe -- run-local \
    --entry-gateway-identity "$ENTRY_ID" \
    --entry-lp-address '127.0.0.1:41264' \
    --exit-gateway-identity "$EXIT_ID" \
    --exit-lp-address '192.168.65.6:41264' \
    --mode "$MODE" \
    --use-mock-ecash
