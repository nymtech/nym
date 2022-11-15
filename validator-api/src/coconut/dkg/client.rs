// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::client::Client;
use crate::coconut::error::CoconutError;
use coconut_dkg_common::dealer::{ContractDealing, DealerDetails, DealerDetailsResponse};
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, EpochState, NodeIndex};
use coconut_dkg_common::verification_key::{ContractVKShare, VerificationKeyShare};
use contracts_common::dealings::ContractSafeBytes;
use multisig_contract_common::msg::ProposalResponse;
use validator_client::nymd::cosmwasm_client::logs::{find_attribute, NODE_INDEX};
use validator_client::nymd::AccountId;

pub(crate) struct DkgClient {
    inner: Box<dyn Client + Send + Sync>,
}

impl DkgClient {
    // Some queries simply don't work the first time
    // Until we determine why that is, retry the query a few more times
    const RETRIES: usize = 3;

    pub(crate) fn new<C>(nymd_client: C) -> Self
    where
        C: Client + Send + Sync + 'static,
    {
        DkgClient {
            inner: Box::new(nymd_client),
        }
    }

    pub(crate) async fn _get_address(&self) -> AccountId {
        self.inner.address().await
    }

    pub(crate) async fn get_current_epoch_state(&self) -> Result<EpochState, CoconutError> {
        let mut ret = self.inner.get_current_epoch_state().await;
        for _ in 0..Self::RETRIES {
            if ret.is_ok() {
                return ret;
            }
            ret = self.inner.get_current_epoch_state().await;
        }
        ret
    }

    pub(crate) async fn get_self_registered_dealer_details(
        &self,
    ) -> Result<DealerDetailsResponse, CoconutError> {
        self.inner.get_self_registered_dealer_details().await
    }

    pub(crate) async fn get_current_dealers(&self) -> Result<Vec<DealerDetails>, CoconutError> {
        self.inner.get_current_dealers().await
    }

    pub(crate) async fn get_dealings(
        &self,
        idx: usize,
    ) -> Result<Vec<ContractDealing>, CoconutError> {
        let mut ret = self.inner.get_dealings(idx).await;
        for _ in 0..Self::RETRIES {
            if ret.is_ok() {
                return ret;
            }
            ret = self.inner.get_dealings(idx).await;
        }
        ret
    }

    pub(crate) async fn get_verification_key_shares(
        &self,
    ) -> Result<Vec<ContractVKShare>, CoconutError> {
        self.inner.get_verification_key_shares().await
    }

    pub(crate) async fn list_proposals(&self) -> Result<Vec<ProposalResponse>, CoconutError> {
        self.inner.list_proposals().await
    }

    pub(crate) async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
    ) -> Result<NodeIndex, CoconutError> {
        let res = self.inner.register_dealer(bte_key).await?;
        let node_index = find_attribute(&res.logs, "wasm", NODE_INDEX)
            .ok_or(CoconutError::NodeIndexRecoveryError {
                reason: String::from("node index not found"),
            })?
            .value
            .parse::<NodeIndex>()
            .map_err(|_| CoconutError::NodeIndexRecoveryError {
                reason: String::from("node index could not be parsed"),
            })?;

        Ok(node_index)
    }

    pub(crate) async fn submit_dealing(
        &self,
        dealing_bytes: ContractSafeBytes,
    ) -> Result<(), CoconutError> {
        self.inner.submit_dealing(dealing_bytes).await?;
        Ok(())
    }

    pub(crate) async fn submit_verification_key_share(
        &self,
        share: VerificationKeyShare,
    ) -> Result<(), CoconutError> {
        let mut ret = self
            .inner
            .submit_verification_key_share(share.clone())
            .await;
        for _ in 0..Self::RETRIES {
            if ret.is_ok() {
                break;
            }
            ret = self
                .inner
                .submit_verification_key_share(share.clone())
                .await;
        }
        Ok(())
    }

    pub(crate) async fn vote_verification_key_share(
        &self,
        proposal_id: u64,
        vote_yes: bool,
    ) -> Result<(), CoconutError> {
        self.inner.vote_proposal(proposal_id, vote_yes, None).await
    }
}
