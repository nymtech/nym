# Nym Function Lexicon
<!-- AIDEV-NOTE: This lexicon catalogs key functions, signatures, and API patterns across the Nym codebase -->
<!-- Last updated: 2024-10-22 (branch: drazen/lp-reg) -->

## Quick Reference Index

| Category | Section | Key Operations |
|----------|---------|----------------|
| [Node Operations](#1-node-operations) | Mixnode & Gateway | Initialization, key management, tasks |
| [Sphinx Protocol](#2-sphinx-packet-protocol) | Packet Processing | Message creation, chunking, routing |
| [Client APIs](#3-client-apis) | Client Operations | Connection, sending, receiving |
| [Network Topology](#4-network-topology) | Routing | Topology queries, route selection |
| [Blockchain](#5-blockchain-operations) | Validator Client | Queries, transactions, contracts |
| [REST APIs](#6-rest-api-endpoints) | HTTP Handlers | API routes and responses |
| [Credentials](#7-credential--ecash) | E-cash | Credential creation, verification |
| [Smart Contracts](#8-smart-contracts) | CosmWasm | Entry points, messages |
| [Common Patterns](#9-common-patterns) | Conventions | Naming, errors, async |

---

## 1. Node Operations

### nym-node Core Functions
<!-- AIDEV-NOTE: Complex area - nym-node unifies mixnode and gateway functionality -->

**Module**: `nym-node/src/node/mod.rs`

```rust
// Node initialization
pub async fn initialise_node(
    config: &Config,
    rng: &mut impl CryptoRng + RngCore,
) -> Result<NodeData, NymNodeError>

// Key management
pub fn load_x25519_wireguard_keypair(
    paths: &KeysPaths,
) -> Result<x25519::KeyPair, NymNodeError>

pub fn load_ed25519_identity_keypair(
    paths: &KeysPaths,
) -> Result<ed25519::KeyPair, NymNodeError>

// Gateway-specific initialization
impl GatewayTasksData {
    pub async fn new(
        config: &GatewayTasksConfig,
        client_storage: ClientStorage,
    ) -> Result<GatewayTasksData, GatewayError>

    pub fn initialise(
        config: &GatewayTasksConfig,
        force_init: bool,
    ) -> Result<(), GatewayError>
}

// Service provider initialization
impl ServiceProvidersData {
    pub fn initialise_client_keys<R: RngCore + CryptoRng>(
        rng: &mut R,
        gateway_paths: &GatewayPaths,
    ) -> Result<ed25519::KeyPair, GatewayError>

    pub async fn initialise_network_requester<R>(
        rng: &mut R,
        config: &Config,
    ) -> Result<Option<LocalNetworkRequester>, GatewayError>
}
```

### Gateway Task Builder Pattern
**Module**: `gateway/src/node/mod.rs`

```rust
pub struct GatewayTasksBuilder {
    // Builder methods
    pub fn new(
        identity_keypair: Arc<ed25519::KeyPair>,
        config: Config,
        client_storage: ClientStorage,
    ) -> GatewayTasksBuilder

    pub fn set_network_requester_opts(
        &mut self,
        opts: Option<LocalNetworkRequesterOpts>
    ) -> &mut Self

    pub fn set_ip_packet_router_opts(
        &mut self,
        opts: Option<LocalIpPacketRouterOpts>
    ) -> &mut Self

    pub async fn build_and_run(
        self,
        shutdown: TaskManager,
    ) -> Result<(), GatewayError>
}
```

<!-- AIDEV-NOTE: Pattern reference - Builder pattern is common for complex initialization -->

---

## 2. Sphinx Packet Protocol

### Message Construction & Processing
**Module**: `common/nymsphinx/src/message.rs`

```rust
// Core message types
pub enum NymMessage {
    Plain(Vec<u8>),
    Repliable(RepliableMessage),
    Reply(ReplyMessage),
}

impl NymMessage {
    // Constructors
    pub fn new_plain(msg: Vec<u8>) -> NymMessage
    pub fn new_repliable(msg: RepliableMessage) -> NymMessage
    pub fn new_reply(msg: ReplyMessage) -> NymMessage
    pub fn new_additional_surbs_request(
        recipient: Recipient,
        amount: u32
    ) -> NymMessage

    // Processing
    pub fn pad_to_full_packet_lengths(
        self,
        plaintext_per_packet: usize
    ) -> PaddedMessage

    pub fn split_into_fragments<R: Rng>(
        self,
        rng: &mut R,
        packet_size: PacketSize,
    ) -> Vec<Fragment>

    pub fn remove_padding(self) -> Result<NymMessage, NymMessageError>

    // Queries
    pub fn is_reply_surb_request(&self) -> bool
    pub fn available_sphinx_plaintext_per_packet(
        &self,
        packet_size: PacketSize
    ) -> usize
    pub fn required_packets(&self, packet_size: PacketSize) -> usize
}
```

### Payload Building & Preparation
**Module**: `common/nymsphinx/src/preparer.rs`

```rust
pub struct NymPayloadBuilder {
    // Main preparation methods
    pub async fn prepare_chunk_for_sending(
        &mut self,
        message: NymMessage,
        topology: &NymTopology,
    ) -> Result<Vec<MixPacket>, NymPayloadBuilderError>

    pub async fn prepare_reply_chunk_for_sending(
        &mut self,
        reply: NymMessage,
        reply_surb: ReplySurb,
    ) -> Result<Vec<MixPacket>, NymPayloadBuilderError>

    // SURB generation
    pub fn generate_reply_surbs(
        &mut self,
        amount: u32,
        topology: &NymTopology,
    ) -> Result<Vec<SurbAck>, NymPayloadBuilderError>

    // Fragment splitting
    pub fn pad_and_split_message(
        &mut self,
        message: NymMessage,
    ) -> Result<Vec<Fragment>, NymPayloadBuilderError>
}

// Builder constructors
pub fn build_regular<R: CryptoRng + Rng>(
    rng: R,
    sender_address: Option<Recipient>,
) -> NymPayloadBuilder

pub fn build_reply(
    sender_address: Recipient,
    sender_tag: AnonymousSenderTag,
) -> NymPayloadBuilder
```

### Chunking & Fragmentation
**Module**: `common/nymsphinx/chunking/src/lib.rs`

<!-- AIDEV-NOTE: Complex area - Chunking splits messages into Sphinx-sized packets -->

```rust
// Main chunking function
pub fn split_into_sets(
    message: &[u8],
    max_plaintext_size: usize,
    max_fragments_per_set: usize,
) -> Result<Vec<Vec<Fragment>>, ChunkingError>

// Fragment monitoring (optional feature)
pub mod monitoring {
    pub fn enable()
    pub fn enabled() -> bool
    pub fn fragment_received(fragment: &Fragment)
    pub fn fragment_sent(
        fragment: &Fragment,
        client_nonce: i32,
        destination: PublicKey
    )
}
```

---

## 3. Client APIs

### Gateway Client
**Module**: `common/client-libs/gateway-client/src/lib.rs`

```rust
pub struct GatewayClient {
    // Connection management
    pub async fn connect(
        config: GatewayClientConfig,
    ) -> Result<GatewayClient, GatewayClientError>

    pub async fn authenticate(
        &mut self,
        credentials: Credentials,
    ) -> Result<(), GatewayClientError>

    // Message operations
    pub async fn send_mix_packet(
        &self,
        packet: MixPacket,
    ) -> Result<(), GatewayClientError>

    pub async fn receive_messages(
        &mut self,
    ) -> Result<Vec<ReconstructedMessage>, GatewayClientError>
}

// Packet routing
pub struct PacketRouter {
    pub fn new(
        mix_tx: MixnetMessageSender,
        ack_tx: AcknowledgementSender,
    ) -> PacketRouter

    pub async fn route_packet(
        &self,
        packet: MixPacket,
    ) -> Result<(), PacketRouterError>
}
```

### Mixnet Client
**Module**: `common/client-libs/mixnet-client/src/lib.rs`

```rust
pub struct Client {
    // Core client operations
    pub async fn new(config: Config) -> Result<Client, ClientError>

    pub async fn send_message(
        &mut self,
        recipient: Recipient,
        message: Vec<u8>,
    ) -> Result<(), ClientError>

    pub async fn receive_message(
        &mut self,
    ) -> Result<ReconstructedMessage, ClientError>

    // Connection management
    pub fn is_connected(&self) -> bool
    pub async fn reconnect(&mut self) -> Result<(), ClientError>
}

// Send without response trait
pub trait SendWithoutResponse {
    fn send_without_response(
        &self,
        packet: MixPacket,
    ) -> io::Result<()>
}
```

<!-- AIDEV-NOTE: Pattern reference - Async/await is standard for network operations -->

### Client Core Initialization
**Module**: `common/client-core/src/init.rs`

```rust
// Key generation
pub fn generate_new_client_keys<R: CryptoRng + Rng>(
    rng: &mut R,
) -> (ed25519::KeyPair, x25519::KeyPair)

// Storage initialization
pub async fn init_storage(
    paths: &ClientPaths,
) -> Result<ClientStorage, ClientCoreError>

// Configuration setup
pub fn setup_client_config(
    id: &str,
    network: Network,
) -> Result<Config, ClientCoreError>
```

---

## 4. Network Topology

### Topology Management
**Module**: `common/topology/src/lib.rs`

```rust
pub struct NymTopology {
    // Query methods
    pub fn mixnodes(&self) -> &[RoutingNode]
    pub fn gateways(&self) -> &[RoutingNode]
    pub fn layer_nodes(&self, layer: MixLayer) -> Vec<&RoutingNode>

    // Route selection
    pub fn random_route<R: Rng>(
        &self,
        rng: &mut R,
    ) -> Option<Vec<RoutingNode>>

    pub fn get_node_by_id(&self, node_id: NodeId) -> Option<&RoutingNode>
}

// Route provider
pub struct NymRouteProvider {
    pub fn new(topology: NymTopology) -> NymRouteProvider

    pub fn random_route<R: Rng>(
        &self,
        rng: &mut R,
    ) -> Option<Vec<RoutingNode>>
}

// Topology provider trait
pub trait TopologyProvider {
    async fn get_topology(&self) -> Result<NymTopology, NymTopologyError>
    async fn refresh_topology(&mut self) -> Result<(), NymTopologyError>
}
```

### Routing Node
**Module**: `common/topology/src/node.rs`

```rust
pub struct RoutingNode {
    pub fn node_id(&self) -> NodeId
    pub fn identity_key(&self) -> &ed25519::PublicKey
    pub fn sphinx_key(&self) -> &x25519::PublicKey
    pub fn mix_host(&self) -> &SocketAddr
    pub fn clients_ws_address(&self) -> Option<&Url>
}
```

---

## 5. Blockchain Operations

### Validator Client
**Module**: `common/client-libs/validator-client/src/client.rs`

<!-- AIDEV-NOTE: Complex area - Handles all blockchain interactions -->

```rust
pub struct Client<C, S = NoSigner> {
    // Contract queries
    pub async fn query_contract_state<T>(
        &self,
        contract: &str,
        query: T,
    ) -> Result<ContractStateResponse, ValidatorClientError>
    where T: Into<Binary>

    // Transaction execution (requires signer)
    pub async fn execute_contract_message<M>(
        &self,
        contract: &str,
        msg: M,
        funds: Vec<Coin>,
    ) -> Result<TxResponse, ValidatorClientError>
    where M: Into<Binary>

    // Specific contract operations
    pub async fn bond_mixnode(
        &self,
        mixnode: MixNode,
        cost_params: MixNodeCostParams,
        pledge: Coin,
    ) -> Result<TxResponse, ValidatorClientError>

    pub async fn unbond_mixnode(&self) -> Result<TxResponse, ValidatorClientError>

    pub async fn delegate_to_mixnode(
        &self,
        mix_id: MixId,
        amount: Coin,
    ) -> Result<TxResponse, ValidatorClientError>
}

// Nyxd-specific client
pub type DirectSigningHttpRpcNyxdClient =
    nyxd::NyxdClient<HttpRpcClient, DirectSecp256k1HdWallet>;
```

### Contract Queries
**Module**: `common/client-libs/validator-client/src/nyxd/contract_traits/`

```rust
// Mixnet contract queries
pub trait MixnetQueryClient {
    async fn get_mixnodes(&self) -> Result<Vec<MixNodeDetails>, NyxdError>
    async fn get_gateways(&self) -> Result<Vec<Gateway>, NyxdError>
    async fn get_current_epoch(&self) -> Result<Epoch, NyxdError>
    async fn get_rewarded_set(&self) -> Result<EpochRewardedSet, NyxdError>
}

// Vesting contract queries
pub trait VestingQueryClient {
    async fn get_vesting_details(&self, address: &str)
        -> Result<VestingDetails, NyxdError>
}

// E-cash contract queries
pub trait EcashQueryClient {
    async fn get_deposit(&self, id: DepositId)
        -> Result<Deposit, NyxdError>
}
```

---

## 6. REST API Endpoints

### nym-api Main Routes
**Module**: `nym-api/src/main.rs` and submodules

<!-- AIDEV-NOTE: Navigation hint - Each module contains router setup and handlers -->

```rust
// Main API setup
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Router configuration
    let app = Router::new()
        .merge(api_routes())
        .merge(swagger_ui())
        .layer(cors_layer())
        .layer(trace_layer());
}

// Core API routes (various modules)
pub fn api_routes() -> Router {
    Router::new()
        .nest("/v1/status", status_routes())
        .nest("/v1/mixnodes", mixnode_routes())
        .nest("/v1/gateways", gateway_routes())
        .nest("/v1/network", network_routes())
        .nest("/v1/ecash", ecash_routes())
}
```

### Status Routes
**Module**: `nym-api/src/status/mod.rs`

```rust
pub async fn status_handler() -> impl IntoResponse {
    Json(ApiStatusResponse {
        status: "ok",
        uptime: get_uptime(),
    })
}

pub async fn health_check() -> impl IntoResponse {
    StatusCode::OK
}
```

### Network Monitor Routes
**Module**: `nym-api/src/network_monitor/mod.rs`

```rust
pub async fn get_monitor_report(
    State(state): State<AppState>,
) -> Result<Json<MonitorReport>, ApiError> {
    // Returns network reliability report
}

pub async fn get_node_reliability(
    Path(node_id): Path<NodeId>,
    State(state): State<AppState>,
) -> Result<Json<NodeReliability>, ApiError> {
    // Returns specific node reliability
}
```

### E-cash API
**Module**: `nym-api/src/ecash/mod.rs`

```rust
pub async fn verify_credential(
    Json(credential): Json<Credential>,
    State(state): State<AppState>,
) -> Result<Json<VerificationResponse>, ApiError> {
    // Verifies e-cash credentials
}

pub async fn issue_credential(
    Json(request): Json<IssuanceRequest>,
    State(state): State<AppState>,
) -> Result<Json<IssuedCredential>, ApiError> {
    // Issues new e-cash credentials
}
```

---

## 7. Credential & E-cash

### Credential Operations
**Module**: `common/credentials/src/ecash/mod.rs`

```rust
// Credential spending
pub struct CredentialSpendingData {
    pub fn new(
        ticketbook: IssuedTicketBook,
        gateway_identity: ed25519::PublicKey,
    ) -> CredentialSpendingData

    pub fn prepare_for_spending(
        &self,
        request_id: i64,
    ) -> PreparedCredential
}

// Credential signing
pub struct CredentialSigningData {
    pub fn sign_credential(
        &self,
        blinded_credential: BlindedCredential,
    ) -> Result<BlindedSignature, CredentialError>
}

// Aggregation utilities
pub fn aggregate_verification_keys(
    keys: Vec<VerificationKey>,
) -> AggregatedVerificationKey

pub fn obtain_aggregate_wallet(
    verification_keys: Vec<VerificationKey>,
    commitments: Vec<Commitment>,
) -> Result<AggregateWallet, CredentialError>
```

### Ticketbook Operations
**Module**: `common/credentials/src/ecash/bandwidth/mod.rs`

<!-- AIDEV-NOTE: Complex area - Ticketbooks contain bandwidth credentials -->

```rust
pub struct IssuedTicketBook {
    pub fn new(
        tickets: Vec<IssuedTicket>,
        expiration: OffsetDateTime,
    ) -> IssuedTicketBook

    pub fn total_bandwidth(&self) -> Bandwidth
    pub fn is_expired(&self) -> bool
    pub fn consume_ticket(&mut self) -> Option<IssuedTicket>
}

pub struct ImportableTicketBook {
    pub fn try_from_base58(s: &str) -> Result<Self, CredentialError>
    pub fn into_issued(self) -> Result<IssuedTicketBook, CredentialError>
}
```

---

## 8. Smart Contracts

### Mixnet Contract Entry Points
**Module**: `contracts/mixnet/src/contract.rs`

```rust
#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError>

#[entry_point]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError>

#[entry_point]
pub fn query(
    deps: Deps,
    env: Env,
    msg: QueryMsg,
) -> StdResult<Binary>
```

### Execute Message Handlers
**Module**: `contracts/mixnet/src/contract.rs`

```rust
// Node operations
fn try_bond_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mixnode: MixNode,
) -> Result<Response, ContractError>

fn try_unbond_mixnode(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
) -> Result<Response, ContractError>

// Delegation operations
fn try_delegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_id: MixId,
) -> Result<Response, ContractError>

fn try_undelegate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    mix_id: MixId,
) -> Result<Response, ContractError>

// Reward operations
fn try_reward_mixnode(
    deps: DepsMut,
    env: Env,
    mix_id: MixId,
    performance: Performance,
) -> Result<Response, ContractError>
```

### Query Message Handlers
```rust
fn query_mixnode(deps: Deps, mix_id: MixId) -> StdResult<MixnodeDetails>
fn query_gateways(deps: Deps) -> StdResult<Vec<Gateway>>
fn query_rewarded_set(deps: Deps, epoch: Epoch) -> StdResult<EpochRewardedSet>
fn query_current_epoch(deps: Deps) -> StdResult<Epoch>
```

---

## 9. Common Patterns

### Function Naming Conventions
<!-- AIDEV-NOTE: Pattern reference - Consistent naming helps code discovery -->

```rust
// Constructors
pub fn new(...) -> Self                    // Standard constructor
pub fn with_defaults() -> Self             // Constructor with defaults
pub fn from_config(config: Config) -> Self // From configuration

// Async initialization
pub async fn init(...) -> Result<T>        // Async initialization
pub async fn initialise(...) -> Result<T>  // British spelling variant
pub async fn setup(...) -> Result<T>       // Setup function

// Builder pattern
pub fn builder() -> TBuilder               // Create builder
pub fn set_field(mut self, val: T) -> Self // Builder setter
pub fn build(self) -> Result<T>           // Build final object

// Getters
pub fn field(&self) -> &T                 // Immutable reference
pub fn field_mut(&mut self) -> &mut T     // Mutable reference
pub fn into_inner(self) -> T              // Consume and return inner

// Queries
pub fn is_valid(&self) -> bool            // Boolean check
pub fn has_field(&self) -> bool           // Existence check
pub fn contains(&self, item: &T) -> bool  // Contains check

// Transformations
pub fn to_type(&self) -> Type             // Convert to type
pub fn into_type(self) -> Type            // Consume and convert
pub fn try_into_type(self) -> Result<Type> // Fallible conversion
```

### Error Handling Patterns

```rust
// Custom error types with thiserror
#[derive(Error, Debug)]
pub enum ModuleError {
    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("Invalid configuration: {reason}")]
    InvalidConfig { reason: String },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

// Result type alias
pub type Result<T> = std::result::Result<T, ModuleError>;

// Error conversion
impl From<io::Error> for ModuleError {
    fn from(err: io::Error) -> Self {
        ModuleError::Io(err)
    }
}
```

### Async Patterns

```rust
// Async trait (with async-trait crate)
#[async_trait]
pub trait AsyncOperation {
    async fn perform(&self) -> Result<()>;
}

// Spawning tasks
tokio::spawn(async move {
    // Task code
});

// Channels for communication
let (tx, mut rx) = mpsc::channel(100);

// Select on multiple futures
tokio::select! {
    result = future1 => { /* handle */ },
    result = future2 => { /* handle */ },
    _ = shutdown.recv() => { /* shutdown */ },
}
```

### Storage Patterns

```rust
// SQLx queries
sqlx::query!(
    "SELECT * FROM nodes WHERE id = ?",
    node_id
)
.fetch_optional(&pool)
.await?;

// In-memory caching
use dashmap::DashMap;
let cache: DashMap<Key, Value> = DashMap::new();

// File storage
use std::fs;
fs::write(path, data)?;
let content = fs::read_to_string(path)?;
```

---

## 10. Import Reference

### Standard Imports by Category

```rust
// Nym crypto
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_crypto::symmetric::stream_cipher;

// Sphinx protocol
use nym_sphinx::forwarding::packet::MixPacket;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx::params::{PacketSize, DEFAULT_PACKET_SIZE};

// Client libraries
use nym_client_core::client::Client;
use nym_gateway_client::GatewayClient;
use nym_validator_client::ValidatorClient;

// Topology
use nym_topology::{NymTopology, RoutingNode};
use nym_mixnet_contract_common::NodeId;

// Configuration
use nym_network_defaults::NymNetworkDetails;
use nym_config::defaults::NymNetwork;

// Async runtime
use tokio::sync::{mpsc, RwLock, Mutex};
use tokio::time::{sleep, Duration};
use futures::{StreamExt, SinkExt};

// Error handling
use thiserror::Error;
use anyhow::{anyhow, Result, Context};

// Logging
use tracing::{debug, info, warn, error, instrument};

// Serialization
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Web framework (API)
use axum::{Router, extract::{Path, Query, State}, response::IntoResponse};
use axum::Json;
```

---

## 11. Feature Flags

### Common Feature Gates

```rust
// Client-specific features
#[cfg(feature = "client")]
#[cfg(feature = "cli")]

// Platform-specific
#[cfg(not(target_arch = "wasm32"))]
#[cfg(target_arch = "wasm32")]

// Testing
#[cfg(test)]
#[cfg(feature = "testing")]
#[cfg(feature = "contract-testing")]

// Storage backends
#[cfg(feature = "fs-surb-storage")]
#[cfg(feature = "fs-credentials-storage")]

// Network features
#[cfg(feature = "http-client")]
#[cfg(feature = "websocket")]
```

---

## Quick Lookup Tables

### Async vs Sync Functions

| Operation Type | Typically Async | Typically Sync |
|---------------|-----------------|----------------|
| Network I/O | ✓ | |
| Database queries | ✓ | |
| Contract execution | ✓ | |
| Cryptographic ops | | ✓ |
| Message construction | | ✓ |
| Configuration parsing | | ✓ |
| Topology queries | Both | Both |

### Return Type Patterns

| Pattern | Usage | Example |
|---------|-------|---------|
| `Result<T, E>` | Fallible operations | `connect() -> Result<Client>` |
| `Option<T>` | May not exist | `get_node() -> Option<Node>` |
| `impl Trait` | Return trait impl | `handler() -> impl IntoResponse` |
| `Box<dyn Trait>` | Dynamic dispatch | `create() -> Box<dyn Storage>` |
| Direct type | Infallible ops | `new() -> Self` |

### Module Organization

| Module Type | Location Pattern | Naming Convention |
|------------|------------------|-------------------|
| Binary entry | `/src/main.rs` | - |
| Library root | `/src/lib.rs` | - |
| Submodules | `/src/module/mod.rs` | snake_case |
| Tests | `/src/module/tests.rs` | #[cfg(test)] |
| Errors | `/src/error.rs` | ModuleError |
| Config | `/src/config.rs` | Config struct |

---

<!-- AIDEV-NOTE: This lexicon provides rapid function lookup. Use Ctrl+F to search for specific operations -->