use crate::clients::directory::presence::Topology;
use crate::utils::{addressing, bytes, topology};
use curve25519_dalek::montgomery::MontgomeryPoint;
use sphinx::route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier};
use sphinx::SphinxPacket;

pub const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

pub fn loop_cover_message(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
    topology: &Topology,
) -> (NodeAddressBytes, SphinxPacket) {
    let destination = Destination::new(our_address, surb_id);

    encapsulate_message(destination, LOOP_COVER_MESSAGE_PAYLOAD.to_vec(), topology)
}

pub fn encapsulate_message(
    recipient: Destination,
    message: Vec<u8>,
    topology: &Topology,
) -> (NodeAddressBytes, SphinxPacket) {
    let mixes_route = topology::route_from(&topology);
    let first_provider = topology.mix_provider_nodes.first().unwrap();
    let decoded_key_bytes =
        base64::decode_config(&first_provider.pub_key, base64::URL_SAFE).unwrap();
    let key_bytes = bytes::zero_pad_to_32(decoded_key_bytes);
    let key = MontgomeryPoint(key_bytes);

    let provider = Node::new(
        addressing::encoded_bytes_from_socket_address(first_provider.host.clone().parse().unwrap()),
        key,
    );

    let route = [mixes_route, vec![provider]].concat();

    let delays = sphinx::header::delays::generate(route.len());

    // build the packet
    let packet = sphinx::SphinxPacket::new(message, &route[..], &recipient, &delays).unwrap();

    let first_node_address = route.first().unwrap().address;

    (first_node_address, packet)
}
