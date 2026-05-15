// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0
use std::{net::SocketAddr, time::Instant};

use nym_lp_data::{PipelinePayload, TimedPayload};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_types::OutfoxPacket;
use tracing::warn;

use crate::node::lp::data::{
    handler::{
        error::LpDataHandlerError,
        messages::{MixMessage, OutfoxMixMessage},
    },
    shared::SharedLpDataState,
};

pub(crate) fn process(
    shared_state: &SharedLpDataState,
    outfox_packet: TimedPayload<Instant>,
    metadata: OutfoxMixMessage,
) -> Result<PipelinePayload<Instant, MixMessage, SocketAddr>, LpDataHandlerError> {
    let TimedPayload {
        data: outfox_bytes,
        timestamp: arrival_timestamp,
    } = outfox_packet;

    let mut outfox_packet = OutfoxPacket::try_from(outfox_bytes.as_slice())?;

    let key = shared_state.resolve_rotation_key(metadata.key_rotation)?;
    let next_address = outfox_packet.decode_next_layer(key.inner().as_ref())?;

    if outfox_packet.is_final_hop() {
        warn!("Dropping final hop packet as it is no longer supported");
        Err(LpDataHandlerError::FinalHop)
    } else {
        Ok(PipelinePayload::new(
            arrival_timestamp,         // Outfox doesn't have mixing delays !!!!!
            outfox_packet.to_bytes()?, // OutfoxPacket::to_bytes is actually infallible
            MixMessage::Outfox(metadata),
            NymNodeRoutingAddress::try_from_bytes(&next_address)?.into(),
        ))
    }
}
