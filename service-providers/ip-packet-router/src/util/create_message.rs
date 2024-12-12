use nym_sdk::mixnet::{InputMessage, Recipient};
use nym_task::connections::TransmissionLane;

pub(crate) fn create_input_message(
    nym_address: Recipient,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    InputMessage::new_regular(nym_address, response_packet, lane, packet_type)
}
