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

use nymsphinx_acknowledgements::surb_ack::SURBAck;
use nymsphinx_acknowledgements::AckAes128Key;
use nymsphinx_addressing::clients::Recipient;
use nymsphinx_addressing::nodes::{NymNodeRoutingAddress, NymNodeRoutingAddressError};
use nymsphinx_chunking::fragment::COVER_FRAG_ID;
use nymsphinx_params::packet_sizes::PacketSize;
use nymsphinx_types::builder::SphinxPacketBuilder;
use nymsphinx_types::{delays, Destination, Error as SphinxError, SphinxPacket};
use rand::{CryptoRng, RngCore};
use std::convert::TryFrom;
use std::net::SocketAddr;
use std::time;
use topology::{NymTopology, NymTopologyError};

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

#[derive(Debug)]
pub enum CoverMessageError {
    NoValidProvidersError,
    InvalidTopologyError,
    SphinxError(SphinxError),
    InvalidFirstMixAddress,
}

impl From<SphinxError> for CoverMessageError {
    fn from(err: SphinxError) -> Self {
        CoverMessageError::SphinxError(err)
    }
}

impl From<NymNodeRoutingAddressError> for CoverMessageError {
    fn from(_: NymNodeRoutingAddressError) -> Self {
        use CoverMessageError::*;
        InvalidFirstMixAddress
    }
}

impl From<NymTopologyError> for CoverMessageError {
    fn from(_: NymTopologyError) -> Self {
        CoverMessageError::InvalidTopologyError
    }
}

pub fn generate_loop_cover_surb_ack<R, T>(
    rng: &mut R,
    topology: &T,
    ack_key: &AckAes128Key,
    full_address: &Recipient,
    average_ack_delay: time::Duration,
) -> Result<SURBAck, CoverMessageError>
where
    R: RngCore + CryptoRng,
    T: NymTopology,
{
    Ok(SURBAck::construct(
        rng,
        full_address,
        ack_key,
        &COVER_FRAG_ID.to_bytes(),
        average_ack_delay,
        topology,
    )?)
}

pub fn generate_loop_cover_packet<R, T>(
    rng: &mut R,
    topology: &T,
    ack_key: &AckAes128Key,
    full_address: &Recipient,
    average_ack_delay: time::Duration,
    average_packet_delay: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), CoverMessageError>
where
    R: RngCore + CryptoRng,
    T: NymTopology,
{
    // we don't care about total ack delay - we will not be retransmitting it anyway
    let (_, ack_bytes) =
        generate_loop_cover_surb_ack(rng, topology, ack_key, full_address, average_ack_delay)?
            .prepare_for_sending();

    let cover_payload: Vec<_> = ack_bytes
        .into_iter()
        .chain(LOOP_COVER_MESSAGE_PAYLOAD.into_iter().cloned())
        .collect();

    let route = topology.random_route_to_gateway(&full_address.gateway())?;
    let delays = delays::generate_from_average_duration(route.len(), average_packet_delay);
    // in our design we don't care about SURB_ID
    let destination = Destination::new(full_address.destination(), Default::default());

    // once merged, that's an easy rng injection point for sphinx packets : )
    let packet = SphinxPacketBuilder::new()
        .with_payload_size(PacketSize::default().payload_size())
        .build_packet(cover_payload, &route, &destination, &delays)
        .unwrap();

    let first_hop_address =
        NymNodeRoutingAddress::try_from(route.first().unwrap().address.clone()).unwrap();

    Ok((first_hop_address.into(), packet))
}
