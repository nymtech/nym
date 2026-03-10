// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpClientError;
use nym_lp::packet::LpMessage;
use nym_lp::state_machine::{LpAction, LpInput};
use nym_lp::{LpStateMachine, packet::EncryptedLpPacket};

/// Attempt to prepare the provided data for sending by wrapping it in appropriate `LpAction`,
/// and attempting to extract `EncryptedLpPacket` from the provided state machine.
pub(crate) fn prepare_send_packet(
    data: LpMessage,
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
    response_packet: EncryptedLpPacket,
    state_machine: &mut LpStateMachine,
) -> Result<LpMessage, LpClientError> {
    let action = state_machine
        .process_input(LpInput::ReceivePacket(response_packet))
        .ok_or(LpClientError::UnexpectedStateMachineHalt)??;

    match action {
        LpAction::DeliverData(data) => Ok(data),
        action => Err(LpClientError::UnexpectedStateMachineAction { action }),
    }
}
