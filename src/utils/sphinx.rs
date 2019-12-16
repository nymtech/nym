use crate::utils::bytes;
use sphinx::route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier};
use sphinx::SphinxPacket;

const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

pub fn loop_cover_message(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
) -> (NodeAddressBytes, SphinxPacket) {
    let destination = Destination::new(our_address, surb_id);

    encapsulate_message(destination, LOOP_COVER_MESSAGE_PAYLOAD.to_vec())
}

pub fn encapsulate_message(
    recipient: Destination,
    message: Vec<u8>,
) -> (NodeAddressBytes, SphinxPacket) {
    // here we would be getting topology, etc
    let first_node_address = bytes::zero_pad_to_32("127.0.0.1:8080".as_bytes().to_vec());
    let dummy_route = vec![
        Node::new(first_node_address, Default::default()),
        Node::new(
            bytes::zero_pad_to_32("127.0.0.1:8081".as_bytes().to_vec()),
            Default::default(),
        ),
    ];

    let delays = sphinx::header::delays::generate(dummy_route.len());

    // build the packet
    let packet = sphinx::SphinxPacket::new(message, &dummy_route[..], &recipient, &delays).unwrap();

    (first_node_address, packet)
}
