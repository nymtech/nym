// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::traits::dkg_query_client::DkgQueryClient;
use crate::nymd::{Fee, NymdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use coconut_dkg_common::types::EncodedBTEPublicKeyWithProof;
use contracts_common::commitment::ContractSafeCommitment;

#[async_trait]
pub trait DkgSigningClient {
    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn submit_dealing_commitment(
        &self,
        commitment: ContractSafeCommitment,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C> DkgSigningClient for NymdClient<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::RegisterDealer {
            bte_key_with_proof: bte_key,
        };
        let deposit = self.get_deposit_amount().await?;

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address(),
                &req,
                fee.unwrap_or_default(),
                format!("registering {} as a dealer", self.address()),
                vec![deposit.amount.into()],
            )
            .await
    }

    async fn submit_dealing_commitment(
        &self,
        commitment: ContractSafeCommitment,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::CommitDealing { commitment };

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address(),
                &req,
                fee.unwrap_or_default(),
                "dealing commitment",
                vec![],
            )
            .await
    }
}
