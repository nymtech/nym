// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::TimedPayload;
use nym_sphinx::{ProcessedPacketData, SphinxPacket};
use tracing::warn;

use crate::client::lp::data::{
    handler::{error::LpDataHandlerError, messages::SphinxMessage},
    shared::SharedLpDataState,
};

pub(crate) fn process(
    shared_state: &SharedLpDataState,
    sphinx_packet: TimedPayload,
    _metadata: SphinxMessage,
) -> Result<TimedPayload, LpDataHandlerError> {
    let TimedPayload {
        data: sphinx_bytes,
        timestamp: arrival_timestamp,
    } = sphinx_packet;

    let sphinx_packet = SphinxPacket::from_bytes(&sphinx_bytes)?;

    // Final processing
    let processed_packet =
        sphinx_packet.process(shared_state.encryption_keys.private_key().as_ref())?;

    match processed_packet.data {
        ProcessedPacketData::ForwardHop { .. } => {
            warn!("Dropping forward hop packet in a client");
            Err(LpDataHandlerError::ForwardHop)
        }
        ProcessedPacketData::FinalHop { payload, .. } => Ok(TimedPayload::new(
            arrival_timestamp,
            payload.recover_plaintext()?,
        )),
    }
}
