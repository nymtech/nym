// Copyright 2022-2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};
use nym_coconut_bandwidth_contract_common::msg::QueryMsg as CoconutBandwidthQueryMsg;
use nym_coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;

use crate::nyxd::contract_traits::NymContractsProvider;
use async_trait::async_trait;
use serde::Deserialize;

#[async_trait]
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
}

#[async_trait]
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

    // it's enough that this compiles
    #[deprecated]
    async fn all_query_variants_are_covered<C: CoconutBandwidthQueryClient + Send + Sync>(
        client: C,
        msg: CoconutBandwidthQueryMsg,
    ) {
        unimplemented!()
    }
}
