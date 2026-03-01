#!/bin/bash
# This is a local version of a script to add the nym github org as owners of a crate, aside from whoever's CI token is being used.
# If you want to add another team member as backup owner, comment out line 5 and use their github handle and they will be invited to be an owner on crates.io, e.g.:
# TEAM="jstuczyn"
TEAM="github:nymtech:core"

echo "Checking and adding $TEAM as owner to workspace crates..."

cargo workspaces list | while read crate; do
    echo "Checking $crate..."

    if cargo owner --list "$crate" 2>/dev/null | grep -q "$TEAM"; then
        echo "  $TEAM already owns $crate, skipping"
    else
        echo "  Adding $TEAM as owner of $crate..."
        cargo owner --add "$TEAM" "$crate"
        sleep 2
    fi
done

echo "Done!"
