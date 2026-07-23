// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The provisioning session: mnemonic → issued ticketbooks → registered gateways.
//!
//! The session runs a [`BandwidthController`] event loop (the single writer to the credential
//! store) and performs all ticket spending through its [`BandwidthControllerRequestSender`], which
//! implements [`BandwidthTicketProvider`]. Chain-side restock (depositing NYM for new ticketbooks)
//! is off by default and opted into via [`SessionConfig::automatic_topups`]; gateway-side top-up of
//! a live tunnel spends already-stored tickets and is driven by the datapath, not here.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

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
use rand09::SeedableRng;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use url::Url;
use zeroize::Zeroizing;

use nym_api_requests::models::described::v2::NymNodeDescriptionV2;

use crate::dvpn::{DvpnDirectory, QuicBridge};
use crate::error::SessionError;
use crate::gateway::{self, GatewayInfo, GatewaySpec, SelectedGateway, WgRole};

/// Number of tickets to reserve when checking for / spending a stored ticketbook.
const TICKETS_TO_SPEND: u32 = 1;
/// Timeout for nym-api requests.
const API_TIMEOUT: Duration = Duration::from_secs(30);

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

/// Opt-in policy for automatic chain-side ticketbook restock. Maps onto the bandwidth controller's
/// restock thresholds. Only in effect when set via [`SessionConfig::automatic_topups`].
#[derive(Clone, Copy, Debug)]
pub struct RestockPolicy {
    /// Restock a ticket type once its usable stock drops to/below this many tickets.
    pub restock_below_tickets: u64,
    /// Minimum usable tickets for a type to be considered "ready to connect".
    pub readiness_min_tickets: u64,
    /// How often to proactively check stock.
    pub check_interval: Duration,
    /// Treat a ticketbook expiring within this window as needing replacement.
    pub soon_expiry: Duration,
}

impl Default for RestockPolicy {
    fn default() -> Self {
        // Mirror `BandwidthControllerConfig::default()`.
        Self {
            restock_below_tickets: 20,
            readiness_min_tickets: 5,
            check_interval: Duration::from_secs(3 * 3600),
            soon_expiry: Duration::from_secs(12 * 3600),
        }
    }
}

impl From<RestockPolicy> for BandwidthControllerConfig {
    fn from(p: RestockPolicy) -> Self {
        BandwidthControllerConfig {
            topup_interval: p.check_interval,
            soon_expiry_threshold: p.soon_expiry,
            nb_ticket_restock: p.restock_below_tickets,
            min_nb_ticket_needed: p.readiness_min_tickets,
            // The session scopes this to its WireGuard types when installing the config; the
            // default is only a placeholder.
            ..Default::default()
        }
    }
}

/// Configuration for creating a [`Session`].
pub struct SessionConfig {
    /// Funded chain mnemonic used to deposit NYM and issue ticketbooks. Ignored when
    /// [`bandwidth_provider`](Self::bandwidth_provider) is set.
    pub mnemonic: bip39::Mnemonic,
    /// Network to operate against (contract addresses, endpoints, denoms).
    pub network: NymNetworkDetails,
    /// Persistent credential store path. `None` uses a file under `data_path`
    /// (a fully ephemeral in-memory store is not used so tickets survive a
    /// bring-down/bring-up cycle).
    pub credential_store_path: Option<PathBuf>,
    /// Directory for the fetcher's pending-request recovery database and other
    /// per-session data.
    pub data_path: PathBuf,
    /// Optional dVPN gateway-directory URL. When set, the session fetches it to
    /// enrich gateway monikers and to enable QUIC-bridge entry selection
    /// (`register_two_hop_quic`). Fetched best-effort — a failure is logged and
    /// treated as an empty directory.
    pub dvpn_directory_url: Option<String>,
    /// Opt-in automatic chain-side restock. `None` (default) provisions once and never deposits in
    /// the background; the tunnel still tops up from already-stored tickets. `Some(policy)` lets a
    /// long-lived session re-issue ticketbooks when stock runs low (this spends NYM).
    pub automatic_topups: Option<RestockPolicy>,
    /// Externally-managed bandwidth provider. When set, the session uses it for all ticket
    /// spending and does NOT spawn its own controller — for callers already running a controller
    /// over the same credential store (preserving the single-writer invariant). `mnemonic` and the
    /// credential store are then unused, and the caller is responsible for provisioning.
    pub bandwidth_provider: Option<Arc<dyn BandwidthTicketProvider>>,
}

impl SessionConfig {
    /// A config with the required fields and sensible defaults (no automatic topups, own controller).
    pub fn new(mnemonic: bip39::Mnemonic, network: NymNetworkDetails, data_path: PathBuf) -> Self {
        Self {
            mnemonic,
            network,
            credential_store_path: None,
            data_path,
            dvpn_directory_url: None,
            automatic_topups: None,
            bandwidth_provider: None,
        }
    }

    /// Opt into automatic chain-side restock with the given policy (this can spend NYM).
    #[must_use]
    pub fn with_automatic_topups(mut self, policy: RestockPolicy) -> Self {
        self.automatic_topups = Some(policy);
        self
    }

    /// Use an externally-managed bandwidth provider instead of spawning an own controller.
    #[must_use]
    pub fn with_bandwidth_provider(mut self, provider: Arc<dyn BandwidthTicketProvider>) -> Self {
        self.bandwidth_provider = Some(provider);
        self
    }

    /// Set the dVPN directory URL.
    #[must_use]
    pub fn with_dvpn_directory_url(mut self, url: impl Into<String>) -> Self {
        self.dvpn_directory_url = Some(url.into());
        self
    }

    /// Set the credential store path.
    #[must_use]
    pub fn with_credential_store_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.credential_store_path = Some(path.into());
        self
    }
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
        } = config;

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

        // Credential fetcher: deposits NYM and aggregates issued wallets.
        let fetcher_db = data_path.join("fetcher-requests.db");
        let fetcher = NyxdCredentialFetcher::new(nyxd, &fetcher_db, client_id)
            .await
            .map_err(|e| SessionError::Issuance(e.to_string()))?;

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
        let Some(owned) = &self.owned else {
            // external provider: the caller is responsible for provisioning
            return Ok(());
        };
        let types = needed_ticket_types(two_hop);
        // Explicit restock request (works regardless of the auto-restock setting) scoped to exactly
        // the needed WireGuard types, then wait until they are usable. Race cancellation so a
        // caller that cancels mid-wait gets a prompt `Cancelled` instead of blocking; this is
        // funds-safe because the deposit itself runs in the controller task and any interrupted
        // issuance is recovered from the fetcher's pending-request store on a later fetch.
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = async {
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
            } => res,
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
        self.api
            .get_all_described_nodes_v2()
            .await
            .map_err(|e| SessionError::Api(e.to_string()))
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
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = self.register_single_inner(gateway) => res,
        }
    }

    async fn register_single_inner(
        &self,
        gateway: &GatewaySpec,
    ) -> Result<Registration, SessionError> {
        self.ensure_ticketbooks(false).await?;
        let nodes = self.fetch_topology().await?;
        let selected = gateway::select(
            &nodes,
            gateway,
            WgRole::Entry,
            self.directory.as_ref(),
            false,
            None,
        )?;
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
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = self.register_two_hop_inner(entry, exit, false) => res,
        }
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
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = self.register_two_hop_inner(entry, exit, true) => res,
        }
    }

    async fn register_two_hop_inner(
        &self,
        entry: &GatewaySpec,
        exit: &GatewaySpec,
        entry_quic: bool,
    ) -> Result<Registration, SessionError> {
        self.ensure_ticketbooks(true).await?;

        // Fetch the topology once and evaluate both selections against it.
        let nodes = self.fetch_topology().await?;
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

        let entry_lp = lp_info(&entry_gw)?;
        let exit_lp = lp_info(&exit_gw)?;

        // Outer session with the entry gateway.
        let entry_keypair = Arc::new(DHKeyPair::new(&mut rand09::rng()));
        let entry_peer =
            LpRemotePeer::new(entry_lp.x25519).with_key_digests(entry_lp.expected_kem_key_hashes);
        let mut entry_client = LpRegistrationClient::<TcpStream>::new_with_default_config(
            entry_keypair,
            entry_peer,
            entry_lp.address,
            entry_lp.ciphersuite,
            entry_lp.lp_protocol_version,
        );
        entry_client
            .perform_handshake()
            .await
            .map_err(|source| SessionError::Registration {
                address: entry_lp.address,
                source,
            })?;

        let mut rng = rand09::rngs::StdRng::from_os_rng();

        // Exit registration via entry forwarding.
        let exit_keypair = Arc::new(DHKeyPair::new(&mut rand09::rng()));
        let exit_peer =
            LpRemotePeer::new(exit_lp.x25519).with_key_digests(exit_lp.expected_kem_key_hashes);
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
                TicketType::V1WireguardExit,
            )
            .await
            .map_err(|source| SessionError::Registration {
                address: exit_lp.address,
                source,
            })?;

        // Entry registration.
        let entry_wg = x25519::KeyPair::new(&mut rand::thread_rng());
        let entry_cfg = entry_client
            .register_dvpn(
                &mut rng,
                &entry_wg,
                &entry_gw.identity,
                self.provider.as_ref(),
                TicketType::V1WireguardEntry,
            )
            .await
            .map_err(|source| SessionError::Registration {
                address: entry_lp.address,
                source,
            })?;

        // The entry hop carries QUIC bridge params only when QUIC was required
        // (selection guarantees `entry_gw.quic` is `Some` in that case).
        let entry_bridge = if entry_quic {
            entry_gw.quic.clone()
        } else {
            None
        };

        Ok(Registration {
            entry: HopConfig {
                wg_config: entry_cfg,
                client_private_key: x25519::PrivateKey::from_secret(
                    entry_wg.private_key().to_bytes(),
                ),
                gateway_identity: entry_gw.identity,
                gateway: entry_gw.info(),
                bridge: entry_bridge,
            },
            exit: Some(HopConfig {
                wg_config: exit_cfg,
                client_private_key: x25519::PrivateKey::from_secret(
                    exit_wg.private_key().to_bytes(),
                ),
                gateway_identity: exit_gw.identity,
                gateway: exit_gw.info(),
                bridge: None,
            }),
        })
    }

    /// Register a single hop against an already-selected gateway.
    async fn register_hop(
        &self,
        selected: &SelectedGateway,
        ticket_type: TicketType,
    ) -> Result<HopConfig, SessionError> {
        let lp = lp_info(selected)?;
        let keypair = Arc::new(DHKeyPair::new(&mut rand09::rng()));
        let peer = LpRemotePeer::new(lp.x25519).with_key_digests(lp.expected_kem_key_hashes);
        let mut client = LpRegistrationClient::<TcpStream>::new_with_default_config(
            keypair,
            peer,
            lp.address,
            lp.ciphersuite,
            lp.lp_protocol_version,
        );

        client
            .perform_handshake()
            .await
            .map_err(|source| SessionError::Registration {
                address: lp.address,
                source,
            })?;

        let mut rng = rand09::rngs::StdRng::from_os_rng();
        let wg = x25519::KeyPair::new(&mut rand::thread_rng());
        let cfg = client
            .register_dvpn(
                &mut rng,
                &wg,
                &selected.identity,
                self.provider.as_ref(),
                ticket_type,
            )
            .await
            .map_err(|source| SessionError::Registration {
                address: lp.address,
                source,
            })?;

        Ok(HopConfig {
            wg_config: cfg,
            client_private_key: x25519::PrivateKey::from_secret(wg.private_key().to_bytes()),
            gateway_identity: selected.identity,
            gateway: selected.info(),
            bridge: None,
        })
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
