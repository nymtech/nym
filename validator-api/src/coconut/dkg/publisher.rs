// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::client::Client;
use crate::coconut::error::CoconutError;
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, NodeIndex};
use contracts_common::commitment::ContractSafeCommitment;
use validator_client::nymd::AccountId;

pub(crate) struct Publisher {
    client: Box<dyn Client + Send + Sync>,
}

impl Publisher {
    pub(crate) fn new<C>(nymd_client: C) -> Self
    where
        C: Client + Send + Sync + 'static,
    {
        let client = Box::new(nymd_client);
        Publisher { client }
    }

    pub(crate) async fn get_address(&self) -> AccountId {
        self.client.address().await
    }

    pub(crate) async fn register_dealer(
        &self,
        bte_key: EncodedBTEPublicKeyWithProof,
    ) -> Result<NodeIndex, CoconutError> {
        self.client.register_dealer(bte_key).await?;

        // once we figure out how to properly deserialize `data` field from the response use that
        // instead of this query
        let self_details = self.client.get_self_registered_dealer_details().await?;
        if let Some(details) = self_details.details {
            if self_details.dealer_type.is_current() {
                return Ok(details.assigned_index);
            }
        }

        Err(CoconutError::NodeIndexRecoveryError)
    }

    pub(crate) async fn submit_dealing_commitment(
        &self,
        commitment: ContractSafeCommitment,
    ) -> Result<(), CoconutError> {
        self.client.submit_dealing_commitment(commitment).await?;
        Ok(())
    }
}
