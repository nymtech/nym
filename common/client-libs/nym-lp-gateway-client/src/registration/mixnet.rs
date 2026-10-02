// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Mixnet registration over an LP channel.
//!
//! One request, one answer, and what it establishes is the session's *name*. The gateway
//! fingerprints the client's ed25519 key into a [`ClientAddress`] and binds it to the session's
//! receiver index; until that happens it holds a session it can decrypt but cannot address, so
//! nothing can be sent back.
//!
//! [`ClientAddress`]: nym_sphinx_addressing::ClientAddress

use crate::client::LpGatewayClient;
use crate::error::{LpClientError, Result};
use crate::registration::frames::exchange_registration;

use nym_crypto::asymmetric::ed25519;
use nym_lp::LpTransportSession;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::{LpDatagramChannel, LpTransportChannel};
use nym_registration_common::{LpRegistrationRequest, RegistrationStatus};
use std::net::SocketAddr;
use tokio::net::{TcpStream, UdpSocket};

/// Registers for mixnet use with the gateway at the other end of the channel it borrows.
///
/// Owns the session, since it is what registration names, and hands it back on success - the data
/// plane keeps using it after the control connection has closed.
pub struct LpMixnetRegistrationClient<'a, S = TcpStream, D = UdpSocket> {
    channel: &'a mut LpGatewayClient<S, D>,
    gateway: SocketAddr,
    session: LpTransportSession,
}

impl<'a, S, D> LpMixnetRegistrationClient<'a, S, D>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
    D: LpDatagramChannel,
{
    /// `session` is the one established with `gateway` over `channel`.
    pub fn new(
        channel: &'a mut LpGatewayClient<S, D>,
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
    /// `identity` is the client's own ed25519 key - the one the gateway fingerprints.
    pub async fn register(mut self, identity: ed25519::PublicKey) -> Result<LpTransportSession> {
        let timeout = self.channel.config.registration_timeout;

        let response = exchange_registration(
            self.channel,
            self.gateway,
            &mut self.session,
            LpRegistrationRequest::new_mixnet(identity),
            timeout,
        )
        .await?;

        match response.status {
            RegistrationStatus::Completed => Ok(self.session),
            RegistrationStatus::Failed => Err(LpClientError::RegistrationRejected {
                reason: response
                    .error_message()
                    .map(str::to_string)
                    .unwrap_or_else(|| "no reason given".to_string()),
            }),
            // nothing in mixnet registration asks for more: it is one request and one answer
            RegistrationStatus::PendingMoreData => Err(LpClientError::unexpected_response(
                "gateway asked for more data during mixnet registration",
            )),
        }
    }
}
