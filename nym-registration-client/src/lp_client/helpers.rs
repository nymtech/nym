// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

#![allow(dead_code)]

use crate::LpClientError;
use nym_lp::packet::message::LpMessageType;
use nym_lp::packet::{ForwardPacketData, LpMessage};
use nym_lp::peer::LpRemotePeer;
use nym_lp::session::{LpAction, LpInput};
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, NymNodeLPInformation,
};
use rand09::Rng;

pub(crate) trait LpDataSendExt {
    fn to_lp_data(&self) -> Result<LpMessage, LpClientError>;
}

pub(crate) trait LpDataDeliverExt: Sized {
    fn from_lp_data(data: LpMessage) -> Result<Self, LpClientError>;
}

impl LpDataSendExt for LpRegistrationRequest {
    fn to_lp_data(&self) -> Result<LpMessage, LpClientError> {
        let request_bytes = self.serialise().map_err(|e| {
            LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {e}"))
        })?;

        tracing::debug!(
            "Sending registration request ({} bytes)",
            request_bytes.len()
        );

        Ok(LpMessage::new_registration(request_bytes))
    }
}

impl LpDataDeliverExt for LpRegistrationResponse {
    fn from_lp_data(data: LpMessage) -> Result<Self, LpClientError> {
        if data.kind() != LpMessageType::Registration {
            return Err(LpClientError::UnexpectedLpPayload { typ: data.kind() });
        }

        let response = LpRegistrationResponse::try_deserialise(&data.content)
            .map_err(|source| LpClientError::MalformedRegistrationData { source })?;

        Ok(response)
    }
}

impl LpDataSendExt for ForwardPacketData {
    fn to_lp_data(&self) -> Result<LpMessage, LpClientError> {
        let request_bytes = self.to_bytes();

        tracing::trace!(
            "Sending forward packet data request ({} bytes)",
            request_bytes.len()
        );

        Ok(LpMessage::new_forward(request_bytes))
    }
}

pub(crate) fn convert_forward_data(request: ForwardPacketData) -> Result<LpInput, LpClientError> {
    Ok(LpInput::SendData(request.to_lp_data()?))
}

pub(crate) fn try_convert_forward_response(action: LpAction) -> Result<Vec<u8>, LpClientError> {
    let response_data = match action {
        LpAction::DeliverData(data) => data,
        action => return Err(LpClientError::UnexpectedStateMachineAction { action }),
    };

    if response_data.kind() != LpMessageType::Forward {
        return Err(LpClientError::UnexpectedLpPayload {
            typ: response_data.kind(),
        });
    }

    Ok(response_data.content.into())
}

pub(crate) fn to_lp_remote_peer(data: NymNodeLPInformation) -> LpRemotePeer {
    LpRemotePeer::new(data.x25519).with_key_digests(data.expected_kem_key_hashes)
}

pub(crate) async fn exponential_backoff_with_jitter(attempt: u32) {
    // Exponential backoff with jitter: 100ms, 200ms, 400ms, 800ms, 1600ms (capped)
    let base_delay_ms = 100u64 * (1 << attempt.min(4));
    let jitter_ms: u64 = rand09::rng().random_range(0..(base_delay_ms / 4 + 1));
    let delay = std::time::Duration::from_millis(base_delay_ms + jitter_ms);
    tracing::info!("Retrying registration after the following delay {delay:?}");
    tokio::time::sleep(delay).await;
}
