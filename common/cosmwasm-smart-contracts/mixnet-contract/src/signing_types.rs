// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nym_node::NymNode;
use crate::{Gateway, MixNode, NodeCostParams};
use contracts_common::signing::{
    ContractMessageContent, LegacyContractMessageContent, MessageType, Nonce, SignableMessage,
    SigningPurpose,
};
use cosmwasm_std::{Addr, Coin};
use serde::Serialize;

pub type SignableMixNodeBondingMsg = SignableMessage<ContractMessageContent<MixnodeBondingPayload>>;
pub type SignableGatewayBondingMsg = SignableMessage<ContractMessageContent<GatewayBondingPayload>>;
pub type SignableNymNodeBondingMsg = SignableMessage<ContractMessageContent<NymNodeBondingPayload>>;
pub type SignableLegacyMixNodeBondingMsg =
    SignableMessage<LegacyContractMessageContent<MixnodeBondingPayload>>;
pub type SignableLegacyGatewayBondingMsg =
    SignableMessage<LegacyContractMessageContent<GatewayBondingPayload>>;

#[derive(Serialize)]
pub struct MixnodeBondingPayload {
    mix_node: MixNode,
    cost_params: NodeCostParams,
}

impl MixnodeBondingPayload {
    pub fn new(mix_node: MixNode, cost_params: NodeCostParams) -> Self {
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
    cost_params: NodeCostParams,
) -> SignableMixNodeBondingMsg {
    let payload = MixnodeBondingPayload::new(mix_node, cost_params);
    let content = ContractMessageContent::new(sender, vec![pledge], payload);

    SignableMessage::new(nonce, content)
}

pub fn construct_legacy_mixnode_bonding_sign_payload(
    nonce: Nonce,
    sender: Addr,
    pledge: Coin,
    mix_node: MixNode,
    cost_params: NodeCostParams,
) -> SignableLegacyMixNodeBondingMsg {
    let payload = MixnodeBondingPayload::new(mix_node, cost_params);
    let content: LegacyContractMessageContent<_> =
        ContractMessageContent::new(sender, vec![pledge], payload).into();

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

pub fn construct_legacy_gateway_bonding_sign_payload(
    nonce: Nonce,
    sender: Addr,
    pledge: Coin,
    gateway: Gateway,
) -> SignableLegacyGatewayBondingMsg {
    let payload = GatewayBondingPayload::new(gateway);
    let content: LegacyContractMessageContent<_> =
        ContractMessageContent::new(sender, vec![pledge], payload).into();

    SignableMessage::new(nonce, content)
}

#[derive(Serialize)]
pub struct NymNodeBondingPayload {
    nym_node: NymNode,
    cost_params: NodeCostParams,
}

impl NymNodeBondingPayload {
    pub fn new(nym_node: NymNode, cost_params: NodeCostParams) -> Self {
        NymNodeBondingPayload {
            nym_node,
            cost_params,
        }
    }
}

impl SigningPurpose for NymNodeBondingPayload {
    fn message_type() -> MessageType {
        MessageType::new("nym-node-bonding")
    }
}

pub fn construct_nym_node_bonding_sign_payload(
    nonce: Nonce,
    sender: Addr,
    pledge: Coin,
    nym_node: NymNode,
    cost_params: NodeCostParams,
) -> SignableNymNodeBondingMsg {
    let payload = NymNodeBondingPayload::new(nym_node, cost_params);
    let content = ContractMessageContent::new(sender, vec![pledge], payload);

    SignableMessage::new(nonce, content)
}

pub fn construct_generic_node_bonding_payload<T>(
    nonce: Nonce,
    sender: Addr,
    pledge: Coin,
    payload: T,
) -> SignableMessage<ContractMessageContent<T>>
where
    T: SigningPurpose,
{
    let content = ContractMessageContent::new(sender, vec![pledge], payload);
    SignableMessage::new(nonce, content)
}
