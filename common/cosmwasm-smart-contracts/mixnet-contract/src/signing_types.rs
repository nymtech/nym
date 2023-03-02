// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Gateway, MixNode, MixNodeCostParams};
use contracts_common::signing::{
    ContractMessageContent, MessageType, Nonce, SignableMessage, SigningPurpose,
};
use cosmwasm_std::{Addr, Coin};
use serde::Serialize;

pub type SignableMixNodeBondingMsg = SignableMessage<ContractMessageContent<MixnodeBondingPayload>>;
pub type SignableGatewayBondingMsg = SignableMessage<ContractMessageContent<GatewayBondingPayload>>;
pub type SignableFamilyCreationMsg = SignableMessage<ContractMessageContent<FamilyCreationPayload>>;
pub type SignableFamilyJoinMsg = SignableMessage<ContractMessageContent<FamilyJoinPayload>>;
pub type SignableFamilyLeaveMsg = SignableMessage<ContractMessageContent<FamilyLeavePayload>>;
pub type SignableFamilyKickMsg = SignableMessage<ContractMessageContent<FamilyKickPayload>>;

#[derive(Serialize)]
pub struct MixnodeBondingPayload {
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
}

impl MixnodeBondingPayload {
    pub fn new(mix_node: MixNode, cost_params: MixNodeCostParams) -> Self {
        Self {
            mix_node,
            cost_params,
        }
    }
}

impl SigningPurpose for MixnodeBondingPayload {
    fn message_type() -> MessageType {
        MessageType::new("mixnode-bonding")
    }
}

pub fn construct_mixnode_bonding_sign_payload(
    nonce: Nonce,
    sender: Addr,
    proxy: Option<Addr>,
    pledge: Coin,
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
) -> SignableMixNodeBondingMsg {
    let payload = MixnodeBondingPayload::new(mix_node, cost_params);
    let content = ContractMessageContent::new(sender, proxy, vec![pledge], payload);

    SignableMessage::new(nonce, content)
}

#[derive(Serialize)]
pub struct GatewayBondingPayload {
    gateway: Gateway,
}

impl GatewayBondingPayload {
    pub fn new(gateway: Gateway) -> Self {
        Self { gateway }
    }
}

impl SigningPurpose for GatewayBondingPayload {
    fn message_type() -> MessageType {
        MessageType::new("gateway-bonding")
    }
}

pub fn construct_gateway_bonding_sign_payload(
    nonce: Nonce,
    sender: Addr,
    proxy: Option<Addr>,
    pledge: Coin,
    gateway: Gateway,
) -> SignableGatewayBondingMsg {
    let payload = GatewayBondingPayload::new(gateway);
    let content = ContractMessageContent::new(sender, proxy, vec![pledge], payload);

    SignableMessage::new(nonce, content)
}

#[derive(Serialize)]
pub struct FamilyCreationPayload {
    label: String,
    // TODO: add any extra fields?
}

impl FamilyCreationPayload {
    pub fn new(label: String) -> Self {
        Self { label }
    }
}

impl SigningPurpose for FamilyCreationPayload {
    fn message_type() -> MessageType {
        MessageType::new("family-creation")
    }
}

pub fn construct_family_creation_sign_payload(
    nonce: Nonce,
    sender: Addr,
    proxy: Option<Addr>,
    label: String,
) -> SignableFamilyCreationMsg {
    let payload = FamilyCreationPayload::new(label);
    let content = ContractMessageContent::new(sender, proxy, Vec::new(), payload);

    SignableMessage::new(nonce, content)
}

#[derive(Serialize)]
pub struct FamilyJoinPayload {
    family_head: String,
    // TODO: add any extra fields?
}

impl FamilyJoinPayload {
    pub fn new(family_head: String) -> Self {
        Self { family_head }
    }
}

impl SigningPurpose for FamilyJoinPayload {
    fn message_type() -> MessageType {
        MessageType::new("family-join")
    }
}

#[derive(Serialize)]
pub struct FamilyLeavePayload {
    family_head: String,
    // TODO: add any extra fields?
}

impl FamilyLeavePayload {
    pub fn new(family_head: String) -> Self {
        Self { family_head }
    }
}

impl SigningPurpose for FamilyLeavePayload {
    fn message_type() -> MessageType {
        MessageType::new("family-leave")
    }
}

#[derive(Serialize)]
pub struct FamilyKickPayload {
    member: String,
    // TODO: add any extra fields?
}

impl FamilyKickPayload {
    pub fn new(member: String) -> Self {
        Self { member }
    }
}

impl SigningPurpose for FamilyKickPayload {
    fn message_type() -> MessageType {
        MessageType::new("family-member-removal")
    }
}

// TODO: depending on our threat model, we should perhaps extend it to include all _on_behalf methods
