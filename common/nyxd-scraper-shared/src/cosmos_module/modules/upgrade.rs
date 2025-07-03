// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::upgrade::v1beta1::{MsgCancelUpgrade, MsgSoftwareUpgrade};

pub(crate) struct Upgrade;

impl CosmosModule for Upgrade {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgSoftwareUpgrade>();
        registry.register::<MsgCancelUpgrade>();
    }
}
