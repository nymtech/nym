// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::auth::v1beta1::MsgUpdateParams;

pub(crate) struct Auth;

impl CosmosModule for Auth {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgUpdateParams>()
    }
}
