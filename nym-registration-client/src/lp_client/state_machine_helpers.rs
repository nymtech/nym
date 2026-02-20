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
/// and attempting to extract `EncryptedLpPacket` from the provided state machine.
pub(crate) fn prepare_send_packet(
    data: LpData,
    state_machine: &mut LpStateMachine,
) -> Result<EncryptedLpPacket, LpClientError> {
    let action = state_machine
        .process_input(LpInput::SendData(data))
        .ok_or(LpClientError::UnexpectedStateMachineHalt)??;

    match action {
        LpAction::SendPacket(packet) => Ok(packet),
        action => Err(LpClientError::UnexpectedStateMachineAction { action }),
    }
}

/// Attempt to recover received `LpData` from the received `LpPacket`
/// using the provided state machine.
pub(crate) fn extract_forwarded_response(
    response_packet: LpPacket,
    state_machine: &mut LpStateMachine,
) -> Result<LpData, LpClientError> {
    let action = state_machine
        .process_input(LpInput::ReceivePacket(response_packet))
        .ok_or(LpClientError::UnexpectedStateMachineHalt)??;

    match action {
        LpAction::DeliverData(data) => Ok(data),
        action => Err(LpClientError::UnexpectedStateMachineAction { action }),
    }
}
