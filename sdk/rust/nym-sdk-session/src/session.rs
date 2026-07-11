// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! The provisioning session: mnemonic → issued ticketbooks → registered gateways.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use nym_bandwidth_controller::{BandwidthController, BandwidthTicketProvider};
use nym_bandwidth_fetcher::NyxdCredentialFetcher;
use nym_credential_storage::persistent_storage::PersistentStorage;
use nym_credentials_interface::{BandwidthCredential, TicketType};
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::peer::{DHKeyPair, LpRemotePeer};
use nym_network_defaults::NymNetworkDetails;
use nym_registration_client::{LpRegistrationClient, NestedLpSession};
use nym_registration_common::WireguardConfiguration;
use nym_validator_client::nym_api::NymApiClientExt;
use nym_validator_client::DirectSigningHttpRpcNyxdClient;
use rand09::SeedableRng;
use time::OffsetDateTime;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use url::Url;
use zeroize::Zeroizing;

use crate::dvpn::{DvpnDirectory, QuicBridge};
use crate::error::SessionError;
use crate::gateway::{self, GatewayInfo, GatewaySpec, SelectedGateway, WgRole};

/// Number of tickets to reserve when checking for / spending a stored ticketbook.
const TICKETS_TO_SPEND: u32 = 1;
/// Timeout for nym-api requests.
const API_TIMEOUT: Duration = Duration::from_secs(30);

/// Configuration for creating a [`Session`].
pub struct SessionConfig {
    /// Funded chain mnemonic used to deposit NYM and issue ticketbooks.
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

/// Provisioning facade over the credential + registration machinery.
pub struct Session {
    api: nym_http_api_client::Client,
    controller: BandwidthController<PersistentStorage>,
    cancel: CancellationToken,
    /// dVPN gateway directory (empty if none configured or the fetch failed).
    directory: Option<DvpnDirectory>,
}

impl Session {
    /// Build a session: connect the signing chain client, open the credential
    /// store, wire the bandwidth controller + credential fetcher, and prepare
    /// the nym-api client.
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
        } = config;

        let nyxd_url = network
            .endpoints
            .first()
            .map(|e| e.nyxd_url.clone())
            .ok_or(SessionError::MissingEndpoint { which: "nyxd" })?;
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

        // Direct-signing chain client from the mnemonic.
        let nyxd = DirectSigningHttpRpcNyxdClient::connect_with_mnemonic_and_network_details(
            nyxd_url.as_str(),
            network.clone(),
            mnemonic.clone(),
        )
        .map_err(|e| SessionError::Chain(e.to_string()))?;
        let nyxd = Arc::new(nyxd);

        // Persistent credential store (survives bring-down / bring-up).
        let store_path = credential_store_path.unwrap_or_else(|| data_path.join("credentials.db"));
        if let Some(parent) = store_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let storage = nym_credential_storage::initialise_persistent_storage(&store_path).await;

        // Credential fetcher: deposits NYM and aggregates issued wallets.
        // The client id is derived from the mnemonic so ecash state is stable
        // across runs of the same account.
        let client_id = Zeroizing::new(mnemonic.to_entropy());
        let fetcher_db = data_path.join("fetcher-requests.db");
        let fetcher = NyxdCredentialFetcher::new(nyxd, &fetcher_db, client_id)
            .await
            .map_err(|e| SessionError::Issuance(e.to_string()))?;

        let controller = BandwidthController::new(storage).with_credential_fetcher(fetcher);
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

        Ok(Self {
            api,
            controller,
            cancel,
            directory,
        })
    }

    /// Ensure the required WireGuard ticketbooks are issued and stored, issuing
    /// (and depositing) only when no usable ticketbook is already stored.
    pub async fn ensure_ticketbooks(&self, two_hop: bool) -> Result<(), SessionError> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = self.ensure_inner(two_hop) => res,
        }
    }

    async fn ensure_inner(&self, two_hop: bool) -> Result<(), SessionError> {
        self.ensure_one(TicketType::V1WireguardEntry).await?;
        if two_hop {
            self.ensure_one(TicketType::V1WireguardExit).await?;
        }
        Ok(())
    }

    async fn ensure_one(&self, ticket_type: TicketType) -> Result<(), SessionError> {
        // Skip issuance if a usable ticketbook of this type is already stored.
        if let Ok(Some(_)) = self
            .controller
            .get_next_usable_ticketbook(ticket_type, TICKETS_TO_SPEND)
            .await
        {
            return Ok(());
        }
        self.controller
            .fetch_ticketbook(ticket_type)
            .await
            .map_err(|e| SessionError::Issuance(e.to_string()))
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
            .controller
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

    /// Select a WireGuard-capable gateway for the given role.
    pub async fn select_gateway(
        &self,
        spec: &GatewaySpec,
        role: WgRole,
    ) -> Result<SelectedGateway, SessionError> {
        tokio::select! {
            biased;
            _ = self.cancel.cancelled() => Err(SessionError::Cancelled),
            res = self.select_inner(spec, role, false, None) => res,
        }
    }

    async fn select_inner(
        &self,
        spec: &GatewaySpec,
        role: WgRole,
        require_quic: bool,
        exclude: Option<&ed25519::PublicKey>,
    ) -> Result<SelectedGateway, SessionError> {
        let nodes = self
            .api
            .get_all_described_nodes_v2()
            .await
            .map_err(|e| SessionError::Chain(e.to_string()))?;
        gateway::select(
            nodes,
            spec,
            role,
            self.directory.as_ref(),
            require_quic,
            exclude,
        )
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
        self.ensure_inner(false).await?;
        let selected = self
            .select_inner(gateway, WgRole::Entry, false, None)
            .await?;
        let hop = self
            .register_hop(&selected, TicketType::V1WireguardEntry, None)
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
        self.ensure_inner(true).await?;
        let entry_gw = self
            .select_inner(entry, WgRole::Entry, entry_quic, None)
            .await?;
        // Exclude the entry gateway so a two-hop tunnel never uses one gateway twice.
        let exit_gw = self
            .select_inner(exit, WgRole::Exit, false, Some(&entry_gw.identity))
            .await?;

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
                &self.controller,
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
                &self.controller,
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
        _forward_via: Option<()>,
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
                &self.controller,
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
