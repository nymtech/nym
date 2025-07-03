// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use cosmos_sdk_proto::cosmos::gov::v1::{
    MsgDeposit, MsgExecLegacyContent, MsgSubmitProposal, MsgUpdateParams, MsgVote, MsgVoteWeighted,
};

pub(crate) struct GovV1;

impl CosmosModule for GovV1 {
    fn register_messages(&self, registry: &mut MessageRegistry) {
        registry.register::<MsgSubmitProposal>();
        registry.register::<MsgDeposit>();
        registry.register::<MsgVote>();
        registry.register::<MsgVoteWeighted>();
        registry.register::<MsgExecLegacyContent>();
        registry.register::<MsgUpdateParams>();
    }
}
