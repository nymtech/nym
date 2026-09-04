// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! dVPN registration over the Lewes Protocol.
//!
//! The channel itself - connecting, handshaking, carrying frames, and telescoping through an entry
//! gateway - belongs to [`nym_lp_gateway_client`]. What lives here is what a client *says* over
//! that channel to register for dVPN: a registration client borrows a channel, owns the session
//! established over it, and exchanges requests.
//!
//! - [`LpDvpnRegistrationClient`] registers with the gateway a channel is connected to.
//! - [`NestedLpDvpnRegistrationClient`] registers with an exit gateway through an entry one.
//!
//! Both need a bandwidth credential, which is why they sit alongside the machinery that acquires
//! one. Mixnet registration needs none, and lives in
//! [`nym_lp_gateway_client::registration`].
//!
//! # Usage
//!
//! ```ignore
//! use nym_lp_gateway_client::LpGatewayClient;
//! use nym_registration_client::LpDvpnRegistrationClient;
//!
//! let mut client = LpGatewayClient::<TcpStream>::new(config);
//! let session = client
//!     .handshake(gateway, local_peer, remote_peer, lp_version, HandshakeMode::OneWayEntry)
//!     .await?;
//!
//! // the registration client borrows the channel, so it is still yours afterwards
//! let gateway_data = LpDvpnRegistrationClient::new(&mut client, gateway, session)
//!     .register(&mut rng, ...)
//!     .await?;
//! ```

use crate::config::RegistrationClientConfig;
use crate::config::RegistrationMode;
use crate::error::RegistrationClientError;
use crate::types::RegistrationResult;
use helpers::to_lp_remote_peer;

use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;

use nym_lp::LpTransportSession;
use nym_lp::peer::{DHKeyPair, LpLocalPeer};
use nym_lp::psq::initiator::HandshakeMode;
use nym_lp_gateway_client::{LpClientError, LpGatewayClient, NestedLpSession};
use nym_registration_common::NymNodeLPInformation;
use rand010::rngs::SysRng;
use rand010::{CryptoRng, Rng, SeedableRng};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::warn;

mod bandwidth_claim;
mod dvpn;
pub(crate) mod helpers;

pub use dvpn::{LpDvpnRegistrationClient, NestedLpDvpnRegistrationClient};

pub struct LpBasedRegistrationClient {
    pub(crate) config: RegistrationClientConfig,
    pub(crate) bandwidth_provider: Box<dyn BandwidthTicketProvider>,
    pub(crate) cancel_token: CancellationToken,
}

/// Everything [`LpBasedRegistrationClient::connect_to_entry`] establishes, kept together because
/// none of it is useful without the rest.
struct EntryConnection<S> {
    channel: LpGatewayClient<S>,
    gateway: SocketAddr,
    session: LpTransportSession,

    /// Generated for the handshake, but outlives the registration: persisting it is what would let
    /// the session be resumed.
    keypair: Arc<DHKeyPair>,
}

impl LpBasedRegistrationClient {
    /// The entry gateway's LP details, or the error saying it has none.
    fn entry_lp_data(&self) -> Result<NymNodeLPInformation, RegistrationClientError> {
        self.config.entry.node.lp_data.clone().ok_or(
            RegistrationClientError::LpRegistrationNotPossible {
                node_id: self.config.entry.node.identity.to_base58_string(),
            },
        )
    }

    /// How a failure talking to the entry gateway is reported.
    fn entry_failure(
        &self,
        lp_address: SocketAddr,
        source: LpClientError,
    ) -> RegistrationClientError {
        RegistrationClientError::EntryGatewayRegisterLp {
            gateway_id: self.config.entry.node.identity.to_base58_string(),
            lp_address,
            source: Box::new(source),
        }
    }

    /// Open a channel to the entry gateway and complete its handshake.
    ///
    /// Returns the keypair alongside the session, since it is generated here but outlives the
    /// registration - persisting it is what would let the session be resumed.
    async fn connect_to_entry(
        &self,
    ) -> Result<EntryConnection<TcpStream>, RegistrationClientError> {
        let lp_data = self.entry_lp_data()?;
        let keypair = Arc::new(DHKeyPair::new(&mut rand010::rng()));

        tracing::debug!("Entry gateway LP address: {}", lp_data.address);

        let mut channel = LpGatewayClient::<TcpStream>::new(self.config.lp_registration_config);

        let session = channel
            .handshake(
                lp_data.address,
                LpLocalPeer::new(lp_data.ciphersuite, keypair.clone()),
                to_lp_remote_peer(lp_data.clone()),
                lp_data.lp_protocol_version,
                HandshakeMode::OneWayEntry,
            )
            .await
            .map_err(|source| self.entry_failure(lp_data.address, source))?;

        Ok(EntryConnection {
            channel,
            gateway: lp_data.address,
            session,
            keypair,
        })
    }

    // create dedicated method taking RNG instance for tests
    async fn register_wg_with_rng<R>(
        self,
        rng: &mut R,
    ) -> Result<RegistrationResult, RegistrationClientError>
    where
        R: Rng + CryptoRng,
    {
        let entry_address = self.entry_lp_data()?.address;

        let exit_lp_data = self.config.exit.node.lp_data.clone().ok_or(
            RegistrationClientError::LpRegistrationNotPossible {
                node_id: self.config.exit.node.identity.to_base58_string(),
            },
        )?;

        let exit_lp_protocol = exit_lp_data.lp_protocol_version;
        let exit_ciphersuite = exit_lp_data.ciphersuite;
        let exit_address = exit_lp_data.address;

        tracing::debug!("Exit gateway LP address: {exit_address}");

        let exit_lp_keypair = Arc::new(DHKeyPair::new(&mut rand010::rng()));
        let exit_peer = to_lp_remote_peer(exit_lp_data);

        // STEP 1: Establish outer session with entry gateway
        // This creates the LP session that will be used to forward packets to exit.
        tracing::info!("Establishing outer session with entry gateway");
        let EntryConnection {
            channel: mut entry_client,
            gateway: entry_gateway,
            session: mut entry_session,
            keypair: entry_lp_keypair,
        } = self.connect_to_entry().await?;

        tracing::info!("Outer session with entry gateway established");

        // STEP 2: Use nested session to register with exit gateway via forwarding
        // This hides the client's IP address from the exit gateway
        tracing::info!("Registering with exit gateway via entry forwarding");
        let nested = NestedLpSession::new(
            exit_address,
            exit_lp_keypair.clone(),
            exit_peer,
            exit_ciphersuite,
            exit_lp_protocol,
        );

        let exit_failure = |source| RegistrationClientError::ExitGatewayRegisterLp {
            gateway_id: self.config.exit.node.identity.to_base58_string(),
            lp_address: exit_address,
            source: Box::new(source),
        };

        let exit_session = nested
            .perform_handshake(&mut entry_client, entry_gateway, &mut entry_session)
            .await
            .map_err(exit_failure)?;

        // Register with the exit gateway over that session (still via entry forwarding)
        let exit_gateway_data = NestedLpDvpnRegistrationClient::new(
            &nested,
            exit_session,
            &mut entry_client,
            entry_gateway,
            &mut entry_session,
        )
        .register(
            rng,
            &self.config.exit.keys,
            &self.config.exit.node.identity,
            &*self.bandwidth_provider,
            self.config.spend_time_skew,
            TicketType::V1WireguardExit,
        )
        .await
        .map_err(exit_failure)?;

        tracing::info!("Exit gateway registration completed via forwarding");

        // STEP 3: Register with entry gateway (packet-per-connection)
        tracing::info!("Registering with entry gateway");
        let entry_gateway_data =
            LpDvpnRegistrationClient::new(&mut entry_client, entry_gateway, entry_session)
                .register(
                    rng,
                    &self.config.entry.keys,
                    &self.config.entry.node.identity,
                    &*self.bandwidth_provider,
                    self.config.spend_time_skew,
                    TicketType::V1WireguardEntry,
                )
                .await
                .map_err(|source| RegistrationClientError::EntryGatewayRegisterLp {
                    gateway_id: self.config.entry.node.identity.to_base58_string(),
                    lp_address: entry_address,
                    source: Box::new(source),
                })?;

        tracing::info!("Entry gateway registration successful");

        tracing::info!("LP registration successful for both gateways");

        // LP is registration-only (packet-per-connection model).
        // All data flows through WireGuard after this point.
        // Each LP packet used its own TCP connection which was closed after the exchange.
        // Exit registration was completed via forwarding through entry gateway.
        Ok(RegistrationResult::wireguard_lp(
            entry_gateway_data,
            exit_gateway_data,
            entry_lp_keypair,
            exit_lp_keypair,
        ))
    }

    async fn register_wg(self) -> Result<RegistrationResult, RegistrationClientError> {
        let mut rng = rand010::rngs::StdRng::try_from_rng(&mut SysRng)?;

        self.register_wg_with_rng(&mut rng).await
    }

    async fn register_inner(self) -> Result<RegistrationResult, RegistrationClientError> {
        match &self.config.mode {
            RegistrationMode::Mixnet => {
                // mixnet registration is not supported for LP
                Err(RegistrationClientError::UnsupportedMode)
            }
            RegistrationMode::Wireguard => {
                let lp_registration_result = self
                    .cancel_token
                    .clone()
                    .run_until_cancelled(self.register_wg())
                    .await;
                match lp_registration_result {
                    // Everything went fine
                    Some(Ok(res)) => Ok(res),

                    Some(Err(e)) => {
                        tracing::error!("LP registration failed : {e}");
                        Err(e)
                    }

                    // Cancelled registration
                    None => Err(RegistrationClientError::Cancelled),
                }
            }
        }
    }

    pub(crate) async fn register(self) -> Result<RegistrationResult, RegistrationClientError> {
        let timeout = self.config.lp_registration_config.exchange_timeout;
        tokio::time::timeout(timeout, self.register_inner())
            .await
            .unwrap_or_else(|timeout| {
                warn!("timed out while attempting to complete LP registration");
                Err(RegistrationClientError::Timeout(timeout))
            })
    }
}
