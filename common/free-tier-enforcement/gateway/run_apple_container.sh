#!/usr/bin/env bash
#
# Build and run the free-tier gateway harness under Apple's `container` runtime.
# Same image + entrypoint as run_docker.sh; only the runtime differs.
#
# IMPORTANT limitation: Apple `container` micro-VMs run a minimal kernel with NO
# netfilter conntrack module (and no `modprobe`), so stateful MASQUERADE / NAT does
# NOT work - i.e. WG traffic canNOT be forwarded to the internet here. This runtime
# is fine for the registration -> tunnel -> free-tier-credential flow (and in-tunnel
# reachability like the gateway metadata endpoint), but the internet-egress /
# walled-garden *datapath* test must run on a real Linux kernel (see ../netns, or a
# Linux CI host with kernel WireGuard). Docker Desktop can't do it either (its gVisor
# netstack proxies the connection). This script is kept for the flows that DO work.
#
# Requires the Apple `container` CLI (https://github.com/apple/container) and Docker
# (the build goes through Docker then transfers the image in: `container build` stalls
# on this repo's large build context, so we use the localnet save->load pattern).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(git -C "${SCRIPT_DIR}" rev-parse --show-toplevel)"
IMAGE="${IMAGE:-nym-free-tier-localtest}"
NAME="${NAME:-nym-free-tier-gw}"
DOCKERFILE="common/free-tier-enforcement/gateway/Dockerfile.localtest"
ENV_FILE="${ENV_FILE:-${SCRIPT_DIR}/.env}"
TAR="${TMPDIR:-/tmp}/${IMAGE}-$$.tar"

echo ">> building ${IMAGE} with Docker (compiles nym-node; first build is slow)"
DOCKER_BUILDKIT=1 docker build -t "${IMAGE}" -f "${REPO_ROOT}/${DOCKERFILE}" "${REPO_ROOT}"

echo ">> transferring image into the container runtime (docker save -> container image load)"
docker save -o "${TAR}" "${IMAGE}"
container image load --input "${TAR}"
rm -f "${TAR}"

echo ">> (re)starting ${NAME}"
container rm -f "${NAME}" >/dev/null 2>&1 || true

# -m 2G: the micro-VM defaults to ~1GB, below the node's replay-protection bloomfilter
#        requirement (~1.08GB) -> it would exit at startup otherwise.
# --cap-add ALL: covers nft/tc + the WireGuard interface (the VM provides /dev/net/tun
#        natively; vmnet gives a directly host-reachable IP, so no -p publishing).
container run -d --rm \
    --name "${NAME}" \
    -m 2G \
    --cap-add ALL \
    --dns 1.1.1.1 \
    --env-file "${ENV_FILE}" \
    "${IMAGE}"

# Discover the container's vmnet IP (directly reachable from the host).
IP=""
for _ in $(seq 1 20); do
    IP="$(container inspect "${NAME}" 2>/dev/null | grep -oE '192\.168\.[0-9]+\.[0-9]+' | head -1)"
    [ -n "${IP}" ] && break
    sleep 1
done

echo
if [ -n "${IP}" ]; then
    echo ">> gateway is up at ${IP} (vmnet). Point the driver directly at it:"
    echo
    echo "     cargo run -p nym-free-tier-gateway-harness -- \\"
    echo "         --gateway-http http://${IP}:8080 --gateway-ip ${IP}"
    echo
    echo "   (omit --reach: internet forwarding does not work under Apple container - no conntrack)"
else
    echo ">> could not auto-detect the container IP; find it with: container inspect ${NAME}"
fi
echo
echo ">> following logs (Ctrl-C detaches; stop with: container rm -f ${NAME})"
echo
exec container logs -f "${NAME}"
