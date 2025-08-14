#!/bin/bash
set -euo pipefail

SERVICE="nym-node.service"
export SYSTEMD_PAGER=""
export SYSTEMD_COLORS="0"
SYSTEMCTL="systemctl --no-ask-password --quiet"
WAIT_TIMEOUT="${WAIT_TIMEOUT:-600}"   # seconds

reload_and_reset() {
  $SYSTEMCTL daemon-reload || true
  $SYSTEMCTL reset-failed "$SERVICE" || true
}

print_state() {
  local active sub result
  active="$(systemctl show -p ActiveState --value "$SERVICE" 2>/dev/null || echo unknown)"
  sub="$(systemctl show -p SubState    --value "$SERVICE" 2>/dev/null || echo unknown)"
  result="$(systemctl show -p Result   --value "$SERVICE" 2>/dev/null || echo unknown)"
  echo "state: ActiveState=${active} SubState=${sub} Result=${result}"
}

wait_until_active_or_fail() {
  local deadline=$(( $(date +%s) + WAIT_TIMEOUT ))
  local last=""
  while [ "$(date +%s)" -lt "$deadline" ]; do
    local active sub result
    active="$(systemctl show -p ActiveState --value "$SERVICE" 2>/dev/null || echo unknown)"
    sub="$(systemctl show -p SubState    --value "$SERVICE" 2>/dev/null || echo unknown)"
    result="$(systemctl show -p Result   --value "$SERVICE" 2>/dev/null || echo unknown)"

    local cur="${active}/${sub}/${result}"
    if [ "$cur" != "$last" ]; then
      echo "state: ActiveState=${active} SubState=${sub} Result=${result}"
      last="$cur"
    fi

    # success
    if [ "$active" = "active" ]; then
      return 0
    fi
    # hard failures
    if [ "$active" = "failed" ] || [ "$result" = "failed" ] || [ "$result" = "exit-code" ] || [ "$result" = "timeout" ]; then
      return 1
    fi

    sleep 1
  done
  echo "timeout: ${WAIT_TIMEOUT}s exceeded while waiting for ${SERVICE}"
  return 1
}

restart_poll() {
  reload_and_reset
  echo "Restarting $SERVICE (non-blocking) and polling up to ${WAIT_TIMEOUT}s..."
  systemctl --no-ask-password restart --no-block "$SERVICE"
  wait_until_active_or_fail
}

start_poll() {
  reload_and_reset
  echo "Starting $SERVICE (non-blocking) and polling up to ${WAIT_TIMEOUT}s..."
  systemctl --no-ask-password start --no-block "$SERVICE"
  wait_until_active_or_fail
}

case "${1:-}" in
  restart-wait|restart-poll)  restart_poll ;;
  start-wait|start-poll)      start_poll   ;;
  *)
    echo "Usage: $0 {start-poll|restart-poll}"
    exit 2
    ;;
esac
