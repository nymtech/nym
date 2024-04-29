#!/bin/bash

release_url="https://api.github.com/repos/nymtech/nym-vpn-client/releases"
version=$(curl -s $release_url | jq -r '.[].tag_name' | grep '^nym-vpn-desktop-v' | sort -Vr | head -n 1 | awk -F'-v' '{print $NF}')

echo "${version}"
