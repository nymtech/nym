// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::{Fee, NymdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use coconut_dkg_common::types::Epoch;

#[async_trait]
pub trait DkgClient {
    async fn get_current_dkg_epoch(&self) -> Result<Epoch, NymdError>;
    async fn submit_dealing_commitment(
        &self,
        epoch_id: u32,
        dealing_digest: [u8; 32],
        receivers: u32,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C> DkgClient for NymdClient<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    async fn get_current_dkg_epoch(&self) -> Result<Epoch, NymdError> {
        let request = DkgQueryMsg::GetCurrentEpoch {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }

    async fn submit_dealing_commitment(
        &self,
        epoch_id: u32,
        dealing_digest: [u8; 32],
        receivers: u32,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::CommitDealing {
            epoch_id,
            dealing_digest,
            receivers,
        };

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address()?,
                &req,
                fee.unwrap_or_default(),
                "dealing commitment",
                Vec::new(),
            )
            .await
    }
}
