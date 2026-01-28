// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpClientError;
use bytes::BytesMut;
use nym_lp::codec::{OuterAeadKey, serialize_lp_packet};
use nym_lp::state_machine::{LpAction, LpData, LpInput};
use nym_lp::{LpPacket, LpStateMachine};

/// Gets the outer AEAD key for sending (encryption) from the state machine.
///
/// Returns `None` during early handshake before PSK derivation.
pub(crate) fn get_send_key(state_machine: &LpStateMachine) -> Option<OuterAeadKey> {
    state_machine
        .session()
        .ok()
        .and_then(|s| s.outer_aead_key_for_sending())
}

/// Gets the outer AEAD key for receiving (decryption) from the state machine.
///
/// Returns `None` during early handshake before PSK derivation.
pub(crate) fn get_recv_key(state_machine: &LpStateMachine) -> Option<OuterAeadKey> {
    state_machine
        .session()
        .ok()
        .and_then(|s| s.outer_aead_key())
}

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
pub(crate) fn serialize_packet(
    packet: &LpPacket,
    outer_key: Option<&OuterAeadKey>,
) -> Result<Vec<u8>, LpClientError> {
    let mut buf = BytesMut::new();
    // Use outer AEAD key when available (after PSK derivation)
    serialize_lp_packet(packet, &mut buf, outer_key)
        .map_err(|e| LpClientError::Transport(format!("Failed to serialize LP packet: {}", e)))?;
    Ok(buf.to_vec())
}

/// Attempt to prepare the provided data for sending by wrapping it in appropriate `LpAction`,
/// and attempting to extract `LpPacket` from the provided srtate machine.
pub(crate) fn prepare_send_packet(
    data: LpData,
    state_machine: &mut LpStateMachine,
) -> Result<LpPacket, LpClientError> {
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

    let send_key = get_send_key(state_machine);
    serialize_packet(&packet, send_key.as_ref())
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
