#!/usr/bin/env bash
#
# Build and run the local free-tier gateway harness (Milestone A).
# Requires Docker with BuildKit. Runnable from anywhere; paths are repo-relative.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(git -C "${SCRIPT_DIR}" rev-parse --show-toplevel)"
IMAGE="${IMAGE:-nym-free-tier-localtest}"
DOCKERFILE="common/free-tier-enforcement/gateway/Dockerfile.localtest"
ENV_FILE="${ENV_FILE:-${SCRIPT_DIR}/.env}"

echo ">> building ${IMAGE} (compiles nym-node; the first build is slow, later ones hit the cargo cache)"
DOCKER_BUILDKIT=1 docker build -t "${IMAGE}" -f "${REPO_ROOT}/${DOCKERFILE}" "${REPO_ROOT}"

# Published ports (host defaults to container; override via env). WireGuard is
# UDP; the LP registration transport and the HTTP API are TCP.
HTTP_PORT="${HTTP_PORT:-8080}"        # node HTTP API (boot health check)
WG_PORT="${WG_PORT:-51822}"           # WireGuard tunnel: the client data plane
LP_CONTROL_PORT="${LP_CONTROL_PORT:-41264}"  # LP control: registration transport
LP_DATA_PORT="${LP_DATA_PORT:-51264}"        # LP data channel

# CAP_NET_ADMIN: the nft/tc enforcement datapath and the WireGuard interface.
# /dev/net/tun: the WireGuard tunnel device.
# --env-file: all node + free-tier knobs, including the TEST-ONLY signer keypair.
echo ">> running ${IMAGE} (env from ${ENV_FILE})"
exec docker run --rm -it \
    --cap-add=NET_ADMIN \
    --device /dev/net/tun \
    --env-file "${ENV_FILE}" \
    -p "${HTTP_PORT}:8080" \
    -p "${WG_PORT}:51822/udp" \
    -p "${LP_CONTROL_PORT}:41264" \
    -p "${LP_DATA_PORT}:51264" \
    "${IMAGE}"
