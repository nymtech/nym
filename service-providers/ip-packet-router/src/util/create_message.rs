use nym_sdk::mixnet::{AnonymousSenderTag, InputMessage, Recipient};
use nym_task::connections::TransmissionLane;

pub(crate) fn create_input_message(
    nym_address: Recipient,
    reply_to_tag: Option<AnonymousSenderTag>,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    if let Some(reply_to_tag) = reply_to_tag {
        log::debug!("Creating message using SURB");
        InputMessage::new_reply(reply_to_tag, response_packet, lane, packet_type)
    } else {
        log::debug!("Creating message using nym_address");
        InputMessage::new_regular(nym_address, response_packet, lane, packet_type)
    }
}
