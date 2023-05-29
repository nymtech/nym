use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::ContractBuildInformation;
use nym_service_provider_directory_common::{
    msg::QueryMsg as SpQueryMsg,
    response::{
        ConfigResponse, PagedServicesListResponse, ServiceInfoResponse, ServicesListResponse,
    },
    NymAddress, ServiceId, Service,
};
use serde::Deserialize;

use crate::nyxd::{error::NyxdError, CosmWasmClient, NyxdClient};

#[async_trait]
pub trait SpDirectoryQueryClient {
    async fn query_service_provider_contract<T>(&self, query: SpQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_service_config(&self) -> Result<ConfigResponse, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::Config {})
            .await
    }

    async fn get_service_info(
        &self,
        service_id: ServiceId,
    ) -> Result<ServiceInfoResponse, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::ServiceId { service_id })
            .await
    }

    async fn get_services_paged(
        &self,
        start_after: Option<ServiceId>,
        limit: Option<u32>,
    ) -> Result<PagedServicesListResponse, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::All { limit, start_after })
            .await
    }

    async fn get_services_by_announcer(
        &self,
        announcer: AccountId,
    ) -> Result<ServicesListResponse, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::ByAnnouncer {
            announcer: announcer.to_string(),
        })
        .await
    }

    async fn get_services_by_nym_address(
        &self,
        nym_address: NymAddress,
    ) -> Result<ServicesListResponse, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::ByNymAddress { nym_address })
            .await
    }

    async fn get_sp_contract_version(&self) -> Result<ContractBuildInformation, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::GetContractVersion {})
            .await
    }

    async fn get_all_services(&self) -> Result<Vec<Service>, NyxdError> {
        let mut services = Vec::new();
        let mut start_after = None;
        loop {
            let mut paged_response = self.get_services_paged(start_after.take(), None).await?;
            services.append(&mut paged_response.services);

            if let Some(start_after_res) = paged_response.start_next_after {
                start_after = Some(start_after_res)
            } else {
                break;
            }
        }

        Ok(services)
    }
}

#[async_trait]
impl<C> SpDirectoryQueryClient for NyxdClient<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn query_service_provider_contract<T>(&self, query: SpQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.client
            .query_contract_smart(
                self.service_provider_contract_address().ok_or(
                    NyxdError::NoContractAddressAvailable(
                        "service provider directory contract".to_string(),
                    ),
                )?,
                &query,
            )
            .await
    }
}

#[async_trait]
impl<C> SpDirectoryQueryClient for crate::Client<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn query_service_provider_contract<T>(&self, query: SpQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        self.nyxd.query_service_provider_contract(query).await
    }
}
