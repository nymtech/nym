use crate::service_providers::ip_packet_router::clients::ConnectedClientId;
use nym_sdk::mixnet::InputMessage;
use nym_task::connections::TransmissionLane;

pub(crate) fn create_input_message(
    recipient: &ConnectedClientId,
    response_packet: Vec<u8>,
) -> InputMessage {
    let lane = TransmissionLane::General;
    let packet_type = None;
    match recipient {
        ConnectedClientId::NymAddress(recipient) => {
            InputMessage::new_regular(**recipient, response_packet, lane, packet_type)
        }
        ConnectedClientId::AnonymousSenderTag(tag) => {
            InputMessage::new_reply(*tag, response_packet, lane, packet_type)
        }
    }
}
