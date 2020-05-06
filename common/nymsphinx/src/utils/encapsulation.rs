// Copyright 2020 Nym Technologies SA
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use crate::{delays, Destination, DestinationAddressBytes, SURBIdentifier, SphinxPacket};
use crate::{Error as SphinxError, Node as SphinxNode};
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::time;

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

#[derive(Debug)]
pub enum SphinxPacketEncapsulationError {
    NoValidProvidersError,
    InvalidTopologyError,
    SphinxError(SphinxError),
    InvalidFirstMixAddress,
}

impl From<SphinxError> for SphinxPacketEncapsulationError {
    fn from(err: SphinxError) -> Self {
        SphinxPacketEncapsulationError::SphinxError(err)
    }
}

impl From<NymNodeRoutingAddressError> for SphinxPacketEncapsulationError {
    fn from(_: NymNodeRoutingAddressError) -> Self {
        use SphinxPacketEncapsulationError::*;
        InvalidFirstMixAddress
    }
}

pub fn loop_cover_message_route(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
    route: Vec<SphinxNode>,
    average_delay: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let destination = Destination::new(our_address, surb_id);

    encapsulate_message_route(
        destination,
        LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
        route,
        average_delay,
    )
}

pub fn encapsulate_message_route(
    recipient: Destination,
    message: Vec<u8>,
    route: Vec<SphinxNode>,
    average_delay: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let delays = delays::generate_from_average_duration(route.len(), average_delay);

    // build the packet
    let packet = SphinxPacket::new(message, &route[..], &recipient, &delays, None)?;

    let first_node_address =
        NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone())?;

    Ok((first_node_address.into(), packet))
}
