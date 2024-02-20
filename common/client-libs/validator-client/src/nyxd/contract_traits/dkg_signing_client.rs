// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nyxd::contract_traits::NymContractsProvider;
use crate::nyxd::cosmwasm_client::types::ExecuteResult;
use crate::nyxd::error::NyxdError;
use crate::nyxd::{Coin, Fee, SigningCosmWasmClient};
use crate::signing::signer::OfflineSigner;
use async_trait::async_trait;
use cosmrs::AccountId;
use nym_coconut_dkg_common::dealing::{DealingChunkInfo, PartialContractDealing};
use nym_coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use nym_coconut_dkg_common::types::{DealingIndex, EncodedBTEPublicKeyWithProof};
use nym_coconut_dkg_common::verification_key::VerificationKeyShare;
use nym_contracts_common::IdentityKey;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait DkgSigningClient {
    async fn execute_dkg_contract(
        &self,
        fee: Option<Fee>,
        msg: DkgExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError>;

    async fn initiate_dkg(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::InitiateDkg {};

        self.execute_dkg_contract(fee, req, "initiating the DKG".to_string(), vec![])
            .await
    }

    async fn advance_dkg_epoch_state(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::AdvanceEpochState {};

        self.execute_dkg_contract(fee, req, "advancing DKG state".to_string(), vec![])
            .await
    }

    async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
        identity_key: IdentityKey,
        announce_address: String,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::RegisterDealer {
            bte_key_with_proof: bte_key,
            identity_key,
            announce_address,
            resharing,
        };

        self.execute_dkg_contract(fee, req, "registering as a dealer".to_string(), vec![])
            .await
    }

    async fn submit_dealing_metadata(
        &self,
        dealing_index: DealingIndex,
        chunks: Vec<DealingChunkInfo>,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::CommitDealingsMetadata {
            dealing_index,
            chunks,
            resharing,
        };

        self.execute_dkg_contract(fee, req, "dealing metadata commitment".to_string(), vec![])
            .await
    }

    async fn submit_dealing_chunk(
        &self,
        chunk: PartialContractDealing,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::CommitDealingsChunk { chunk };

        self.execute_dkg_contract(fee, req, "dealing chunk commitment".to_string(), vec![])
            .await
    }

    async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::CommitVerificationKeyShare { share, resharing };

        self.execute_dkg_contract(
            fee,
            req,
            "verification key share commitment".to_string(),
            vec![],
        )
        .await
    }

    async fn verify_verification_key_share(
        &self,
        owner: &AccountId,
        resharing: bool,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::VerifyVerificationKeyShare {
            owner: owner.to_string(),
            resharing,
        };

        self.execute_dkg_contract(
            fee,
            req,
            "verification key VerifyVerificationKeyShare".to_string(),
            vec![],
        )
        .await
    }

    async fn trigger_dkg_reset(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::TriggerReset {};

        self.execute_dkg_contract(fee, req, "trigger DKG reset".to_string(), vec![])
            .await
    }

    async fn trigger_dkg_resharing(&self, fee: Option<Fee>) -> Result<ExecuteResult, NyxdError> {
        let req = DkgExecuteMsg::TriggerResharing {};

        self.execute_dkg_contract(fee, req, "trigger DKG resharing".to_string(), vec![])
            .await
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<C> DkgSigningClient for C
where
    C: SigningCosmWasmClient + NymContractsProvider + Sync,
    NyxdError: From<<Self as OfflineSigner>::Error>,
{
    async fn execute_dkg_contract(
        &self,
        fee: Option<Fee>,
        msg: DkgExecuteMsg,
        memo: String,
        funds: Vec<Coin>,
    ) -> Result<ExecuteResult, NyxdError> {
        let dkg_contract_address = self
            .dkg_contract_address()
            .ok_or_else(|| NyxdError::unavailable_contract_address("dkg contract"))?;

        let fee = fee.unwrap_or(Fee::Auto(Some(self.simulated_gas_multiplier())));
        let signer_address = &self.signer_addresses()?[0];

        self.execute(signer_address, dkg_contract_address, &msg, fee, memo, funds)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::nyxd::contract_traits::tests::IgnoreValue;

    // it's enough that this compiles and clippy is happy about it
    #[allow(dead_code)]
    fn all_execute_variants_are_covered<C: DkgSigningClient + Send + Sync>(
        client: C,
        msg: DkgExecuteMsg,
    ) {
        match msg {
            DkgExecuteMsg::InitiateDkg {} => client.initiate_dkg(None).ignore(),
            DkgExecuteMsg::RegisterDealer {
                bte_key_with_proof,
                identity_key,
                announce_address,
                resharing,
            } => client
                .register_dealer(
                    bte_key_with_proof,
                    identity_key,
                    announce_address,
                    resharing,
                    None,
                )
                .ignore(),
            DkgExecuteMsg::CommitDealingsMetadata {
                dealing_index,
                chunks,
                resharing,
            } => client
                .submit_dealing_metadata(dealing_index, chunks, resharing, None)
                .ignore(),
            DkgExecuteMsg::CommitDealingsChunk { chunk } => {
                client.submit_dealing_chunk(chunk, None).ignore()
            }
            DkgExecuteMsg::CommitVerificationKeyShare { share, resharing } => client
                .submit_verification_key_share(share, resharing, None)
                .ignore(),
            DkgExecuteMsg::VerifyVerificationKeyShare { owner, resharing } => client
                .verify_verification_key_share(&owner.parse().unwrap(), resharing, None)
                .ignore(),
            DkgExecuteMsg::AdvanceEpochState {} => client.advance_dkg_epoch_state(None).ignore(),
            DkgExecuteMsg::TriggerReset {} => client.trigger_dkg_reset(None).ignore(),
            DkgExecuteMsg::TriggerResharing {} => client.trigger_dkg_resharing(None).ignore(),
        };
    }
}
