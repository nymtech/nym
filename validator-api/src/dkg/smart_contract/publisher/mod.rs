// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::Client;
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey, NodeIndex};
use validator_client::nymd::{AccountId, SigningCosmWasmClient};

pub(crate) struct Publisher<C> {
    client: Client<C>,
}

impl<C> Publisher<C>
where
    C: SigningCosmWasmClient + Send + Sync,
{
    pub(crate) fn new(client: Client<C>) -> Self {
        Publisher { client }
    }

    pub(crate) async fn get_address(&self) -> AccountId {
        self.client.address().await
    }

    pub(crate) async fn register_dealer(
        &self,
        identity: EncodedEd25519PublicKey,
        bte_key: EncodedBTEPublicKeyWithProof,
        owner_signature: String,
        listening_address: String,
    ) -> Result<NodeIndex, DkgError> {
        self.client
            .register_dealer(identity, bte_key, owner_signature, listening_address)
            .await?;

        // once we figure out how to properly deserialize `data` field from the response use that
        // instead of this query
        let self_details = self.client.get_self_registered_dealer_details().await?;
        if let Some(details) = self_details.details {
            if self_details.dealer_type.is_current() {
                return Ok(details.assigned_index);
            }
        }

        Err(DkgError::NodeIndexRecoveryError)
    }

    pub(crate) async fn submit_dealing_commitment(&self) {
        // self.client.submit_dealing_commitment().await;
        //
    }
}
