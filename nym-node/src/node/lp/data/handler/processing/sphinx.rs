// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use nym_lp_data::{PipelinePayload, TimedPayload};
use nym_sphinx_addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx_framing::processing::PacketProcessingError;
use nym_sphinx_types::SphinxPacket;
use tracing::{error, warn};

use crate::node::lp::data::{
    handler::{
        error::LpDataHandlerError,
        messages::{ForwardSphinxMessage, MixMessage, NymNodeMessage, SphinxMixMessage},
    },
    shared::{SharedGatewayLpDataState, SharedLpDataState},
};

pub(crate) fn process(
    shared_state: &SharedLpDataState,
    sphinx_packet: TimedPayload,
    metadata: SphinxMixMessage,
) -> Result<PipelinePayload<MixMessage, NymNodeRoutingAddress>, LpDataHandlerError> {
    let TimedPayload {
        data: sphinx_bytes,
        timestamp: arrival_timestamp,
    } = sphinx_packet;

    let sphinx_packet = SphinxPacket::from_bytes(&sphinx_bytes)?;

    // Extracting shared_secret
    let key = shared_state.resolve_rotation_key(metadata.key_rotation)?;
    let rotation_id = key.rotation_id();
    let expanded_shared_secret = sphinx_packet
        .header
        .compute_expanded_shared_secret(key.inner().as_ref());

    // Replay detection
    if !shared_state.replay_protection_filter.disabled() {
        let replay_tag = expanded_shared_secret.replay_tag();
        let Ok(replayed_packet) = shared_state
            .replay_protection_filter
            .check_and_set(rotation_id, replay_tag)
        else {
            // our mutex got poisoned - we have to shut down
            error!("CRITICAL FAILURE: replay bloomfilter mutex poisoning!");
            shared_state.shutdown_token.cancel();
            Err(LpDataHandlerError::internal(
                "replay bloomfilter mutex poisoning!",
            ))?
        };
        if replayed_packet {
            warn!(
                event = "packet.dropped.replay",
                rotation_id, "dropping replayed packet"
            );
            Err(PacketProcessingError::PacketReplay)?
        }
    }

    // Final processing
    let processed_packet = sphinx_packet.process_with_expanded_secret(&expanded_shared_secret)?;

    match processed_packet.data {
        nym_sphinx_types::ProcessedPacketData::ForwardHop {
            next_hop_packet,
            next_hop_address,
            delay,
        } => {
            let mut delay = delay.to_duration();
            // Prevent excessively high delay that would DoS the nodes
            if delay > shared_state.processing_config.maximum_packet_delay {
                shared_state.excessive_delay_packet();
                delay = shared_state.processing_config.maximum_packet_delay;
            }

            Ok(PipelinePayload::new(
                arrival_timestamp + delay,
                next_hop_packet.to_bytes(),
                MixMessage::Sphinx(metadata),
                NymNodeRoutingAddress::try_from(next_hop_address)?,
            ))
        }
        nym_sphinx_types::ProcessedPacketData::FinalHop { .. } => {
            warn!("Dropping final hop packet as it is no longer supported");
            Err(LpDataHandlerError::FinalHop)
        }
    }
}

pub(crate) fn process_forward(
    shared_gateway_state: &SharedGatewayLpDataState,
    sphinx_packet: TimedPayload,
    metadata: ForwardSphinxMessage,
) -> Result<PipelinePayload<NymNodeMessage, NymNodeRoutingAddress>, LpDataHandlerError> {
    // SW TODO add bandwidth check here

    let node = shared_gateway_state
        .cached_topology
        .find_node(metadata.next_hop)
        .ok_or(LpDataHandlerError::UnknownNodeId {
            node_id: metadata.next_hop,
        })?;

    let routing_address: NymNodeRoutingAddress = node.mix_host.into();
    let nym_node_message = NymNodeMessage::new_sphinx_mix_message(metadata.into());

    Ok(PipelinePayload::new(
        sphinx_packet.timestamp,
        sphinx_packet.data,
        nym_node_message,
        routing_address,
    ))
}
