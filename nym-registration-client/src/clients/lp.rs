// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::builder::RegistrationClientBuilder;
use crate::config::RegistrationClientConfig;
use crate::config::RegistrationMode;
use crate::error::RegistrationClientError;
use crate::lp_client::helpers::to_lp_remote_peer;
use crate::lp_client::{LpRegistrationClient, NestedLpSession};
use crate::types::{RegistrationResult, WireguardRegistrationResult};

use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::ed25519;

use nym_lp::peer::DHKeyPair;
use rand010::rngs::SysRng;
use rand010::{CryptoRng, Rng, SeedableRng};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;
use tracing::warn;

pub struct LpBasedRegistrationClient {
    pub(crate) config: RegistrationClientConfig,
    pub(crate) bandwidth_provider: Box<dyn BandwidthTicketProvider>,
    pub(crate) cancel_token: CancellationToken,
}

impl LpBasedRegistrationClient {
    // create dedicated method taking RNG instance for tests
    async fn register_wg_with_rng<R>(
        self,
        rng: &mut R,
    ) -> Result<RegistrationResult, RegistrationClientError>
    where
        R: Rng + CryptoRng,
    {
        // Extract and validate LP data
        let entry_lp_data = self.config.entry.node.lp_data.ok_or(
            RegistrationClientError::LpRegistrationNotPossible {
                node_id: self.config.entry.node.identity.to_base58_string(),
            },
        )?;

        let exit_lp_data = self.config.exit.node.lp_data.ok_or(
            RegistrationClientError::LpRegistrationNotPossible {
                node_id: self.config.exit.node.identity.to_base58_string(),
            },
        )?;

        let entry_lp_protocol = entry_lp_data.lp_protocol_version;
        let exit_lp_protocol = exit_lp_data.lp_protocol_version;

        let entry_ciphersuite = entry_lp_data.ciphersuite;
        let exit_ciphersuite = exit_lp_data.ciphersuite;

        let entry_address = entry_lp_data.address;
        let exit_address = exit_lp_data.address;

        tracing::debug!("Entry gateway LP address: {entry_address}");
        tracing::debug!("Exit gateway LP address: {exit_address}");

        // Generate fresh x25519 keypairs for LP registration
        let entry_lp_keypair = Arc::new(DHKeyPair::new(&mut rand010::rng()));
        let exit_lp_keypair = Arc::new(DHKeyPair::new(&mut rand010::rng()));

        let entry_peer = to_lp_remote_peer(entry_lp_data);
        let exit_peer = to_lp_remote_peer(exit_lp_data);

        // STEP 1: Establish outer session with entry gateway
        // This creates the LP session that will be used to forward packets to exit.
        // Uses packet-per-connection model: each handshake packet on new TCP connection.
        tracing::info!("Establishing outer session with entry gateway");
        let mut entry_client = LpRegistrationClient::new(
            entry_lp_keypair.clone(),
            entry_peer,
            entry_address,
            entry_ciphersuite,
            entry_lp_protocol,
            self.config.lp_registration_config,
        );

        // Perform handshake with entry gateway (outer session now established)
        entry_client.perform_handshake().await.map_err(|source| {
            RegistrationClientError::EntryGatewayRegisterLp {
                gateway_id: self.config.entry.node.identity.to_base58_string(),
                lp_address: entry_address,
                source: Box::new(source),
            }
        })?;

        tracing::info!("Outer session with entry gateway established");

        // STEP 2: Use nested session to register with exit gateway via forwarding
        // This hides the client's IP address from the exit gateway
        tracing::info!("Registering with exit gateway via entry forwarding");
        let mut nested_session = NestedLpSession::new(
            exit_address,
            exit_lp_keypair.clone(),
            exit_peer,
            exit_ciphersuite,
            exit_lp_protocol,
        );

        // Perform handshake and registration with exit gateway (all via entry forwarding)
        let exit_gateway_data = nested_session
            .handshake_and_register_dvpn::<TcpStream, _>(
                &mut entry_client,
                rng,
                &self.config.exit.keys,
                &self.config.exit.node.identity,
                &*self.bandwidth_provider,
                self.config.spend_time_skew,
                TicketType::V1WireguardExit,
            )
            .await
            .map_err(|source| RegistrationClientError::ExitGatewayRegisterLp {
                gateway_id: self.config.exit.node.identity.to_base58_string(),
                lp_address: exit_address,
                source: Box::new(source),
            })?;

        tracing::info!("Exit gateway registration completed via forwarding");

        // STEP 3: Register with entry gateway (packet-per-connection)
        tracing::info!("Registering with entry gateway");
        let entry_gateway_data = entry_client
            .register_dvpn(
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

    async fn register_inner(mut self) -> Result<RegistrationResult, RegistrationClientError> {
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

    pub(crate) async fn register(mut self) -> Result<RegistrationResult, RegistrationClientError> {
        let timeout = self.config.lp_registration_config.exchange_timeout;
        tokio::time::timeout(timeout, self.register_inner())
            .await
            .unwrap_or_else(|timeout| {
                warn!("timed out while attempting to complete LP registration");
                Err(RegistrationClientError::Timeout(timeout))
            })
    }
}
