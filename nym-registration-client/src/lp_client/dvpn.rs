// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! dVPN registration over an LP channel.
//!
//! [`nym_lp_gateway_client`] owns the channel; this is what a client *says* over it. A registration
//! client borrows a channel for as long as it takes to register over it, which keeps the two apart:
//! a data-plane client wants [`LpGatewayClient`] with none of this attached.

use crate::lp_client::bandwidth_claim::produce_bandwidth_claim;
use crate::lp_client::helpers::{LpFrameDeliverExt, LpFrameSendExt};
use nym_bandwidth_controller::BandwidthTicketProvider;
use nym_credentials_interface::TicketType;
use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::LpTransportChannel;
use nym_lp_gateway_client::{
    LpClientError, LpGatewayClient, NestedLpSession, Result, exponential_backoff_with_jitter,
    extract_forwarded_response, prepare_send_packet,
};
use nym_registration_common::dvpn::LpDvpnRegistrationResponseMessageContent;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, WireguardConfiguration,
    WireguardRegistrationData,
};
use nym_wireguard_types::PeerPublicKey;
use rand010::{CryptoRng, Rng};
use std::net::SocketAddr;
use time::Duration as TimeDuration;
use tokio::net::TcpStream;
use tracing::{debug, warn};

/// Acquire a credential and turn it into a finalisation request.
///
/// Both the direct and the nested flow reach this the same way - the gateway answers the initial
/// request with `RequiresCredential` - and differ only in how they put the request on the wire.
async fn build_finalisation_request(
    gateway_identity: ed25519::PublicKey,
    bandwidth_provider: &dyn BandwidthTicketProvider,
    spend_time_skew: Option<TimeDuration>,
    ticket_type: TicketType,
) -> Result<LpRegistrationRequest> {
    tracing::debug!("Acquiring bandwidth credential for registration");

    let credential = produce_bandwidth_claim(
        bandwidth_provider,
        gateway_identity,
        spend_time_skew,
        ticket_type,
    )
    .await?;

    tracing::trace!("Built dVPN registration finalisation request");

    Ok(LpRegistrationRequest::new_finalise_dvpn(credential))
}

/// Build an initial dVPN request, returning it alongside the PSK it carries.
fn build_initial_request<R>(
    rng: &mut R,
    wg_keypair: &x25519::KeyPair,
) -> (LpRegistrationRequest, [u8; 32])
where
    R: Rng + CryptoRng,
{
    let wg_public_key = PeerPublicKey::from(*wg_keypair.public_key());
    let mut psk = [0u8; 32];
    rng.fill_bytes(&mut psk);

    let request = LpRegistrationRequest::new_initial_dvpn(wg_public_key, psk);
    tracing::trace!("Built dVPN registration request: {request:?}");

    (request, psk)
}

/// The gateway's answer to a dVPN request, once the shape of it has been checked.
///
/// `RequiresCredential` is not an error - it is the expected first answer from a gateway this
/// client has not paid yet - so it is kept distinct from the failure case.
enum DvpnAnswer {
    Completed(Box<WireguardRegistrationData>),
    RequiresCredential,
}

fn interpret_response(response: LpRegistrationResponse) -> Result<DvpnAnswer> {
    let Some(dvpn_response) = response.into_dvpn_response() else {
        return Err(LpClientError::unexpected_response(
            "did not get a dvpn registration response after sending initial request",
        ));
    };

    match dvpn_response.content {
        LpDvpnRegistrationResponseMessageContent::RegistrationFailure(res) => {
            let reason = res.error;
            warn!("Gateway rejected registration: {reason}");
            Err(LpClientError::RegistrationRejected { reason })
        }
        LpDvpnRegistrationResponseMessageContent::CompletedRegistration(res) => {
            Ok(DvpnAnswer::Completed(Box::new(res.config)))
        }
        LpDvpnRegistrationResponseMessageContent::RequiresCredential(_) => {
            Ok(DvpnAnswer::RequiresCredential)
        }
    }
}

fn wireguard_configuration(
    gateway: SocketAddr,
    data: WireguardRegistrationData,
    psk: [u8; 32],
) -> WireguardConfiguration {
    WireguardConfiguration {
        public_key: data.public_key,
        psk: Some(psk.into()),
        endpoint: SocketAddr::new(gateway.ip(), data.port),
        private_ipv4: data.private_ipv4,
        private_ipv6: data.private_ipv6,
    }
}

/// Registers for dVPN with the gateway at the other end of the channel it borrows.
///
/// Holds the channel only for as long as the registration takes, so the caller keeps it afterwards.
/// That is what the entry leg of a two-hop tunnel needs: it carries the exit registration first,
/// then registers itself.
pub struct LpDvpnRegistrationClient<'a, S = TcpStream> {
    channel: &'a mut LpGatewayClient<S>,
}

impl<'a, S> LpDvpnRegistrationClient<'a, S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    pub fn new(channel: &'a mut LpGatewayClient<S>) -> Self {
        Self { channel }
    }

    /// Register for dVPN over the established session.
    ///
    /// Acquires a bandwidth credential only if the gateway asks for one, which it does on the
    /// first interaction with a given client. Does **not** retry on network failure - use
    /// [`Self::handshake_and_register_with_retry`] for that.
    pub async fn register<R>(
        &mut self,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_provider: &dyn BandwidthTicketProvider,
        spend_time_skew: Option<TimeDuration>,
        ticket_type: TicketType,
    ) -> Result<WireguardConfiguration>
    where
        R: Rng + CryptoRng,
    {
        let (request, psk) = build_initial_request(rng, wg_keypair);

        let final_response = match self.exchange(request).await? {
            DvpnAnswer::Completed(config) => *config,
            DvpnAnswer::RequiresCredential => {
                // we're registering for the first time with this gateway - attach a credential
                let finalisation = build_finalisation_request(
                    *gateway_identity,
                    bandwidth_provider,
                    spend_time_skew,
                    ticket_type,
                )
                .await?;

                match self.exchange(finalisation).await? {
                    DvpnAnswer::Completed(config) => *config,
                    DvpnAnswer::RequiresCredential => {
                        return Err(LpClientError::unexpected_response(
                            "received request for additional dvpn data after sending credential!",
                        ));
                    }
                }
            }
        };

        Ok(wireguard_configuration(
            self.channel.gateway_address(),
            final_response,
            psk,
        ))
    }

    /// Handshake and register, retrying the handshake on network failure.
    ///
    /// The credential is acquired once. The gateway's registration is idempotent on the WireGuard
    /// key, so a retry after a lost response returns the cached result rather than spending a
    /// second ticket - which is what makes this safe on an unreliable network.
    ///
    /// Unlike [`Self::register`], this drives the handshake itself; do not call
    /// `perform_handshake` beforehand.
    #[allow(clippy::too_many_arguments)]
    pub async fn handshake_and_register_with_retry<R>(
        &mut self,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_provider: &dyn BandwidthTicketProvider,
        spend_time_skew: Option<TimeDuration>,
        ticket_type: TicketType,
        max_retries: u32,
    ) -> Result<WireguardConfiguration>
    where
        R: Rng + CryptoRng,
    {
        tracing::debug!("Starting resilient registration (max_retries={max_retries})");

        let mut last_error = None;
        for attempt in 0..=max_retries {
            let attempt_display = attempt + 1;
            debug!("registration attempt {attempt_display}");

            if attempt > 0 {
                // Clear any stale state before re-handshaking
                self.channel.reset();
                exponential_backoff_with_jitter(attempt).await
            }

            match self.channel.perform_handshake().await {
                Ok(_) => break,
                Err(e) => {
                    warn!("Handshake failed on attempt {attempt_display}: {e}");
                    last_error = Some(e);
                }
            }
        }

        if !self.channel.is_handshake_complete() {
            return Err(last_error.unwrap_or(LpClientError::RegistrationFailure {
                message: "Registration failed after all retries".to_string(),
            }));
        }

        self.register(
            rng,
            wg_keypair,
            gateway_identity,
            bandwidth_provider,
            spend_time_skew,
            ticket_type,
        )
        .await
        .inspect_err(|e| warn!("Registration failed: {e}"))
    }

    /// One registration request out, one response back, on the direct session.
    async fn exchange(&mut self, request: LpRegistrationRequest) -> Result<DvpnAnswer> {
        let lp_data = request.to_lp_frame()?;
        let timeout = self.channel.config.registration_timeout;

        let request_packet = prepare_send_packet(lp_data, self.channel.transport_session_mut()?)?;

        let response_packet = self
            .channel
            .send_and_receive_data_packet_with_timeout(&request_packet, timeout)
            .await?;

        // re-borrow: the send held the session mutably
        let received_data =
            extract_forwarded_response(response_packet, self.channel.transport_session_mut()?)?;

        interpret_response(LpRegistrationResponse::from_lp_frame(received_data)?)
    }
}

/// Registers for dVPN with an exit gateway, over a nested session forwarded through an entry one.
///
/// Borrows both halves because neither is enough on its own: the nested session has no connection
/// of its own, so its packets ride the carrier channel. See [`NestedLpSession`].
pub struct NestedLpDvpnRegistrationClient<'a, S = TcpStream> {
    session: &'a mut NestedLpSession,
    carrier: &'a mut LpGatewayClient<S>,
}

impl<'a, S> NestedLpDvpnRegistrationClient<'a, S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    /// `carrier` is the channel to the entry gateway that forwards for `session`.
    pub fn new(session: &'a mut NestedLpSession, carrier: &'a mut LpGatewayClient<S>) -> Self {
        Self { session, carrier }
    }

    /// Register for dVPN over the already-handshaked nested session.
    pub async fn register<R>(
        &mut self,
        rng: &mut R,
        wg_keypair: &x25519::KeyPair,
        gateway_identity: &ed25519::PublicKey,
        bandwidth_provider: &dyn BandwidthTicketProvider,
        spend_time_skew: Option<TimeDuration>,
        ticket_type: TicketType,
    ) -> Result<WireguardConfiguration>
    where
        R: Rng + CryptoRng,
    {
        tracing::debug!("Building registration request for exit gateway");
        let (request, psk) = build_initial_request(rng, wg_keypair);

        let final_response = match self.exchange(request).await? {
            DvpnAnswer::Completed(config) => *config,
            DvpnAnswer::RequiresCredential => {
                let finalisation = build_finalisation_request(
                    *gateway_identity,
                    bandwidth_provider,
                    spend_time_skew,
                    ticket_type,
                )
                .await?;

                match self.exchange(finalisation).await? {
                    DvpnAnswer::Completed(config) => *config,
                    DvpnAnswer::RequiresCredential => {
                        return Err(LpClientError::unexpected_response(
                            "received request for additional dvpn data after sending credential!",
                        ));
                    }
                }
            }
        };

        Ok(wireguard_configuration(
            self.session.exit_address(),
            final_response,
            psk,
        ))
    }

    /// One registration request out, one response back, forwarded through the carrier.
    async fn exchange(&mut self, request: LpRegistrationRequest) -> Result<DvpnAnswer> {
        // encrypted on the *inner* session, then handed to the carrier to carry
        let forward_packet = self
            .session
            .prepare_transport_packet(request.to_lp_frame()?)?;
        let exit_address = self.session.exit_address();

        let mut nested_connection = self.carrier.as_nested_connection(exit_address);
        nested_connection
            .send_length_prefixed_transport_packet(&forward_packet)
            .await?;
        let response = nested_connection
            .receive_length_prefixed_transport_packet()
            .await?;

        let response_data = self.session.extract_forwarded_response(response)?;

        interpret_response(LpRegistrationResponse::from_lp_frame(response_data)?)
    }
}
