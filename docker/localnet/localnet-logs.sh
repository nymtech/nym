#!/bin/bash

# Tmux-based log viewer for Nym Localnet containers
# Shows all container logs in a multi-pane layout

SESSION_NAME="nym-localnet-logs"

# Container names
CONTAINERS=(
    "nym-mixnode1"
    "nym-mixnode2"
    "nym-mixnode3"
    "nym-gateway"
    "nym-network-requester"
    "nym-socks5-client"
)

# Check if containers are running
running_containers=()
for container in "${CONTAINERS[@]}"; do
    if container inspect "$container" &>/dev/null; then
        running_containers+=("$container")
    fi
done

if [ ${#running_containers[@]} -eq 0 ]; then
    echo "Error: No containers are running"
    echo "Start the localnet first: ./localnet.sh start"
    exit 1
fi

# Check if we're already in tmux
if [ -n "$TMUX" ]; then
    # Inside tmux - create new window
    tmux new-window -n "logs" "container logs -f ${running_containers[0]}"

    # Split for remaining containers
    for ((i=1; i<${#running_containers[@]}; i++)); do
        tmux split-window -t logs "container logs -f ${running_containers[$i]}"
        tmux select-layout -t logs tiled
    done

    tmux select-layout -t logs tiled
else
    # Not in tmux - check if session exists
    if tmux has-session -t "$SESSION_NAME" 2>/dev/null; then
        # Session exists - attach to it
        exec tmux attach-session -t "$SESSION_NAME"
    else
        # Create new session
        tmux new-session -d -s "$SESSION_NAME" -n "logs" "container logs -f ${running_containers[0]}"

        # Split for remaining containers
        for ((i=1; i<${#running_containers[@]}; i++)); do
            tmux split-window -t "$SESSION_NAME:logs" "container logs -f ${running_containers[$i]}"
            tmux select-layout -t "$SESSION_NAME:logs" tiled
        done

        tmux select-layout -t "$SESSION_NAME:logs" tiled

        # Attach to the session
        exec tmux attach-session -t "$SESSION_NAME"
    fi
fi
