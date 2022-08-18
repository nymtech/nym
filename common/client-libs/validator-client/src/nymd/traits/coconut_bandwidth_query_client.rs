// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::error::NymdError;
use crate::nymd::{CosmWasmClient, NymdClient};

use coconut_bandwidth_contract_common::msg::QueryMsg;
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;

use async_trait::async_trait;

#[async_trait]
pub trait CoconutBandwidthQueryClient {
    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse, NymdError>;
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send> CoconutBandwidthQueryClient for NymdClient<C> {
    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse, NymdError> {
        let request = QueryMsg::GetSpentCredential {
            blinded_serial_number,
        };
        self.client
            .query_contract_smart(self.coconut_bandwidth_contract_address(), &request)
            .await
    }
}
