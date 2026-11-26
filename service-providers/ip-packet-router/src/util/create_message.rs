use nym_sdk::mixnet::{AnonymousSenderTag, InputMessage};
use nym_task::connections::TransmissionLane;

pub(crate) fn create_input_message(
    recipient: &AnonymousSenderTag,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    InputMessage::new_reply(*recipient, response_packet, lane, packet_type)
}
