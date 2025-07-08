// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::feegrant::v1beta1::{
    MsgGrantAllowance, MsgPruneAllowances, MsgRevokeAllowance,
};

pub(crate) struct Feegrant;

impl CosmosModule for Feegrant {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgGrantAllowance>();
        registry.register::<MsgRevokeAllowance>();
        registry.register::<MsgPruneAllowances>();
    }
}
