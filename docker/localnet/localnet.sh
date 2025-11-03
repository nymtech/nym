#!/bin/bash

set -ex

# Nym Localnet Orchestration Script for Apple Container Runtime
# Emulates docker-compose functionality

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
IMAGE_NAME="nym-localnet:latest"
VOLUME_NAME="nym-localnet-data"
VOLUME_PATH="/tmp/nym-localnet-$$"
NYM_VOLUME_PATH="/tmp/nym-localnet-home-$$"

SUFFIX=${NYM_NODE_SUFFIX:-localnet}

# Container names
INIT_CONTAINER="nym-localnet-init"
MIXNODE1_CONTAINER="nym-mixnode1"
MIXNODE2_CONTAINER="nym-mixnode2"
MIXNODE3_CONTAINER="nym-mixnode3"
GATEWAY_CONTAINER="nym-gateway"
REQUESTER_CONTAINER="nym-network-requester"
SOCKS5_CONTAINER="nym-socks5-client"

ALL_CONTAINERS=(
    "$MIXNODE1_CONTAINER"
    "$MIXNODE2_CONTAINER"
    "$MIXNODE3_CONTAINER"
    "$GATEWAY_CONTAINER"
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
    for node in mix1 mix2 mix3 gateway; do
        rm -rf "$HOME/.nym/nym-nodes/${node}-${SUFFIX}"
    done
}

# Check if container command exists
check_prerequisites() {
    if ! command -v container &> /dev/null; then
        log_error "Apple 'container' command not found"
        log_error "Install from: https://github.com/apple/container"
        exit 1
    fi
}

# Build the Docker image
build_image() {
    log_info "Building image: $IMAGE_NAME"
    log_warn "This will take 15-30 minutes on first build..."

    cd "$PROJECT_ROOT"

    # Build with Docker
    log_info "Building with Docker..."
    if ! docker build \
        -f "$SCRIPT_DIR/Dockerfile.localnet" \
        -t "$IMAGE_NAME" \
        "$PROJECT_ROOT"; then
        log_error "Docker build failed"
        exit 1
    fi

    # Transfer image to container runtime
    log_info "Transferring image to container runtime..."

    # Save to temporary file (container image load doesn't support stdin)
    TEMP_IMAGE="/tmp/nym-localnet-image-$$.tar"
    if ! docker save -o "$TEMP_IMAGE" "$IMAGE_NAME"; then
        log_error "Failed to save Docker image"
        exit 1
    fi

    # Load into container runtime from file
    if ! container image load --input "$TEMP_IMAGE"; then
        rm -f "$TEMP_IMAGE"
        log_error "Failed to load image into container runtime"
        exit 1
    fi

    # Clean up temporary file
    rm -f "$TEMP_IMAGE"

    # Verify image is available
    if ! container image inspect "$IMAGE_NAME" &>/dev/null; then
        log_error "Image not found in container runtime after load"
        exit 1
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
    if container network create "$NETWORK_NAME" 2>/dev/null; then
        log_success "Network created: $NETWORK_NAME"
    else
        log_info "Network $NETWORK_NAME already exists or creation failed"
    fi
}

# Remove container network
remove_network() {
    if container network list | grep -q "$NETWORK_NAME"; then
        log_info "Removing network: $NETWORK_NAME"
        container network rm "$NETWORK_NAME" 2>/dev/null || true
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

    container run \
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
            exec nym-node run --id mix'"${node_id}"'-localnet --unsafe-disable-replay-protection --local
        '

    log_success "$container_name started"
}
# Start gateway
start_gateway() {
    log_info "Starting $GATEWAY_CONTAINER..."

    container run \
        --name "$GATEWAY_CONTAINER" \
        -m 2G \
        --network "$NETWORK_NAME" \
        -p 9000:9000 \
        -p 10004:10004 \
        -p 20004:20004 \
        -p 30004:30004 \
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
                --output=json \
                --bonding-information-output="/localnet/gateway.json";

            echo "Waiting for network.json...";
            while [ ! -f /localnet/network.json ]; do
                sleep 2;
            done;
            echo "Starting gateway...";
            exec nym-node run --id gateway-localnet --unsafe-disable-replay-protection --local
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
# Start network requester
start_network_requester() {
    log_info "Starting $REQUESTER_CONTAINER..."

    # Get gateway IP address
    log_info "Getting gateway IP address..."
    GATEWAY_IP=$(container exec "$GATEWAY_CONTAINER" hostname -i)
    log_info "Gateway IP: $GATEWAY_IP"

    container run \
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

    container run \
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
        if container inspect "$container_name" &>/dev/null; then
            log_info "Stopping $container_name"
            container stop "$container_name" 2>/dev/null || true
            container rm "$container_name" 2>/dev/null || true
        fi
    done

    # Also clean up init container if it exists
    container rm "$INIT_CONTAINER" 2>/dev/null || true

    log_success "All containers stopped"

    cleanup_host_state
    remove_network
}

# Show container logs
show_logs() {
    local container_name=${1:-}

    if [ -z "$container_name" ]; then
        # No container specified - launch tmux log viewer
        log_info "Launching tmux log viewer for all containers..."
        exec "$SCRIPT_DIR/localnet-logs.sh"
    fi

    # Show logs for specific container
    if container inspect "$container_name" &>/dev/null; then
        container logs -f "$container_name"
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
        if container inspect "$container_name" &>/dev/null; then
            local status=$(container inspect "$container_name" 2>/dev/null | grep -o '"Status":"[^"]*"' | cut -d'"' -f4 || echo "unknown")
            echo -e "  ${GREEN}●${NC} $container_name - $status"
        else
            echo -e "  ${RED}○${NC} $container_name - not running"
        fi
    done

    echo ""
    log_info "Port status:"
    for port in 9000 1080 10001 10002 10003 10004; do
        if nc -z 127.0.0.1 $port 2>/dev/null; then
            echo -e "  ${GREEN}●${NC} Port $port - listening"
        else
            echo -e "  ${RED}○${NC} Port $port - not listening"
        fi
    done
}

# Build network topology with container IPs
build_topology() {
    log_info "Building network topology with container IPs..."

    # Wait for all bonding JSON files to be created
    log_info "Waiting for all nodes to complete initialization..."
    for file in mix1.json mix2.json mix3.json gateway.json; do
        while [ ! -f "$VOLUME_PATH/$file" ]; do
            echo "  Waiting for $file..."
            sleep 1
        done
        log_success "  $file created"
    done

    # Get container IPs
    log_info "Getting container IP addresses..."
    MIX1_IP=$(container exec "$MIXNODE1_CONTAINER" hostname -i)
    MIX2_IP=$(container exec "$MIXNODE2_CONTAINER" hostname -i)
    MIX3_IP=$(container exec "$MIXNODE3_CONTAINER" hostname -i)
    GATEWAY_IP=$(container exec "$GATEWAY_CONTAINER" hostname -i)

    log_info "Container IPs:"
    echo "  mix1:    $MIX1_IP"
    echo "  mix2:    $MIX2_IP"
    echo "  mix3:    $MIX3_IP"
    echo "  gateway: $GATEWAY_IP"

    # Run build_topology.py in a container with access to the volumes
    container run \
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
            "$GATEWAY_IP"

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
    build_topology
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
