// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{Gateway, MixNode, MixNodeCostParams};
use contracts_common::signing::{MessageType, SigningPurpose};
use serde::Serialize;

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

#[derive(Serialize)]
pub struct FamilyCreationSignature {
    label: String,
    // TODO: add any extra fields?
}

impl FamilyCreationSignature {
    pub fn new(label: String) -> Self {
        Self { label }
    }
}

impl SigningPurpose for FamilyCreationSignature {
    fn message_type() -> MessageType {
        MessageType::new("family-creation")
    }
}

#[derive(Serialize)]
pub struct FamilyJoinSignature {
    family_head: String,
    // TODO: add any extra fields?
}

impl FamilyJoinSignature {
    pub fn new(family_head: String) -> Self {
        Self { family_head }
    }
}

impl SigningPurpose for FamilyJoinSignature {
    fn message_type() -> MessageType {
        MessageType::new("family-join")
    }
}

#[derive(Serialize)]
pub struct FamilyLeaveSignature {
    family_head: String,
    // TODO: add any extra fields?
}

impl FamilyLeaveSignature {
    pub fn new(family_head: String) -> Self {
        Self { family_head }
    }
}

impl SigningPurpose for FamilyLeaveSignature {
    fn message_type() -> MessageType {
        MessageType::new("family-leave")
    }
}

#[derive(Serialize)]
pub struct FamilyKickSignature {
    member: String,
    // TODO: add any extra fields?
}

impl FamilyKickSignature {
    pub fn new(member: String) -> Self {
        Self { member }
    }
}

impl SigningPurpose for FamilyKickSignature {
    fn message_type() -> MessageType {
        MessageType::new("family-member-removal")
    }
}

// TODO: depending on our threat model, we should perhaps extend it to include all _on_behalf methods
