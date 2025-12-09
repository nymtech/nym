// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{CosmosModule, MessageRegistry};
use ibc_proto::ibc::applications::fee::v1::{
    MsgPayPacketFee, MsgPayPacketFeeAsync, MsgRegisterPayee, RegisteredCounterpartyPayee,
};

pub(crate) struct IbcFee;

impl CosmosModule for IbcFee {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgRegisterPayee>();
        registry.register::<RegisteredCounterpartyPayee>();
        registry.register::<MsgPayPacketFee>();
        registry.register::<MsgPayPacketFeeAsync>();
    }
}
