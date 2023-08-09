use cosmwasm_std::{Addr, Coin};
use nym_contracts_common::signing::{
    ContractMessageContent, MessageType, Nonce, SignableMessage, SigningPurpose,
};
use serde::Serialize;

use crate::NameDetails;

pub type SignableNameRegisterMsg = SignableMessage<ContractMessageContent<NameRegister>>;

#[derive(Serialize)]
pub struct NameRegister {
    name: NameDetails,
}

impl SigningPurpose for NameRegister {
    fn message_type() -> MessageType {
        MessageType::new("name-register")
    }
}

pub fn construct_name_register_sign_payload(
    nonce: Nonce,
    sender: Addr,
    deposit: Coin,
    name: NameDetails,
) -> SignableNameRegisterMsg {
    let payload = NameRegister { name };
    let proxy = None;
    let content = ContractMessageContent::new(sender, proxy, vec![deposit], payload);
    SignableMessage::new(nonce, content)
}
