use crate::utils::bytes;
use addressing;
use curve25519_dalek::montgomery::MontgomeryPoint;
use sphinx::route::{Destination, DestinationAddressBytes, Node, SURBIdentifier};
use sphinx::SphinxPacket;
use std::net::SocketAddr;
use topology::NymTopology;

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

pub fn loop_cover_message<T: NymTopology>(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
    topology: &T,
) -> (SocketAddr, SphinxPacket) {
    let destination = Destination::new(our_address, surb_id);

    encapsulate_message(destination, LOOP_COVER_MESSAGE_PAYLOAD.to_vec(), topology)
}

pub fn encapsulate_message<T: NymTopology>(
    recipient: Destination,
    message: Vec<u8>,
    topology: &T,
) -> (SocketAddr, SphinxPacket) {
    let mixes_route = topology.route_from();
    let providers = topology.get_mix_provider_nodes();
    let first_provider = providers.first().unwrap();
    let decoded_key_bytes =
        base64::decode_config(&first_provider.pub_key, base64::URL_SAFE).unwrap();
    let key_bytes = bytes::zero_pad_to_32(decoded_key_bytes);
    let key = MontgomeryPoint(key_bytes);

    let provider = Node::new(
        addressing::encoded_bytes_from_socket_address(first_provider.mixnet_listener.clone()),
        key,
    );

    let route = [mixes_route, vec![provider]].concat();

    // Set average packet delay to an arbitrary but at least not super-slow value for testing.
    let average_delay = 0.1;
    let delays = sphinx::header::delays::generate(route.len(), average_delay);

    // build the packet
    let packet = sphinx::SphinxPacket::new(message, &route[..], &recipient, &delays).unwrap();

    let first_node_address =
        addressing::socket_address_from_encoded_bytes(route.first().unwrap().address);

    (first_node_address, packet)
}
