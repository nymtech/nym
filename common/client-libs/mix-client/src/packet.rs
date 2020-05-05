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

use nymsphinx::addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nymsphinx::{delays, Destination, DestinationAddressBytes, SURBIdentifier, SphinxPacket};
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::time;
use topology::{NymTopology, NymTopologyError};

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

#[derive(Debug)]
pub enum SphinxPacketEncapsulationError {
    NoValidProvidersError,
    InvalidTopologyError,
    SphinxError(nymsphinx::Error),
    InvalidFirstMixAddress,
}

impl From<topology::NymTopologyError> for SphinxPacketEncapsulationError {
    fn from(_: NymTopologyError) -> Self {
        use SphinxPacketEncapsulationError::*;
        InvalidTopologyError
    }
}

impl From<nymsphinx::Error> for SphinxPacketEncapsulationError {
    fn from(err: nymsphinx::Error) -> Self {
        SphinxPacketEncapsulationError::SphinxError(err)
    }
}

impl From<NymNodeRoutingAddressError> for SphinxPacketEncapsulationError {
    fn from(_: NymNodeRoutingAddressError) -> Self {
        use SphinxPacketEncapsulationError::*;
        InvalidFirstMixAddress
    }
}

#[deprecated(note = "please use loop_cover_message_route instead")]
pub fn loop_cover_message<T: NymTopology>(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
    topology: &T,
    average_delay: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let destination = Destination::new(our_address, surb_id);

    #[allow(deprecated)]
    encapsulate_message(
        destination,
        LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
        topology,
        average_delay,
    )
}

pub fn loop_cover_message_route(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
    route: Vec<nymsphinx::Node>,
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

#[deprecated(note = "please use encapsulate_message_route instead")]
pub fn encapsulate_message<T: NymTopology>(
    recipient: Destination,
    message: Vec<u8>,
    topology: &T,
    average_delay: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let mut providers = topology.providers();
    if providers.is_empty() {
        return Err(SphinxPacketEncapsulationError::NoValidProvidersError);
    }
    // unwrap is fine here as we asserted there is at least single provider
    let provider = providers.pop().unwrap().into();

    let route = topology.random_route_to(provider)?;

    let delays = delays::generate_from_average_duration(route.len(), average_delay);

    // build the packet
    let packet = SphinxPacket::new(message, &route[..], &recipient, &delays, None)?;

    // we know the mix route must be valid otherwise we would have already returned an error
    let first_node_address =
        NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone())?;

    Ok((first_node_address.into(), packet))
}

pub fn encapsulate_message_route(
    recipient: Destination,
    message: Vec<u8>,
    route: Vec<nymsphinx::Node>,
    average_delay: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let delays = delays::generate_from_average_duration(route.len(), average_delay);

    // build the packet
    let packet = SphinxPacket::new(message, &route[..], &recipient, &delays, None)?;

    let first_node_address =
        NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone())?;

    Ok((first_node_address.into(), packet))
}
