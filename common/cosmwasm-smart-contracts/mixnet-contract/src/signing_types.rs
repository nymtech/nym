// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::families::FamilyHead;
use crate::{Gateway, IdentityKey, MixNode, MixNodeCostParams};
use contracts_common::signing::{
    ContractMessageContent, MessageType, Nonce, SignableMessage, SigningPurpose,
};
use cosmwasm_std::{Addr, Coin};
use serde::Serialize;

pub type SignableMixNodeBondingMsg = SignableMessage<ContractMessageContent<MixnodeBondingPayload>>;
pub type SignableGatewayBondingMsg = SignableMessage<ContractMessageContent<GatewayBondingPayload>>;
pub type SignableFamilyJoinPermitMsg = SignableMessage<FamilyJoinPermit>;

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
    pledge: Coin,
    mix_node: MixNode,
    cost_params: MixNodeCostParams,
) -> SignableMixNodeBondingMsg {
    let payload = MixnodeBondingPayload::new(mix_node, cost_params);
    let content = ContractMessageContent::new(sender, vec![pledge], payload);

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
    pledge: Coin,
    gateway: Gateway,
) -> SignableGatewayBondingMsg {
    let payload = GatewayBondingPayload::new(gateway);
    let content = ContractMessageContent::new(sender, vec![pledge], payload);

    SignableMessage::new(nonce, content)
}

#[derive(Serialize)]
pub struct FamilyJoinPermit {
    // the granter of this permit
    family_head: FamilyHead,
    // the actual member we want to permit to join
    member_node: IdentityKey,
}

impl FamilyJoinPermit {
    pub fn new(family_head: FamilyHead, member_node: IdentityKey) -> Self {
        Self {
            family_head,
            member_node,
        }
    }
}

impl SigningPurpose for FamilyJoinPermit {
    fn message_type() -> MessageType {
        MessageType::new("family-join-permit")
    }
}

pub fn construct_family_join_permit(
    nonce: Nonce,
    family_head: FamilyHead,
    member_node: IdentityKey,
) -> SignableFamilyJoinPermitMsg {
    let payload = FamilyJoinPermit::new(family_head, member_node);

    // note: we're NOT wrapping it in `ContractMessageContent` because the family head is not going to be the one
    // sending the message to the contract
    SignableMessage::new(nonce, payload)
}

// TODO: depending on our threat model, we should perhaps extend it to include all _on_behalf methods
// (update: but we trust our vesting contract since its compromise would be even more devastating so there's no need)
