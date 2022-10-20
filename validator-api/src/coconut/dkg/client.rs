// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::client::Client;
use crate::coconut::error::CoconutError;
use coconut_dkg_common::dealer::DealerDetailsResponse;
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, EpochState, NodeIndex};
use contracts_common::commitment::ContractSafeCommitment;
use validator_client::nymd::cosmwasm_client::logs::{find_attribute, NODE_INDEX};

pub(crate) struct DkgClient {
    inner: Box<dyn Client + Send + Sync>,
}

impl DkgClient {
    pub(crate) fn new<C>(nymd_client: C) -> Self
    where
        C: Client + Send + Sync + 'static,
    {
        DkgClient {
            inner: Box::new(nymd_client),
        }
    }

    pub(crate) async fn get_current_epoch_state(&self) -> Result<EpochState, CoconutError> {
        self.inner.get_current_epoch_state().await
    }

    pub(crate) async fn get_self_registered_dealer_details(
        &self,
    ) -> Result<DealerDetailsResponse, CoconutError> {
        self.inner.get_self_registered_dealer_details().await
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

    pub(crate) async fn _submit_dealing_commitment(
        &self,
        commitment: ContractSafeCommitment,
    ) -> Result<(), CoconutError> {
        self.inner.submit_dealing_commitment(commitment).await?;
        Ok(())
    }
}
