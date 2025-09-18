#!/bin/sh

echo "Starting agent loop with sleep interval: ${SLEEP_TIME}s"

# Trap SIGTERM to allow graceful shutdown
trap "echo 'Stopping...'; exit 0" SIGTERM

DEFAULT_ARGS="run-probe --server \"${NODE_STATUS_AGENT_SERVER_ADDRESS}|${NODE_STATUS_AGENT_SERVER_PORT}\" --mnemonic \"${NYM_NODE_MNEMONICS}\""
ARGS=${NODE_STATUS_AGENT_ARGS:-${DEFAULT_ARGS}}
COMMAND="/nym/nym-node-status-agent ${ARGS}"

echo "default_args = '${DEFAULT_ARGS}'"
echo "args = '${ARGS}'"
echo "command = '${COMMAND}'"

# Run probe in an infinite loop
while true; do
    eval $COMMAND
    sleep "$SLEEP_TIME"
done
