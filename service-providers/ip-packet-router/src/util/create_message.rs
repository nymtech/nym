use nym_sdk::mixnet::InputMessage;
use nym_task::connections::TransmissionLane;

use crate::clients::ConnectedClientId;

pub(crate) fn create_input_message(
    recipient: &ConnectedClientId,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    let disable_retransmissions = false;
    match recipient {
        ConnectedClientId::NymAddress(recipient) => InputMessage::new_regular(
            **recipient,
            response_packet,
            lane,
            packet_type,
            disable_retransmissions,
        ),
        ConnectedClientId::AnonymousSenderTag(tag) => InputMessage::new_reply(
            *tag,
            response_packet,
            lane,
            packet_type,
            disable_retransmissions,
        ),
    }
}
