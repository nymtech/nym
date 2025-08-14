#!/bin/bash
set -euo pipefail

SERVICE="nym-node.service"
SYSTEMCTL="systemctl --no-ask-password --quiet"
TIMEOUT_BIN="$(command -v timeout || true)"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-600}"   # seconds

reload_and_reset() {
  $SYSTEMCTL daemon-reload || true
  $SYSTEMCTL reset-failed "$SERVICE" || true
}

do_wait_cmd() {
  if [[ -n "$TIMEOUT_BIN" && "$WAIT_TIMEOUT" -gt 0 ]]; then
    $TIMEOUT_BIN "${WAIT_TIMEOUT}s" "$@"
  else
    "$@"
  fi
}

restart_wait() {
  reload_and_reset
  echo "Restarting $SERVICE and waiting (timeout=${WAIT_TIMEOUT}s)..."
  do_wait_cmd systemctl --no-ask-password restart --wait "$SERVICE"
  echo "$SERVICE is $(systemctl show -p ActiveState --value "$SERVICE")"
}

start_wait() {
  reload_and_reset
  echo "Starting $SERVICE and waiting (timeout=${WAIT_TIMEOUT}s)..."
  do_wait_cmd systemctl --no-ask-password start --wait "$SERVICE"
  echo "$SERVICE is $(systemctl show -p ActiveState --value "$SERVICE")"
}

case "${1:-}" in
  restart-wait)  restart_wait ;;
  start-wait)    start_wait   ;;
  *)             echo "Usage: $0 {start-wait|restart-wait}"; exit 2 ;;
esac
