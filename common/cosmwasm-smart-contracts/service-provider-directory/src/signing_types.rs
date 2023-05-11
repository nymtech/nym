use cosmwasm_std::{Addr, Coin};
use nym_contracts_common::signing::{
    ContractMessageContent, MessageType, Nonce, SignableMessage, SigningPurpose,
};
use serde::Serialize;

use crate::ServiceDetails;

pub type SignableServiceProviderAnnounceMsg =
    SignableMessage<ContractMessageContent<ServiceProviderAnnounce>>;

#[derive(Serialize)]
pub struct ServiceProviderAnnounce {
    service: ServiceDetails,
}

impl SigningPurpose for ServiceProviderAnnounce {
    fn message_type() -> MessageType {
        MessageType::new("service-provider-announce")
    }
}

pub fn construct_service_provider_announce_sign_payload(
    nonce: Nonce,
    sender: Addr,
    pledge: Coin,
    service: ServiceDetails,
) -> SignableServiceProviderAnnounceMsg {
    let payload = ServiceProviderAnnounce { service };
    let proxy = None;
    let content = ContractMessageContent::new(sender, proxy, vec![pledge], payload);
    SignableMessage::new(nonce, content)
}
