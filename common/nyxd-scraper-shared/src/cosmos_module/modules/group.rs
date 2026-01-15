// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::CosmosModule;
use crate::cosmos_module::message_registry::MessageRegistry;
use tracing::warn;

pub(crate) struct Group;

impl CosmosModule for Group {
    fn register_messages(&self, _registry: &mut MessageRegistry) {
        warn!("missing cosmos-sdk-proto definition for 'group::MsgCreateGroup'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgUpdateGroupMembers'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgUpdateGroupAdmin'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgUpdateGroupMetadata'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgCreateGroupWithPolicy'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgCreateGroupPolicy'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgUpdateGroupPolicyAdmin'");
        warn!(
            "missing cosmos-sdk-proto definition for 'group::MsgUpdateGroupPolicyDecisionPolicy'"
        );
        warn!("missing cosmos-sdk-proto definition for 'group::MsgUpdateGroupPolicyMetadata'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgSubmitProposal'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgWithdrawProposal'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgVote'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgExec'");
        warn!("missing cosmos-sdk-proto definition for 'group::MsgLeaveGroup'");
    }
}
