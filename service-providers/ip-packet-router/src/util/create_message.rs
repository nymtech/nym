use nym_sdk::mixnet::InputMessage;
use nym_task::connections::TransmissionLane;

use crate::mixnet_listener::RequestSender;

pub(crate) fn create_input_message(
    recipient: &RequestSender,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    match recipient {
        RequestSender::NymAddress(recipient) => {
            InputMessage::new_regular(recipient, response_packet, lane, packet_type)
        }
        RequestSender::AnonymousSenderTag(tag) => {
            InputMessage::new_reply(tag, response_packet, lane, packet_type)
        }
    }
}
