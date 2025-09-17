#!/bin/sh

echo "Starting agent loop with sleep interval: ${SLEEP_TIME}s"

# Trap SIGTERM to allow graceful shutdown
trap "echo 'Stopping...'; exit 0" SIGTERM

# Run probe in an infinite loop
while true; do
    /nym/nym-node-status-agent run-probe --server "${NODE_STATUS_AGENT_SERVER_ADDRESS}|${NODE_STATUS_AGENT_SERVER_PORT}" --mnemonic "${NYM_NODE_MNEMONICS}"
    sleep "$SLEEP_TIME"
done
