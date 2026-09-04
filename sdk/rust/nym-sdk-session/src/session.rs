// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The provisioning session: mnemonic → issued ticketbooks → registered gateways.
//!
//! The session uses the [`BandwidthController`] in one of two modes, fixed at construction by
//! [`SessionConfig::automatic_topups`]:
//!
//! - **Automatic top-up** (`Some(policy)`): the controller is built with the credential fetcher
//!   installed and its event loop runs for the session's lifetime. The controller's own sweep
//!   (which fires once at startup and then per policy) provisions and restocks the managed
//!   WireGuard types; provisioning here is a readiness wait. Spending goes through the running
//!   controller's request sender.
//! - **One-shot** (`None`, the default): nothing runs in the background. Spending goes through a
//!   non-running controller that has no credential fetcher (so it cannot deposit); provisioning
//!   builds a short-lived, non-running controller (over the same store handle) with a fresh
//!   fetcher, issues inline exactly the ticketbooks that are missing, then drops the controller
//!   and cleans its fetcher up.
//!
//! Gateway-side top-up of a live tunnel spends already-stored tickets and is driven by the
//! datapath, not here.

use nym_bandwidth_controller::config::BandwidthControllerConfig;
use nym_bandwidth_controller::error::BandwidthControllerError;
use nym_bandwidth_controller::requests::BandwidthControllerRequestSender;
use nym_bandwidth_controller::{
    AvailableTicketbooks, BandwidthController, BandwidthTicketProvider, CredentialFetcher,
    TicketType,
};
use nym_bandwidth_fetcher::{NyxdCredentialFetcher, NyxdGlobalDataFetcher};
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_credential_storage::storage::Storage;
use nym_credentials_interface::BandwidthCredential;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::peer::{DHKeyPair, LpRemotePeer};
use nym_network_defaults::NymNetworkDetails;
use nym_registration_client::{
    LpDvpnRegistrationClient, LpGatewayClient, NestedLpDvpnRegistrationClient, NestedLpSession,
};
use nym_registration_common::WireguardConfiguration;
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use rand010::rngs::SysRng;
use rand010::SeedableRng;
use sha2::{Digest, Sha256};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use url::Url;
use zeroize::Zeroizing;

use crate::config::{RestockPolicy, SessionConfig};
use crate::dvpn::{DvpnDirectory, QuicBridge};
use crate::error::SessionError;
use crate::fetcher::TimeoutFetcher;
use crate::gateway::{self, GatewayInfo, GatewaySpec, SelectedGateway, WgRole};
use crate::registration_cache::RegistrationCache;
use nym_api_requests::models::described::v2::NymNodeDescriptionV2;

/// Number of tickets to reserve when checking for / spending a stored ticketbook.
const TICKETS_TO_SPEND: u32 = 1;
/// Timeout for nym-api requests.
const API_TIMEOUT: Duration = Duration::from_secs(30);
/// Overall budget for `ensure_ticketbooks` (deposit + issuance + signing-data fetches). Generous —
/// it backstops *unforeseen* stalls; the per-fetch bounds live in [`TimeoutFetcher`].
const PROVISIONING_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// The ticket types needed for a given tunnel shape. Never includes mixnet types, so a session can
/// never deposit for bandwidth it does not use; single-hop omits the exit type so it provisions no
/// unused exit ticketbook.
fn needed_ticket_types(two_hop: bool) -> Vec<TicketType> {
    let mut types = vec![TicketType::V1WireguardEntry];
    if two_hop {
        types.push(TicketType::V1WireguardExit);
    }
    types
}

/// Everything the datapath needs to bring up ONE WireGuard hop.
pub struct HopConfig {
    /// Gateway-returned WireGuard configuration (pubkey, PSK, endpoint, IPs).
    pub wg_config: WireguardConfiguration,
    /// The client WireGuard private key generated for this hop.
    pub client_private_key: x25519::PrivateKey,
    /// The gateway's ed25519 identity.
    pub gateway_identity: ed25519::PublicKey,
    /// Directory metadata for this hop's gateway (identity, node id, country, IP).
    pub gateway: GatewayInfo,
    /// QUIC bridge params for this hop, set only for a QUIC entry hop (see
    /// [`Session::register_two_hop_quic`]); `None` for direct/exit hops.
    pub bridge: Option<QuicBridge>,
}

/// The result of registering a tunnel: one hop for single-hop, two for two-hop.
pub struct Registration {
    /// Entry (or sole) hop.
    pub entry: HopConfig,
    /// Exit hop; `None` for single-hop tunnels.
    pub exit: Option<HopConfig>,
}

/// The chain-backed credential fetcher both modes use: a `NyxdCredentialFetcher` (deposits NYM and
/// aggregates issued wallets) wrapped in [`TimeoutFetcher`] so the read-only global-signing-data
/// fetches are time-bounded. Issuance itself (the deposit) is deliberately not timed.
type ChainFetcher = TimeoutFetcher<NyxdCredentialFetcher<DirectSigningHttpRpcNyxdClient>>;

/// Chain-side inputs shared by both controller modes: the signing chain client, the issuance client
/// id, the on-disk stores, and the topology-scoped controller config.
struct ChainSetup {
    nyxd: Arc<DirectSigningHttpRpcNyxdClient>,
    /// Stable, non-reversible client id derived from the mnemonic (see [`derive_client_id`]).
    client_id: Zeroizing<Vec<u8>>,
    /// The session's persistent credential store handle (survives bring-down / bring-up). Clones
    /// share the underlying pool, so one-shot provisioning works on the same handle as spending —
    /// a second pool on the file would make closing either one wait on the other's file handles.
    storage: PersistentStorage,
    /// The fetcher's pending-request recovery database; carries an interrupted issuance forward
    /// across fetcher instances.
    fetcher_db: PathBuf,
    /// `managed_ticket_types` scoped to the topology's WireGuard types (two-hop: entry + exit;
    /// single-hop: entry only — never an unused exit book, and never a mixnet type).
    config: BandwidthControllerConfig,
}

impl ChainSetup {
    /// Build a fresh chain-backed fetcher. Every one-shot provision builds its own because teardown
    /// `cleanup`s (closes) the fetcher's recovery store; the running mode builds exactly one.
    async fn build_fetcher(&self) -> Result<ChainFetcher, SessionError> {
        let fetcher =
            NyxdCredentialFetcher::new(self.nyxd.clone(), &self.fetcher_db, self.client_id.clone())
                .await
                .map_err(|e| SessionError::Issuance(e.to_string()))?;
        Ok(TimeoutFetcher::new(fetcher))
    }

    /// The one-shot provisioning task for `types`: builds a fresh fetcher and runs
    /// [`provision_once`] over the session's store handle. Owns everything it touches so it can be
    /// spawned and left to finish even if the caller stops waiting.
    fn provision(
        &self,
        types: Vec<TicketType>,
    ) -> impl Future<Output = Result<(), SessionError>> + Send + 'static {
        let nyxd = self.nyxd.clone();
        let client_id = self.client_id.clone();
        let storage = self.storage.clone();
        let fetcher_db = self.fetcher_db.clone();
        let config = self.config.clone();
        async move {
            let fetcher = NyxdCredentialFetcher::new(nyxd, &fetcher_db, client_id)
                .await
                .map_err(|e| SessionError::Issuance(e.to_string()))?;
            provision_once(storage, TimeoutFetcher::new(fetcher), config, types).await
        }
    }
}

/// How the session owns its bandwidth controller (absent with an external provider).
enum ControllerMode {
    /// Automatic top-up: the controller event loop runs with the fetcher installed; the sender is
    /// the spending provider and drives readiness waits.
    Running {
        sender: BandwidthControllerRequestSender,
        task: JoinHandle<()>,
    },
    /// One-shot: no event loop. Provisioning spins up a short-lived controller per call from
    /// `chain`; `lock` serialises those calls so two concurrent provisions can't issue twice for
    /// the same type.
    OneShot {
        chain: ChainSetup,
        lock: tokio::sync::Mutex<()>,
    },
}

/// Open the persistent credential store at `path`, creating its directory.
async fn open_store(path: &Path) -> Result<PersistentStorage, SessionError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| SessionError::Storage(e.to_string()))?;
    }
    PersistentStorage::init(path)
        .await
        .map_err(|e| SessionError::Storage(e.to_string()))
}

/// Read the stored ticketbook stock through the public `Storage` trait.
async fn read_stock<St: Storage>(storage: &St) -> Result<AvailableTicketbooks, SessionError> {
    let info = storage
        .get_ticketbooks_info()
        .await
        .map_err(|e| SessionError::Storage(e.to_string()))?;
    AvailableTicketbooks::try_from(info).map_err(|e| SessionError::Storage(e.to_string()))
}

/// One-shot issuance on a **non-running** controller: for each of `types` whose stock the
/// controller's own restock predicate judges low or about to expire, issue one ticketbook inline
/// via [`BandwidthController::fetch_ticketbook`]; then require every requested type to be usable.
/// A sufficiently stocked type is never fetched.
///
/// The controller is ALWAYS torn down afterwards (success or failure): it is dropped and its
/// fetcher cleaned up (closing the fetcher's recovery store). `storage` is the caller's shared
/// handle and is left open. No event loop ever runs, so nothing here can deposit except the
/// explicit fetches above.
pub(crate) async fn provision_once<St, F>(
    storage: St,
    fetcher: F,
    config: BandwidthControllerConfig,
    types: Vec<TicketType>,
) -> Result<(), SessionError>
where
    St: Storage + 'static,
    F: CredentialFetcher + Clone + 'static,
{
    // The controller takes its fetcher by value and never hands it back, so give it a clone of the
    // shared handle and keep ours for the cleanup below.
    let controller = BandwidthController::new(storage.clone())
        .with_config(config.clone())
        .with_credential_fetcher(fetcher.clone());

    let outcome = issue_missing(&controller, &storage, &config, &types).await;

    // Teardown: a non-running controller has no in-flight work, so dropping it is complete; the
    // fetcher's recovery store is closed through our retained handle.
    drop(controller);
    fetcher.cleanup().await;

    outcome
}

/// The issuance half of [`provision_once`]: fetch what is low, then verify what was requested.
async fn issue_missing<St: Storage>(
    controller: &BandwidthController<St>,
    storage: &St,
    config: &BandwidthControllerConfig,
    types: &[TicketType],
) -> Result<(), SessionError> {
    let stock = read_stock(storage).await?;
    for &typ in types {
        if stock.needs_restock(typ, config) {
            tracing::info!("issuing a {typ} ticketbook (stock at or below the restock threshold)");
            controller
                .fetch_ticketbook(typ)
                .await
                .map_err(|e| SessionError::Issuance(format!("{typ}: {e}")))?;
        } else {
            tracing::debug!("{typ} ticketbooks sufficiently stocked; no issuance needed");
        }
    }

    // The one-shot analogue of the running mode's readiness gate: every requested type must now be
    // spendable. (Signing data is best-effort here, as it is there — a missing piece is fetched at
    // spend time through the provider's public-data fetcher.)
    let stock = read_stock(storage).await?;
    for &typ in types {
        if !stock.contains_minimal_tickets(typ, config) {
            return Err(SessionError::Issuance(format!(
                "no usable {typ} ticketbook after issuance"
            )));
        }
    }
    Ok(())
}

/// Running-mode provisioning: wait until `types` are stocked and spendable. The controller's own
/// sweep (fired once at startup, then per policy) is what issues. If the wait reports the types as
/// neither stocked nor in flight — the startup fetch already failed, or the wait was processed
/// before the startup tick (the first tick is not guaranteed to run ahead of an already-queued
/// request) — request a restock of exactly these types once and wait again. A restock for a type
/// already in flight is a no-op on the controller side, so this can never double-issue.
/// `restock_ticketbooks` is documented as a manual safety valve rather than a routine call, which
/// is why it is only the recovery step and not the first move.
async fn await_stocked(
    sender: &BandwidthControllerRequestSender,
    types: Vec<TicketType>,
) -> Result<(), SessionError> {
    let issuance = |e: BandwidthControllerError| SessionError::Issuance(e.to_string());
    match sender.wait_for_ticketbooks(types.clone()).await {
        Ok(()) => Ok(()),
        Err(BandwidthControllerError::TicketbooksUnavailable) => {
            tracing::info!(
                "required ticketbooks are neither stocked nor being fetched; requesting a restock"
            );
            sender
                .restock_ticketbooks(types.clone())
                .await
                .map_err(issuance)?;
            sender.wait_for_ticketbooks(types).await.map_err(issuance)
        }
        Err(e) => Err(issuance(e)),
    }
}

/// Provisioning facade over the credential + registration machinery.
pub struct Session {
    api: nym_http_api_client::Client,
    /// Bandwidth provider used for all ticket spending (registration + gateway top-up). The
    /// running controller's sender, the one-shot non-running controller, or a caller-supplied
    /// external provider.
    provider: Arc<dyn BandwidthTicketProvider>,
    /// Present when the session owns its controller; drives provisioning and shutdown.
    mode: Option<ControllerMode>,
    cancel: CancellationToken,
    /// dVPN gateway directory (empty if none configured or the fetch failed).
    directory: Option<DvpnDirectory>,
    /// Persistent per-gateway registration cache; `None` when reuse is disabled (then nothing is
    /// read from nor written to disk). Guarded by a std mutex — held only across synchronous
    /// lookup/insert calls, never across an await.
    reg_cache: Option<std::sync::Mutex<RegistrationCache>>,
    /// The topology this session was configured for (`SessionConfig::two_hop`). A single-hop
    /// session provisions entry ticketbooks only, so a two-hop registration on it is rejected up
    /// front with [`SessionError::TopologyMismatch`] rather than failing later with a generic
    /// "ticketbooks unavailable" from the exit-type wait. With an external bandwidth provider the
    /// caller provisions, so the guard is not applied.
    two_hop: bool,
}

impl Session {
    /// Build a session. Unless an external `bandwidth_provider` is supplied, this connects the
    /// signing chain client, opens the credential store, and sets the bandwidth controller up in
    /// the mode selected by `automatic_topups` (see the module docs).
    pub async fn new(
        config: SessionConfig,
        cancel: CancellationToken,
    ) -> Result<Self, SessionError> {
        let SessionConfig {
            mnemonic,
            network,
            credential_store_path,
            data_path,
            dvpn_directory_url,
            automatic_topups,
            bandwidth_provider,
            reuse_registrations,
            two_hop,
        } = config;

        // Registration reuse: load the per-network cache from the data directory (before
        // `network`/`data_path` are moved into the controller). Disabled => no cache at all.
        let reg_cache = reuse_registrations.then(|| {
            std::sync::Mutex::new(RegistrationCache::load(
                &data_path,
                network.network_name.clone(),
            ))
        });

        let api_url_str = network
            .endpoints
            .iter()
            .find_map(|e| e.api_url.clone())
            .ok_or(SessionError::MissingEndpoint { which: "nym-api" })?;
        let api_url = Url::parse(&api_url_str).map_err(|source| SessionError::InvalidUrl {
            which: "nym-api",
            url: api_url_str.clone(),
            source,
        })?;
        let api = nym_http_api_client::Client::new(api_url, Some(API_TIMEOUT));

        // Best-effort dVPN directory (monikers + QUIC bridge params).
        let directory = match dvpn_directory_url {
            Some(url) => match DvpnDirectory::fetch(&url).await {
                Ok(dir) => Some(dir),
                Err(e) => {
                    tracing::warn!("failed to fetch dVPN directory at {url}: {e}");
                    Some(DvpnDirectory::default())
                }
            },
            None => None,
        };

        // Bandwidth provider: an external one (caller runs its own controller) or our own.
        let (provider, mode) = match bandwidth_provider {
            Some(external) => (external, None),
            None => {
                let (provider, mode) = Self::own_controller(
                    mnemonic,
                    network,
                    credential_store_path,
                    data_path,
                    automatic_topups,
                    two_hop,
                    cancel.clone(),
                )
                .await?;
                (provider, Some(mode))
            }
        };

        Ok(Self {
            api,
            provider,
            mode,
            cancel,
            directory,
            reg_cache,
            two_hop,
        })
    }

    /// Build the chain client + credential store and set the controller up in the selected mode.
    /// `two_hop` scopes the managed WireGuard ticket types (single-hop manages entry only).
    async fn own_controller(
        mnemonic: bip39::Mnemonic,
        network: NymNetworkDetails,
        credential_store_path: Option<PathBuf>,
        data_path: PathBuf,
        automatic_topups: Option<RestockPolicy>,
        two_hop: bool,
        cancel: CancellationToken,
    ) -> Result<(Arc<dyn BandwidthTicketProvider>, ControllerMode), SessionError> {
        let nyxd_url = network
            .endpoints
            .first()
            .map(|e| e.nyxd_url.clone())
            .ok_or(SessionError::MissingEndpoint { which: "nyxd" })?;

        // Derive a stable, non-reversible client id from the mnemonic entropy BEFORE moving the
        // mnemonic into the chain client (so we neither clone it nor hand the raw entropy on).
        let client_id = derive_client_id(&mnemonic);

        // Direct-signing chain client from the mnemonic (consumes it — no clone).
        let nyxd = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic_and_network_details(
            nyxd_url.as_str(),
            network,
            mnemonic,
        )?;
        let nyxd = Arc::new(nyxd);

        let store_path = credential_store_path.unwrap_or_else(|| data_path.join("credentials.db"));
        let managed = needed_ticket_types(two_hop);
        let config = match automatic_topups {
            Some(policy) => {
                let mut config: BandwidthControllerConfig = policy.into();
                config.managed_ticket_types = managed;
                config
            }
            None => BandwidthControllerConfig {
                managed_ticket_types: managed,
                ..Default::default()
            },
        };
        let storage = open_store(&store_path).await?;
        let chain = ChainSetup {
            nyxd,
            client_id,
            storage: storage.clone(),
            fetcher_db: data_path.join("fetcher-requests.db"),
            config,
        };

        if automatic_topups.is_some() {
            // Automatic top-up: fetcher installed at construction, event loop running. The
            // controller's sweep fires once immediately at startup and then per policy, so it
            // provisions and restocks the managed WireGuard types on its own.
            let fetcher = match chain.build_fetcher().await {
                Ok(fetcher) => fetcher,
                Err(e) => {
                    storage.close().await;
                    return Err(e);
                }
            };
            let controller = BandwidthController::new(storage)
                .with_config(chain.config.clone())
                .with_credential_fetcher(fetcher);
            let sender = controller.get_request_sender();
            let shutdown = ShutdownToken::new_from_tokio_token(cancel);
            let task = tokio::spawn(async move { controller.run(shutdown).await });
            let provider: Arc<dyn BandwidthTicketProvider> = Arc::new(sender.clone());
            Ok((provider, ControllerMode::Running { sender, task }))
        } else {
            // One-shot: no event loop. Spending goes through a non-running controller with NO
            // credential fetcher — nothing on this path can deposit. Its public-data fetcher lets a
            // spend fetch signing data that was missing at issuance time.
            let provider = BandwidthController::new(storage)
                .with_config(chain.config.clone())
                .with_credential_public_data_fetcher(NyxdGlobalDataFetcher::new(
                    chain.nyxd.clone(),
                ));
            let provider: Arc<dyn BandwidthTicketProvider> = Arc::new(provider);
            Ok((
                provider,
                ControllerMode::OneShot {
                    chain,
                    lock: tokio::sync::Mutex::new(()),
                },
            ))
        }
    }

    /// Ensure the WireGuard ticketbooks needed for the tunnel shape are stored and usable, issuing
    /// (and depositing) only for a required type whose stock is low or about to expire.
    ///
    /// Bounded by the overall provisioning budget and the session's cancellation token. In one-shot
    /// mode the issuance runs in a spawned task that is never aborted: a call that is cancelled or
    /// times out returns at once while the task finishes (or records its deposit for recovery) in
    /// the background, so a deposit is never dropped mid-flight.
    ///
    /// With an external bandwidth provider this is a no-op — the caller provisions.
    pub async fn ensure_ticketbooks(&self, two_hop: bool) -> Result<(), SessionError> {
        self.ensure_ticket_types(needed_ticket_types(two_hop)).await
    }

    /// [`ensure_ticketbooks`](Self::ensure_ticketbooks) scoped to exactly `types` — used by the
    /// registration paths to provision only for hops that actually need a fresh (ticket-spending)
    /// registration. An empty `types` is a no-op, so a fully cache-served registration never
    /// triggers ticketbook provisioning (or its deposits).
    async fn ensure_ticket_types(&self, types: Vec<TicketType>) -> Result<(), SessionError> {
        if types.is_empty() {
            return Ok(());
        }
        let Some(mode) = &self.mode else {
            // external provider: the caller is responsible for provisioning
            return Ok(());
        };
        let timed_out = || {
            Err(SessionError::ProvisioningTimeout {
                after: PROVISIONING_TIMEOUT,
            })
        };
        match mode {
            ControllerMode::Running { sender, .. } => {
                // Safe to race cancel around the wait: deposits run inside the controller task, not
                // in this future, so dropping it loses nothing.
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
                    res = tokio::time::timeout(PROVISIONING_TIMEOUT, await_stocked(sender, types))
                        => res.unwrap_or_else(|_| timed_out()),
                }
            }
            ControllerMode::OneShot { chain, lock } => {
                // Acquiring the lock is itself cancellable, so a caller queued behind an in-progress
                // provision is not stuck for that provision's whole budget. (`tokio::sync::Mutex::
                // lock` is cancel-safe.) The holder is bounded by `PROVISIONING_TIMEOUT`, so the wait
                // is already finite.
                let _guard = tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => return Err(SessionError::Cancelled),
                    guard = lock.lock() => guard,
                };
                // Spawned, never aborted: the deposit must not be dropped mid-flight if the caller
                // cancels or the budget elapses (see `ensure_ticketbooks`).
                let task = tokio::spawn(chain.provision(types));
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
                    res = tokio::time::timeout(PROVISIONING_TIMEOUT, task) => match res {
                        Ok(Ok(outcome)) => outcome,
                        Ok(Err(join)) => Err(SessionError::Issuance(format!(
                            "provisioning task failed: {join}"
                        ))),
                        Err(_elapsed) => timed_out(),
                    },
                }
            }
        }
    }

    /// Obtain a spendable bandwidth credential for `gateway_id` by spending one
    /// stored WireGuard ticket. Feeds the gateway `metadata` endpoint's
    /// `topup_bandwidth` so a long-lived tunnel can extend its bandwidth.
    pub async fn obtain_wireguard_credential(
        &self,
        gateway_id: ed25519::PublicKey,
        role: WgRole,
    ) -> Result<BandwidthCredential, SessionError> {
        let ticket_type = match role {
            WgRole::Entry => TicketType::V1WireguardEntry,
            WgRole::Exit => TicketType::V1WireguardExit,
        };
        let prepared = self
            .provider
            .get_ecash_ticket(
                ticket_type,
                gateway_id,
                TICKETS_TO_SPEND,
                OffsetDateTime::now_utc(),
            )
            .await
            .map_err(|e| SessionError::Issuance(e.to_string()))?
            .ok_or_else(|| {
                SessionError::Issuance("no stored ticket available for top-up".into())
            })?;
        Ok(BandwidthCredential::from(prepared.data))
    }

    /// The bandwidth provider used for ticket spending. Hand this to the datapath so a live tunnel
    /// can top up from stored tickets.
    pub fn bandwidth_provider(&self) -> Arc<dyn BandwidthTicketProvider> {
        self.provider.clone()
    }

    /// Fetch the current described-node topology once.
    async fn fetch_topology(&self) -> Result<Vec<NymNodeDescriptionV2>, SessionError> {
        Ok(self.api.get_all_described_nodes_v2().await?)
    }

    /// Fetch the topology, racing the cancellation token. Safe to abort: no ticket is spent during
    /// selection, so registration callers use this for the pre-spend phase and then run the
    /// (ticket-spending) exchange without racing cancel.
    async fn fetch_topology_cancellable(&self) -> Result<Vec<NymNodeDescriptionV2>, SessionError> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = self.fetch_topology() => res,
        }
    }

    /// Select a WireGuard-capable gateway for the given role (fetches topology).
    pub async fn select_gateway(
        &self,
        spec: &GatewaySpec,
        role: WgRole,
    ) -> Result<SelectedGateway, SessionError> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = async {
                let nodes = self.fetch_topology().await?;
                gateway::select(&nodes, spec, role, self.directory.as_ref(), false, &[])
            } => res,
        }
    }

    /// Register a single-hop tunnel against one gateway via the LP
    /// single-gateway `register_dvpn` path (spends a `V1WireguardEntry` ticket).
    pub async fn register_single_hop(
        &self,
        gateway: &GatewaySpec,
    ) -> Result<Registration, SessionError> {
        self.register_single_inner(gateway).await
    }

    /// Run `f` over the registration cache; `None` when reuse is disabled. The lock is only
    /// ever held across the synchronous `f` (never an await).
    fn with_cache<R>(&self, f: impl FnOnce(&mut RegistrationCache) -> R) -> Option<R> {
        self.reg_cache.as_ref().map(|cache| {
            f(&mut cache
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()))
        })
    }

    /// Look up a cached registration for the gateway with `identity` in `role`; a hit is
    /// assembled into a [`HopConfig`] carrying the given directory metadata (no gateway
    /// exchange, no ticket spent) and logged so the zero-spend behavior is auditable. Always
    /// `None` when reuse is disabled.
    fn cached_hop(
        &self,
        identity: &ed25519::PublicKey,
        gateway: GatewayInfo,
        role: WgRole,
    ) -> Option<HopConfig> {
        let cached = self.with_cache(|cache| cache.lookup(identity, role))??;
        tracing::info!(
            "reusing cached registration for {} ({role:?}) — no ticket spent",
            identity.to_base58_string()
        );
        Some(HopConfig {
            wg_config: cached.wg_config,
            client_private_key: cached.client_private_key,
            gateway_identity: *identity,
            gateway,
            bridge: None,
        })
    }

    /// Persist a fresh registration and assemble its [`HopConfig`] — the shared tail of every
    /// successful `register_dvpn` exchange. Persisting is a no-op when reuse is disabled
    /// (nothing is written to disk then — see `SessionConfig::reuse_registrations`).
    fn finalize_hop(
        &self,
        identity: &ed25519::PublicKey,
        gateway: GatewayInfo,
        role: WgRole,
        client_private_key: x25519::PrivateKey,
        wg_config: WireguardConfiguration,
    ) -> HopConfig {
        self.with_cache(|cache| cache.insert(identity, role, &client_private_key, &wg_config));
        HopConfig {
            wg_config,
            client_private_key,
            gateway_identity: *identity,
            gateway,
            bridge: None,
        }
    }

    /// Remove a cached registration for (gateway, role) — the fallback path when a reused
    /// registration fails to establish (see `Tunnel::await_established` in `smoldvpn`):
    /// invalidate the failed hop(s), then register again for a fresh (ticket-spending) peer.
    /// A missing entry (or reuse disabled) is a no-op.
    pub fn invalidate_registration(&self, gateway: &ed25519::PublicKey, role: WgRole) {
        self.with_cache(|cache| cache.remove(gateway, role));
    }

    async fn register_single_inner(
        &self,
        gateway: &GatewaySpec,
    ) -> Result<Registration, SessionError> {
        // Everything up to and including the LP handshake spends no ticket and stays cancellable
        // (topology fetch here; handshake inside `register_hop`). Only the ticket-spending
        // `register_dvpn` call runs without racing cancel, so a cancel can't drop the future after
        // the gateway has processed the spend and lose the ticket.
        let nodes = self.fetch_topology_cancellable().await?;
        let selected = gateway::select(
            &nodes,
            gateway,
            WgRole::Entry,
            self.directory.as_ref(),
            false,
            &[],
        )?;
        // Cache first: a reusable registration needs no ticketbooks and no gateway exchange.
        if let Some(hop) = self.cached_hop(&selected.identity, selected.info(), WgRole::Entry) {
            return Ok(Registration {
                entry: hop,
                exit: None,
            });
        }
        self.ensure_ticket_types(vec![TicketType::V1WireguardEntry])
            .await?;
        let hop = self
            .register_hop(&selected, TicketType::V1WireguardEntry)
            .await?;
        Ok(Registration {
            entry: hop,
            exit: None,
        })
    }

    /// Register a two-hop tunnel: an outer LP session with the entry gateway,
    /// the exit registered via entry forwarding, then the entry itself.
    pub async fn register_two_hop(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
    ) -> Result<Registration, SessionError> {
        self.register_two_hop_inner(entry, exit, false, &[]).await
    }

    /// Like [`register_two_hop`](Self::register_two_hop), but excludes a set of
    /// entry gateway identities from **entry** selection. A caller that has
    /// implicated an entry gateway (e.g. one that does not forward the tunnelled
    /// exit handshake, so the exit never establishes) passes it here so a fresh
    /// registration re-selects a different entry instead of re-picking the bad one.
    ///
    /// Exclusion applies to entry selection only; the exit is still chosen distinct
    /// from the entry. A pinned entry `Identity` that appears in `avoid_entries` is
    /// never substituted — registration fails with the distinct-gateways error (see
    /// [`gateway::select`]). Passing an empty `avoid_entries` is equivalent to
    /// [`register_two_hop`](Self::register_two_hop).
    pub async fn register_two_hop_avoiding_entries(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
        avoid_entries: &[ed25519::PublicKey],
    ) -> Result<Registration, SessionError> {
        self.register_two_hop_inner(entry, exit, false, avoid_entries)
            .await
    }

    /// Like [`register_two_hop`](Self::register_two_hop), but the ENTRY gateway
    /// must advertise a QUIC bridge (per the configured dVPN directory). The
    /// returned `entry` hop carries its [`QuicBridge`] in `bridge`. Fails with
    /// [`SessionError::NoQuicGateway`] if no QUIC entry matches the spec.
    /// (QUIC only fronts the two-hop entry leg; the exit is registered normally.)
    pub async fn register_two_hop_quic(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
    ) -> Result<Registration, SessionError> {
        self.register_two_hop_inner(entry, exit, true, &[]).await
    }

    /// Like [`register_two_hop_quic`](Self::register_two_hop_quic), but excludes
    /// `avoid_entries` from **entry** selection, exactly as
    /// [`register_two_hop_avoiding_entries`](Self::register_two_hop_avoiding_entries)
    /// does for the plain two-hop path. A non-forwarding entry is just as possible
    /// behind a QUIC bridge (and the QUIC-capable pool is smaller, so re-picking it
    /// is likelier), so a retrying caller passes its implicated entries here too.
    pub async fn register_two_hop_quic_avoiding_entries(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
        avoid_entries: &[ed25519::PublicKey],
    ) -> Result<Registration, SessionError> {
        self.register_two_hop_inner(entry, exit, true, avoid_entries)
            .await
    }

    /// `avoid_entries` is a set of entry gateway identities excluded from entry
    /// selection (see [`register_two_hop_avoiding_entries`](Self::register_two_hop_avoiding_entries)).
    async fn register_two_hop_inner(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
        entry_quic: bool,
        avoid_entries: &[ed25519::PublicKey],
    ) -> Result<Registration, SessionError> {
        // A single-hop session manages entry ticketbooks only; fail clearly before any network
        // work rather than later, from the exit-type wait, with a generic "unavailable" error.
        if !self.two_hop {
            return Err(SessionError::TopologyMismatch);
        }
        let mut rng = rand010::rngs::StdRng::try_from_rng(&mut SysRng)?;

        // Selection and the LP handshake spend no ticket and stay cancellable (topology fetch here,
        // handshake below); only the ticket-spending calls (`handshake_and_register_dvpn`,
        // `register_dvpn`) run without racing the cancel token, so a cancel can't drop the future
        // mid-spend and lose a ticket. Topology is fetched once.
        let nodes = self.fetch_topology_cancellable().await?;
        let entry_gw = gateway::select(
            &nodes,
            entry,
            WgRole::Entry,
            self.directory.as_ref(),
            entry_quic,
            avoid_entries,
        )?;
        // Exclude the entry gateway so a two-hop tunnel never uses one gateway twice.
        let exit_gw = gateway::select(
            &nodes,
            exit,
            WgRole::Exit,
            self.directory.as_ref(),
            false,
            std::slice::from_ref(&entry_gw.identity),
        )?;

        // The entry hop carries QUIC bridge params only when QUIC was required
        // (selection guarantees `entry_gw.quic` is `Some` in that case).
        let entry_bridge = if entry_quic {
            entry_gw.quic.clone()
        } else {
            None
        };

        // Cache first: each hop may be independently reusable. Only uncached hops need
        // ticketbooks, an LP session, and a (ticket-spending) registration.
        // Both hops served from cache: no ticketbooks, no LP exchange at all.
        let (cached_entry, cached_exit) = match (
            self.cached_hop(&entry_gw.identity, entry_gw.info(), WgRole::Entry),
            self.cached_hop(&exit_gw.identity, exit_gw.info(), WgRole::Exit),
        ) {
            (Some(mut entry_hop), Some(exit_hop)) => {
                entry_hop.bridge = entry_bridge;
                return Ok(Registration {
                    entry: entry_hop,
                    exit: Some(exit_hop),
                });
            }
            partial => partial,
        };

        // Ticketbooks only for the hop(s) that will actually spend.
        let mut needed = Vec::new();
        if cached_entry.is_none() {
            needed.push(TicketType::V1WireguardEntry);
        }
        if cached_exit.is_none() {
            needed.push(TicketType::V1WireguardExit);
        }
        self.ensure_ticket_types(needed).await?;

        let entry_lp = lp_info(&entry_gw)?;
        let exit_lp = lp_info(&exit_gw)?;

        // Outer session with the entry gateway — needed to register either hop (the exit is
        // registered THROUGH the entry's LP forwarding).
        let entry_keypair = Arc::new(DHKeyPair::new(&mut rng));
        let entry_peer =
            LpRemotePeer::new(entry_lp.x25519).with_key_digests(entry_lp.expected_kem_key_hashes);
        let mut entry_client = LpGatewayClient::<TcpStream>::new_with_default_config(
            entry_keypair,
            entry_peer,
            entry_lp.address,
            entry_lp.ciphersuite,
            entry_lp.lp_protocol_version,
        );
        // The LP handshake spends no ticket, so it stays cancellable — otherwise a stalled/
        // black-holed entry gateway would ignore the cancel token until the OS TCP timeout. Only
        // the ticket-spending registration calls below run without racing cancel.
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => return Err(SessionError::Cancelled),
            r = entry_client.perform_handshake() => r.map_err(|source| SessionError::Registration {
                address: entry_lp.address,
                source,
            })?,
        }

        // Exit hop: reuse or register via entry forwarding.
        let exit_hop = match cached_exit {
            Some(hop) => hop,
            None => {
                let exit_keypair = Arc::new(DHKeyPair::new(&mut rng));
                let exit_peer = LpRemotePeer::new(exit_lp.x25519)
                    .with_key_digests(exit_lp.expected_kem_key_hashes);
                let mut nested = NestedLpSession::new(
                    exit_lp.address,
                    exit_keypair,
                    exit_peer,
                    exit_lp.ciphersuite,
                    exit_lp.lp_protocol_version,
                );
                nested
                    .perform_handshake(&mut entry_client)
                    .await
                    .map_err(|source| SessionError::Registration {
                        address: exit_lp.address,
                        source,
                    })?;
                let exit_wg = x25519::KeyPair::new(&mut rand::thread_rng());
                let exit_cfg = NestedLpDvpnRegistrationClient::new(&mut nested, &mut entry_client)
                    .register(
                        &mut rng,
                        &exit_wg,
                        &exit_gw.identity,
                        self.provider.as_ref(),
                        None,
                        TicketType::V1WireguardExit,
                    )
                    .await
                    .map_err(|source| SessionError::Registration {
                        address: exit_lp.address,
                        source,
                    })?;
                self.finalize_hop(
                    &exit_gw.identity,
                    exit_gw.info(),
                    WgRole::Exit,
                    x25519::PrivateKey::from_secret(exit_wg.private_key().to_bytes()),
                    exit_cfg,
                )
            }
        };

        // Entry hop: reuse or register on the outer session.
        let mut entry_hop = match cached_entry {
            Some(hop) => hop,
            None => {
                let entry_wg = x25519::KeyPair::new(&mut rand::thread_rng());
                let entry_cfg = LpDvpnRegistrationClient::new(&mut entry_client)
                    .register(
                        &mut rng,
                        &entry_wg,
                        &entry_gw.identity,
                        self.provider.as_ref(),
                        None,
                        TicketType::V1WireguardEntry,
                    )
                    .await
                    .map_err(|source| SessionError::Registration {
                        address: entry_lp.address,
                        source,
                    })?;
                self.finalize_hop(
                    &entry_gw.identity,
                    entry_gw.info(),
                    WgRole::Entry,
                    x25519::PrivateKey::from_secret(entry_wg.private_key().to_bytes()),
                    entry_cfg,
                )
            }
        };
        entry_hop.bridge = entry_bridge;

        Ok(Registration {
            entry: entry_hop,
            exit: Some(exit_hop),
        })
    }

    /// Register a single hop against an already-selected gateway.
    async fn register_hop(
        &self,
        selected: &SelectedGateway,
        ticket_type: TicketType,
    ) -> Result<HopConfig, SessionError> {
        let lp = lp_info(selected)?;
        let mut rng = rand010::rngs::StdRng::try_from_rng(&mut SysRng)?;
        let keypair = Arc::new(DHKeyPair::new(&mut rng));
        let peer = LpRemotePeer::new(lp.x25519).with_key_digests(lp.expected_kem_key_hashes);
        let mut client = LpGatewayClient::<TcpStream>::new_with_default_config(
            keypair,
            peer,
            lp.address,
            lp.ciphersuite,
            lp.lp_protocol_version,
        );

        // The LP handshake spends no ticket, so it stays cancellable (a stalled gateway would
        // otherwise hang past the cancel token); only the ticket-spending `register_dvpn` below runs
        // without racing cancel.
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => return Err(SessionError::Cancelled),
            r = client.perform_handshake() => r.map_err(|source| SessionError::Registration {
                address: lp.address,
                source,
            })?,
        }

        let wg = x25519::KeyPair::new(&mut rand::thread_rng());
        let cfg = LpDvpnRegistrationClient::new(&mut client)
            .register(
                &mut rng,
                &wg,
                &selected.identity,
                self.provider.as_ref(),
                None,
                ticket_type,
            )
            .await
            .map_err(|source| SessionError::Registration {
                address: lp.address,
                source,
            })?;

        let role = match ticket_type {
            TicketType::V1WireguardExit => WgRole::Exit,
            _ => WgRole::Entry,
        };
        Ok(self.finalize_hop(
            &selected.identity,
            selected.info(),
            role,
            x25519::PrivateKey::from_secret(wg.private_key().to_bytes()),
            cfg,
        ))
    }

    /// Shut down the session's bandwidth controller (if it owns one) so the credential store is
    /// closed cleanly. Stored tickets are retained. Running mode awaits the event loop, whose exit
    /// path cleans up the fetcher and closes the store; one-shot mode closes the non-running
    /// provider's store (a detached provisioning task shares that handle and cleans up its own
    /// fetcher when it finishes).
    pub async fn shutdown(mut self) {
        self.cancel.cancel();
        match self.mode.take() {
            Some(ControllerMode::Running { task, .. }) => {
                let _ = task.await;
            }
            Some(ControllerMode::OneShot { .. }) => self.provider.close().await,
            None => {}
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Best-effort: signal the controller to stop if `shutdown()` was not called. The spawned
        // task observes the cancelled token and cleans up on its own.
        self.cancel.cancel();
    }
}

/// Derive a stable, non-reversible client id from a mnemonic's entropy. Domain-separated so it can
/// never collide with another use of the same entropy, and hashed so the raw entropy is never
/// handed to issuance.
fn derive_client_id(mnemonic: &bip39::Mnemonic) -> Zeroizing<Vec<u8>> {
    let entropy = Zeroizing::new(mnemonic.to_entropy());
    let mut hasher = Sha256::new();
    hasher.update(b"nym-sdk-session::client-id::v1");
    hasher.update(entropy.as_slice());
    Zeroizing::new(hasher.finalize().to_vec())
}

/// Extract the LP info for a selected gateway or fail with a clear error.
fn lp_info(
    selected: &SelectedGateway,
) -> Result<nym_registration_common::NymNodeLPInformation, SessionError> {
    selected
        .node
        .node
        .lp_data
        .clone()
        .ok_or_else(|| SessionError::MalformedGateway {
            identity: selected.identity.to_base58_string(),
            reason: "gateway advertises no LP data".to_string(),
        })
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
