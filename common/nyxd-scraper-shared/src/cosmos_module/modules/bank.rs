// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::bank::v1beta1::{
    MsgMultiSend, MsgSend, MsgSetSendEnabled, MsgUpdateParams,
};

pub(crate) struct Bank;

impl CosmosModule for Bank {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgSend>();
        registry.register::<MsgMultiSend>();
        registry.register::<MsgUpdateParams>();
        registry.register::<MsgSetSendEnabled>();
    }
}
