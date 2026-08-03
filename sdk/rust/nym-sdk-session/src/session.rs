// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The provisioning session: mnemonic → issued ticketbooks → registered gateways.
//!
//! The session runs a [`BandwidthController`] event loop (the single writer to the credential
//! store) and performs all ticket spending through its [`BandwidthControllerRequestSender`], which
//! implements [`BandwidthTicketProvider`]. Chain-side restock (depositing NYM for new ticketbooks)
//! is off by default and opted into via [`SessionConfig::automatic_topups`]; gateway-side top-up of
//! a live tunnel spends already-stored tickets and is driven by the datapath, not here.

use nym_bandwidth_controller::config::BandwidthControllerConfig;
use nym_bandwidth_controller::requests::BandwidthControllerRequestSender;
use nym_bandwidth_controller::{BandwidthController, BandwidthTicketProvider, TicketType};
use nym_bandwidth_fetcher::NyxdCredentialFetcher;
use nym_credentials_interface::BandwidthCredential;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::peer::{DHKeyPair, LpRemotePeer};
use nym_network_defaults::NymNetworkDetails;
use nym_registration_client::{LpRegistrationClient, NestedLpSession};
use nym_registration_common::WireguardConfiguration;
use nym_task::ShutdownToken;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use rand010::rngs::SysRng;
use rand010::SeedableRng;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
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

/// The WireGuard ticket types a dVPN session ever provisions. Never includes mixnet types, so a
/// session can never deposit for bandwidth it does not use.
fn wireguard_ticket_types() -> Vec<TicketType> {
    vec![TicketType::V1WireguardEntry, TicketType::V1WireguardExit]
}

/// The ticket types needed for a given tunnel shape.
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

/// A session-owned, running bandwidth controller: the request sender used for spending/provisioning
/// and the join handle for the background event loop.
struct OwnedController {
    sender: BandwidthControllerRequestSender,
    task: JoinHandle<()>,
}

/// Provisioning facade over the credential + registration machinery.
pub struct Session {
    api: nym_http_api_client::Client,
    /// Bandwidth provider used for all ticket spending (registration + gateway top-up). Either the
    /// session's own controller sender or a caller-supplied external provider.
    provider: Arc<dyn BandwidthTicketProvider>,
    /// Present when the session spawned its own controller; drives provisioning and shutdown.
    owned: Option<OwnedController>,
    cancel: CancellationToken,
    /// dVPN gateway directory (empty if none configured or the fetch failed).
    directory: Option<DvpnDirectory>,
    /// Persistent per-gateway registration cache; `None` when reuse is disabled (then nothing is
    /// read from nor written to disk). Guarded by a std mutex — held only across synchronous
    /// lookup/insert calls, never across an await.
    reg_cache: Option<std::sync::Mutex<RegistrationCache>>,
}

impl Session {
    /// Build a session. Unless an external `bandwidth_provider` is supplied, this connects the
    /// signing chain client, opens the credential store, wires the bandwidth controller + credential
    /// fetcher (scoped to WireGuard ticket types), and spawns the controller event loop.
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
        let (provider, owned) = match bandwidth_provider {
            Some(external) => (external, None),
            None => {
                let (provider, owned) = Self::spawn_controller(
                    mnemonic,
                    network,
                    credential_store_path,
                    data_path,
                    automatic_topups,
                    cancel.clone(),
                )
                .await?;
                (provider, Some(owned))
            }
        };

        Ok(Self {
            api,
            provider,
            owned,
            cancel,
            directory,
            reg_cache,
        })
    }

    /// Build the nyxd client + credential store + scoped fetcher, then spawn the controller loop.
    async fn spawn_controller(
        mnemonic: bip39::Mnemonic,
        network: NymNetworkDetails,
        credential_store_path: Option<PathBuf>,
        data_path: PathBuf,
        automatic_topups: Option<RestockPolicy>,
        cancel: CancellationToken,
    ) -> Result<(Arc<dyn BandwidthTicketProvider>, OwnedController), SessionError> {
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

        // Persistent credential store (survives bring-down / bring-up).
        let store_path = credential_store_path.unwrap_or_else(|| data_path.join("credentials.db"));
        if let Some(parent) = store_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SessionError::Storage(e.to_string()))?;
        }
        let storage = nym_credential_storage::initialise_persistent_storage(&store_path).await;

        // Credential fetcher: deposits NYM and aggregates issued wallets. Wrapped so the
        // read-only global-signing-data fetches are time-bounded: unresponsive ecash signers
        // (a permanent fact of the distributed deployment) must yield a fast fetch error —
        // which the controller's best-effort store path tolerates, persisting the paid-for
        // ticketbook anyway — rather than hanging the controller loop and losing the book.
        // Issuance itself (the deposit) is deliberately not timed; see `TimeoutFetcher` docs.
        let fetcher_db = data_path.join("fetcher-requests.db");
        let fetcher = NyxdCredentialFetcher::new(nyxd, &fetcher_db, client_id)
            .await
            .map_err(|e| SessionError::Issuance(e.to_string()))?;
        let fetcher = TimeoutFetcher::new(fetcher);

        // The controller only ever proactively restocks (and thus deposits for) the types in
        // `managed_ticket_types`. Opt-in automatic top-up manages the WireGuard types (with the
        // caller's thresholds); the default leaves it empty, so the session provisions on demand
        // (via `ensure_ticketbooks`) but the controller never deposits in the background — while the
        // fetcher stays installed so it can serve those on-demand fetches and the global signing
        // data needed to spend. Either way, mixnet types are never in the managed set, so the
        // session can never deposit for mixnet bandwidth.
        let config = match automatic_topups {
            Some(policy) => {
                let mut config: BandwidthControllerConfig = policy.into();
                config.managed_ticket_types = wireguard_ticket_types();
                config
            }
            None => BandwidthControllerConfig {
                managed_ticket_types: Vec::new(),
                ..Default::default()
            },
        };
        let controller = BandwidthController::new(storage)
            .with_config(config)
            .with_credential_fetcher(fetcher);

        let sender = controller.get_request_sender();
        let shutdown = ShutdownToken::new_from_tokio_token(cancel.clone());
        let task = tokio::spawn(async move { controller.run(shutdown).await });

        let provider: Arc<dyn BandwidthTicketProvider> = Arc::new(sender.clone());
        Ok((provider, OwnedController { sender, task }))
    }

    /// Ensure the WireGuard ticketbooks needed for the tunnel shape are stored, issuing (and
    /// depositing) only when no usable ticketbook of a required type is already stored.
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
        let Some(owned) = &self.owned else {
            // external provider: the caller is responsible for provisioning
            return Ok(());
        };
        // Explicit restock request (works regardless of the auto-restock setting) scoped to exactly
        // the needed WireGuard types, then wait until they are usable. Race cancellation so a
        // caller that cancels mid-wait gets a prompt `Cancelled` instead of blocking; this is
        // funds-safe because the deposit itself runs in the controller task and any interrupted
        // issuance is recovered from the fetcher's pending-request store on a later fetch.
        //
        // The work arm is additionally bounded by an overall budget (defense in depth over the
        // per-fetch `TimeoutFetcher` bounds): whatever else might stall, provisioning surfaces a
        // `ProvisioningTimeout` naming unresponsive signers as the likely cause instead of
        // blocking forever. Interrupting the wait is funds-safe for the same reason as above.
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = tokio::time::timeout(PROVISIONING_TIMEOUT, async {
                owned
                    .sender
                    .restock_ticketbooks(types.clone())
                    .await
                    .map_err(|e| SessionError::Issuance(e.to_string()))?;
                owned
                    .sender
                    .wait_for_ticketbooks(types)
                    .await
                    .map_err(|e| SessionError::Issuance(e.to_string()))
            }) => res.unwrap_or(Err(SessionError::ProvisioningTimeout {
                after: PROVISIONING_TIMEOUT,
            })),
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
                gateway::select(&nodes, spec, role, self.directory.as_ref(), false, None)
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
            None,
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
        self.register_two_hop_inner(entry, exit, false).await
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
        self.register_two_hop_inner(entry, exit, true).await
    }

    async fn register_two_hop_inner(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
        entry_quic: bool,
    ) -> Result<Registration, SessionError> {
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
            None,
        )?;
        // Exclude the entry gateway so a two-hop tunnel never uses one gateway twice.
        let exit_gw = gateway::select(
            &nodes,
            exit,
            WgRole::Exit,
            self.directory.as_ref(),
            false,
            Some(&entry_gw.identity),
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
        let mut entry_client = LpRegistrationClient::<TcpStream>::new_with_default_config(
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
                let exit_wg = x25519::KeyPair::new(&mut rand::thread_rng());
                let exit_cfg = nested
                    .handshake_and_register_dvpn::<TcpStream, _>(
                        &mut entry_client,
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
                let entry_cfg = entry_client
                    .register_dvpn(
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
        let mut client = LpRegistrationClient::<TcpStream>::new_with_default_config(
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
        let cfg = client
            .register_dvpn(
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

    /// Shut down the session's bandwidth controller (if it owns one), awaiting its cleanup so the
    /// credential store is closed cleanly. Stored tickets are retained.
    pub async fn shutdown(mut self) {
        self.cancel.cancel();
        if let Some(owned) = self.owned.take() {
            let _ = owned.task.await;
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
