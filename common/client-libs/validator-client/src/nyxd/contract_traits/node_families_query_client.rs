// Copyright 2026 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cosmrs::AccountId;
use serde::Deserialize;

pub use node_families_contract_common::{
    msg::QueryMsg as NodeFamiliesQueryMsg, AllPastFamilyInvitationsPagedResponse,
    FamiliesPagedResponse, FamilyMemberRecord, FamilyMembersPagedResponse,
    GlobalPastFamilyInvitationCursor, NodeFamily, NodeFamilyByNameResponse,
    NodeFamilyByOwnerResponse, NodeFamilyId, NodeFamilyMembershipResponse, NodeFamilyResponse,
    PastFamilyInvitation, PastFamilyInvitationCursor, PastFamilyInvitationForNodeCursor,
    PastFamilyInvitationsForNodePagedResponse, PastFamilyInvitationsPagedResponse,
    PastFamilyMember, PastFamilyMemberCursor, PastFamilyMemberForNodeCursor,
    PastFamilyMembersForNodePagedResponse, PastFamilyMembersPagedResponse,
    PendingFamilyInvitationDetails, PendingFamilyInvitationResponse,
    PendingFamilyInvitationsPagedResponse, PendingInvitationsForNodePagedResponse,
    PendingInvitationsPagedResponse,
};
use nym_mixnet_contract_common::NodeId;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NodeFamiliesQueryClient {
    async fn query_node_families_contract<T>(
        &self,
        query: NodeFamiliesQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_family_by_id(
        &self,
        family_id: NodeFamilyId,
    ) -> Result<NodeFamilyResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetFamilyById { family_id })
            .await
    }

    async fn get_family_by_owner(
        &self,
        owner: &AccountId,
    ) -> Result<NodeFamilyByOwnerResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetFamilyByOwner {
            owner: owner.to_string(),
        })
        .await
    }

    async fn get_family_by_name(
        &self,
        name: String,
    ) -> Result<NodeFamilyByNameResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetFamilyByName { name })
            .await
    }

    async fn get_families_paged(
        &self,
        start_after: Option<NodeFamilyId>,
        limit: Option<u32>,
    ) -> Result<FamiliesPagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetFamiliesPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_family_membership(
        &self,
        node_id: NodeId,
    ) -> Result<NodeFamilyMembershipResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetFamilyMembership { node_id })
            .await
    }

    async fn get_family_members_paged(
        &self,
        family_id: NodeFamilyId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    ) -> Result<FamilyMembersPagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetFamilyMembersPaged {
            family_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_pending_invitation(
        &self,
        family_id: NodeFamilyId,
        node_id: NodeId,
    ) -> Result<PendingFamilyInvitationResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetPendingInvitation {
            family_id,
            node_id,
        })
        .await
    }

    async fn get_pending_invitations_for_family_paged(
        &self,
        family_id: NodeFamilyId,
        start_after: Option<NodeId>,
        limit: Option<u32>,
    ) -> Result<PendingFamilyInvitationsPagedResponse, NyxdError> {
        self.query_node_families_contract(
            NodeFamiliesQueryMsg::GetPendingInvitationsForFamilyPaged {
                family_id,
                start_after,
                limit,
            },
        )
        .await
    }

    async fn get_pending_invitations_for_node_paged(
        &self,
        node_id: NodeId,
        start_after: Option<NodeFamilyId>,
        limit: Option<u32>,
    ) -> Result<PendingInvitationsForNodePagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetPendingInvitationsForNodePaged {
            node_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_all_pending_invitations_paged(
        &self,
        start_after: Option<(NodeFamilyId, NodeId)>,
        limit: Option<u32>,
    ) -> Result<PendingInvitationsPagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetAllPendingInvitationsPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_past_invitations_for_family_paged(
        &self,
        family_id: NodeFamilyId,
        start_after: Option<PastFamilyInvitationCursor>,
        limit: Option<u32>,
    ) -> Result<PastFamilyInvitationsPagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetPastInvitationsForFamilyPaged {
            family_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_past_invitations_for_node_paged(
        &self,
        node_id: NodeId,
        start_after: Option<PastFamilyInvitationForNodeCursor>,
        limit: Option<u32>,
    ) -> Result<PastFamilyInvitationsForNodePagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetPastInvitationsForNodePaged {
            node_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_all_past_invitations_paged(
        &self,
        start_after: Option<GlobalPastFamilyInvitationCursor>,
        limit: Option<u32>,
    ) -> Result<AllPastFamilyInvitationsPagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetAllPastInvitationsPaged {
            start_after,
            limit,
        })
        .await
    }

    async fn get_past_members_for_family_paged(
        &self,
        family_id: NodeFamilyId,
        start_after: Option<PastFamilyMemberCursor>,
        limit: Option<u32>,
    ) -> Result<PastFamilyMembersPagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetPastMembersForFamilyPaged {
            family_id,
            start_after,
            limit,
        })
        .await
    }

    async fn get_past_members_for_node_paged(
        &self,
        node_id: NodeId,
        start_after: Option<PastFamilyMemberForNodeCursor>,
        limit: Option<u32>,
    ) -> Result<PastFamilyMembersForNodePagedResponse, NyxdError> {
        self.query_node_families_contract(NodeFamiliesQueryMsg::GetPastMembersForNodePaged {
            node_id,
            start_after,
            limit,
        })
        .await
    }
}

// extension trait to the query client to deal with the paged queries
// (it didn't feel appropriate to combine it with the existing trait)
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedNodeFamiliesQueryClient: NodeFamiliesQueryClient {
    async fn get_all_families(&self) -> Result<Vec<NodeFamily>, NyxdError> {
        collect_paged!(self, get_families_paged, families)
    }

    async fn get_all_family_members(
        &self,
        family_id: NodeFamilyId,
    ) -> Result<Vec<FamilyMemberRecord>, NyxdError> {
        collect_paged!(self, get_family_members_paged, members, family_id)
    }

    async fn get_all_pending_invitations_for_family(
        &self,
        family_id: NodeFamilyId,
    ) -> Result<Vec<PendingFamilyInvitationDetails>, NyxdError> {
        collect_paged!(
            self,
            get_pending_invitations_for_family_paged,
            invitations,
            family_id
        )
    }

    async fn get_all_pending_invitations_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<PendingFamilyInvitationDetails>, NyxdError> {
        collect_paged!(
            self,
            get_pending_invitations_for_node_paged,
            invitations,
            node_id
        )
    }

    async fn get_all_pending_invitations(
        &self,
    ) -> Result<Vec<PendingFamilyInvitationDetails>, NyxdError> {
        collect_paged!(self, get_all_pending_invitations_paged, invitations)
    }

    async fn get_all_past_invitations_for_family(
        &self,
        family_id: NodeFamilyId,
    ) -> Result<Vec<PastFamilyInvitation>, NyxdError> {
        collect_paged!(
            self,
            get_past_invitations_for_family_paged,
            invitations,
            family_id
        )
    }

    async fn get_all_past_invitations_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<PastFamilyInvitation>, NyxdError> {
        collect_paged!(
            self,
            get_past_invitations_for_node_paged,
            invitations,
            node_id
        )
    }

    async fn get_all_past_invitations(&self) -> Result<Vec<PastFamilyInvitation>, NyxdError> {
        collect_paged!(self, get_all_past_invitations_paged, invitations)
    }

    async fn get_all_past_members_for_family(
        &self,
        family_id: NodeFamilyId,
    ) -> Result<Vec<PastFamilyMember>, NyxdError> {
        collect_paged!(self, get_past_members_for_family_paged, members, family_id)
    }

    async fn get_all_past_members_for_node(
        &self,
        node_id: NodeId,
    ) -> Result<Vec<PastFamilyMember>, NyxdError> {
        collect_paged!(self, get_past_members_for_node_paged, members, node_id)
    }
}

#[async_trait]
impl<T> PagedNodeFamiliesQueryClient for T where T: NodeFamiliesQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> NodeFamiliesQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_node_families_contract<T>(
        &self,
        query: NodeFamiliesQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let node_families_contract_address = &self
            .node_families_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("node families contract"))?;
        self.query_contract_smart(node_families_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;
    use node_families_contract_common::QueryMsg;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: NodeFamiliesQueryClient + Send + Sync>(
        client: C,
        msg: NodeFamiliesQueryMsg,
    ) {
        match msg {
            NodeFamiliesQueryMsg::GetFamilyById { family_id } => {
                client.get_family_by_id(family_id).ignore()
            }
            NodeFamiliesQueryMsg::GetFamilyByOwner { owner } => {
                client.get_family_by_owner(&owner.parse().unwrap()).ignore()
            }
            NodeFamiliesQueryMsg::GetFamilyByName { name } => {
                client.get_family_by_name(name).ignore()
            }
            NodeFamiliesQueryMsg::GetFamiliesPaged { start_after, limit } => {
                client.get_families_paged(start_after, limit).ignore()
            }
            NodeFamiliesQueryMsg::GetFamilyMembership { node_id } => {
                client.get_family_membership(node_id).ignore()
            }
            NodeFamiliesQueryMsg::GetFamilyMembersPaged {
                family_id,
                start_after,
                limit,
            } => client
                .get_family_members_paged(family_id, start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetPendingInvitation { family_id, node_id } => {
                client.get_pending_invitation(family_id, node_id).ignore()
            }
            NodeFamiliesQueryMsg::GetPendingInvitationsForFamilyPaged {
                family_id,
                start_after,
                limit,
            } => client
                .get_pending_invitations_for_family_paged(family_id, start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetPendingInvitationsForNodePaged {
                node_id,
                start_after,
                limit,
            } => client
                .get_pending_invitations_for_node_paged(node_id, start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetAllPendingInvitationsPaged { start_after, limit } => client
                .get_all_pending_invitations_paged(start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetPastInvitationsForFamilyPaged {
                family_id,
                start_after,
                limit,
            } => client
                .get_past_invitations_for_family_paged(family_id, start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetPastInvitationsForNodePaged {
                node_id,
                start_after,
                limit,
            } => client
                .get_past_invitations_for_node_paged(node_id, start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetAllPastInvitationsPaged { start_after, limit } => client
                .get_all_past_invitations_paged(start_after, limit)
                .ignore(),
            NodeFamiliesQueryMsg::GetPastMembersForFamilyPaged {
                family_id,
                start_after,
                limit,
            } => client
                .get_past_members_for_family_paged(family_id, start_after, limit)
                .ignore(),
            QueryMsg::GetPastMembersForNodePaged {
                node_id,
                start_after,
                limit,
            } => client
                .get_past_members_for_node_paged(node_id, start_after, limit)
                .ignore(),
        };
    }
}
