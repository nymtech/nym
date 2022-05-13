// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::nymd::cosmwasm_client::types::ExecuteResult;
use crate::nymd::error::NymdError;
use crate::nymd::{cosmwasm_coin_to_cosmos_coin, Fee, NymdClient, SigningCosmWasmClient};
use async_trait::async_trait;
use coconut_dkg_common::msg::ExecuteMsg as DkgExecuteMsg;
use coconut_dkg_common::msg::QueryMsg as DkgQueryMsg;
use coconut_dkg_common::types::{
    BlacklistingResponse, EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey, Epoch,
    MinimumDepositResponse, PagedBlacklistingResponse, PagedDealerResponse,
};

#[async_trait]
pub trait DkgClient {
    async fn get_current_dkg_epoch(&self) -> Result<Epoch, NymdError>;
    async fn get_current_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NymdError>;
    async fn get_past_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NymdError>;

    async fn get_blacklisted_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedBlacklistingResponse, NymdError>;

    async fn get_blacklisting(&self, dealer: String) -> Result<BlacklistingResponse, NymdError>;
    async fn get_deposit_amount(&self) -> Result<MinimumDepositResponse, NymdError>;

    async fn register_dealer(
        &self,
        identity: EncodedEd25519PublicKey,
        bte_key: EncodedBTEPublicKeyWithProof,
        owner_signature: String,
        listening_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError>;
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
    async fn get_current_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NymdError> {
        let request = DkgQueryMsg::GetCurrentDealers {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }
    async fn get_past_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedDealerResponse, NymdError> {
        let request = DkgQueryMsg::GetPastDealers {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }

    async fn get_blacklisted_dealers_paged(
        &self,
        start_after: Option<String>,
        page_limit: Option<u32>,
    ) -> Result<PagedBlacklistingResponse, NymdError> {
        let request = DkgQueryMsg::GetBlacklistedDealers {
            start_after,
            limit: page_limit,
        };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }

    async fn get_blacklisting(&self, dealer: String) -> Result<BlacklistingResponse, NymdError> {
        let request = DkgQueryMsg::GetBlacklisting { dealer };
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }

    async fn get_deposit_amount(&self) -> Result<MinimumDepositResponse, NymdError> {
        let request = DkgQueryMsg::GetDepositAmount {};
        self.client
            .query_contract_smart(self.coconut_dkg_contract_address()?, &request)
            .await
    }

    async fn register_dealer(
        &self,
        identity: EncodedEd25519PublicKey,
        bte_key: EncodedBTEPublicKeyWithProof,
        owner_signature: String,
        listening_address: String,
        fee: Option<Fee>,
    ) -> Result<ExecuteResult, NymdError> {
        let req = DkgExecuteMsg::RegisterDealer {
            ed25519_key: identity,
            bte_key_with_proof: bte_key,
            owner_signature,
            host: listening_address,
        };
        let deposit = self.get_deposit_amount().await?;

        self.client
            .execute(
                self.address(),
                self.coconut_dkg_contract_address()?,
                &req,
                fee.unwrap_or_default(),
                format!("registering {} as a dealer", self.address()),
                vec![cosmwasm_coin_to_cosmos_coin(deposit.amount)],
            )
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
