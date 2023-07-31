// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};
use async_trait::async_trait;
use cw4::MemberResponse;
use nym_group_contract_common::msg::QueryMsg as GroupQueryMsg;
use serde::Deserialize;

#[async_trait]
pub trait GroupQueryClient {
    async fn query_group_contract<T>(&self, query: GroupQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn member(
        &self,
        addr: String,
        at_height: Option<u64>,
    ) -> Result<MemberResponse, NyxdError> {
        self.query_group_contract(GroupQueryMsg::Member { addr, at_height })
            .await
    }
}

#[async_trait]
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

    // it's enough that this compiles
    #[deprecated]
    async fn all_query_variants_are_covered<C: GroupQueryClient + Send + Sync>(
        client: C,
        msg: GroupQueryMsg,
    ) {
        unimplemented!()
    }
}
