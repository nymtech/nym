use crate::clients::directory::presence::Topology;
use crate::utils::{bytes, topology};
use sphinx::route::{Destination, DestinationAddressBytes, Node, NodeAddressBytes, SURBIdentifier};
use sphinx::SphinxPacket;

const LOOP_COVER_MESSAGE_PAYLOAD: &[u8] = b"The cake is a lie!";

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
    // here we would be getting topology, etc

    let mixes_route = topology::route_from(&topology, 1);

    let provider = Node::new(
        topology::socket_bytes_from_string("127.0.0.1:8081".to_string()),
        //        bytes::zero_pad_to_32("127.0.0.1:8081".as_bytes().to_vec()),
        Default::default(),
    );

    let route = [mixes_route, vec![provider]].concat();

    let delays = sphinx::header::delays::generate(route.len());

    // build the packet
    let packet = sphinx::SphinxPacket::new(message, &route[..], &recipient, &delays).unwrap();

    let first_node_address = route.first().unwrap().address;

    (first_node_address, packet)
}
