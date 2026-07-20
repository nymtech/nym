#!/usr/bin/env bash
#
# Cross-compile nym-node for Linux, ship it to a VPS, and (re)start it there.
#
# The container harness (run_docker.sh / run_apple_container.sh) can't forward WG
# traffic to the open internet on macOS (Docker's gVisor netstack proxies; Apple
# `container` has no conntrack). A real Linux box can, so this deploys the same
# standalone free-tier gateway to a VPS running a real kernel.
#
# Usage:
#   ./deploy_vps.sh user 1.2.3.4
#
# First run installs a systemd unit and starts the node; later runs just replace
# the binary and `systemctl restart`. The node is always launched from the local
# `.env` (same knobs as the container), which is shipped on every deployment.
#
# NAT / forwarding (network-tunnel-manager.sh apply_iptables_rules_wg) is assumed
# already applied on the box - this script does not touch it.
#
# Env overrides:
#   REMOTE_DIR          install dir on the VPS         (default /opt/nym-free-tier)
#   SERVICE             systemd unit name              (default nym-node-free-tier.service)
#   NYMNODE_PUBLIC_IPS  advertised public IP           (default: the ip argument)
#   SSH_PORT            ssh/scp port                   (default 22)
#   SSH_OPTS            extra ssh/scp options          (default "")
#   SKIP_BUILD=1        redeploy the existing binary without rebuilding
set -euo pipefail

if [ "$#" -lt 2 ]; then
    echo "usage: $0 <user> <ip>" >&2
    exit 1
fi

SSH_USER="$1"
IP="$2"
TARGET="${SSH_USER}@${IP}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(git -C "${SCRIPT_DIR}" rev-parse --show-toplevel)"
ENV_FILE="${ENV_FILE:-${SCRIPT_DIR}/.env}"

TARGET_TRIPLE="x86_64-unknown-linux-gnu"
BIN="${CARGO_TARGET_DIR:-${REPO_ROOT}/target}/${TARGET_TRIPLE}/release/nym-node"

REMOTE_DIR="${REMOTE_DIR:-/opt/nym-free-tier}"
SERVICE="${SERVICE:-nym-node-free-tier.service}"
STAGE="/tmp/nym-ft-stage"

# ssh/scp share options; scp wants -P for the port, ssh wants -p.
SSH_PORT="${SSH_PORT:-22}"
SSH_OPTS="${SSH_OPTS:-}"
# shellcheck disable=SC2206
SSH=(ssh -p "${SSH_PORT}" ${SSH_OPTS})
# shellcheck disable=SC2206
SCP=(scp -P "${SSH_PORT}" ${SSH_OPTS})

# Advertise the VPS's own IP (the ip argument) so a driver running off-box reaches it.
PUBLIC_IPS="${NYMNODE_PUBLIC_IPS:-${IP}}"

if [ "${SKIP_BUILD:-0}" != "1" ]; then
    echo ">> cross build --release --bin nym-node --target ${TARGET_TRIPLE}"
    ( cd "${REPO_ROOT}" && cross build --release --bin nym-node --target "${TARGET_TRIPLE}" )
fi

if [ ! -f "${BIN}" ]; then
    echo "error: binary not found at ${BIN}" >&2
    echo "       (set CARGO_TARGET_DIR if your build output lives elsewhere)" >&2
    exit 1
fi

echo ">> staging binary + .env on ${TARGET}"
"${SSH[@]}" "${TARGET}" "mkdir -p ${STAGE}"
"${SCP[@]}" "${BIN}" "${TARGET}:${STAGE}/nym-node"
"${SCP[@]}" "${ENV_FILE}" "${TARGET}:${STAGE}/.env"

echo ">> installing + restarting ${SERVICE} on ${TARGET}"
"${SSH[@]}" "${TARGET}" 'bash -s' -- \
    "${REMOTE_DIR}" "${SERVICE}" "${PUBLIC_IPS}" "${STAGE}" <<'REMOTE'
set -euo pipefail
REMOTE_DIR="$1"; SERVICE="$2"; PUBLIC_IPS="$3"; STAGE="$4"
SUDO=""; [ "$(id -u)" -ne 0 ] && SUDO="sudo"

${SUDO} mkdir -p "${REMOTE_DIR}"
${SUDO} install -m 0755 "${STAGE}/nym-node" "${REMOTE_DIR}/nym-node"
${SUDO} install -m 0644 "${STAGE}/.env" "${REMOTE_DIR}/.env"
rm -rf "${STAGE}"

# HOME points config/keys at the install dir so they persist across restarts.
# NYMNODE_PUBLIC_IPS (if derived) overrides the .env value; systemd applies later
# directives last, so this line wins over EnvironmentFile.
PUBLIC_LINE=""
[ -n "${PUBLIC_IPS}" ] && PUBLIC_LINE="Environment=NYMNODE_PUBLIC_IPS=${PUBLIC_IPS}"

${SUDO} tee "/etc/systemd/system/${SERVICE}" >/dev/null <<UNIT
[Unit]
Description=Nym free-tier standalone gateway (localtest)
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=${REMOTE_DIR}
Environment=HOME=${REMOTE_DIR}
EnvironmentFile=${REMOTE_DIR}/.env
${PUBLIC_LINE}
ExecStart=${REMOTE_DIR}/nym-node run
Restart=on-failure
RestartSec=3
LimitNOFILE=65536

[Install]
WantedBy=multi-user.target
UNIT

${SUDO} systemctl daemon-reload
${SUDO} systemctl enable "${SERVICE}" >/dev/null 2>&1 || true
${SUDO} systemctl restart "${SERVICE}"
sleep 2
${SUDO} systemctl --no-pager --full status "${SERVICE}" || true
echo "--- recent logs ---"
${SUDO} journalctl -u "${SERVICE}" -n 30 --no-pager || true
REMOTE

echo
echo ">> done. tail logs with:"
echo "     ${SSH[*]} ${TARGET} 'sudo journalctl -u ${SERVICE} -f'"
if [ -n "${PUBLIC_IPS}" ]; then
    echo ">> point the driver at the box:"
    echo "     cargo run -p nym-free-tier-gateway-harness -- \\"
    echo "         --gateway-http http://${PUBLIC_IPS}:8080 --gateway-ip ${PUBLIC_IPS} --free-tier --reach 1.1.1.1:80"
fi
