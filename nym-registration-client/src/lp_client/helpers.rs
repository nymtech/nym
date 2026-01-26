// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpClientError;
use nym_crypto::asymmetric::ed25519;
use nym_lp::message::ForwardPacketData;
use nym_lp::peer::LpRemotePeer;
use nym_lp::state_machine::{LpAction, LpData, LpDataKind, LpInput};
use nym_registration_common::{
    LpRegistrationRequest, LpRegistrationResponse, NymNodeLPInformation,
};

pub(crate) fn convert_registration_request(
    request: LpRegistrationRequest,
) -> Result<LpInput, LpClientError> {
    let request_bytes = request.serialise().map_err(|e| {
        LpClientError::SendRegistrationRequest(format!("Failed to serialize request: {e}"))
    })?;

    tracing::debug!(
        "Sending registration request ({} bytes)",
        request_bytes.len()
    );

    let data = LpData::new_registration(request_bytes);
    Ok(LpInput::SendData(data))
}

pub(crate) fn try_convert_registration_response(
    action: LpAction,
) -> Result<LpRegistrationResponse, LpClientError> {
    let response_data = match action {
        LpAction::DeliverData(data) => data,
        other => {
            return Err(LpClientError::Transport(format!(
                "Unexpected action when receiving registration response: {other:?}"
            )));
        }
    };

    if response_data.kind != LpDataKind::Registration {
        return Err(LpClientError::Transport(format!(
            "did not receive a valid registration response. got {:?} instead",
            response_data.kind
        )));
    }

    let response =
        LpRegistrationResponse::try_deserialise(&response_data.content).map_err(|e| {
            LpClientError::Transport(format!("Failed to deserialize registration response: {e}",))
        })?;

    Ok(response)
}

pub(crate) fn convert_forward_data(request: ForwardPacketData) -> LpInput {
    let request_bytes = request.to_bytes();

    tracing::trace!(
        "Sending forward packet data request ({} bytes)",
        request_bytes.len()
    );

    let data = LpData::new_forward(request_bytes);
    LpInput::SendData(data)
}

pub(crate) fn try_convert_forward_response(action: LpAction) -> Result<Vec<u8>, LpClientError> {
    let response_data = match action {
        LpAction::DeliverData(data) => data,
        other => {
            return Err(LpClientError::Transport(format!(
                "Unexpected action when receiving forward response: {:?}",
                other
            )));
        }
    };

    if response_data.kind != LpDataKind::Forward {
        return Err(LpClientError::Transport(format!(
            "did not receive a valid foreward response. got {:?} instead",
            response_data.kind
        )));
    }

    Ok(response_data.content.into())
}

pub(crate) fn to_lp_remote_peer(
    identity: ed25519::PublicKey,
    data: NymNodeLPInformation,
) -> LpRemotePeer {
    LpRemotePeer::new(identity, data.x25519).with_kem_key_digests(data.expected_kem_key_hashes)
}
