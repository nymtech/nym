// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use cw4::{Member, MemberListResponse, MemberResponse, TotalWeightResponse};
use nym_group_contract_common::msg::QueryMsg as GroupQueryMsg;
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait GroupQueryClient {
    async fn query_group_contract<T>(&self, query: GroupQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn admin(&self) -> Result<cw_controllers::AdminResponse, NyxdError> {
        self.query_group_contract(GroupQueryMsg::Admin {}).await
    }

    async fn total_weight(&self, at_height: Option<u64>) -> Result<TotalWeightResponse, NyxdError> {
        self.query_group_contract(GroupQueryMsg::TotalWeight { at_height })
            .await
    }

    async fn list_members_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<MemberListResponse, NyxdError> {
        self.query_group_contract(GroupQueryMsg::ListMembers { start_after, limit })
            .await
    }

    async fn member(
        &self,
        addr: String,
        at_height: Option<u64>,
    ) -> Result<MemberResponse, NyxdError> {
        self.query_group_contract(GroupQueryMsg::Member { addr, at_height })
            .await
    }

    async fn hooks(&self) -> Result<cw_controllers::HooksResponse, NyxdError> {
        self.query_group_contract(GroupQueryMsg::Hooks {}).await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedGroupQueryClient: GroupQueryClient {
    // can't use the macro due to different paging behaviour
    async fn get_all_members(&self) -> Result<Vec<Member>, NyxdError> {
        let mut members = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self.list_members_paged(start_after.take(), None).await?;

            let last_id = paged_response.members.last().map(|mem| mem.addr.clone());
            members.append(&mut paged_response.members);

            if let Some(start_after_res) = last_id {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(members)
    }
}

#[async_trait]
impl<T> PagedGroupQueryClient for T where T: GroupQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> GroupQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_group_contract<T>(&self, query: GroupQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let group_contract_address = &self
            .group_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("group contract"))?;
        self.query_contract_smart(group_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: GroupQueryClient + Send + Sync>(
        client: C,
        msg: GroupQueryMsg,
    ) {
        match msg {
            GroupQueryMsg::Admin {} => client.admin().ignore(),
            GroupQueryMsg::TotalWeight { at_height } => client.total_weight(at_height).ignore(),
            GroupQueryMsg::ListMembers { start_after, limit } => {
                client.list_members_paged(start_after, limit).ignore()
            }
            GroupQueryMsg::Member { addr, at_height } => client.member(addr, at_height).ignore(),
            GroupQueryMsg::Hooks {} => client.hooks().ignore(),
        };
    }
}
