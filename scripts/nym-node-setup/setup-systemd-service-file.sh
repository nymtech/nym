#!/bin/bash
#set -euo pipefail

# Setup systemd service config file
SERVICE_PATH="/etc/systemd/system/nym-node.service"

echo "Setting up systemd service config file for node automation"

# --- helpers ---
normalize_mode() {
  local input="${1,,}"  # lowercase
  case "$input" in
    1|"mixnode") echo "mixnode" ;;
    2|"entry-gateway"|"entrygateway"|"entry") echo "entry-gateway" ;;
    3|"exit-gateway"|"exitgateway"|"exit") echo "exit-gateway" ;;
    *) echo "" ;;
  esac
}

ensure_mode() {
  local m="${MODE:-}"
  if [[ -z "$m" ]]; then
    read -rp "Select mode (you may type a number or a name): " m
  fi

  m="$(normalize_mode "$m")"
  while [[ -z "$m" ]]; do
    echo "Invalid mode. Allowed: mixnode, entry-gateway, exit-gateway (or 1/2/3)."
    read -rp "Select mode: " m
    m="$(normalize_mode "$m")"
  done

  export MODE="$m"
}

create_service_file() {
  # Create the service file with MODE substituted at write-time
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

  echo "Reloading systemd and enabling service..."
  systemctl daemon-reload && systemctl enable nym-node.service
}

start_service_now() {
  # Start (or restart) the service
  if systemctl is-active --quiet nym-node.service; then
    echo "Service already active. Restarting..."
    systemctl restart nym-node.service
  else
    echo "Starting service..."
    systemctl start nym-node.service
  fi
  systemctl status --no-pager --lines=5 nym-node.service || true
}

# --- main flow ---
ensure_mode

if [[ -f "$SERVICE_PATH" ]]; then
  echo "Service file already exists at: $SERVICE_PATH"
  read -rp "Do you want to start (or restart) the service now? [y/N]: " ans
  if [[ "${ans:-}" =~ ^[Yy]$ ]]; then
    start_service_now
  else
    echo "Okay, not starting the service."
  fi
else
  read -rp "Service file not found. Create it now? [y/N]: " create_ans
  if [[ "${create_ans:-}" =~ ^[Yy]$ ]]; then
    create_service_file
    read -rp "Do you want to start the service now? [y/N]: " start_ans
    if [[ "${start_ans:-}" =~ ^[Yy]$ ]]; then
      start_service_now
    else
      echo "Service created but not started."
    fi
  else
    echo "Not creating the service file."
  fi
fi
