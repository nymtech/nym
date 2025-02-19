use nym_sdk::mixnet::{AnonymousSenderTag, InputMessage, Recipient};
use nym_task::connections::TransmissionLane;

pub(crate) fn create_input_message(
    reply_to_tag: AnonymousSenderTag,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    InputMessage::new_reply(reply_to_tag, response_packet, lane, packet_type)
}
