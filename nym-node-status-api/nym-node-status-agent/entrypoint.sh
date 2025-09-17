#!/bin/sh

echo "Starting agent loop..."

# Trap SIGTERM to allow graceful shutdown
trap "echo 'Stopping...'; exit 0" SIGTERM

# Run probe in an infinite loop
while true; do
    /nym/nym-node-status-agent run-probe
    sleep 5
done