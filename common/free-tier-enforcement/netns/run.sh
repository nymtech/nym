#!/usr/bin/env bash
# Run the free-tier netns datapath tests in a privileged Linux container
# (needed on macOS and any host without root/NET_ADMIN). On a privileged Linux
# host it runs them directly instead.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../../.." && pwd)"
image="nym-free-tier-netns"
test_cmd="cargo test -p nym-free-tier-enforcement --test datapath -- --ignored --nocapture --test-threads=1"

# Privileged Linux host: no container needed.
if [[ "$(uname)" == "Linux" && "$(id -u)" == "0" ]]; then
    cd "$repo"
    export NYM_FREE_TIER_NETNS_TESTS=1
    exec bash -c "$test_cmd"
fi

run_in() { # $1 = docker|container
    local rt="$1"
    echo ">> building image with $rt"
    "$rt" build -t "$image" -f "$here/Dockerfile" "$here"
    echo ">> running tests with $rt (--privileged)"
    # a container-local target dir keeps Linux artifacts out of the host's ./target
    "$rt" run --rm --privileged \
        -v "$repo":/workspace -w /workspace \
        -e CARGO_TARGET_DIR=/tmp/ft-target \
        -e NYM_FREE_TIER_NETNS_TESTS=1 \
        "$image" bash -c "$test_cmd"
}

# Prefer Docker (its --privileged netns path is well-trodden); fall back to
# Apple's `container` runtime.
if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    run_in docker
elif command -v container >/dev/null 2>&1; then
    run_in container
else
    echo "error: no usable container runtime (docker or apple 'container') found" >&2
    exit 1
fi
