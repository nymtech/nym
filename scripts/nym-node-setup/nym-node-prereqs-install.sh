#!/bin/bash

if [[ "$(id -u)" -ne 0 ]]; then
  echo "This script must be run as root."
  exit 1
fi

# update, upgrade and install dependencies
echo -e "\n* * * Installing needed prerequisities * * *"

apt update  -y && apt --fix-broken install
apt upgrade
apt install apt ca-certificates jq curl wget ufw jq tmux pkg-config build-essential libssl-dev git ntp ntpdate neovim tree tmux tig nginx -y
