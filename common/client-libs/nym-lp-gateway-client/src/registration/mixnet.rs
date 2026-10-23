// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Mixnet registration over an LP channel.
//!
//! Two exchanges, and what they establish is the session's *name*. The client names its keys, the
//! gateway answers with a nonce, and the client signs that nonce with its ed25519 key; the gateway
//! then fingerprints both keys into a `ClientAddress` and binds it to the session's receiver index.
//! Until that happens it holds a session it can decrypt but cannot address, so nothing can be sent
//! back.
//!
//! The nonce is what makes the signature worth anything: without one, a captured request replays.

use crate::control::LpGatewayControlClient;
use crate::error::{LpClientError, Result};
use crate::registration::frames::exchange_registration;

use nym_crypto::asymmetric::{ed25519, x25519};
use nym_lp::LpTransportSession;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::LpTransportChannel;
use nym_registration_common::mixnet::{
    LpMixnetRegistrationResponseMessageContent, RegistrationNonce,
};
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, LpRegistrationResponseData, RegistrationStatus,
};
use std::net::SocketAddr;
use tokio::net::TcpStream;

/// Registers for mixnet use with the gateway at the other end of the channel it borrows.
///
/// Owns the session, since it is what registration names, and hands it back on success - the data
/// plane keeps using it after the control connection has closed.
pub struct LpMixnetRegistrationClient<'a, S = TcpStream> {
    channel: &'a mut LpGatewayControlClient<S>,
    gateway: SocketAddr,
    session: LpTransportSession,
}

impl<'a, S> LpMixnetRegistrationClient<'a, S>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    /// `session` is the one established with `gateway` over `channel`.
    pub fn new(
        channel: &'a mut LpGatewayControlClient<S>,
        gateway: SocketAddr,
        session: LpTransportSession,
    ) -> Self {
        Self {
            channel,
            gateway,
            session,
        }
    }

    /// Register the session for mixnet use, returning it now that it has a name.
    ///
    /// Two exchanges: the first names the keys the gateway will fingerprint and asks for a
    /// challenge, the second signs that challenge with `identity`'s private half to prove the keys
    /// are the client's. Both keys, because the name the gateway binds derives from both - see
    /// [`ClientAddress::from_keys`](nym_sphinx_addressing::ClientAddress::from_keys).
    pub async fn register(
        mut self,
        identity: &ed25519::KeyPair,
        encryption_key: &x25519::PublicKey,
        gateway_identity: &ed25519::PublicKey,
    ) -> Result<LpTransportSession> {
        let nonce = self
            .request_challenge(LpRegistrationRequest::new_initial_mixnet(
                identity.public_key(),
                encryption_key,
                gateway_identity,
            ))
            .await?;

        self.finalise(LpRegistrationRequest::new_finalise_mixnet(
            identity,
            encryption_key,
            gateway_identity,
            nonce,
            // nothing to claim yet; the field is carried so that the day there is costs no change
            None,
        ))
        .await?;

        Ok(self.session)
    }

    /// First leg: the gateway answers an opening request with the nonce to sign.
    async fn request_challenge(
        &mut self,
        request: LpRegistrationRequest,
    ) -> Result<RegistrationNonce> {
        let response = self.exchange(request).await?;

        // the opening request is unsigned, so there is nothing here a gateway could have completed
        if !matches!(response.status, RegistrationStatus::PendingMoreData) {
            return Err(LpClientError::unexpected_response(
                "gateway completed a mixnet registration before it was signed",
            ));
        }

        let LpRegistrationResponseData::Mixnet { data } = response.response_data else {
            return Err(LpClientError::unexpected_response(
                "gateway answered a mixnet registration in another mode",
            ));
        };

        let LpMixnetRegistrationResponseMessageContent::Challenge(challenge) = data.content else {
            return Err(LpClientError::unexpected_response(
                "gateway opened a mixnet registration without issuing a challenge",
            ));
        };

        Ok(challenge.nonce)
    }

    /// Second leg: anything short of completion leaves the session unnamed, including a gateway
    /// asking for more than the signature it just got.
    async fn finalise(&mut self, request: LpRegistrationRequest) -> Result<()> {
        let response = self.exchange(request).await?;

        if !matches!(response.status, RegistrationStatus::Completed) {
            return Err(LpClientError::unexpected_response(
                "gateway did not complete a signed mixnet registration",
            ));
        }

        Ok(())
    }

    /// One registration request out, one answer back, with a refusal raised as one.
    async fn exchange(&mut self, request: LpRegistrationRequest) -> Result<LpRegistrationResponse> {
        let timeout = self.channel.config.registration_timeout;

        let response = exchange_registration(
            self.channel,
            self.gateway,
            &mut self.session,
            request,
            timeout,
        )
        .await?;

        if matches!(response.status, RegistrationStatus::Failed) {
            return Err(LpClientError::RegistrationRejected {
                reason: response
                    .error_message()
                    .map(str::to_string)
                    .unwrap_or_else(|| "no reason given".to_string()),
            });
        }

        Ok(response)
    }
}
