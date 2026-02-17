# Nym Localnet

A complete Nym mixnet test environment with OpenTelemetry instrumentation.
Supports both Docker Desktop and Apple Container Runtime on macOS.

## Overview

This localnet setup provides a fully functional Nym mixnet for local development and testing:
- **3 mixnodes** (layer 1, 2, 3)
- **2 gateways** (entry + exit mode)
- **1 network-requester** (service provider)
- **1 SOCKS5 client**
- **OpenTelemetry tracing** via OTLP/gRPC to SigNoz (or any OTLP collector)

All components run in isolated containers with proper networking and dynamic IP resolution.
When the `otel` feature is enabled (default), every nym-node exports traces covering
the full packet lifecycle: ingress, Sphinx processing, forwarding, and final-hop delivery.

## Prerequisites

### Required
- **macOS** (tested on macOS Sequoia 15.0+)
- **Docker Desktop** (recommended) or **Apple Container Runtime**
- **Python 3** with `base58` library

### SigNoz (for trace viewing)

SigNoz is an open-source APM that receives and visualises OpenTelemetry data.
Install it locally with Docker Compose -- this takes about 2 minutes:

```bash
# Clone the SigNoz repository
git clone -b main https://github.com/SigNoz/signoz.git ~/signoz
cd ~/signoz/deploy

# Start SigNoz (runs ClickHouse, otel-collector, query-service, frontend)
docker compose up -d

# Verify it is running
docker ps --filter "name=signoz" --format "table {{.Names}}\t{{.Status}}"
```

Once running:
- **SigNoz UI**: http://localhost:8080
- **OTLP gRPC collector**: localhost:4317 (used by nym-nodes)
- **OTLP HTTP collector**: localhost:4318

The localnet script auto-detects the SigNoz Docker network (`signoz-net`) and
routes OTel traffic directly to the collector container -- no manual endpoint
configuration needed.

To stop SigNoz later:
```bash
cd ~/signoz/deploy && docker compose down
```

### Installation
```bash
# Install Python dependencies
pip3 install --break-system-packages base58

# Verify Docker is installed
docker --version
```

If using Apple Container Runtime instead of Docker:
```bash
container --version
```

## Quick Start

```bash
# Navigate to the localnet directory
cd docker/localnet

# Build the container image
./localnet.sh build

# Start the localnet
./localnet.sh start

# Test the SOCKS5 proxy
curl -L --socks5 localhost:1080 https://nymtech.net

# View logs
./localnet.sh logs gateway
./localnet.sh logs socks5

# Stop the localnet
./localnet.sh stop

# Clean up everything
./localnet.sh clean
```

## Architecture

### Container Network

All containers run on a custom bridge network (`nym-localnet-network`) with dynamic IP assignment:

```
Host Machine (macOS)
├── nym-localnet-network (bridge)
│   ├── nym-mixnode1    (192.168.66.3)
│   ├── nym-mixnode2    (192.168.66.4)
│   ├── nym-mixnode3    (192.168.66.5)
│   ├── nym-gateway     (192.168.66.6)
│   ├── nym-network-requester (192.168.66.7)
│   └── nym-socks5-client (192.168.66.8)
```

Ports published to host:
- 1080 → SOCKS5 proxy
- 9000/9001 → Gateway entry ports
- 10001-10005 → Mixnet ports
- 20001-20005 → Verloc ports
- 30001-30005 → HTTP APIs
- 41264/41265 → LP control ports (registration)
- 51822/51823 → WireGuard tunnel ports (gateway/gateway2; only used when WireGuard is enabled)

### WireGuard and privileges

By default, gateways run with **WireGuard disabled** (`--wireguard-enabled false`). No elevated capabilities are required: the script does not use `--cap-add=NET_ADMIN` or `--device /dev/net/tun`, so localnet runs without net admin privileges and is suitable for mixnet packet testing and SOCKS5 over the mixnet.

To enable WireGuard VPN routing in localnet (e.g. for two-hop VPN tests), set `WIREGUARD_ENABLED=1` before starting. The script will then add `--cap-add=NET_ADMIN` and `--device /dev/net/tun` to the gateway containers and configure IP forwarding and NAT. This may not work in all Docker environments (e.g. some hosted runners restrict capabilities).

```bash
WIREGUARD_ENABLED=1 ./localnet.sh start
```

### Startup Flow

1. **Container Initialization** (parallel)
   - Each container starts and gets a dynamic IP
   - Each node runs `nym-node run --init-only` with its container IP
   - Bonding JSON files are written to shared volume

2. **Topology Generation** (sequential)
   - Wait for all 4 bonding JSON files
   - Get container IPs dynamically
   - Run `build_topology.py` with container IPs
   - Generate `network.json` with correct addresses

3. **Node Startup** (parallel)
   - Each container starts its node with `--local` flag
   - Nodes read configuration from init phase
   - Clients use custom topology file

4. **Service Providers** (sequential)
   - Network requester initializes and starts
   - SOCKS5 client initializes with requester address

### Network Topology

The `network.json` file contains the complete network topology:

```json
{
  "metadata": {
    "key_rotation_id": 0,
    "absolute_epoch_id": 0,
    "refreshed_at": "2025-11-03T..."
  },
  "rewarded_set": {
    "epoch_id": 0,
    "entry_gateways": [4],
    "exit_gateways": [4],
    "layer1": [1],
    "layer2": [2],
    "layer3": [3],
    "standby": []
  },
  "node_details": {
    "1": { "mix_host": "192.168.66.3:10001", ... },
    "2": { "mix_host": "192.168.66.4:10002", ... },
    "3": { "mix_host": "192.168.66.5:10003", ... },
    "4": { "mix_host": "192.168.66.6:10004", ... }
  }
}
```

## Commands

### Build
```bash
./localnet.sh build
```
Builds the Docker image and loads it into Apple container runtime.

**Note**: First build takes ~5-10 minutes to compile all components.

### Start
```bash
./localnet.sh start
```
Starts all containers, generates topology, and launches the complete network.

**Expected output**:
```
[INFO] Starting Nym Localnet...
[SUCCESS] Network created: nym-localnet-network
[INFO] Starting nym-mixnode1...
[SUCCESS] nym-mixnode1 started
...
[INFO] Building network topology with container IPs...
[SUCCESS] Network topology created successfully
[SUCCESS] Nym Localnet is running!

Test with:
  curl -x socks5h://127.0.0.1:1080 https://nymtech.net
```

### Stop
```bash
./localnet.sh stop
```
Stops and removes all running containers.

### Clean
```bash
./localnet.sh clean
```
Complete cleanup: removes containers, volumes, network, and temporary files.

### Logs
```bash
# View logs for a specific container
./localnet.sh logs <container-name>

# Container names:
# - mix1, mix2, mix3
# - gateway
# - requester
# - socks5

# Examples:
./localnet.sh logs gateway
./localnet.sh logs socks5
container logs nym-gateway --follow
```

### Status
```bash
# List all containers
docker ps --filter "name=nym-" --format "table {{.Names}}\t{{.Status}}"

# Check specific container
docker logs nym-gateway

# Inspect network
docker network inspect nym-localnet-network
```

## Testing

### Basic SOCKS5 Test
```bash
# Simple HTTP request through the mixnet
curl -x socks5h://127.0.0.1:1080 https://httpbin.org/get

# HTTPS request
curl -x socks5h://127.0.0.1:1080 https://nymtech.net

# Download a file
curl -x socks5h://127.0.0.1:1080 \
  https://test-download-files-nym.s3.amazonaws.com/download-files/1MB.zip \
  --output /tmp/test.zip
```

### Load Testing

A load test script is included to generate sustained traffic and populate SigNoz
with meaningful trace data:

```bash
# Default: 10 concurrent workers, 60 seconds
./loadtest.sh

# Heavier load: 20 workers for 2 minutes
./loadtest.sh -c 20 -d 120

# Light single-threaded test
./loadtest.sh -c 1 -d 10

# Target a specific URL
./loadtest.sh -c 5 -d 30 -u https://httpbin.org/bytes/4096
```

The script reports live progress, then prints a summary with request counts,
throughput, and latency percentiles (p50/p95/p99).

### Verify Network Topology
```bash
# View the generated topology
docker exec nym-gateway cat /localnet/network.json | jq .

# Check container status
docker ps --filter "name=nym-" --format "table {{.Names}}\t{{.Status}}"

# Verify all bonding files exist
docker exec nym-gateway ls -la /localnet/
```

### Test Mixnet Routing
```bash
# All traffic flows through: client -> gateway -> mix1 -> mix2 -> mix3 -> gateway -> internet
# Watch logs to verify routing:
docker logs nym-mixnode1 --follow &
docker logs nym-mixnode2 --follow &
docker logs nym-mixnode3 --follow &
docker logs nym-gateway --follow &

# Make a request
curl -x socks5h://127.0.0.1:1080 https://nymtech.net
```

## OpenTelemetry

OTel is enabled by default. Each nym-node exports traces via OTLP/gRPC covering
packet ingress, Sphinx processing, forwarding, and final-hop delivery.

### Viewing Traces

- **SigNoz UI**: http://localhost:8080 -- filter by `serviceName = nym-node`
- **Terminal report** (queries ClickHouse directly, no login needed):

```bash
./otel-report.sh           # last 15 minutes
./otel-report.sh 60        # last 60 minutes
./otel-report.sh live      # auto-refresh every 10s
```

### Disabling OTel

```bash
OTEL_ENABLE=0 ./localnet.sh start                           # disable
OTEL_ENDPOINT=http://my-collector:4317 ./localnet.sh start   # custom collector
```

### LP (Lewes Protocol) Testing

The gateway is configured with LP listener enabled and **mock ecash verification** for testing:

```bash
# LP listener ports (exposed on host):
# - 41264: LP control port (TCP registration)
# - 51264: LP data port

# Check LP ports are listening
nc -zv localhost 41264
nc -zv localhost 51264

# Test LP registration with nym-gateway-probe
cargo run -p nym-gateway-probe run-local \
  --mnemonic "test mnemonic here" \
  --gateway-ip 'localhost:41264' \
  --only-lp-registration
```

**Mock Ecash Mode**:
- Gateway uses `--lp.use-mock-ecash true` flag
- Accepts ANY bandwidth credential without blockchain verification
- Perfect for testing LP protocol implementation
- **WARNING**: Never use mock ecash in production!

**Testing without blockchain**:
The mock ecash manager allows testing the complete LP registration flow without requiring:
- Running nyxd blockchain
- Deploying smart contracts
- Acquiring real bandwidth credentials
- Setting up coconut signers

This makes localnet perfect for rapid LP protocol development and testing.

## File Structure

```
docker/localnet/
├── README.md              # This file
├── localnet.sh           # Main orchestration script
├── loadtest.sh           # Load test / traffic generator
├── otel-report.sh        # Terminal-based OTel metrics report
├── Dockerfile.localnet   # Multi-stage Docker image (builder + slim runtime)
├── build_topology.py     # Topology generator
└── localnet-logs.sh      # Tmux-based multi-container log viewer
```

## How It Works

### Node Initialization

Each node initializes itself at runtime inside its container:

```bash
# Get container IP
CONTAINER_IP=$(hostname -i)

# Initialize with container IP
nym-node run --id mix1-localnet --init-only \
    --unsafe-disable-replay-protection \
    --local \
    --mixnet-bind-address=0.0.0.0:10001 \
    --verloc-bind-address=0.0.0.0:20001 \
    --http-bind-address=0.0.0.0:30001 \
    --http-access-token=lala \
    --public-ips $CONTAINER_IP \
    --output=json \
    --bonding-information-output="/localnet/mix1.json"
```

**Key flags**:
- `--local`: Accept private IPs for local development
- `--public-ips`: Announce the container's IP address
- `--unsafe-disable-replay-protection`: Disable bloomfilter to save memory

### Dynamic Topology

The topology is built **after** containers start:

```bash
# Get container IPs
MIX1_IP=$(container exec nym-mixnode1 hostname -i)
MIX2_IP=$(container exec nym-mixnode2 hostname -i)
MIX3_IP=$(container exec nym-mixnode3 hostname -i)
GATEWAY_IP=$(container exec nym-gateway hostname -i)

# Build topology with actual IPs
python3 build_topology.py /localnet localnet \
    $MIX1_IP $MIX2_IP $MIX3_IP $GATEWAY_IP
```

This ensures the topology contains reachable container addresses.

### Client Configuration

Clients use `--custom-mixnet` to read the local topology:

```bash
# Network requester
nym-network-requester init \
    --id "network-requester-$SUFFIX" \
    --open-proxy=true \
    --custom-mixnet /localnet/network.json

# SOCKS5 client
nym-socks5-client init \
    --id "socks5-client-$SUFFIX" \
    --provider "$REQUESTER_ADDRESS" \
    --custom-mixnet /localnet/network.json \
    --host 0.0.0.0
```

The `--custom-mixnet` flag tells clients to use our local topology instead of fetching from nym-api.

## Troubleshooting

### Container Build Issues

**Problem**: Docker build fails
```bash
# Check Docker is running
docker info

# Clean Docker cache
docker system prune -a

# Rebuild with no cache
./localnet.sh build
```

**Problem**: Container image load fails
```bash
# Verify temp file was created
ls -lh /tmp/nym-localnet-image-*

# Check container runtime
container image list

# Manually load if needed
docker save -o /tmp/nym-image.tar nym-localnet:latest
container image load --input /tmp/nym-image.tar
```

### Network Issues

**Problem**: Containers can't communicate
```bash
# Check network exists
container network list | grep nym-localnet

# Inspect network
container network inspect nym-localnet-network

# Verify containers are on the network
container list | grep nym-
```

**Problem**: SOCKS5 connection refused
```bash
# Check SOCKS5 is listening
container logs nym-socks5-client | grep "Listening on"

# Verify port mapping
container list | grep socks5

# Test from host
nc -zv localhost 1080
```

### Node Issues

**Problem**: "No valid public addresses" error
- Ensure `--local` flag is present in both init and run commands
- Check container can resolve its own IP: `container exec nym-mixnode1 hostname -i`
- Verify `--public-ips` is using `$CONTAINER_IP` variable

**Problem**: "TUN device error"
- The gateway needs TUN device support for exit functionality
- Verify `iproute2` is installed in the image (adds `ip` command)
- Check gateway logs: `container logs nym-gateway`
- The gateway should show: "Created TUN device: nymtun0"

**Problem**: "Noise handshake" warnings
- These are warnings, not errors - nodes fall back to TCP
- Does not affect functionality in local development
- Safe to ignore for testing purposes

### Topology Issues

**Problem**: Network.json not created
```bash
# Check all bonding files exist
container exec nym-gateway ls -la /localnet/

# Verify build_topology.py ran
container logs nym-gateway | grep "Building network topology"

# Check Python dependencies
container exec nym-gateway python3 -c "import base58"
```

**Problem**: Clients can't connect to nodes
```bash
# Verify IPs in topology match container IPs
container exec nym-gateway cat /localnet/network.json | jq '.node_details'
container list | grep nym-

# Check containers can reach each other
container exec nym-socks5-client ping -c 1 192.168.66.6
```

### Startup Issues

**Problem**: Containers exit immediately
```bash
# Check logs for errors
container logs nym-mixnode1

# Common issues:
# - Missing network.json: Wait for topology to be built
# - Port already in use: Check for conflicting services
# - Init failed: Check for correct container IP
```

**Problem**: Topology build times out
```bash
# Verify all containers initialized
container exec nym-gateway ls -la /localnet/*.json

# Check for init errors
container logs nym-mixnode1 | grep -i error

# Manual cleanup and restart
./localnet.sh clean
./localnet.sh start
```

## Performance Notes

### Memory Usage
- Each mixnode: ~200MB
- Gateway: ~300MB (includes TUN device)
- Network requester: ~150MB
- SOCKS5 client: ~150MB
- **Total**: ~1.2GB + overhead

**Recommended**: 4GB+ system memory

### Startup Time
- Image build: ~5-10 minutes (first time)
- Network start: ~20-30 seconds
- Node initialization: ~5-10 seconds per node (parallel)

### Latency
Mixnet adds latency by design for privacy:
- ~1-3 seconds for SOCKS5 requests
- Cover traffic adds random delays
- Local testing may show variable timing

This is **expected behavior** - the mixnet provides privacy through traffic mixing.

## Advanced Configuration

### Custom Node Configuration

Edit node init commands in `localnet.sh` (search for `nym-node run --init-only`):

```bash
# Example: Change mixnode ports
--mixnet-bind-address=0.0.0.0:11001 \
--verloc-bind-address=0.0.0.0:21001 \
--http-bind-address=0.0.0.0:31001 \
```

Remember to update port mappings in the `container run` command as well.

### Enable Replay Protection

Remove `--unsafe-disable-replay-protection` flags (requires more memory):

```bash
# In start_mixnode() and start_gateway() functions
nym-node run --id mix1-localnet --init-only \
    --local \
    --mixnet-bind-address=0.0.0.0:10001 \
    # ... other flags (without --unsafe-disable-replay-protection)
```

**Note**: Each node will require an additional ~1.5GB memory for bloomfilter.

### API Access

Each node exposes an HTTP API:

```bash
# Get gateway info
curl -H "Authorization: Bearer lala" http://localhost:30004/api/v1/gateway

# Get mixnode stats
curl -H "Authorization: Bearer lala" http://localhost:30001/api/v1/stats

# Get node description
curl -H "Authorization: Bearer lala" http://localhost:30001/api/v1/description
```

Access token is `lala` (configured with `--http-access-token=lala`).

### Add More Mixnodes

To add a 4th mixnode:

1. **Update constants** in `localnet.sh`:
```bash
MIXNODE4_CONTAINER="nym-mixnode4"
```

2. **Add start call** in `start_all()`:
```bash
start_mixnode 4 "$MIXNODE4_CONTAINER"
```

3. **Update topology builder** to include the new node

4. **Rebuild and restart**:
```bash
./localnet.sh clean
./localnet.sh build
./localnet.sh start
```

## Technical Details

### Container Runtime

**Docker Desktop** is the default and recommended runtime; no extra setup is required for mixnet testing.

**Apple Container Runtime** is an optional alternative on macOS. It natively uses [Kata Containers](https://github.com/kata-containers/kata-containers) images and is only required if you use `container` instead of Docker (e.g. for consistency with other Apple tooling). Kata is also the path that provides a kernel with `CONFIG_TUN=y` if you need TUN/WireGuard inside containers under the Apple runtime.

### Initial setup for [Container Runtime](https://github.com/apple/container) (optional)

- **MUST** have MacOS Tahoe for inter-container networking
- `brew install --cask container`
- Download Kata Containers 3.20, this one can be loaded by `container` and has `CONFIG_TUN=y` kernel flag
  - `https://github.com/kata-containers/kata-containers/releases/download/3.20.0/kata-static-3.20.0-arm64.tar.xz`
- Load new kernel
  - `container system kernel set --tar kata-static-3.20.0-arm64.tar.xz --binary opt/kata/share/kata-containers/vmlinux-6.12.42-162`
- Validate kernel version once you have container running
  - `uname -r` should return `6.12.42`
  - `cat /proc/config.gz | grep CONFIG_TUN` should return `CONFIG_TUN=y`

### Image Building

Images are built with Docker then transferred:
1. `docker build` creates the image
2. `docker save` exports to tar file
3. `container image load` imports into container runtime
4. Temporary file is cleaned up

This approach allows using Docker's build cache while running on Apple's runtime.

### Network Architecture

The custom bridge network (`nym-localnet-network`):
- Provides container-to-container communication
- Assigns dynamic IPs from 192.168.66.0/24
- NAT for outbound internet access
- Port publishing for host access

### Volumes

Two types of volumes:
1. **Shared data** (`/tmp/nym-localnet-*`): Bonding files and topology
2. **Node configs** (`/tmp/nym-localnet-home-*`): Node configurations

Both are ephemeral by default (cleaned up on stop).

## Known Limitations

- **macOS only**: Apple container runtime requires macOS
- **No Docker Compose**: Uses custom orchestration script
- **Dynamic IPs**: Container IPs may change between restarts
- **Port conflicts**: Cannot run alongside services using same ports
- **TUN device**: Only required when `WIREGUARD_ENABLED=1`; otherwise gateways run without it

## Support

For issues and questions:
- **GitHub Issues**: https://github.com/nymtech/nym/issues
- **Documentation**: https://nymtech.net/docs
- **Discord**: https://discord.gg/nym

## License

This localnet setup is part of the Nym project and follows the same license.
