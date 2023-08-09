use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::ContractBuildInformation;
use nym_name_service_common::{
    msg::QueryMsg as NameQueryMsg,
    response::{ConfigResponse, NamesListResponse, PagedNamesListResponse},
    Address, NameId, RegisteredName,
};
use serde::Deserialize;

use crate::nyxd::{error::NyxdError, CosmWasmClient, NyxdClient};

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
impl<C> NameServiceQueryClient for NyxdClient<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn query_name_service_contract<T>(&self, query: NameQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.client
            .query_contract_smart(
                self.name_service_contract_address().ok_or(
                    NyxdError::NoContractAddressAvailable("name service contract".to_string()),
                )?,
                &query,
            )
            .await
    }
}

#[async_trait]
impl<C> NameServiceQueryClient for crate::Client<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn query_name_service_contract<T>(&self, query: NameQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.nyxd.query_name_service_contract(query).await
    }
}
