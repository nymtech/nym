#!/bin/sh

echo "Starting agent loop with sleep interval: ${SLEEP_TIME}s"

# Trap SIGTERM to allow graceful shutdown
trap "echo 'Stopping...'; exit 0" SIGTERM

DEFAULT_ARGS="run-agent --orchestrator_address \"${NETWORK_MONITOR_AGENT_SERVER_ADDRESS}:${NETWORK_MONITOR_AGENT_SERVER_PORT}\" "
ARGS=${NETWORK_MONITOR_AGENT_ARGS:-${DEFAULT_ARGS}}
COMMAND="/nym/nym-network-monitor-agent ${ARGS}"

echo "default_args = '${DEFAULT_ARGS}'"
echo "args = '${ARGS}'"
echo "command = '${COMMAND}'"

# Run agent in an infinite loop
while true; do
    eval "$COMMAND"
    sleep "$SLEEP_TIME"
done
