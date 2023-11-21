// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::client::Client as LocalClient;
use crate::coconut::comm::APICommunicationChannel;
use crate::coconut::error::Result;
use crate::coconut::keypair::KeyPair;
use crate::support::storage::NymApiStorage;
use nym_api_requests::coconut::BlindedSignatureResponse;
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_interface::{BlindedSignature, VerificationKey};
use nym_credentials::coconut::params::{
    NymApiCredentialEncryptionAlgorithm, NymApiCredentialHkdfAlgorithm,
};
use nym_crypto::asymmetric::encryption;
use nym_crypto::shared_key::new_ephemeral_shared_key;
use nym_crypto::symmetric::stream_cipher;
use rand_07::rngs::OsRng;
use std::sync::Arc;

pub struct State {
    pub(crate) client: Arc<dyn LocalClient + Send + Sync>,
    pub(crate) mix_denom: String,
    pub(crate) key_pair: KeyPair,
    pub(crate) comm_channel: Arc<dyn APICommunicationChannel + Send + Sync>,
    pub(crate) storage: NymApiStorage,
}

impl State {
    pub(crate) fn new<C, D>(
        client: C,
        mix_denom: String,
        key_pair: KeyPair,
        comm_channel: D,
        storage: NymApiStorage,
    ) -> Self
    where
        C: LocalClient + Send + Sync + 'static,
        D: APICommunicationChannel + Send + Sync + 'static,
    {
        let client = Arc::new(client);
        let comm_channel = Arc::new(comm_channel);
        Self {
            client,
            mix_denom,
            key_pair,
            comm_channel,
            storage,
        }
    }

    pub async fn signed_before(&self, tx_hash: &str) -> Result<Option<BlindedSignatureResponse>> {
        let ret = self.storage.get_blinded_signature_response(tx_hash).await?;
        if let Some(blinded_signature_reponse) = ret {
            Ok(Some(BlindedSignatureResponse::from_base58_string(
                blinded_signature_reponse,
            )?))
        } else {
            Ok(None)
        }
    }

    pub async fn encrypt_and_store(
        &self,
        tx_hash: &str,
        remote_key: &encryption::PublicKey,
        signature: &BlindedSignature,
    ) -> Result<BlindedSignatureResponse> {
        let (keypair, shared_key) = {
            let mut rng = OsRng;
            new_ephemeral_shared_key::<
                NymApiCredentialEncryptionAlgorithm,
                NymApiCredentialHkdfAlgorithm,
                _,
            >(&mut rng, remote_key)
        };

        let chunk_data = signature.to_bytes();

        let zero_iv = stream_cipher::zero_iv::<NymApiCredentialEncryptionAlgorithm>();
        let encrypted_data = stream_cipher::encrypt::<NymApiCredentialEncryptionAlgorithm>(
            &shared_key,
            &zero_iv,
            &chunk_data,
        );

        let response =
            BlindedSignatureResponse::new(encrypted_data, keypair.public_key().to_bytes());

        // Atomically insert data, only if there is no signature stored in the meantime
        // This prevents race conditions on storing two signatures for the same deposit transaction

        // TODO: JS: how is it atomic? don't we need a lock or something?
        if self
            .storage
            .insert_blinded_signature_response(tx_hash, &response.to_base58_string())
            .await
            .is_err()
        {
            Ok(self
                .signed_before(tx_hash)
                .await?
                .expect("The signature was expected to be there"))
        } else {
            Ok(response)
        }
    }

    pub async fn verification_key(&self, epoch_id: EpochId) -> Result<VerificationKey> {
        self.comm_channel
            .aggregated_verification_key(epoch_id)
            .await
    }
}
