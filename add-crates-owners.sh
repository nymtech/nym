#!/bin/bash

TEAM="github:nymtech:core"

echo "Adding $TEAM as owner to all workspace crates..."

cargo workspaces list | while read crate; do
    echo "Adding $TEAM as owner of $crate..."
    cargo owner --add "$TEAM" "$crate"
    sleep 2
done

echo "Done!"
