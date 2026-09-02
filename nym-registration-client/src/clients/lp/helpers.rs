// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

//! Turning registration messages into LP frames and back.
//!
//! The channel these travel over knows nothing about them - see
//! [`nym_lp_gateway_client`].

use nym_lp::peer::LpRemotePeer;
use nym_lp_data::packet::LpFrame;
use nym_lp_data::packet::frame::LpFrameKind;
use nym_lp_gateway_client::LpClientError;
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, NymNodeLPInformation,
};

pub(crate) trait LpFrameSendExt {
    fn to_lp_frame(&self) -> Result<LpFrame, LpClientError>;
}

pub(crate) trait LpFrameDeliverExt: Sized {
    fn from_lp_frame(frame: LpFrame) -> Result<Self, LpClientError>;
}

impl LpFrameSendExt for LpRegistrationRequest {
    fn to_lp_frame(&self) -> Result<LpFrame, LpClientError> {
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
    fn from_lp_frame(frame: LpFrame) -> Result<Self, LpClientError> {
        if frame.kind() != LpFrameKind::Registration {
            return Err(LpClientError::UnexpectedLpPayload { typ: frame.kind() });
        }

        let response = LpRegistrationResponse::try_deserialise(&frame.content)
            .map_err(|source| LpClientError::MalformedRegistrationData { source })?;

        Ok(response)
    }
}

pub(crate) fn to_lp_remote_peer(data: NymNodeLPInformation) -> LpRemotePeer {
    LpRemotePeer::new(data.x25519).with_key_digests(data.expected_kem_key_hashes)
}
