use crate::collect_paged;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_contracts_common::{signing::Nonce, ContractBuildInformation};
use nym_service_provider_directory_common::{
    msg::QueryMsg as SpQueryMsg,
    response::{
        ConfigResponse, PagedServicesListResponse, ServiceInfoResponse, ServicesListResponse,
    },
    NymAddress, Service, ServiceId,
};
use serde::Deserialize;

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::{error::NyxdError, CosmWasmClient};

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
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

    async fn get_sp_contract_cw2_version(&self) -> Result<cw2::ContractVersion, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::GetCW2ContractVersion {})
            .await
    }

    async fn get_service_signing_nonce(&self, address: &AccountId) -> Result<Nonce, NyxdError> {
        self.query_service_provider_contract(SpQueryMsg::SigningNonce {
            address: address.to_string(),
        })
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedSpDirectoryQueryClient: SpDirectoryQueryClient {
    async fn get_all_services(&self) -> Result<Vec<Service>, NyxdError> {
        collect_paged!(self, get_services_paged, services)
    }
}

#[async_trait]
impl<T> PagedSpDirectoryQueryClient for T where T: SpDirectoryQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> SpDirectoryQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_service_provider_contract<T>(&self, query: SpQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let sp_directory_contract_address =
            &self.service_provider_contract_address().ok_or_else(|| {
                NyxdError::unavailable_contract_address("service provider directory contract")
            })?;
        self.query_contract_smart(sp_directory_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: SpDirectoryQueryClient + Send + Sync>(
        client: C,
        msg: SpQueryMsg,
    ) {
        match msg {
            SpQueryMsg::ServiceId { service_id } => client.get_service_info(service_id).ignore(),
            SpQueryMsg::ByAnnouncer { announcer } => client
                .get_services_by_announcer(announcer.parse().unwrap())
                .ignore(),
            SpQueryMsg::ByNymAddress { nym_address } => {
                client.get_services_by_nym_address(nym_address).ignore()
            }
            SpQueryMsg::All { limit, start_after } => {
                client.get_services_paged(start_after, limit).ignore()
            }
            SpQueryMsg::SigningNonce { address } => client
                .get_service_signing_nonce(&address.parse().unwrap())
                .ignore(),
            SpQueryMsg::Config {} => client.get_service_config().ignore(),
            SpQueryMsg::GetContractVersion {} => client.get_sp_contract_version().ignore(),
            SpQueryMsg::GetCW2ContractVersion {} => client.get_sp_contract_cw2_version().ignore(),
        };
    }
}
