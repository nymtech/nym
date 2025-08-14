#!/bin/bash
# setup_service.sh
set -euo pipefail

SERVICE_PATH="/etc/systemd/system/nym-node.service"

# Flags / env for non-interactive runs:
#   NONINTERACTIVE=1  -> do not prompt; create if missing
#   MODE=<mixnode|entry-gateway|exit-gateway|1|2|3>

normalize_mode() {
  local input="${1,,}"
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
    read -rp "Select mode: " m
  fi
  m="$(normalize_mode "$m")"
  while [[ -z "$m" ]]; do
    echo "Invalid mode. Allowed: mixnode, entry-gateway, exit-gateway (or 1/2/3)."
    read -rp "Select mode: " m
    m="$(normalize_mode "$m")"
  done
  MODE="$m"
}

create_service_file() {
  cat > "$SERVICE_PATH" <<EOF
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
# Optional safeguards; tweak or remove if undesired:
# MemoryHigh=800M
# MemoryMax=1G
# MemorySwapMax=1G
# OOMScoreAdjust=500

[Install]
WantedBy=multi-user.target
EOF

  echo "Service file saved in $SERVICE_PATH, printing it below for control:"
  cat "$SERVICE_PATH"
}

echo -e "\n* * * Setting up systemd service config file for node automation * * *"

if [[ -f "$SERVICE_PATH" ]]; then
  echo "Service file already exists at: $SERVICE_PATH"
  echo "* * * Reloading systemd and enabling service..."
  systemctl daemon-reload
  systemctl enable nym-node.service
  exit 0
fi

# Service file missing
if [[ "${NONINTERACTIVE:-}" = "1" ]]; then
  MODE="$(normalize_mode "${MODE:-}")"
  if [[ -z "$MODE" ]]; then
    echo "NONINTERACTIVE=1 requires MODE to be set to 1/2/3 or mixnode|entry-gateway|exit-gateway."
    exit 2
  fi
  create_service_file
  echo "* * * Reloading systemd and enabling service..."
  systemctl daemon-reload
  systemctl enable nym-node.service
  exit 0
fi

# Interactive path:
ensure_mode
read -rp "Service file not found. Create it now? [y/N]: " create_ans
if [[ "${create_ans:-}" =~ ^[Yy]$ ]]; then
  create_service_file
  echo "* * * Reloading systemd and enabling service..."
  systemctl daemon-reload
  systemctl enable nym-node.service
else
  echo "Not creating the service file."
fi
