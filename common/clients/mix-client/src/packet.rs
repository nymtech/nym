use addressing;
use addressing::AddressTypeError;
use sphinx::route::{Destination, DestinationAddressBytes, SURBIdentifier};
use sphinx::SphinxPacket;
use std::net::SocketAddr;
use std::time;
use topology::{NymTopology, NymTopologyError};

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

#[derive(Debug)]
pub enum SphinxPacketEncapsulationError {
    NoValidProvidersError,
    InvalidTopologyError,
    SphinxEncapsulationError(sphinx::header::SphinxUnwrapError),
    InvalidFirstMixAddress,
}

impl From<topology::NymTopologyError> for SphinxPacketEncapsulationError {
    fn from(_: NymTopologyError) -> Self {
        use SphinxPacketEncapsulationError::*;
        InvalidTopologyError
    }
}

// it is correct error we're converting from, it just has an unfortunate name
// related issue: https://github.com/nymtech/sphinx/issues/40
impl From<sphinx::header::SphinxUnwrapError> for SphinxPacketEncapsulationError {
    fn from(err: sphinx::header::SphinxUnwrapError) -> Self {
        use SphinxPacketEncapsulationError::*;
        SphinxEncapsulationError(err)
    }
}

impl From<AddressTypeError> for SphinxPacketEncapsulationError {
    fn from(_: AddressTypeError) -> Self {
        use SphinxPacketEncapsulationError::*;
        InvalidFirstMixAddress
    }
}

pub fn loop_cover_message<T: NymTopology>(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
    topology: &T,
    average_delay_duration: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let destination = Destination::new(our_address, surb_id);

    encapsulate_message(
        destination,
        LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
        topology,
        average_delay_duration,
    )
}

pub fn encapsulate_message<T: NymTopology>(
    recipient: Destination,
    message: Vec<u8>,
    topology: &T,
    average_delay_duration: time::Duration,
) -> Result<(SocketAddr, SphinxPacket), SphinxPacketEncapsulationError> {
    let mut providers = topology.providers();
    if providers.len() == 0 {
        return Err(SphinxPacketEncapsulationError::NoValidProvidersError);
    }
    // unwrap is fine here as we asserted there is at least single provider
    let provider = providers.pop().unwrap().into();

    let route = topology.route_to(provider)?;

    let delays =
        sphinx::header::delays::generate_from_average_duration(route.len(), average_delay_duration);

    // build the packet
    let packet = sphinx::SphinxPacket::new(message, &route[..], &recipient, &delays)?;

    // we know the mix route must be valid otherwise we would have already returned an error
    let first_node_address =
        addressing::socket_address_from_encoded_bytes(route.first().unwrap().address.to_bytes())?;

    Ok((first_node_address, packet))
}
