// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::builder::RegistrationClientBuilder;
use crate::config::RegistrationClientConfig;
use crate::config::RegistrationMode;
use crate::error::RegistrationClientError;
use crate::lp_client::helpers::to_lp_remote_peer;
use crate::lp_client::{LpRegistrationClient, NestedLpSession};
use crate::types::{LpRegistrationResult, RegistrationResult};

use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_crypto::aes::cipher::crypto_common::rand_core::{CryptoRng, RngCore};
use nym_crypto::asymmetric::ed25519;

use rand::rngs::OsRng;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

pub struct LpBasedRegistrationClient {
    pub(crate) config: RegistrationClientConfig,
    pub(crate) bandwidth_controller: Box<dyn BandwidthTicketProvider>,
    pub(crate) cancel_token: CancellationToken,
    // While we allow a fallback, we need to be able to build it
    pub(crate) fallback_client_builder: Option<RegistrationClientBuilder>,
}

impl LpBasedRegistrationClient {
    // create dedicated method taking RNG instance for tests
    async fn register_wg_with_rng<R>(
        self,
        rng: &mut R,
    ) -> Result<RegistrationResult, RegistrationClientError>
    where
        R: RngCore + CryptoRng,
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

        let entry_address = entry_lp_data.address;
        let exit_address = exit_lp_data.address;

        tracing::debug!("Entry gateway LP address: {entry_address}");
        tracing::debug!("Exit gateway LP address: {exit_address}");

        // Generate fresh Ed25519 keypairs for LP registration
        // These are ephemeral and used only for the LP handshake protocol
        let entry_lp_keypair = Arc::new(ed25519::KeyPair::new(&mut OsRng));
        let exit_lp_keypair = Arc::new(ed25519::KeyPair::new(&mut OsRng));

        let entry_peer = to_lp_remote_peer(self.config.entry.node.identity, entry_lp_data);
        let exit_peer = to_lp_remote_peer(self.config.exit.node.identity, exit_lp_data);

        // STEP 1: Establish outer session with entry gateway
        // This creates the LP session that will be used to forward packets to exit.
        // Uses packet-per-connection model: each handshake packet on new TCP connection.
        tracing::info!("Establishing outer session with entry gateway");
        let mut entry_client = LpRegistrationClient::new(
            entry_lp_keypair.clone(),
            entry_peer,
            entry_address,
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
        let mut nested_session =
            NestedLpSession::new(exit_address, exit_lp_keypair, exit_peer, exit_lp_protocol);

        // Perform handshake and registration with exit gateway (all via entry forwarding)
        let exit_gateway_data = nested_session
            .handshake_and_register_dvpn::<TcpStream, _>(
                &mut entry_client,
                rng,
                &self.config.exit.keys,
                &self.config.exit.node.identity,
                &*self.bandwidth_controller,
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
                &*self.bandwidth_controller,
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
        Ok(RegistrationResult::Lp(Box::new(LpRegistrationResult {
            entry_gateway_data,
            exit_gateway_data,
            bw_controller: self.bandwidth_controller,
        })))
    }

    async fn register_wg(self) -> Result<RegistrationResult, RegistrationClientError> {
        let mut rng = rand::rngs::OsRng;

        self.register_wg_with_rng(&mut rng).await
    }

    pub(crate) async fn register(mut self) -> Result<RegistrationResult, RegistrationClientError> {
        let fallback = self.fallback_client_builder.take();
        match &self.config.mode {
            RegistrationMode::Mixnet => {
                if let Some(fallback) = fallback {
                    register_with_fallback(fallback).await
                } else {
                    Err(RegistrationClientError::UnsupportedMode)
                }
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

                    // LP reg failed, try fallback if we have one
                    Some(Err(e)) => {
                        tracing::error!("LP registration failed : {e}");
                        if let Some(fallback) = fallback {
                            tracing::info!("Registering with fallback");
                            register_with_fallback(fallback).await
                        } else {
                            Err(e)
                        }
                    }

                    // Cancelled registration
                    None => Err(RegistrationClientError::Cancelled),
                }
            }
        }
    }
}

async fn register_with_fallback(
    client_builder: RegistrationClientBuilder,
) -> Result<RegistrationResult, RegistrationClientError> {
    // This is forcefully building a mixnet based client
    let fallback_client = client_builder.build_mixnet().await?;
    fallback_client.register().await
}
