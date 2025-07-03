// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::staking::v1beta1::{
    MsgBeginRedelegate, MsgCancelUnbondingDelegation, MsgCreateValidator, MsgDelegate,
    MsgEditValidator, MsgUndelegate, MsgUpdateParams,
};

pub(crate) struct Staking;

impl CosmosModule for Staking {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgCreateValidator>();
        registry.register::<MsgEditValidator>();
        registry.register::<MsgDelegate>();
        registry.register::<MsgUndelegate>();
        registry.register::<MsgBeginRedelegate>();
        registry.register::<MsgCancelUnbondingDelegation>();
        registry.register::<MsgUpdateParams>();
    }
}
