// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::{error::NyxdError, CosmWasmClient};
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::ContractBuildInformation;
use nym_name_service_common::{
    msg::QueryMsg as NameQueryMsg,
    response::{ConfigResponse, NamesListResponse, PagedNamesListResponse},
    Address, NameId, RegisteredName,
};
use serde::Deserialize;

#[async_trait]
pub trait NameServiceQueryClient {
    async fn query_name_service_contract<T>(&self, query: NameQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_name_service_config(&self) -> Result<ConfigResponse, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::Config {})
            .await
    }

    async fn get_name_entry(&self, name_id: NameId) -> Result<RegisteredName, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::NameId { name_id })
            .await
    }

    async fn get_names_paged(
        &self,
        start_after: Option<NameId>,
        limit: Option<u32>,
    ) -> Result<PagedNamesListResponse, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::All { limit, start_after })
            .await
    }

    async fn get_names_by_owner(&self, owner: AccountId) -> Result<NamesListResponse, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::ByOwner {
            owner: owner.to_string(),
        })
        .await
    }

    async fn get_names_by_address(&self, address: Address) -> Result<NamesListResponse, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::ByAddress { address })
            .await
    }

    async fn get_name_service_contract_version(
        &self,
    ) -> Result<ContractBuildInformation, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::GetContractVersion {})
            .await
    }

    async fn get_all_names(&self) -> Result<Vec<RegisteredName>, NyxdError> {
        let mut services = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self.get_names_paged(start_after.take(), None).await?;

            let last_id = paged_response.names.last().map(|serv| serv.id);
            services.append(&mut paged_response.names);

            if let Some(start_after_res) = last_id {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(services)
    }
}
#[async_trait]
impl<C> NameServiceQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_name_service_contract<T>(&self, query: NameQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let name_service_contract_address = &self
            .name_service_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("name service contract"))?;
        self.query_contract_smart(name_service_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // it's enough that this compiles
    #[deprecated]
    async fn all_query_variants_are_covered<C: NameServiceQueryClient + Send + Sync>(
        client: C,
        msg: NameQueryMsg,
    ) {
        unimplemented!()
    }
}
