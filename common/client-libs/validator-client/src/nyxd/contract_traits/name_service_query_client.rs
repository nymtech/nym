// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::{error::NyxdError, CosmWasmClient};
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::{signing::Nonce, ContractBuildInformation};
use nym_name_service_common::{
    msg::QueryMsg as NameQueryMsg,
    response::{ConfigResponse, NamesListResponse, PagedNamesListResponse},
    Address, NameId, NymName, RegisteredName,
};
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

    async fn get_name_signing_nonce(&self, address: &AccountId) -> Result<Nonce, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::SigningNonce {
            address: address.to_string(),
        })
        .await
    }

    async fn get_names_by_owner(&self, owner: AccountId) -> Result<NamesListResponse, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::ByOwner {
            owner: owner.to_string(),
        })
        .await
    }

    async fn get_names_by_nym_name(&self, name: NymName) -> Result<NamesListResponse, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::ByName { name })
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

    async fn get_name_service_contract_cw2_version(
        &self,
    ) -> Result<cw2::ContractVersion, NyxdError> {
        self.query_name_service_contract(NameQueryMsg::GetCW2ContractVersion {})
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedNameServiceQueryClient: NameServiceQueryClient {
    async fn get_all_names(&self) -> Result<Vec<RegisteredName>, NyxdError> {
        collect_paged!(self, get_names_paged, names)
    }
}

#[async_trait]
impl<T> PagedNameServiceQueryClient for T where T: NameServiceQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: NameServiceQueryClient + Send + Sync>(
        client: C,
        msg: NameQueryMsg,
    ) {
        match msg {
            NameQueryMsg::NameId { name_id } => client.get_name_entry(name_id).ignore(),
            NameQueryMsg::ByOwner { owner } => {
                client.get_names_by_owner(owner.parse().unwrap()).ignore()
            }
            NameQueryMsg::ByName { name } => client.get_names_by_nym_name(name).ignore(),
            NameQueryMsg::ByAddress { address } => client.get_names_by_address(address).ignore(),
            NameQueryMsg::All { limit, start_after } => {
                client.get_names_paged(limit, start_after).ignore()
            }
            NameQueryMsg::SigningNonce { address } => client
                .get_name_signing_nonce(&address.parse().unwrap())
                .ignore(),
            NameQueryMsg::Config {} => client.get_name_service_config().ignore(),
            NameQueryMsg::GetContractVersion {} => {
                client.get_name_service_contract_version().ignore()
            }
            NameQueryMsg::GetCW2ContractVersion {} => {
                client.get_name_service_contract_cw2_version().ignore()
            }
        };
    }
}
