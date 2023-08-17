// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use nym_coconut_bandwidth_contract_common::msg::QueryMsg as CoconutBandwidthQueryMsg;
use nym_coconut_bandwidth_contract_common::spend_credential::{
    PagedSpendCredentialResponse, SpendCredential, SpendCredentialResponse,
};
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait CoconutBandwidthQueryClient {
    async fn query_coconut_bandwidth_contract<T>(
        &self,
        query: CoconutBandwidthQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse, NyxdError> {
        self.query_coconut_bandwidth_contract(CoconutBandwidthQueryMsg::GetSpentCredential {
            blinded_serial_number,
        })
        .await
    }

    async fn get_all_spent_credential_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedSpendCredentialResponse, NyxdError> {
        self.query_coconut_bandwidth_contract(CoconutBandwidthQueryMsg::GetAllSpentCredentials {
            limit,
            start_after,
        })
        .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedCoconutBandwidthQueryClient: CoconutBandwidthQueryClient {
    async fn get_all_spent_credentials(&self) -> Result<Vec<SpendCredential>, NyxdError> {
        collect_paged!(self, get_all_spent_credential_paged, spend_credentials)
    }
}

#[async_trait]
impl<T> PagedCoconutBandwidthQueryClient for T where T: CoconutBandwidthQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> CoconutBandwidthQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_coconut_bandwidth_contract<T>(
        &self,
        query: CoconutBandwidthQueryMsg,
    ) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let coconut_bandwidth_contract_address = self
            .coconut_bandwidth_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("coconut bandwidth contract"))?;
        self.query_contract_smart(coconut_bandwidth_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: CoconutBandwidthQueryClient + Send + Sync>(
        client: C,
        msg: CoconutBandwidthQueryMsg,
    ) {
        match msg {
            CoconutBandwidthQueryMsg::GetSpentCredential {
                blinded_serial_number,
            } => client.get_spent_credential(blinded_serial_number).ignore(),
            CoconutBandwidthQueryMsg::GetAllSpentCredentials { limit, start_after } => client
                .get_all_spent_credential_paged(start_after, limit)
                .ignore(),
        };
    }
}
