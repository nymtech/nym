// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Registration messages, on and off the wire.
//!
//! The transport moves packets and does not know what they say; this is where a registration
//! message becomes a frame, a frame becomes ciphertext, and the answer comes back the other way.
//! [`exchange_registration`] is the whole round trip, shared by every registration mode that talks
//! to a gateway directly.

use crate::client::LpGatewayClient;
use crate::error::{LpClientError, Result};
use crate::session_helpers::{extract_forwarded_response, prepare_send_packet};
use nym_lp::LpTransportSession;
use nym_lp::transport::LpHandshakeChannel;
use nym_lp::transport::traits::{LpDatagramChannel, LpTransportChannel};
use nym_lp_data::packet::LpFrame;
use nym_lp_data::packet::frame::LpFrameKind;
use nym_registration_common::{LpRegistrationRequest, LpRegistrationResponse};
use std::net::SocketAddr;
use std::time::Duration;

/// Serialise a registration message into the frame that carries it.
pub trait LpFrameSendExt {
    fn to_lp_frame(&self) -> Result<LpFrame>;
}

/// Recover a registration message from the frame that carried it.
pub trait LpFrameDeliverExt: Sized {
    fn from_lp_frame(frame: LpFrame) -> Result<Self>;
}

impl LpFrameSendExt for LpRegistrationRequest {
    fn to_lp_frame(&self) -> Result<LpFrame> {
        let request_bytes = self.serialise().map_err(|e| {
            LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {e}"))
        })?;

        tracing::debug!(
            "Sending registration request ({} bytes)",
            request_bytes.len()
        );

        Ok(LpFrame::new_registration(request_bytes))
    }
}

impl LpFrameDeliverExt for LpRegistrationResponse {
    fn from_lp_frame(frame: LpFrame) -> Result<Self> {
        if frame.kind() != LpFrameKind::Registration {
            return Err(LpClientError::UnexpectedLpPayload { typ: frame.kind() });
        }

        LpRegistrationResponse::try_deserialise(&frame.content)
            .map_err(|source| LpClientError::MalformedRegistrationData { source })
    }
}

/// One registration request out, one response back, on the gateway's control connection.
///
/// Interpreting the response is the caller's: what counts as success differs per mode.
pub async fn exchange_registration<S, D>(
    client: &mut LpGatewayClient<S, D>,
    gateway: SocketAddr,
    session: &mut LpTransportSession,
    request: LpRegistrationRequest,
    timeout: Duration,
) -> Result<LpRegistrationResponse>
where
    S: LpTransportChannel + LpHandshakeChannel + Unpin,
    D: LpDatagramChannel,
{
    let request_packet = prepare_send_packet(request.to_lp_frame()?, session)?;

    let response_packet = client
        .exchange_control(gateway, &request_packet, timeout)
        .await?;

    let response_frame = extract_forwarded_response(response_packet, session)?;

    LpRegistrationResponse::from_lp_frame(response_frame)
}
