#!/bin/bash

set -ex

# Nym Localnet Orchestration Script
# Supports both Docker and Apple Container Runtime

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE_NAME="nym-localnet:latest"
VOLUME_NAME="nym-localnet-data"
VOLUME_PATH="/tmp/nym-localnet-$$"
NYM_VOLUME_PATH="/tmp/nym-localnet-home-$$"

SUFFIX=${NYM_NODE_SUFFIX:-localnet}

# Detect container runtime: prefer Apple 'container' if available, fall back to docker
if command -v container &> /dev/null; then
    RUNTIME="container"
    HOST_INTERNAL="host.containers.internal"
else
    RUNTIME="docker"
    HOST_INTERNAL="host.docker.internal"
fi

# OpenTelemetry configuration
# Set OTEL_ENABLE=1 to enable OTel tracing on all nym-node instances.
# OTEL_ENDPOINT should point to the OTLP gRPC collector reachable from containers.
# When SigNoz runs in Docker (signoz-net), we route to its collector directly.
OTEL_ENABLE=${OTEL_ENABLE:-1}
if [ -z "${OTEL_ENDPOINT:-}" ]; then
    SIGNOZ_NET=$(docker network ls --filter name=signoz-net --format '{{.Name}}' 2>/dev/null || true)
    if [ "$RUNTIME" = "docker" ] && [ -n "$SIGNOZ_NET" ]; then
        OTEL_ENDPOINT="http://signoz-otel-collector:4317"
        OTEL_SIGNOZ_NET="$SIGNOZ_NET"
    else
        OTEL_ENDPOINT="http://${HOST_INTERNAL}:4317"
        OTEL_SIGNOZ_NET=""
    fi
fi

# Build OTel flags for nym-node run commands
otel_flags() {
    if [ "$OTEL_ENABLE" = "1" ]; then
        echo "--otel --otel-endpoint $OTEL_ENDPOINT"
    fi
}

# Container names
INIT_CONTAINER="nym-localnet-init"
MIXNODE1_CONTAINER="nym-mixnode1"
MIXNODE2_CONTAINER="nym-mixnode2"
MIXNODE3_CONTAINER="nym-mixnode3"
GATEWAY_CONTAINER="nym-gateway"
GATEWAY2_CONTAINER="nym-gateway2"
REQUESTER_CONTAINER="nym-network-requester"
SOCKS5_CONTAINER="nym-socks5-client"

ALL_CONTAINERS=(
    "$MIXNODE1_CONTAINER"
    "$MIXNODE2_CONTAINER"
    "$MIXNODE3_CONTAINER"
    "$GATEWAY_CONTAINER"
    "$GATEWAY2_CONTAINER"
    "$REQUESTER_CONTAINER"
    "$SOCKS5_CONTAINER"
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $*"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $*"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $*"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $*"
}

cleanup_host_state() {
    log_info "Cleaning local nym-node state for suffix ${SUFFIX}"
    for node in mix1 mix2 mix3 gateway gateway2; do
        rm -rf "$HOME/.nym/nym-nodes/${node}-${SUFFIX}"
    done
}

# Check prerequisites
check_prerequisites() {
    if ! command -v docker &> /dev/null; then
        log_error "Docker not found"
        exit 1
    fi
    log_info "Using runtime: $RUNTIME"
}

# Build the Docker image
build_image() {
    log_info "Building image: $IMAGE_NAME"
    log_warn "This will take 15-30 minutes on first build..."

    cd "$PROJECT_ROOT"

    log_info "Building with Docker..."
    if ! docker build \
        -f "$SCRIPT_DIR/Dockerfile.localnet" \
        -t "$IMAGE_NAME" \
        "$PROJECT_ROOT"; then
        log_error "Docker build failed"
        exit 1
    fi

    # If using Apple container runtime, transfer image from Docker
    if [ "$RUNTIME" = "container" ]; then
        log_info "Transferring image to Apple container runtime..."
        TEMP_IMAGE="/tmp/nym-localnet-image-$$.tar"
        if ! docker save -o "$TEMP_IMAGE" "$IMAGE_NAME"; then
            log_error "Failed to save Docker image"
            exit 1
        fi
        if ! container image load --input "$TEMP_IMAGE"; then
            rm -f "$TEMP_IMAGE"
            log_error "Failed to load image into container runtime"
            exit 1
        fi
        rm -f "$TEMP_IMAGE"
        if ! container image inspect "$IMAGE_NAME" &>/dev/null; then
            log_error "Image not found in container runtime after load"
            exit 1
        fi
    fi

    log_success "Image built and loaded: $IMAGE_NAME"
}

# Create shared volume directory
create_volume() {
    log_info "Creating shared volume at: $VOLUME_PATH"
    mkdir -p "$VOLUME_PATH"
    chmod 777 "$VOLUME_PATH"
    log_success "Volume created"
}

# Create shared nym home directory
create_nym_volume() {
    log_info "Creating shared nym home volume at: $NYM_VOLUME_PATH"
    mkdir -p "$NYM_VOLUME_PATH"
    chmod 777 "$NYM_VOLUME_PATH"
    log_success "Nym home volume created"
}

# Remove shared volume directory
remove_volume() {
    if [ -d "$VOLUME_PATH" ]; then
        log_info "Removing volume: $VOLUME_PATH"
        rm -rf "$VOLUME_PATH"
        log_success "Volume removed"
    fi
    if [ -d "$NYM_VOLUME_PATH" ]; then
        log_info "Removing nym home volume: $NYM_VOLUME_PATH"
        rm -rf "$NYM_VOLUME_PATH"
        log_success "Nym home volume removed"
    fi
}

# Network name
NETWORK_NAME="nym-localnet-network"

# Create container network
create_network() {
    log_info "Creating container network: $NETWORK_NAME"
    if $RUNTIME network create "$NETWORK_NAME" 2>/dev/null; then
        log_success "Network created: $NETWORK_NAME"
    else
        log_info "Network $NETWORK_NAME already exists or creation failed"
    fi
}

# Remove container network
remove_network() {
    if $RUNTIME network list | grep -q "$NETWORK_NAME"; then
        log_info "Removing network: $NETWORK_NAME"
        $RUNTIME network rm "$NETWORK_NAME" 2>/dev/null || true
        log_success "Network removed"
    fi
}

# Start a mixnode
start_mixnode() {
    local node_id=$1
    local container_name=$2

    log_info "Starting $container_name..."

    # Calculate port numbers based on node_id
    local mixnet_port="1000${node_id}"
    local verloc_port="2000${node_id}"
    local http_port="3000${node_id}"

    local otel_args
    otel_args=$(otel_flags)

    $RUNTIME run \
        --name "$container_name" \
        -m 2G \
        --network "$NETWORK_NAME" \
        -p "${mixnet_port}:${mixnet_port}" \
        -p "${verloc_port}:${verloc_port}" \
        -p "${http_port}:${http_port}" \
        -v "$VOLUME_PATH:/localnet" \
        -v "$NYM_VOLUME_PATH:/root/.nym" \
        -d \
        -e "NYM_NODE_SUFFIX=$SUFFIX" \
        "$IMAGE_NAME" \
        sh -c '
            CONTAINER_IP=$(hostname -i);
            echo "Container IP: $CONTAINER_IP";
            echo "Initializing mix'"${node_id}"'...";
            nym-node run --id mix'"${node_id}"'-localnet --init-only \
                --unsafe-disable-replay-protection \
                --local \
                --mixnet-bind-address=0.0.0.0:'"${mixnet_port}"' \
                --verloc-bind-address=0.0.0.0:'"${verloc_port}"' \
                --http-bind-address=0.0.0.0:'"${http_port}"' \
                --http-access-token=lala \
                --public-ips $CONTAINER_IP \
                --output=json \
                --bonding-information-output="/localnet/mix'"${node_id}"'.json";

            echo "Waiting for network.json...";
            while [ ! -f /localnet/network.json ]; do
                sleep 2;
            done;
            echo "Starting mix'"${node_id}"'...";
            exec nym-node '"${otel_args}"' run --id mix'"${node_id}"'-localnet --unsafe-disable-replay-protection --local
        '

    log_success "$container_name started"
}
# Start gateway
start_gateway() {
    log_info "Starting $GATEWAY_CONTAINER..."

    local otel_args
    otel_args=$(otel_flags)

        $RUNTIME run \
        --name "$GATEWAY_CONTAINER" \
        -m 2G \
        --cap-add=NET_ADMIN \
        --device /dev/net/tun \
        --network "$NETWORK_NAME" \
        -p 9000:9000 \
        -p 10004:10004 \
        -p 20004:20004 \
        -p 30004:30004 \
        -p 41264:41264 \
        -p 51264:51264 \
        -p 51822:51822/udp \
        -v "$VOLUME_PATH:/localnet" \
        -v "$NYM_VOLUME_PATH:/root/.nym" \
        -d \
        -e "NYM_NODE_SUFFIX=$SUFFIX" \
        "$IMAGE_NAME" \
        sh -c '
            CONTAINER_IP=$(hostname -i);
            echo "Container IP: $CONTAINER_IP";
            echo "Initializing gateway...";
            nym-node run --id gateway-localnet --init-only \
                --unsafe-disable-replay-protection \
                --local \
                --mode entry-gateway \
                --mode exit-gateway \
                --mixnet-bind-address=0.0.0.0:10004 \
                --entry-bind-address=0.0.0.0:9000 \
                --verloc-bind-address=0.0.0.0:20004 \
                --http-bind-address=0.0.0.0:30004 \
                --http-access-token=lala \
                --public-ips $CONTAINER_IP \
                --lp-use-mock-ecash true \
                --output=json \
                --wireguard-enabled false \
                --bonding-information-output="/localnet/gateway.json";

            echo "Waiting for network.json...";
            while [ ! -f /localnet/network.json ]; do
                sleep 2;
            done;
            echo "Starting gateway with LP listener (mock ecash)...";
            exec nym-node '"${otel_args}"' run --id gateway-localnet --unsafe-disable-replay-protection --local --wireguard-enabled false --lp-use-mock-ecash true
        '

    log_success "$GATEWAY_CONTAINER started"

    # Wait for gateway to be ready
    log_info "Waiting for gateway to listen on port 9000..."
    local retries=0
    local max_retries=30
    while ! nc -z 127.0.0.1 9000 2>/dev/null; do
        sleep 2
        retries=$((retries + 1))
        if [ $retries -ge $max_retries ]; then
            log_error "Gateway failed to start on port 9000"
            return 1
        fi
    done
    log_success "Gateway is ready on port 9000"
}

# Start gateway2
start_gateway2() {
    log_info "Starting $GATEWAY2_CONTAINER..."

    local otel_args
    otel_args=$(otel_flags)

        $RUNTIME run \
        --name "$GATEWAY2_CONTAINER" \
        -m 2G \
        --cap-add=NET_ADMIN \
        --device /dev/net/tun \
        --network "$NETWORK_NAME" \
        -p 9001:9001 \
        -p 10005:10005 \
        -p 20005:20005 \
        -p 30005:30005 \
        -p 41265:41265 \
        -p 51265:51265 \
        -p 51823:51822/udp \
        -v "$VOLUME_PATH:/localnet" \
        -v "$NYM_VOLUME_PATH:/root/.nym" \
        -d \
        -e "NYM_NODE_SUFFIX=$SUFFIX" \
        "$IMAGE_NAME" \
        sh -c '
            CONTAINER_IP=$(hostname -i);
            echo "Container IP: $CONTAINER_IP";
            echo "Initializing gateway2...";
            nym-node run --id gateway2-localnet --init-only \
                --unsafe-disable-replay-protection \
                --local \
                --mode entry-gateway \
                --mode exit-gateway \
                --mixnet-bind-address=0.0.0.0:10005 \
                --entry-bind-address=0.0.0.0:9001 \
                --verloc-bind-address=0.0.0.0:20005 \
                --http-bind-address=0.0.0.0:30005 \
                --http-access-token=lala \
                --public-ips $CONTAINER_IP \
                --lp-use-mock-ecash true \
                --output=json \
                --wireguard-enabled false \
                --bonding-information-output="/localnet/gateway2.json";

            echo "Waiting for network.json...";
            while [ ! -f /localnet/network.json ]; do
                sleep 2;
            done;
            echo "Starting gateway2 with LP listener (mock ecash)...";
            exec nym-node '"${otel_args}"' run --id gateway2-localnet --unsafe-disable-replay-protection --local --wireguard-enabled false --lp-use-mock-ecash true
        '

    log_success "$GATEWAY2_CONTAINER started"

    # Wait for gateway2 to be ready
    log_info "Waiting for gateway2 to listen on port 9001..."
    local retries=0
    local max_retries=30
    while ! nc -z 127.0.0.1 9001 2>/dev/null; do
        sleep 2
        retries=$((retries + 1))
        if [ $retries -ge $max_retries ]; then
            log_error "Gateway2 failed to start on port 9001"
            return 1
        fi
    done
    log_success "Gateway2 is ready on port 9001"
}

# Start network requester
start_network_requester() {
    log_info "Starting $REQUESTER_CONTAINER..."

    # Get gateway IP address (first IP only, in case container has multiple networks)
    log_info "Getting gateway IP address..."
    GATEWAY_IP=$($RUNTIME exec "$GATEWAY_CONTAINER" hostname -i | awk '{print $1}')
    log_info "Gateway IP: $GATEWAY_IP"

    $RUNTIME run \
        --name "$REQUESTER_CONTAINER" \
        --network "$NETWORK_NAME" \
        -v "$VOLUME_PATH:/localnet" \
        -v "$NYM_VOLUME_PATH:/root/.nym" \
        -e "GATEWAY_IP=$GATEWAY_IP" \
        -d \
        "$IMAGE_NAME" \
        sh -c '
            while [ ! -f /localnet/network.json ]; do
                echo "Waiting for network.json...";
                sleep 2;
            done;
            while ! nc -z $GATEWAY_IP 9000 2>/dev/null; do
                echo "Waiting for gateway on port 9000 ($GATEWAY_IP)...";
                sleep 2;
            done;
            SUFFIX=$(date +%s);
            nym-network-requester init \
                --id "network-requester-$SUFFIX" \
                --open-proxy=true \
                --custom-mixnet /localnet/network.json \
                --output=json > /localnet/network_requester.json;
            exec nym-network-requester run \
                --id "network-requester-$SUFFIX" \
                --custom-mixnet /localnet/network.json
        '

    log_success "$REQUESTER_CONTAINER started"
}

# Start SOCKS5 client
start_socks5_client() {
    log_info "Starting $SOCKS5_CONTAINER..."

    $RUNTIME run \
        --name "$SOCKS5_CONTAINER" \
        --network "$NETWORK_NAME" \
        -p 1080:1080 \
        -v "$VOLUME_PATH:/localnet:ro" \
        -v "$NYM_VOLUME_PATH:/root/.nym" \
        -d \
        "$IMAGE_NAME" \
        sh -c '
            while [ ! -f /localnet/network_requester.json ]; do
                echo "Waiting for network requester...";
                sleep 2;
            done;
            SUFFIX=$(date +%s);
            PROVIDER=$(cat /localnet/network_requester.json | grep -o "\"client_address\":\"[^\"]*\"" | cut -d\" -f4);
            if [ -z "$PROVIDER" ]; then
                echo "Error: Could not extract provider address";
                exit 1;
            fi;
            nym-socks5-client init \
                --id "socks5-client-$SUFFIX" \
                --provider "$PROVIDER" \
                --custom-mixnet /localnet/network.json \
                --no-cover;
            exec nym-socks5-client run \
                --id "socks5-client-$SUFFIX" \
                --custom-mixnet /localnet/network.json \
                --host 0.0.0.0
        '

    log_success "$SOCKS5_CONTAINER started"

    # Wait for SOCKS5 to be ready
    log_info "Waiting for SOCKS5 proxy on port 1080..."
    sleep 5
    local retries=0
    local max_retries=15
    while ! nc -z 127.0.0.1 1080 2>/dev/null; do
        sleep 2
        retries=$((retries + 1))
        if [ $retries -ge $max_retries ]; then
            log_warn "SOCKS5 proxy not responding on port 1080 yet"
            return 0
        fi
    done
    log_success "SOCKS5 proxy is ready on port 1080"
}

# Stop all containers
stop_containers() {
    log_info "Stopping all containers..."

    for container_name in "${ALL_CONTAINERS[@]}"; do
        if $RUNTIME inspect "$container_name" &>/dev/null; then
            log_info "Stopping $container_name"
            $RUNTIME stop "$container_name" 2>/dev/null || true
            $RUNTIME rm "$container_name" 2>/dev/null || true
        fi
    done

    # Also clean up init container if it exists
    $RUNTIME rm "$INIT_CONTAINER" 2>/dev/null || true

    log_success "All containers stopped"

    cleanup_host_state
    remove_network
}

# Show $RUNTIME logs
show_logs() {
    local container_name=${1:-}

    if [ -z "$container_name" ]; then
        # No container specified - launch tmux log viewer
        log_info "Launching tmux log viewer for all containers..."
        exec "$SCRIPT_DIR/localnet-logs.sh"
    fi

    # Show logs for specific container
    if $RUNTIME inspect "$container_name" &>/dev/null; then
        $RUNTIME logs -f "$container_name"
    else
        log_error "Container not found: $container_name"
        log_info "Available containers:"
        for name in "${ALL_CONTAINERS[@]}"; do
            echo "  - $name"
        done
        exit 1
    fi
}

# Show container status
show_status() {
    log_info "Container status:"
    echo ""

    for container_name in "${ALL_CONTAINERS[@]}"; do
        if $RUNTIME inspect "$container_name" &>/dev/null; then
            local status=$($RUNTIME inspect "$container_name" 2>/dev/null | grep -o '"Status":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
            echo -e "  ${GREEN}●${NC} $container_name - $status"
        else
            echo -e "  ${RED}○${NC} $container_name - not running"
        fi
    done

    echo ""
    log_info "Port status:"
    echo "  Mixnet:"
    for port in 10001 10002 10003 10004; do
        if nc -z 127.0.0.1 $port 2>/dev/null; then
            echo -e "    ${GREEN}●${NC} Port $port - listening"
        else
            echo -e "    ${RED}○${NC} Port $port - not listening"
        fi
    done
    echo "  Gateway:"
    for port in 9000 30004; do
        if nc -z 127.0.0.1 $port 2>/dev/null; then
            echo -e "    ${GREEN}●${NC} Port $port - listening"
        else
            echo -e "    ${RED}○${NC} Port $port - not listening"
        fi
    done
    echo "  LP (Lewes Protocol):"
    for port in 41264 51264; do
        if nc -z 127.0.0.1 $port 2>/dev/null; then
            echo -e "    ${GREEN}●${NC} Port $port - listening"
        else
            echo -e "    ${RED}○${NC} Port $port - not listening"
        fi
    done
    echo "  SOCKS5:"
    if nc -z 127.0.0.1 1080 2>/dev/null; then
        echo -e "    ${GREEN}●${NC} Port 1080 - listening"
    else
        echo -e "    ${RED}○${NC} Port 1080 - not listening"
    fi
}

# Build network topology with container IPs
build_topology() {
    log_info "Building network topology with container IPs..."

    # Wait for all bonding JSON files to be created
    log_info "Waiting for all nodes to complete initialization..."
    for file in mix1.json mix2.json mix3.json gateway.json gateway2.json; do
        while [ ! -f "$VOLUME_PATH/$file" ]; do
            echo "  Waiting for $file..."
            sleep 1
        done
        log_success "  $file created"
    done

    # Get container IPs (first IP only, containers may be on multiple networks)
    log_info "Getting container IP addresses..."
    MIX1_IP=$($RUNTIME exec "$MIXNODE1_CONTAINER" hostname -i | awk '{print $1}')
    MIX2_IP=$($RUNTIME exec "$MIXNODE2_CONTAINER" hostname -i | awk '{print $1}')
    MIX3_IP=$($RUNTIME exec "$MIXNODE3_CONTAINER" hostname -i | awk '{print $1}')
    GATEWAY_IP=$($RUNTIME exec "$GATEWAY_CONTAINER" hostname -i | awk '{print $1}')
    GATEWAY2_IP=$($RUNTIME exec "$GATEWAY2_CONTAINER" hostname -i | awk '{print $1}')

    log_info "Container IPs:"
    echo "  mix1:     $MIX1_IP"
    echo "  mix2:     $MIX2_IP"
    echo "  mix3:     $MIX3_IP"
    echo "  gateway:  $GATEWAY_IP"
    echo "  gateway2: $GATEWAY2_IP"

    # Run build_topology.py in a container with access to the volumes
    $RUNTIME run \
        --name "nym-localnet-topology-builder" \
        --network "$NETWORK_NAME" \
        -v "$VOLUME_PATH:/localnet" \
        -v "$NYM_VOLUME_PATH:/root/.nym" \
        --rm \
        "$IMAGE_NAME" \
        python3 /usr/local/bin/build_topology.py \
            /localnet \
            "$SUFFIX" \
            "$MIX1_IP" \
            "$MIX2_IP" \
            "$MIX3_IP" \
            "$GATEWAY_IP" \
            "$GATEWAY2_IP"

    # Verify network.json was created
    if [ -f "$VOLUME_PATH/network.json" ]; then
        log_success "Network topology created successfully"
    else
        log_error "Failed to create network topology"
        exit 1
    fi
}

# Start all services
start_all() {
    log_info "Starting Nym Localnet..."

    cleanup_host_state
    create_network
    create_volume
    create_nym_volume

    start_mixnode 1 "$MIXNODE1_CONTAINER"
    start_mixnode 2 "$MIXNODE2_CONTAINER"
    start_mixnode 3 "$MIXNODE3_CONTAINER"
    start_gateway
    start_gateway2

    # Connect nym containers to SigNoz network for direct OTLP routing
    if [ -n "${OTEL_SIGNOZ_NET:-}" ]; then
        log_info "Connecting containers to SigNoz network ($OTEL_SIGNOZ_NET)..."
        for c in "$MIXNODE1_CONTAINER" "$MIXNODE2_CONTAINER" "$MIXNODE3_CONTAINER" \
                 "$GATEWAY_CONTAINER" "$GATEWAY2_CONTAINER"; do
            docker network connect "$OTEL_SIGNOZ_NET" "$c" 2>/dev/null && \
                log_success "  $c connected to $OTEL_SIGNOZ_NET" || true
        done
    fi

    build_topology

    # Configure networking for two-hop WireGuard routing on both gateways
    # Note: Requires --privileged or --cap-add=NET_ADMIN on the containers.
    # Non-fatal: only needed for WireGuard VPN routing, not mixnet packet testing.
    log_info "Configuring gateway networking (IP forwarding, NAT)..."
    for gw in "$GATEWAY_CONTAINER" "$GATEWAY2_CONTAINER"; do
        if $RUNTIME exec "$gw" sh -c "
            echo 1 > /proc/sys/net/ipv4/ip_forward 2>/dev/null
            iptables-legacy -t nat -A POSTROUTING -o eth0 -j MASQUERADE 2>/dev/null
        " 2>/dev/null; then
            log_success "Configured $gw"
        else
            log_warn "Could not configure NAT on $gw (needs --privileged). WireGuard VPN routing will not work."
        fi
    done

    start_network_requester
    start_socks5_client

    echo ""
    log_success "Nym Localnet is running!"
    echo ""
    echo "Test with:"
    echo "  curl -x socks5h://127.0.0.1:1080 https://nymtech.net"
    echo ""
    echo "View logs:"
    echo "  $0 logs              # All containers in tmux"
    echo "  $0 logs gateway      # Single container"
    echo ""
    echo "Stop:"
    echo "  $0 down"
    echo ""
}

# Main command handler
main() {
    check_prerequisites

    local command=${1:-help}
    shift || true

    case "$command" in
        build)
            build_image
            ;;
        up)
            build_image
            start_all
            ;;
        start)
            start_all
            ;;
        down|stop)
            stop_containers
            remove_volume
            ;;
        restart)
            stop_containers
            start_all
            ;;
        logs)
            show_logs "$@"
            ;;
        status|ps)
            show_status
            ;;
        help|--help|-h)
            cat <<EOF
Nym Localnet Orchestration Script

Usage: $0 <command> [options]

Commands:
  build          Build the localnet image
  up             Build image and start all services
  start          Start all services (requires built image)
  down, stop     Stop all services and clean up
  restart        Restart all services
  logs [name]    Show logs (no args = tmux overlay, with name = single container)
  status, ps     Show status of all containers and ports
  help           Show this help message

Examples:
  $0 up                    # Build and start everything
  $0 logs                  # View all logs in tmux overlay
  $0 logs gateway          # View gateway logs only
  $0 status                # Check what's running
  $0 down                  # Stop and clean up

EOF
            ;;
        *)
            log_error "Unknown command: $command"
            echo "Run '$0 help' for usage information"
            exit 1
            ;;
    esac
}

main "$@"
