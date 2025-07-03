// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::distribution::v1beta1::{
    MsgCommunityPoolSpend, MsgFundCommunityPool, MsgSetWithdrawAddress, MsgUpdateParams,
    MsgWithdrawDelegatorReward, MsgWithdrawValidatorCommission,
};

pub(crate) struct Distribution;

impl CosmosModule for Distribution {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgWithdrawDelegatorReward>();
        registry.register::<MsgWithdrawValidatorCommission>();
        registry.register::<MsgSetWithdrawAddress>();
        registry.register::<MsgFundCommunityPool>();
        registry.register::<MsgUpdateParams>();
        registry.register::<MsgCommunityPoolSpend>();
    }
}
