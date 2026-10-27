// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! What a client says over an LP channel to become known to a gateway.
//!
//! The transport in this crate moves packets and holds no session; a registration client borrows a
//! channel, owns a session, and does one request/response over it. [`exchange_registration`] is
//! that round trip, shared by every mode.
//!
//! # Why it lives here rather than in `nym-registration-client`
//!
//! Because `nym-client-core` needs to register at startup and cannot reach that crate:
//! `nym-registration-client` -> `nym-sdk` -> `nym-client-core` is a cycle, and feature-gating does
//! not help since Cargo resolves optional dependencies too.
//!
//! This is temporary. Once the legacy websocket path leaves `nym-registration-client`, it sheds
//! `nym-sdk`, drops below `nym-client-core`, and this module moves there to sit beside dVPN
//! registration - which stays where it is in the meantime, with the credential and WireGuard
//! machinery it needs.

use crate::control::LpGatewayControlClient;
use crate::error::Result;
use crate::session_helpers::{extract_forwarded_response, prepare_send_packet};
use nym_lp::LpTransportSession;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::LpTransportChannel;
use nym_registration_common::{LpRegistrationRequest, LpRegistrationResponse};
use std::net::SocketAddr;
use std::time::Duration;

mod mixnet;
pub use mixnet::LpMixnetRegistrationClient;

/// One registration request out, one response back, on the gateway's control connection.
///
/// Interpreting the response is the caller's: what counts as success differs per mode.
pub async fn exchange_registration<S>(
    client: &mut LpGatewayControlClient<S>,
    gateway: SocketAddr,
    session: &mut LpTransportSession,
    request: LpRegistrationRequest,
    timeout: Duration,
) -> Result<LpRegistrationResponse>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
{
    let request_packet = prepare_send_packet(request.to_lp_frame()?, session)?;

    let response_packet = client
        .exchange_control(gateway, &request_packet, timeout)
        .await?;

    let response_frame = extract_forwarded_response(response_packet, session)?;

    Ok(LpRegistrationResponse::from_lp_frame(response_frame)?)
}
