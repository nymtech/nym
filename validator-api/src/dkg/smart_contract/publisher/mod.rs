// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::dkg::error::DkgError;
use crate::Client;
use coconut_dkg_common::types::{EncodedBTEPublicKeyWithProof, EncodedEd25519PublicKey};
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
    ) -> Result<(), DkgError> {
        self.client
            .register_dealer(identity, bte_key, owner_signature, listening_address)
            .await?;
        Ok(())
    }

    pub(crate) async fn submit_dealing_commitment(&self) {
        // self.client.submit_dealing_commitment().await;
        //
    }
}
