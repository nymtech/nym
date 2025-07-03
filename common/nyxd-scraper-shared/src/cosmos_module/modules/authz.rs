// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::authz::v1beta1::{MsgExec, MsgGrant, MsgRevoke};

pub(crate) struct Authz;

impl CosmosModule for Authz {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgGrant>();
        registry.register::<MsgExec>();
        registry.register::<MsgRevoke>();
    }
}
