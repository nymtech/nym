// Copyright 2025 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::cosmos_module::message_registry::MessageRegistry;
use crate::cosmos_module::CosmosModule;
use tracing::warn;

pub(crate) struct Group;

impl CosmosModule for Group {
    fn register_messages(&self, _registry: &mut MessageRegistry) {
        warn!("mising cosmos-sdk-proto definition for 'group::MsgCreateGroup'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgUpdateGroupMembers'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgUpdateGroupAdmin'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgUpdateGroupMetadata'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgCreateGroupWithPolicy'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgCreateGroupPolicy'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgUpdateGroupPolicyAdmin'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgUpdateGroupPolicyDecisionPolicy'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgUpdateGroupPolicyMetadata'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgSubmitProposal'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgWithdrawProposal'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgVote'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgExec'");
        warn!("mising cosmos-sdk-proto definition for 'group::MsgLeaveGroup'");
    }
}
