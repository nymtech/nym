// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::ServiceDetails;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use nym_contracts_common::signing::{
    ContractMessageContent, MessageType, Nonce, SignableMessage, SigningPurpose,
};

pub type SignableServiceProviderAnnounceMsg =
    SignableMessage<ContractMessageContent<ServiceProviderAnnounce>>;

#[cw_serde]
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
    deposit: Coin,
    service: ServiceDetails,
) -> SignableServiceProviderAnnounceMsg {
    let payload = ServiceProviderAnnounce { service };
    let proxy = None;
    let content = ContractMessageContent::new(sender, proxy, vec![deposit], payload);
    SignableMessage::new(nonce, content)
}
