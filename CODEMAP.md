# Nym Repository Codemap
<!-- AIDEV-NOTE: This codemap provides structural navigation for the Nym privacy platform monorepo -->
<!-- Last updated: 2024-10-22 (branch: drazen/lp-reg) -->

## Quick Navigation Index

| Component | Location | Purpose |
|-----------|----------|---------|
| [Main Executables](#main-executables) | Root directories | Core binaries and services |
| [Client Implementations](#client-implementations) | `/clients/` | Various client types |
| [Common Libraries](#common-libraries) | `/common/` | 70+ shared modules |
| [Smart Contracts](#smart-contracts) | `/contracts/` | CosmWasm contracts |
| [SDKs](#sdks) | `/sdk/` | Multi-language SDKs |
| [WASM Modules](#wasm-modules) | `/wasm/` | Browser implementations |
| [Service Providers](#service-providers) | `/service-providers/` | Exit nodes & routers |
| [Tools](#tools-and-utilities) | `/tools/` | CLI tools & utilities |
| [Configuration](#configuration-and-environments) | `/envs/` | Environment configs |

## Repository Structure Overview

```
nym/
├── Cargo.toml              # Workspace manifest (170+ members)
├── Cargo.lock              # Locked dependencies
├── Makefile                # Build automation
├── CLAUDE.md               # Development guidelines
├── envs/                   # Environment configurations
│   ├── local.env          # Local development
│   ├── sandbox.env        # Test network
│   ├── mainnet.env        # Production
│   └── canary.env         # Pre-release
├── assets/                 # Images, logos, fonts
├── docker/                 # Docker configurations
└── scripts/               # Deployment & setup scripts
```

<!-- AIDEV-NOTE: Navigation hint - Use envs/ for network-specific configurations -->

## Main Executables

### Core Network Nodes

#### **nym-node** (v1.19.0) - Universal Node Binary
- **Path**: `/nym-node/`
- **Entry**: `src/main.rs`
- **Modes**: `mixnode`, `gateway`
- **Key Modules**:
  - `cli/` - Command-line interface
  - `config/` - Configuration management
  - `node/` - Core node logic
  - `wireguard/` - WireGuard VPN integration
  - `throughput_tester/` - Performance testing

<!-- AIDEV-NOTE: Complex area - nym-node replaces legacy gateway and mixnode binaries -->

#### **nym-api** - Network API Server
- **Path**: `/nym-api/`
- **Entry**: `src/main.rs`
- **Database**: PostgreSQL with SQLx
- **Migrations**: `/migrations/` (25+ migration files)
- **Key Subsystems**:
  - `circulating_supply_api/` - Token supply tracking
  - `ecash/` - E-cash credential management
  - `epoch_operations/` - Epoch advancement
  - `network_monitor/` - Health monitoring
  - `node_performance/` - Performance metrics
  - `nym_nodes/` - Node registry

#### **gateway** (Legacy, v1.1.36)
- **Path**: `/gateway/`
- **Status**: Being phased out for nym-node
- **New**: `src/node/lp_listener/` (branch: drazen/lp-reg)

### Supporting Services

| Service | Path | Purpose |
|---------|------|---------|
| `nym-network-monitor` | `/nym-network-monitor/` | Network reliability testing |
| `nym-validator-rewarder` | `/nym-validator-rewarder/` | Reward calculation |
| `nyx-chain-watcher` | `/nyx-chain-watcher/` | Blockchain monitoring |
| `nym-credential-proxy` | `/nym-credential-proxy/` | Credential services |
| `nym-statistics-api` | `/nym-statistics-api/` | Statistics aggregation |
| `nym-node-status-api` | `/nym-node-status-api/` | Node status tracking |

## Client Implementations

### Directory: `/clients/`

```
clients/
├── native/                 # Native Rust client
│   └── websocket-requests/ # WebSocket protocol
├── socks5/                 # SOCKS5 proxy client
├── validator/              # Blockchain validator client
└── webassembly/           # Browser-based client
```

<!-- AIDEV-NOTE: Pattern reference - All clients use common/client-core for shared functionality -->

## Common Libraries

### Directory: `/common/` (70+ modules)

### Core Infrastructure
| Module | Purpose | Key Types |
|--------|---------|-----------|
| `nym-common` | Shared utilities | Constants, helpers |
| `types` | Common data types | NodeId, MixId |
| `config` | Configuration system | Config traits |
| `commands` | CLI structures | Command builders |
| `bin-common` | Binary utilities | Logging, banners |

### Cryptography & Security
| Module | Purpose | Dependencies |
|--------|---------|-------------|
| `crypto` | Crypto primitives | Ed25519, X25519 |
| `credentials` | Credential system | BLS12-381 |
| `credentials-interface` | Interface definitions | - |
| `credential-verification` | Validation logic | - |
| `pemstore` | PEM storage | - |

### Network Protocol (Sphinx)
<!-- AIDEV-NOTE: Complex area - Sphinx is the core privacy protocol -->

```
nymsphinx/
├── types/              # Core types
├── chunking/           # Message fragmentation
├── forwarding/         # Packet forwarding
├── routing/            # Route selection
├── addressing/         # Address handling
├── anonymous-replies/  # SURB system
├── acknowledgements/   # ACK handling
├── cover/              # Cover traffic
├── params/             # Protocol parameters
└── framing/            # Wire format
```

### New Components (Branch: drazen/lp-reg)
<!-- AIDEV-NOTE: Current branch changes - These are new additions -->

| Module | Path | Status |
|--------|------|--------|
| `nym-lp` | `/common/nym-lp/` | New LP protocol |
| `nym-lp-common` | `/common/nym-lp-common/` | LP utilities |
| `nym-kcp` | `/common/nym-kcp/` | KCP protocol |

### Client Systems
```
client-core/
├── config-types/       # Configuration types
├── gateways-storage/   # Gateway persistence
└── surb-storage/       # SURB storage

client-libs/
├── gateway-client/     # Gateway connection
├── mixnet-client/      # Mixnet interaction
└── validator-client/   # Blockchain queries
```

### Additional Common Modules

**Storage & Data**:
- `statistics/` - Statistical collection
- `topology/` - Network topology
- `node-tester-utils/` - Testing utilities
- `ticketbooks-merkle/` - Merkle trees

**Advanced Features**:
- `dkg/` - Distributed Key Generation
- `ecash-signer-check/` - E-cash validation
- `nym_offline_compact_ecash/` - Offline e-cash

**Blockchain**:
- `ledger/` - Ledger operations
- `nyxd-scraper/` - Chain scraping
- `cosmwasm-smart-contracts/` - Contract interfaces

**Utilities**:
- `task/` - Async task management
- `async-file-watcher/` - File watching
- `nym-cache/` - Caching layer
- `nym-metrics/` - Metrics (Prometheus)
- `bandwidth-controller/` - Bandwidth accounting

## Smart Contracts

### Directory: `/contracts/`

<!-- AIDEV-NOTE: Navigation hint - All contracts use CosmWasm 2.2.2 -->

```
contracts/
├── Cargo.toml                      # Workspace config
├── .cargo/config.toml             # WASM build config
├── coconut-dkg/                   # DKG contract
├── ecash/                         # E-cash contract
├── mixnet/                        # Node registry
├── vesting/                       # Token vesting
├── nym-pool/                      # Liquidity pool
├── multisig/                      # Multi-sig wallet
├── performance/                   # Performance tracking
└── mixnet-vesting-integration-tests/
```

### Contract Build Process
```bash
make contracts              # Build all
make contract-schema       # Generate schemas
make wasm-opt-contracts    # Optimize
```

## SDKs

### Directory: `/sdk/`

```
sdk/
├── rust/
│   └── nym-sdk/           # Primary Rust SDK
├── typescript/
│   ├── packages/          # NPM packages
│   ├── codegen/          # Code generation
│   └── examples/         # Usage examples
└── ffi/
    ├── cpp/              # C++ bindings
    ├── go/               # Go bindings
    └── shared/           # Shared FFI code
```

## WASM Modules

### Directory: `/wasm/`

| Module | Purpose | Build Command |
|--------|---------|---------------|
| `client` | Browser client | `make` in directory |
| `mix-fetch` | Privacy fetch API | `make` in directory |
| `node-tester` | Network testing | `make` in directory |
| `zknym-lib` | Zero-knowledge lib | `make` in directory |

<!-- AIDEV-NOTE: Pattern reference - WASM modules compile from Rust using wasm-pack -->

## Service Providers

### Directory: `/service-providers/`

```
service-providers/
├── network-requester/      # Exit node for external requests
├── ip-packet-router/       # IP packet routing (VPN-like)
└── common/                 # Shared utilities
```

## Tools and Utilities

### Directory: `/tools/`

### Public Tools
| Tool | Path | Purpose |
|------|------|---------|
| `nym-cli` | `/tools/nym-cli/` | Node management CLI |
| `nym-id-cli` | `/tools/nym-id-cli/` | Identity management |
| `nymvisor` | `/tools/nymvisor/` | Process supervisor |
| `nym-nr-query` | `/tools/nym-nr-query/` | Network queries |
| `echo-server` | `/tools/echo-server/` | Testing server |

### Internal Tools
```
internal/
├── mixnet-connectivity-check/  # Network diagnostics
├── contract-state-importer/    # Migration tools
├── validator-status-check/     # Validator health
├── ssl-inject/                 # SSL injection
├── testnet-manager/            # Testnet management
└── sdk-version-bump/           # Version management
```

## Configuration and Environments

### Environment Files: `/envs/`

<!-- AIDEV-NOTE: Navigation hint - Always use dotenv -f envs/[env].env for proper configuration -->

| Environment | File | API Endpoint | Use Case |
|------------|------|--------------|----------|
| Local | `local.env` | localhost | Development |
| Sandbox | `sandbox.env` | sandbox-nym-api1.nymtech.net | Testing |
| Mainnet | `mainnet.env` | validator.nymtech.net | Production |
| Canary | `canary.env` | - | Pre-release |

### Key Environment Variables
```bash
NETWORK_NAME            # Network identifier
NYM_API                 # API endpoint
NYXD                    # Blockchain RPC
MIXNET_CONTRACT_ADDRESS # Contract addresses
MNEMONIC               # Test mnemonic (NEVER in production)
RUST_LOG               # Logging level
DATABASE_URL           # PostgreSQL connection
```

## Build System

### Primary Build Commands
```bash
make build              # Debug build
make build-release      # Release build
make test              # Run tests
make clippy            # Lint code
make fmt               # Format code
make contracts         # Build contracts
make sdk-wasm-build    # Build WASM
```

### Workspace Configuration

<!-- AIDEV-NOTE: Complex area - Root Cargo.toml manages 170+ workspace members -->

**Root Cargo.toml Structure**:
- `[workspace]` - Lists all 170+ members
- `[workspace.dependencies]` - Shared dependency versions
- `[workspace.lints]` - Shared lint rules
- `[profile.*]` - Build profiles

## Database Structure

### SQLx Usage Pattern
- **Compile-time verified**: All queries checked at build
- **Migration files**: In package `/migrations/` directories
- **Query cache**: `.sqlx/` directory

### Key Tables (nym-api)
```sql
-- Network monitoring
mixnode_status
gateway_status
routes
monitor_run

-- Node registry
nym_nodes
node_descriptions

-- Performance
node_uptime
node_performance
```

## Current Branch Context (drazen/lp-reg)

### New Additions
- `/common/nym-lp/` - Low-level protocol implementation
- `/common/nym-lp-common/` - LP common utilities
- `/common/nym-kcp/` - KCP protocol
- `/gateway/src/node/lp_listener/` - LP listener

### Modified Files
```
M Cargo.lock
M Cargo.toml
M common/registration/
M common/wireguard/
M gateway/
M nym-node/
M nym-node/nym-node-metrics/
```

## Navigation Patterns

<!-- AIDEV-NOTE: Navigation hint - Use these patterns to quickly find code -->

### Finding Code by Type
| Code Type | Look In |
|-----------|---------|
| Main executables | Root directories with `src/main.rs` |
| Libraries | `/common/` with descriptive names |
| Contracts | `/contracts/[name]/src/contract.rs` |
| Tests | Colocated with source, `#[cfg(test)]` |
| Configurations | `/envs/` and `config/` subdirs |
| Database queries | Files with `.sql` or SQLx macros |
| API endpoints | `/nym-api/src/` subdirectories |
| CLI commands | `/cli/commands/` in executables |

### Common Import Locations
```rust
// Crypto
use nym_crypto::asymmetric::{ed25519, x25519};

// Network
use nym_sphinx::forwarding::packet::MixPacket;
use nym_topology::NymTopology;

// Client
use nym_client_core::client::Client;

// Configuration
use nym_network_defaults::NymNetworkDetails;

// Contracts
use nym_mixnet_contract_common::*;
```

## Module Relationships

<!-- AIDEV-NOTE: Complex area - Understanding dependencies helps navigation -->

### Dependency Graph (Simplified)
```
nym-node
├── common/nym-common
├── common/crypto
├── common/nymsphinx
├── common/topology
├── common/client-libs/validator-client
└── common/wireguard

nym-api
├── common/nym-common
├── nym-api-requests
├── common/client-libs/validator-client
├── common/credentials
└── sqlx (database)

clients/native
├── common/client-core
├── common/client-libs/gateway-client
├── common/nymsphinx
└── common/credentials
```

## Development Workflows

### Adding New Feature
1. Check `/envs/` for configuration
2. Find similar code in `/common/`
3. Implement in appropriate module
4. Add tests colocated with code
5. Update `/nym-api/` if needed
6. Run `make test` and `make clippy`

### Debugging Network Issues
1. Start with `/nym-network-monitor/`
2. Check `/common/topology/` for routing
3. Review `/common/nymsphinx/` for protocol
4. Examine logs with `RUST_LOG=debug`

### Contract Development
1. Create in `/contracts/[name]/`
2. Use existing contracts as templates
3. Build with `make contracts`
4. Test with `cw-multi-test`

---

<!-- AIDEV-NOTE: This codemap is optimized for LLM navigation. Use Ctrl+F to quickly find components -->