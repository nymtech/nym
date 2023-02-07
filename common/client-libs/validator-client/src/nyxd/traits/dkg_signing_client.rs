// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Fee, NyxdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use coconut_dkg_common::types::EncodedBTEPublicKeyWithProof;
use coconut_dkg_common::verification_key::VerificationKeyShare;
use contracts_common::dealings::ContractSafeBytes;

#[async_trait]
pub trait DkgSigningClient {
    async fn advance_dkg_epoch_state(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError>;
    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        announce_address: String,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn submit_dealing_bytes(
        &self,
        commitment: ContractSafeBytes,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError>;
}

#[async_trait]
impl<C> DkgSigningClient for NyxdClient<C>
where
    C: SigningCosmWasmClient + Send + Sync + Clone,
{
    async fn advance_dkg_epoch_state(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
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
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::RegisterDealer {
            bte_key_with_proof: bte_key,
            announce_address,
            resharing,
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
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::CommitDealing {
            dealing_bytes,
            resharing,
        };

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
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::CommitVerificationKeyShare { share, resharing };

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
