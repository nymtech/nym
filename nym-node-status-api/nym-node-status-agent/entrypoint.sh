#!/bin/sh

echo "Starting agent loop with sleep interval: ${SLEEP_TIME}s"

# Trap SIGTERM to allow graceful shutdown
trap "echo 'Stopping...'; exit 0" SIGTERM

# Run probe in an infinite loop
while true; do
    /nym/nym-node-status-agent run-probe
    sleep "$SLEEP_TIME"
done