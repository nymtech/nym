// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::crisis::v1beta1::{MsgUpdateParams, MsgVerifyInvariant};

pub(crate) struct Crisis;

impl CosmosModule for Crisis {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgVerifyInvariant>();
        registry.register::<MsgUpdateParams>();
    }
}
