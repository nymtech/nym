use crate::utils::bytes;
use sphinx::route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier};
use sphinx::SphinxPacket;

const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

pub fn loop_cover_message(
    our_address: DestinationAddressBytes,
    surb_id: SURBIdentifier,
) -> (NodeAddressBytes, SphinxPacket) {
    let first_node_address = bytes::zero_pad_to_32("127.0.0.1:8080".as_bytes().to_vec());
    let dummy_route = vec![
        Node::new(first_node_address, Default::default()),
        Node::new(
            bytes::zero_pad_to_32("127.0.0.1:8081".as_bytes().to_vec()),
            Default::default(),
        ),
    ];

    let destination = Destination::new(our_address, surb_id);
    let delays = sphinx::header::delays::generate(dummy_route.len());

    // build the packet
    let packet = sphinx::SphinxPacket::new(
        LOOP_COVER_MESSAGE_PAYLOAD.to_vec(),
        &dummy_route[..],
        &destination,
        &delays,
    )
    .unwrap();

    (first_node_address, packet)
}
