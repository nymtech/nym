#!/bin/bash

# Setup systemd service config file
SERVICE_PATH="/etc/systemd/system/nym-node.service"

echo "Setting up systemd service config file for node automation"

# Check if the service file already exists
if [[ -f "$SERVICE_PATH" ]]; then
  echo "WARNING: Service file already exists at $SERVICE_PATH"
  echo "Choose what to do:"
  echo "1) Replace existing"
  echo "2) Backup existing and create new"
  echo "3) Cancel"

  read -rp "Press 1, 2, or 3 and enter: " choice

  case "$choice" in
    1)
      echo "Replacing existing service file..."
      ;;
    2)
      backup_path="${SERVICE_PATH}.bak.$(date +%s)"
      echo "Backing up to $backup_path"
      cp "$SERVICE_PATH" "$backup_path"
      ;;
    3)
      echo "Cancelled by user."
      exit 0
      ;;
    *)
      echo "Invalid choice. Aborting."
      exit 1
      ;;
  esac

fi

# Create the service file
cat > "$SERVICE_PATH" << EOF
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

echo "Service file saved in $SERVICE_PATH, printing it below for control:"
cat "$SERVICE_PATH"

echo "Restarting systemd to pick up the changes..."

systemctl daemon-reload && systemctl enable nym-node.service
