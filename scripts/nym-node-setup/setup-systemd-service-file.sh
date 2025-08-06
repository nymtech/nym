#!/bin/bash

# Setup systemd service config file
echo "Setting up systemd service config file for node automation"

cat > /etc/systemd/system/nym-node.service << EOF
[Unit]
Description=Nym Node
StartLimitInterval=350
StartLimitBurst=10

[Service]
User=root
LimitNOFILE=65536
ExecStart=/root/nym-binaries/nym-node run --mode ${MODE} --accept-operator-terms-and-conditions
KillSignal=SIGINT
Restart=on-failure
RestartSec=30

[Install]
WantedBy=multi-user.target

EOF

echo "Service file saved in /etc/systemd/system/nym-node.service, printing it below for control:"

cat /etc/systemd/system/nym-node.service

echo "Restarting systemd to pick up the changes..."

systemctl daemon-reload && systemctl enable nym-node.service
