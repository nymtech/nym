// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::LpClientError;
use nym_lp::LpTransportSession;
use nym_lp::session::{LpAction, LpInput};
use nym_lp_data::packet::{EncryptedLpPacket, LpFrame};

/// Attempt to prepare the provided data for sending by wrapping it in appropriate `LpAction`,
/// and attempting to extract `EncryptedLpPacket` from the provided state machine.
pub(crate) fn prepare_send_packet(
    frame: LpFrame,
    state_machine: &mut LpTransportSession,
) -> Result<EncryptedLpPacket, LpClientError> {
    let action = state_machine.process_input(LpInput::SendFrame(frame))?;

    match action {
        LpAction::SendPacket(packet) => Ok(packet),
        action => Err(LpClientError::UnexpectedStateMachineAction { action }),
    }
}

/// Attempt to recover received `LpData` from the received `LpPacket`
/// using the provided state machine.
pub(crate) fn extract_forwarded_response(
    response_packet: EncryptedLpPacket,
    state_machine: &mut LpTransportSession,
) -> Result<LpFrame, LpClientError> {
    let action = state_machine.process_input(LpInput::ReceivePacket(response_packet))?;

    match action {
        LpAction::DeliverFrame(frame) => Ok(frame),
        action => Err(LpClientError::UnexpectedStateMachineAction { action }),
    }
}
