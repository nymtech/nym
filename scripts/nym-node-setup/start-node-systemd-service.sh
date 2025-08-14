#!/bin/bash
#set -euo pipefail

SERVICE="nym-node.service"

reload_and_reset() {
  # Keep going if these fail, we just want best-effort freshness
  systemctl daemon-reload || true
  systemctl reset-failed "$SERVICE" || true
}

restart_service() {
  reload_and_reset
  systemctl restart "$SERVICE"
  echo "$SERVICE restart requested."
}

start_service() {
  reload_and_reset
  systemctl start "$SERVICE"
  echo "$SERVICE start requested."
}

case "${1:-}" in
  restart)
    restart_service
    ;;
  start)
    start_service
    ;;
  *)
    # Interactive mode: only used when Python detected "not running"
    if systemctl is-active --quiet "$SERVICE"; then
      # Normally Python handles the running-case prompt, but keep this safe-guard:
      read -rp "$SERVICE is running. Restart now? [y/N]: " ans
      if [[ "${ans:-}" =~ ^[Yy]$ ]]; then
        restart_service
      else
        echo "Skipping restart."
      fi
    else
      read -rp "$SERVICE is not running. Start now? [y/N]: " ans
      if [[ "${ans:-}" =~ ^[Yy]$ ]]; then
        start_service
      else
        echo "Skipping start."
      fi
    fi
    ;;
esac
