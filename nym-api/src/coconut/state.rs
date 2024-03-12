// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::client::Client as LocalClient;
use crate::coconut::comm::APICommunicationChannel;
use crate::coconut::deposit::validate_deposit_tx;
use crate::coconut::error::Result;
use crate::coconut::keys::KeyPair;
use crate::coconut::storage::CoconutStorageExt;
use crate::support::storage::NymApiStorage;
use nym_api_requests::coconut::helpers::issued_credential_plaintext;
use nym_api_requests::coconut::BlindSignRequestBody;
use nym_coconut::{BlindedSignature, VerificationKey};
use nym_coconut_dkg_common::types::EpochId;
use nym_crypto::asymmetric::identity;
use nym_validator_client::nyxd::{AccountId, Hash, TxResponse};
use rand::rngs::OsRng;
use rand::RngCore;
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

pub use nym_credentials::coconut::bandwidth::bandwidth_credential_params;

pub struct State {
    pub(crate) client: Arc<dyn LocalClient + Send + Sync>,
    pub(crate) bandwidth_contract_admin: OnceCell<Option<AccountId>>,
    pub(crate) mix_denom: String,
    pub(crate) coconut_keypair: KeyPair,
    pub(crate) identity_keypair: identity::KeyPair,
    pub(crate) comm_channel: Arc<dyn APICommunicationChannel + Send + Sync>,
    pub(crate) storage: NymApiStorage,
    pub(crate) freepass_nonce: Arc<RwLock<[u8; 16]>>,
}

impl State {
    pub(crate) fn new<C, D>(
        client: C,
        mix_denom: String,
        identity_keypair: identity::KeyPair,
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

        let mut nonce = [0u8; 16];
        OsRng.fill_bytes(&mut nonce);

        Self {
            client,
            bandwidth_contract_admin: OnceCell::new(),
            mix_denom,
            coconut_keypair: key_pair,
            identity_keypair,
            comm_channel,
            storage,
            freepass_nonce: Arc::new(RwLock::new(nonce)),
        }
    }

    /// Check if this nym-api has already issued a credential for the provided deposit hash.
    /// If so, return it.
    pub async fn already_issued(&self, tx_hash: Hash) -> Result<Option<BlindedSignature>> {
        self.storage
            .get_issued_bandwidth_credential_by_hash(&tx_hash.to_string())
            .await?
            .map(|cred| cred.try_into())
            .transpose()
    }

    pub async fn get_transaction(&self, tx_hash: Hash) -> Result<TxResponse> {
        self.client.get_tx(tx_hash).await
    }

    pub async fn get_bandwidth_contract_admin(&self) -> Result<&Option<AccountId>> {
        self.bandwidth_contract_admin
            .get_or_try_init(|| async { self.client.bandwidth_contract_admin().await })
            .await
    }

    pub async fn validate_request(
        &self,
        request: &BlindSignRequestBody,
        tx: TxResponse,
    ) -> Result<()> {
        validate_deposit_tx(request, tx).await
    }

    pub(crate) async fn sign_and_store_credential(
        &self,
        current_epoch: EpochId,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<i64> {
        let encoded_commitments = request_body.encode_commitments();

        let plaintext = issued_credential_plaintext(
            current_epoch as u32,
            request_body.tx_hash,
            blinded_signature,
            &encoded_commitments,
            request_body.expiration_date.try_into().unwrap(), //will fail in approx 290 billion years
        );

        let signature = self.identity_keypair.private_key().sign(plaintext);

        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database
        let credential_id = self
            .storage
            .store_issued_credential(
                current_epoch as u32,
                request_body.tx_hash,
                blinded_signature,
                signature,
                encoded_commitments,
                request_body.expiration_date.try_into().unwrap(), //will fail in approx 290 billion years
            )
            .await?;

        Ok(credential_id)
    }

    pub async fn store_issued_credential(
        &self,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<()> {
        let current_epoch = self.comm_channel.current_epoch().await?;

        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database
        let credential_id = self
            .sign_and_store_credential(current_epoch, request_body, blinded_signature)
            .await?;
        self.storage
            .update_epoch_credentials_entry(current_epoch, credential_id)
            .await?;
        debug!("the stored credential has id {credential_id}");

        Ok(())
    }

    pub async fn verification_key(&self, epoch_id: EpochId) -> Result<VerificationKeyAuth> {
        self.comm_channel
            .aggregated_verification_key(epoch_id)
            .await
    }
}
