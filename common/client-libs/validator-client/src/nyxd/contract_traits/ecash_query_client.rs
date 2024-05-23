// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::collect_paged;
use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::error::NyxdError;
use crate::nyxd::CosmWasmClient;
use async_trait::async_trait;
use nym_ecash_contract_common::blacklist::BlacklistedAccountResponse;
use nym_ecash_contract_common::msg::QueryMsg as EcashQueryMsg;
use nym_ecash_contract_common::spend_credential::{
    EcashSpentCredential, EcashSpentCredentialResponse, PagedEcashSpentCredentialResponse,
};
use serde::Deserialize;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait EcashQueryClient {
    async fn query_ecash_contract<T>(&self, query: EcashQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>;

    async fn get_spent_credential(
        &self,
        serial_number: String,
    ) -> Result<EcashSpentCredentialResponse, NyxdError> {
        self.query_ecash_contract(EcashQueryMsg::GetSpentCredential { serial_number })
            .await
    }

    async fn get_all_spent_credential_paged(
        &self,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> Result<PagedEcashSpentCredentialResponse, NyxdError> {
        self.query_ecash_contract(EcashQueryMsg::GetAllSpentCredentialsPaged { limit, start_after })
            .await
    }

    async fn get_blacklisted_account(
        &self,
        public_key: String,
    ) -> Result<BlacklistedAccountResponse, NyxdError> {
        self.query_ecash_contract(EcashQueryMsg::GetBlacklistedAccount { public_key })
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait PagedEcashQueryClient: EcashQueryClient {
    async fn get_all_spent_credentials(&self) -> Result<Vec<EcashSpentCredential>, NyxdError> {
        collect_paged!(self, get_all_spent_credential_paged, spend_credentials)
    }
}

#[async_trait]
impl<T> PagedEcashQueryClient for T where T: EcashQueryClient {}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> EcashQueryClient for C
where
    C: CosmWasmClient + NymContractsProvider + Send + Sync,
{
    async fn query_ecash_contract<T>(&self, query: EcashQueryMsg) -> Result<T, NyxdError>
    where
        for<'a> T: Deserialize<'a>,
    {
        let ecash_contract_address = self
            .coconut_bandwidth_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("coconut bandwidth contract"))?;
        self.query_contract_smart(ecash_contract_address, &query)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_query_variants_are_covered<C: EcashQueryClient + Send + Sync>(
        client: C,
        msg: EcashQueryMsg,
    ) {
        match msg {
            EcashQueryMsg::GetSpentCredential { serial_number } => {
                client.get_spent_credential(serial_number).ignore()
            }
            EcashQueryMsg::GetAllSpentCredentialsPaged { limit, start_after } => client
                .get_all_spent_credential_paged(start_after, limit)
                .ignore(),
            EcashQueryMsg::GetBlacklistedAccount { public_key } => {
                client.get_blacklisted_account(public_key).ignore()
            }
        };
    }
}
