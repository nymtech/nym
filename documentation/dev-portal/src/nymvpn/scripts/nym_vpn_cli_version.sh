#!/bin/bash

release_url="https://api.github.com/repos/nymtech/nym-vpn-client/releases"
current_cli_version=$(curl -s $release_url | jq -r '.[].tag_name' | grep '^nym-vpn-cli-v' | sort -Vr | head -n 1 | awk -F'-v' '{print $NF}')

echo "${current_cli_version}"
