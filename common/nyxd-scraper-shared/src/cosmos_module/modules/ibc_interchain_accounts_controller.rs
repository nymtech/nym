// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::{CosmosModule, MessageRegistry};
use ibc_proto::ibc::applications::interchain_accounts::controller::v1::{
    MsgRegisterInterchainAccount, MsgSendTx, MsgUpdateParams,
};

pub(crate) struct IbcInterchainAccountsController;

impl CosmosModule for IbcInterchainAccountsController {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgRegisterInterchainAccount>();
        registry.register::<MsgSendTx>();
        registry.register::<MsgUpdateParams>();
    }
}
