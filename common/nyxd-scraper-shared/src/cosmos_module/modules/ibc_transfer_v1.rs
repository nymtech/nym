// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{CosmosModule, MessageRegistry};
use ibc_proto::ibc::applications::transfer::v1::{MsgTransfer, MsgUpdateParams};

pub(crate) struct IbcTransferV1;

impl CosmosModule for IbcTransferV1 {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgTransfer>();
        registry.register::<MsgUpdateParams>();
    }
}
