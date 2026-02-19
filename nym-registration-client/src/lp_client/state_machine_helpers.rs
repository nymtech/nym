// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpClientError;
use nym_lp::state_machine::{LpAction, LpData, LpInput};
use nym_lp::{EncryptedLpPacket, LpPacket, LpStateMachine};

/// Serializes an LP packet to bytes.
///
/// # Arguments
/// * `packet` - The LP packet to serialize
///
/// # Returns
/// * `Ok(Vec<u8>)` - Serialized packet bytes
///
/// # Errors
/// Returns an error if serialization fails
pub(crate) fn serialize_packet(packet: &LpPacket) -> Result<Vec<u8>, LpClientError> {
    todo!()
    // let mut buf = BytesMut::new();
    // // Use outer AEAD key when available (after PSK derivation)
    // serialize_lp_packet(packet, &mut buf, outer_key)
    //     .map_err(|e| LpClientError::Transport(format!("Failed to serialize LP packet: {}", e)))?;
    // Ok(buf.to_vec())
}

/// Attempt to prepare the provided data for sending by wrapping it in appropriate `LpAction`,
/// and attempting to extract `LpPacket` from the provided srtate machine.
pub(crate) fn prepare_send_packet(
    data: LpData,
    state_machine: &mut LpStateMachine,
) -> Result<EncryptedLpPacket, LpClientError> {
    let action = state_machine
        .process_input(LpInput::SendData(data))
        .ok_or_else(|| LpClientError::transport("State machine returned no action"))?
        .map_err(|e| {
            LpClientError::SendRegistrationRequest(format!(
                "Failed to encrypt registration request: {e}",
            ))
        })?;

    match action {
        LpAction::SendPacket(packet) => Ok(packet),
        other => Err(LpClientError::Transport(format!(
            "Unexpected action when trying to send packet data: {other:?}",
        ))),
    }
}

/// Attempt to prepare the provided data for sending by wrapping it in appropriate `LpAction`,
/// serialising and finally encrypting (if appropriate key is available) the resultant `LpPacket`
/// It uses the provided state machine.
pub(crate) fn prepare_serialised_send_packet(
    data: LpData,
    state_machine: &mut LpStateMachine,
) -> Result<Vec<u8>, LpClientError> {
    let packet = prepare_send_packet(data, state_machine)?;
    todo!()
    // serialize_packet(&packet, Some(send_key))
}

/// Attempt to recover received `LpData` from the received `LpPacket`
/// using the provided state machine.
pub(crate) fn extract_forwarded_response(
    response_packet: LpPacket,
    state_machine: &mut LpStateMachine,
) -> Result<LpData, LpClientError> {
    let action = state_machine
        .process_input(LpInput::ReceivePacket(response_packet))
        .ok_or_else(|| LpClientError::Transport("State machine returned no action".to_string()))?
        .map_err(|e| {
            LpClientError::Transport(format!("Failed to decrypt received response: {e}"))
        })?;

    match action {
        LpAction::DeliverData(data) => Ok(data),
        other => Err(LpClientError::Transport(format!(
            "Unexpected action when receiving response: {other:?}"
        ))),
    }
}
