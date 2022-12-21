// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::{Fee, NymdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use coconut_dkg_common::types::EncodedBTEPublicKeyWithProof;
use coconut_dkg_common::verification_key::VerificationKeyShare;
use contracts_common::dealings::ContractSafeBytes;

#[async_trait]
pub trait DkgSigningClient {
    async fn advance_dkg_epoch_state(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError>;
    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        announce_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn submit_dealing_bytes(
        &self,
        commitment: ContractSafeBytes,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
}

#[async_trait]
impl<C> DkgSigningClient for NymdClient<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    async fn advance_dkg_epoch_state(&self, fee: Option<Fee>) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::AdvanceEpochState {};

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address(),
                &req,
                fee.unwrap_or_default(),
                "advancing DKG state",
                vec![],
            )
            .await
    }

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        announce_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::RegisterDealer {
            bte_key_with_proof: bte_key,
            announce_address,
        };

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address(),
                &req,
                fee.unwrap_or_default(),
                format!("registering {} as a dealer", self.address()),
                vec![],
            )
            .await
    }

    async fn submit_dealing_bytes(
        &self,
        dealing_bytes: ContractSafeBytes,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::CommitDealing { dealing_bytes };

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

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::CommitVerificationKeyShare { share };

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address(),
                &req,
                fee.unwrap_or_default(),
                "verification key share commitment",
                vec![],
            )
            .await
    }
}
