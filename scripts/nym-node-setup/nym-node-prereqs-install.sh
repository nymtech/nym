#!/bin/bash


# Update, upgrade & install dependencies
apt update  -y && apt --fix-broken install
apt upgrade
apt -y install apt -y install ca-certificates jq curl wget ufw jq tmux pkg-config build-essential libssl-dev git ntp ntpdate neovim tree tmux tig nginx -y
apt install ufw --fix-missing


# Enable & setup firewall
ufw enable
ufw allow 22/tcp    # SSH - you're in control of these ports
ufw allow 80/tcp    # HTTP
ufw allow 443/tcp   # HTTPS
ufw allow 1789/tcp  # Nym specific
ufw allow 1790/tcp  # Nym specific
ufw allow 8080/tcp  # Nym specific - nym-node-api
ufw allow 9000/tcp  # Nym Specific - clients port
ufw allow 9001/tcp  # Nym specific - wss port
ufw allow 51822/udp # WireGuard
ufw allow 'Nginx Full' && \
ufw reload && \
ufw status
