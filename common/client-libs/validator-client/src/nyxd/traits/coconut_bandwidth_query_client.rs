// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::error::NyxdError;
use crate::nyxd::{CosmWasmClient, NyxdClient};

use coconut_bandwidth_contract_common::msg::QueryMsg;
use coconut_bandwidth_contract_common::spend_credential::SpendCredentialResponse;

use async_trait::async_trait;

#[async_trait]
pub trait CoconutBandwidthQueryClient {
    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse, NyxdError>;
}

#[async_trait]
impl<C: CosmWasmClient + Sync + Send + Clone> CoconutBandwidthQueryClient for NyxdClient<C> {
    async fn get_spent_credential(
        &self,
        blinded_serial_number: String,
    ) -> Result<SpendCredentialResponse, NyxdError> {
        let request = QueryMsg::GetSpentCredential {
            blinded_serial_number,
        };
        self.client
            .query_contract_smart(self.coconut_bandwidth_contract_address(), &request)
            .await
    }
}
