use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::ContractBuildInformation;
use nym_service_provider_directory_common::{
    msg::QueryMsg as SpQuery,
    response::{
        ConfigResponse, PagedServicesListResponse, ServiceInfoResponse, ServicesListResponse,
    },
    NymAddress, ServiceId, ServiceInfo,
};
use serde::Deserialize;

use crate::nyxd::{error::NyxdError, CosmWasmClient, NyxdClient};

#[async_trait]
pub trait SpDirectoryQueryClient {
    async fn query_service_provider_contract<T>(&self, query: SpQuery) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_service_config(&self) -> Result<ConfigResponse, NyxdError> {
        self.query_service_provider_contract(SpQuery::Config {})
            .await
    }

    async fn get_service_info(
        &self,
        service_id: ServiceId,
    ) -> Result<ServiceInfoResponse, NyxdError> {
        self.query_service_provider_contract(SpQuery::ServiceId { service_id })
            .await
    }

    async fn get_services_paged(
        &self,
        start_after: Option<ServiceId>,
        limit: Option<u32>,
    ) -> Result<PagedServicesListResponse, NyxdError> {
        self.query_service_provider_contract(SpQuery::All { limit, start_after })
            .await
    }

    async fn get_services_by_announcer(
        &self,
        announcer: AccountId,
    ) -> Result<ServicesListResponse, NyxdError> {
        self.query_service_provider_contract(SpQuery::ByAnnouncer {
            announcer: announcer.to_string(),
        })
        .await
    }

    async fn get_services_by_nym_address(
        &self,
        nym_address: NymAddress,
    ) -> Result<ServicesListResponse, NyxdError> {
        self.query_service_provider_contract(SpQuery::ByNymAddress { nym_address })
            .await
    }

    async fn get_sp_contract_version(&self) -> Result<ContractBuildInformation, NyxdError> {
        self.query_service_provider_contract(SpQuery::GetContractVersion {})
            .await
    }

    async fn get_all_services(&self) -> Result<Vec<ServiceInfo>, NyxdError> {
        let mut services = Vec::new();
        let mut start_after = None;

        loop {
            let mut paged_response = self.get_services_paged(start_after.take(), None).await?;

            let last_id = paged_response.services.last().map(|serv| serv.service_id);
            services.append(&mut paged_response.services);

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
impl<C> SpDirectoryQueryClient for NyxdClient<C>
where
    C: CosmWasmClient + Send + Sync,
{
    async fn query_service_provider_contract<T>(&self, query: SpQuery) -> Result<T, NyxdError>
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
