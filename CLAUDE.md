# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Nym is a privacy platform that uses mixnet technology to protect against metadata surveillance. The platform consists of several key components:
- Mixnet nodes (mixnodes) for packet mixing
- Gateways (entry/exit points for the network)
- Clients for interacting with the network
- Network monitoring tools
- Validators for network consensus
- Various service providers and integrations

## Build Commands

### Rust Components

```bash
# Default build (debug)
cargo build

# Release build
cargo build --release

# Build a specific package
cargo build -p <package-name>

# Build main components
make build

# Build release versions of main binaries and contracts
make build-release

# Build specific binaries
make build-nym-cli
cargo build -p nym-node --release
cargo build -p nym-api --release
```

### Testing

```bash
# Run clippy, unit tests, and formatting
make test

# Run all tests including slow tests
make test-all

# Run clippy on all workspaces
make clippy

# Run unit tests for a specific package
cargo test -p <package-name>

# Run only expensive/ignored tests
cargo test --workspace -- --ignored

# Run API tests
dotenv -f envs/sandbox.env -- cargo test --test public-api-tests

# Run tests with specific log level
RUST_LOG=debug cargo test -p <package-name>

# Run specific test scripts
./nym-node/tests/test_apis.sh
./scripts/wireguard-exit-policy/exit-policy-tests.sh
```

### Linting and Formatting

```bash
# Run rustfmt on all code
make fmt

# Check formatting without modifying
cargo fmt --all -- --check

# Run clippy with all targets
cargo clippy --workspace --all-targets -- -D warnings

# TypeScript linting
yarn lint
yarn lint:fix
yarn types:lint:fix

# Check dependencies for security/licensing issues
cargo deny check
```

### WASM Components

```bash
# Build all WASM components
make sdk-wasm-build

# Build TypeScript SDK
yarn build:sdk
npx lerna run --scope @nymproject/sdk build --stream

# Build and test WASM components
make sdk-wasm

# Build specific WASM packages
cd wasm/client && make
cd wasm/mix-fetch && make
cd wasm/node-tester && make
```

### Contract Development

```bash
# Build all contracts
make contracts

# Build contracts in release mode
make build-release-contracts

# Generate contract schemas
make contract-schema

# Run wasm-opt on contracts
make wasm-opt-contracts

# Check contracts with cosmwasm-check
make cosmwasm-check-contracts
```

### Running Components

```bash
# Run nym-node as a mixnode
cargo run -p nym-node -- run --mode mixnode

# Run nym-node as a gateway
cargo run -p nym-node -- run --mode gateway

# Run the network monitor
cargo run -p nym-network-monitor

# Run the API server
cargo run -p nym-api

# Run with specific environment
dotenv -f envs/sandbox.env -- cargo run -p nym-api

# Start a local network
./scripts/localnet_start.sh
```

## Architecture

The Nym platform consists of various components organized as a monorepo:

1. **Core Mixnet Infrastructure**:
   - `nym-node`: Core binary supporting mixnode and gateway modes
   - `common/nymsphinx`: Implementation of the Sphinx packet format
   - `common/topology`: Network topology management
   - `common/types`: Shared data types across components

2. **Network Monitoring**:
   - `nym-network-monitor`: Monitors the network's reliability and performance
   - `nym-api`: API server for network stats and monitoring data
   - Metrics tracking for nodes, routes, and overall network health

3. **Client Implementations**:
   - `clients/native`: Native Rust client implementation
   - `clients/socks5`: SOCKS5 proxy client for standard applications
   - `wasm`: WebAssembly client implementations (for browsers)
   - `nym-connect`: Desktop and mobile clients

4. **Blockchain & Smart Contracts**:
   - `common/cosmwasm-smart-contracts`: Smart contract implementations
   - `contracts`: CosmWasm contracts for the Nym network
   - `common/ledger`: Blockchain integration

5. **Utilities & Tools**:
   - `tools`: Various CLI tools and utilities
   - `sdk`: SDKs for different languages and platforms
   - `documentation`: Documentation generation and management

## Packet System

Nym uses a modified Sphinx packet format for its mixnet:

1. **Message Chunking**:
   - Messages are divided into "sets" and "fragments"
   - Each fragment fits in a single Sphinx packet
   - The `common/nymsphinx/chunking` module handles message fragmentation

2. **Routing**:
   - Packets traverse through 3 layers of mixnodes
   - Routing information is encrypted in layers (onion routing)
   - The final gateway receives and processes the messages

3. **Monitoring**:
   - Monitoring system tracks packet delivery through the network
   - Routes are analyzed for reliability statistics
   - Node performance metrics are collected

## Network Protocol

Nym implements the Loopix mixnet design with several key privacy features:

1. **Continuous-time Mixing**:
   - Each mixnode delays messages independently with an exponential distribution
   - This creates random reordering of packets, destroying timing correlations
   - Offers better anonymity properties than batch mixing approaches

2. **Cover Traffic**:
   - Clients and nodes generate dummy "loop" packets that circulate through the network
   - These packets are indistinguishable from real traffic
   - Creates a baseline level of traffic that hides actual communication patterns
   - Provides unobservability (hiding when and how much real traffic is being sent)

3. **Stratified Network Architecture**:
   - Traffic flows through Entry Gateway → 3 Mixnode Layers → Exit Gateway
   - Path selection is independent per-message (unlike Tor)
   - Each node connects only to adjacent layers

4. **Anonymous Replies**:
   - Single-Use Reply Blocks (SURBs) allow receiving messages without revealing identity
   - Enables bidirectional communication while maintaining privacy

## Network Monitoring Architecture

The network monitoring system is a core component that measures mixnet reliability:

1. The `nym-network-monitor` sends test packets through the network
2. These packets follow predefined routes through multiple mixnodes
3. Metrics are collected about:
   - Successful and failed packet deliveries
   - Node reliability (percentage of successful packet handling)
   - Route reliability (which specific route combinations work best)
4. Results are stored in the database and used by `nym-api` to:
   - Present node performance statistics
   - Determine network rewards
   - Provide route selection guidance to clients

In the current branch, metrics collection is being enhanced with a fanout approach to submit to multiple API endpoints.

## Development Environment

### Required Dependencies

- Rust toolchain (stable, 1.80+)
- Node.js (v20+) and yarn for TypeScript components
- SQLite for local database development
- PostgreSQL for API database (optional, for full API functionality)
- CosmWasm tools for contract development
- For building contracts: `wasm-opt` tool from `binaryen`
- Python 3.8+ for some scripts
- Docker (optional, for containerized development)
- protoc (Protocol Buffers compiler) for some components

### Environment Configurations

The `envs/` directory contains pre-configured environments:

#### Available Environments

- **`local.env`**: Local development environment
  - Points to local services (localhost)
  - Uses test mnemonics and keys
  - Ideal for testing without external dependencies

- **`sandbox.env`**: Sandbox test network
  - Public test network with real nodes
  - Test tokens available from faucet
  - Contract addresses for sandbox deployment
  - API: https://sandbox-nym-api1.nymtech.net

- **`mainnet.env`**: Production mainnet
  - Real network with real tokens
  - Production contract addresses
  - API: https://validator.nymtech.net
  - Use with caution!

- **`canary.env`**: Canary deployment
  - Pre-release testing environment
  - Tests new features before mainnet

- **`mainnet-local-api.env`**: Hybrid environment
  - Uses mainnet contracts but local API
  - Useful for API development against mainnet data

#### Key Environment Variables

```bash
# Network configuration
NETWORK_NAME=sandbox              # Network identifier
BECH32_PREFIX=n                   # Address prefix (n for sandbox, n for mainnet)
NYM_API=https://sandbox-nym-api1.nymtech.net/api
NYXD=https://rpc.sandbox.nymtech.net
NYM_API_NETWORK=sandbox

# Contract addresses (network-specific)
MIXNET_CONTRACT_ADDRESS=n1xr3rq8yvd7qplsw5yx90ftsr2zdhg4e9z60h5duusgxpv72hud3sjkxkav
VESTING_CONTRACT_ADDRESS=n1unyuj8qnmygvzuex3dwmg9yzt9alhvyeat0uu0jedg2wj33efl5qackslz
# ... other contract addresses

# Mnemonic for testing (NEVER use in production)
MNEMONIC="clutch captain shoe salt awake harvest setup primary inmate ugly among become"

# API Keys and tokens
IPINFO_API_TOKEN=your_token_here
AUTHENTICATOR_PASSWORD=password_here

# Logging
RUST_LOG=info                    # Options: error, warn, info, debug, trace
RUST_BACKTRACE=1                # Enable backtraces

# Database
DATABASE_URL=postgresql://user:pass@localhost/nym_api
```

#### Using Environment Files

```bash
# Load environment and run command
dotenv -f envs/sandbox.env -- cargo run -p nym-api

# Export to shell
source envs/sandbox.env

# Use with make targets
dotenv -f envs/sandbox.env -- make run-api-tests
```

## Initial Setup

### First Time Setup

1. **Install Prerequisites**
   ```bash
   # Install Rust
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   
   # Install Node.js and yarn
   # Via nvm (recommended):
   curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
   nvm install 20
   npm install -g yarn
   
   # Install build tools
   # Ubuntu/Debian:
   sudo apt-get install build-essential pkg-config libssl-dev protobuf-compiler libpq-dev
   
   # macOS:
   brew install protobuf postgresql
   
   # Install wasm-opt for contract builds
   npm install -g wasm-opt
   
   # Add wasm target for Rust
   rustup target add wasm32-unknown-unknown
   ```

2. **Clone and Setup Repository**
   ```bash
   git clone https://github.com/nymtech/nym.git
   cd nym/nym
   
   # Install JavaScript dependencies
   yarn install
   
   # Build the project
   make build
   ```

3. **Database Setup (Optional, for API development)**
   ```bash
   # Install PostgreSQL
   # Create database
   createdb nym_api
   
   # Run migrations (from nym-api directory)
   cd nym-api
   sqlx migrate run
   ```

### Quick Start

```bash
# Run a mixnode locally
dotenv -f envs/sandbox.env -- cargo run -p nym-node -- run --mode mixnode --id my-mixnode

# Run a gateway locally
dotenv -f envs/sandbox.env -- cargo run -p nym-node -- run --mode gateway --id my-gateway

# Run the API server
dotenv -f envs/sandbox.env -- cargo run -p nym-api

# Run a client
cargo run -p nym-client -- init --id my-client
cargo run -p nym-client -- run --id my-client
```

## CI/CD Pipeline

The project uses GitHub Actions for CI/CD with several key workflows:

1. **Build and Test**:
   - `ci-build.yml`: Main build workflow for Rust components
   - Tests are run on multiple platforms (Linux, Windows, macOS)
   - Includes formatting check (rustfmt) and linting (clippy)

2. **Release Process**:
   - Binary artifacts are published on release tags
   - Multiple platform builds are created

3. **Documentation**:
   - Documentation is automatically built and deployed

## Database Structure

The system uses SQLite databases with tables like:
- `mixnode_status`: Status information about mixnodes
- `gateway_status`: Status information about gateways
- `routes`: Route performance information (success/failure of specific paths)
- `monitor_run`: Information about monitoring test runs

## Development Workflows

### Running a Node

To run the mixnode or gateway:

```bash
# Run nym-node as a mixnode with specified identity
cargo run -p nym-node -- run --mode mixnode --id my-mixnode

# Run nym-node as a gateway
cargo run -p nym-node -- run --mode gateway --id my-gateway
```

### Configuration

Nodes can be configured with files in various locations:
- Command-line arguments
- Environment variables
- `.env` files specified with `--config-env-file`

### Monitoring

To monitor the health of your node:
- View logs for real-time information
- Use the node's HTTP API for status information
- Check the explorer for public node statistics

## Common Libraries

- `common/types`: Shared data types across all components
- `common/crypto`: Cryptographic primitives and wrappers
- `common/client-core`: Core client functionality
- `common/gateway-client`: Client-gateway communication
- `common/task`: Task management and concurrency utilities
- `common/nymsphinx`: Sphinx packet implementation for mixnet
- `common/topology`: Network topology management
- `common/credentials`: Credential system for privacy-preserving authentication
- `common/bandwidth-controller`: Bandwidth management and accounting

## Code Conventions

- Error handling: Use anyhow/thiserror for structured error handling
- Logging: Use the tracing framework for logging and diagnostics
- State management: Generally use Tokio/futures for async code
- Configuration: Use the config crate and env vars with defaults
- Database: Use sqlx for type-safe database queries
- Follow clippy recommendations and rustfmt formatting
- Use semantic commit messages: feat, fix, docs, refactor, test, chore

## When Making Changes

- Run `make test` before submitting PRs
- Follow Rust naming conventions
- Use `clippy` to check for common issues
- Update SQLx query caches when modifying DB queries: `cargo sqlx prepare`
- Consider backward compatibility for protocol changes
- Use lefthook pre-commit hooks for TypeScript formatting
- Run `cargo deny check` to verify dependency compliance
- Test against both sandbox and local environments when possible
- Update relevant documentation and CHANGELOG.md

## Development Tools

### Useful Cargo Commands

```bash
# Check for outdated dependencies
cargo outdated

# Analyze binary size
cargo bloat --release -p nym-node

# Generate dependency graph
cargo tree -p nym-api

# Run with instrumentation
cargo run --features profiling -p nym-node

# Check for security advisories
cargo audit
```

### Database Tools

```bash
# SQLx CLI for migrations
cargo install sqlx-cli

# Create new migration
cd nym-api && sqlx migrate add <migration_name>

# Prepare query metadata for offline compilation
cargo sqlx prepare --workspace

# View database schema
./nym-api/enter_db.sh
```

### Development Scripts

- `scripts/build_topology.py`: Generate network topology files
- `scripts/node_api_check.py`: Verify node API endpoints
- `scripts/network_tunnel_manager.sh`: Manage network tunnels
- `scripts/localnet_start.sh`: Start a local test network
- Various deployment scripts in `deployment/` for different environments

## Debugging

- Enable more verbose logging with the RUST_LOG environment variable:
  ```
  RUST_LOG=debug,nym_node=trace cargo run -p nym-node -- run --mode mixnode
  ```
- Use the HTTP API endpoints for status information
- Check monitoring data in the database for network performance metrics
- For complex issues, use tracing tools to follow packet flow
- Enable backtraces: `RUST_BACKTRACE=full`
- For WASM debugging: Use browser developer tools with source maps

## Deployment and Advanced Configurations

### Deployment Structure

The `deployment/` directory contains Ansible playbooks and configurations for various deployment scenarios:

- **`aws/`**: AWS-specific deployment configurations
- **`mixnode/`**: Mixnode deployment playbooks
- **`gateway/`**: Gateway deployment playbooks
- **`validator/`**: Validator node deployment
- **`sandbox-v2/`**: Complete sandbox environment setup
- **`big-dipper-2/`**: Block explorer deployment

### Sandbox V2 Deployment

The sandbox-v2 deployment (`deployment/sandbox-v2/`) provides a complete test environment:

```bash
# Key playbooks:
- deploy.yaml              # Main deployment orchestrator
- deploy-mixnodes.yaml    # Deploy mixnodes
- deploy-gateways.yaml    # Deploy gateways
- deploy-validators.yaml  # Deploy validator nodes
- deploy-nym-api.yaml     # Deploy API services
```

### Custom Environment Setup

To create a custom environment:

1. Copy an existing env file: `cp envs/sandbox.env envs/custom.env`
2. Modify the network endpoints and contract addresses
3. Update the `NETWORK_NAME` to your identifier
4. Set appropriate mnemonics and keys (use fresh ones for production!)

### Contract Addresses

Contract addresses are network-specific and defined in environment files:
- Mixnet contract: Manages mixnode/gateway registry
- Vesting contract: Handles token vesting schedules
- Coconut contracts: Privacy-preserving credentials
- Name service: Human-readable address mapping
- Ecash contract: Electronic cash functionality

### Local Network Setup

For a completely local network:
```bash
# Start local chain
./scripts/localnet_start.sh

# Deploy contracts
cd contracts
make deploy-local

# Start nodes with local config
dotenv -f envs/local.env -- cargo run -p nym-node -- run --mode mixnode
```

## Common Issues and Troubleshooting

### Database Issues

- When modifying database queries, you must update SQLx query caches:
  ```bash
  cargo sqlx prepare
  ```
- If you see SQLx errors about missing query files, this is likely the cause
- For "database is locked" errors with SQLite, ensure only one process accesses the DB
- For PostgreSQL connection issues, verify DATABASE_URL and that the server is running

### API Connection Issues

- Check the environment variables pointing to the APIs (NYM_API, NYXD)
- Verify network connectivity and API health endpoints
- For authentication issues, check node keys and credentials
- Common endpoints to verify:
  - API health: `$NYM_API/health`
  - Chain status: `$NYXD/status`
  - Contract info: `$NYXD/cosmwasm/wasm/v1/contract/$CONTRACT_ADDRESS`

### Build Problems

- Clean dependencies with `cargo clean` for a fresh build
- Check for compatible Rust version (1.80+ recommended)
- For smart contract builds, ensure wasm-opt is installed: `npm install -g wasm-opt`
- For cross-compilation issues, check target-specific dependencies
- WASM build issues: Ensure wasm32-unknown-unknown target is installed:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- For "cannot find -lpq" errors, install PostgreSQL development files:
  ```bash
  # Ubuntu/Debian
  sudo apt-get install libpq-dev
  # macOS
  brew install postgresql
  ```

### Environment Issues

- Contract address mismatches: Ensure you're using the correct environment file
- "Account sequence mismatch": The account nonce is out of sync, wait and retry
- Token decimal issues: Sandbox uses different decimal places than mainnet
- API version mismatches: Ensure your local API version matches the network
- "Insufficient funds": Get test tokens from faucet (sandbox) or check balance
- Gateway/mixnode bonding issues: Verify minimum stake requirements

## Working with Routes and Monitoring

1. Route monitoring metrics are stored in a `routes` table with:
   - Layer node IDs (layer1, layer2, layer3, gw)
   - Success flag (boolean)
   - Timestamp

2. To analyze routes:
   - Check `NetworkAccount` and `AccountingRoute` in `nym-network-monitor/src/accounting.rs`
   - View monitoring logic in `common/nymsphinx/chunking/monitoring.rs`
   - Observe how routes are submitted to the database in the `submit_accounting_routes_to_db` function

## Performance Optimization

### Profiling and Benchmarking

```bash
# Run benchmarks
cargo bench -p nym-node

# Profile with perf (Linux)
cargo build --release --features profiling
perf record --call-graph=dwarf ./target/release/nym-node run --mode mixnode
perf report

# Generate flamegraph
cargo install flamegraph
cargo flamegraph --bin nym-node -- run --mode mixnode
```

### Common Performance Considerations

- Use bounded channels for backpressure
- Batch database operations where possible
- Monitor memory usage with `RUST_LOG=nym_node::metrics=debug`
- Use connection pooling for database connections
- Consider using `jemalloc` for better memory allocation performance