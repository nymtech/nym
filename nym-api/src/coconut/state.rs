// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: GPL-3.0-only

use crate::coconut::client::Client as LocalClient;
use crate::coconut::comm::APICommunicationChannel;
use crate::coconut::error::{CoconutError, Result};
use crate::coconut::keypair::KeyPair;
use crate::coconut::storage::CoconutStorageExt;
use crate::support::storage::NymApiStorage;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use nym_api_requests::coconut::helpers::issued_credential_plaintext;
use nym_api_requests::coconut::{
    BlindSignRequestBody, BlindedSignatureResponse, BlindedSignatureResponseNew,
};
use nym_coconut::Base58;
use nym_coconut::Parameters;
use nym_coconut_bandwidth_contract_common::events::{
    COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_IDENTITY_KEY, DEPOSIT_INFO, DEPOSIT_VALUE,
};
use nym_coconut_dkg_common::types::EpochId;
use nym_coconut_interface::{BlindedSignature, VerificationKey};
use nym_credentials::coconut::bandwidth::BandwidthVoucher;
use nym_credentials::coconut::params::{
    NymApiCredentialEncryptionAlgorithm, NymApiCredentialHkdfAlgorithm,
};
use nym_crypto::asymmetric::{encryption, identity};
use nym_crypto::shared_key::new_ephemeral_shared_key;
use nym_crypto::symmetric::stream_cipher;
use nym_validator_client::nyxd::helpers::find_tx_attribute;
use nym_validator_client::nyxd::{Hash, TxResponse};
use rand_07::rngs::OsRng;
use std::sync::Arc;

// keep it as a global static due to relatively high cost of computing the curve points;
// plus we expect all clients to use the same set of parameters
//
// future note: once we allow for credentials with variable number of attributes, just create Parameters(max_allowed_attributes)
// and take as many hs elements as required (since they will match for all variants)
lazy_static! {
    pub(crate) static ref BANDWIDTH_CREDENTIAL_PARAMS: Parameters =
        BandwidthVoucher::default_parameters();
}

pub struct State {
    pub(crate) current_epoch: EpochId,
    pub(crate) client: Arc<dyn LocalClient + Send + Sync>,
    pub(crate) mix_denom: String,
    pub(crate) coconut_key_pair: KeyPair,
    pub(crate) identity_key_pair: identity::KeyPair,
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

        todo!()
        // let current_epoch = todo!();
        // Self {
        //     current_epoch,
        //     client,
        //     mix_denom,
        //     coconut_key_pair: key_pair,
        //     comm_channel,
        //     storage,
        // }
    }

    /// Check if this nym-api has already issued a credential for the provided deposit hash.
    /// If so, return it.
    pub async fn already_issued(
        &self,
        tx_hash: Hash,
    ) -> Result<Option<BlindedSignatureResponseNew>> {
        self.storage
            .get_issued_bandwidth_credential_by_hash(&tx_hash.to_string())
            .await?
            .map(|cred| cred.try_into())
            .transpose()
    }

    pub async fn get_transaction(&self, tx_hash: Hash) -> Result<TxResponse> {
        Ok(self.client.get_tx(tx_hash).await?)
    }

    pub async fn validate_request(
        &self,
        request: &BlindSignRequestBody,
        tx: TxResponse,
    ) -> Result<()> {
        if request.public_attributes_plain.len() != BandwidthVoucher::PUBLIC_ATTRIBUTES as usize {
            return Err(CoconutError::InconsistentPublicAttributes);
        }

        // extract actual public attributes + associated x25519 public key
        let deposit_value =
            find_tx_attribute(&tx, COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_VALUE)
                .ok_or(CoconutError::DepositValueNotFound)?;

        let deposit_info =
            find_tx_attribute(&tx, COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE, DEPOSIT_INFO)
                .ok_or(CoconutError::DepositInfoNotFound)?;

        let x25519_raw = find_tx_attribute(
            &tx,
            COSMWASM_DEPOSITED_FUNDS_EVENT_TYPE,
            DEPOSIT_IDENTITY_KEY,
        )
        .ok_or(CoconutError::DepositVerifKeyNotFound)?;

        // check public attributes against request data
        // (thinking about it attaching that data might be redundant since we have the source of truth on the chain)
        // safety: we won't read data out of bounds since we just checked we have BandwidthVoucher::PUBLIC_ATTRIBUTES values in the vec
        if deposit_value != request.public_attributes_plain[0] {
            return Err(CoconutError::InconsistentDepositValue {
                request: request.public_attributes_plain[0].clone(),
                on_chain: deposit_value,
            });
        }

        if deposit_info != request.public_attributes_plain[1] {
            return Err(CoconutError::InconsistentDepositInfo {
                request: request.public_attributes_plain[1].clone(),
                on_chain: deposit_info,
            });
        }

        // verify signature
        let x25519 = identity::PublicKey::from_base58_string(x25519_raw)?;
        let plaintext =
            BandwidthVoucher::signable_plaintext(&request.inner_sign_request, request.tx_hash);
        x25519.verify(&plaintext, &request.signature)?;

        Ok(())
    }

    async fn sign_and_store_credential(
        &self,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<i64> {
        let encoded_commitments = request_body
            .inner_sign_request
            .get_private_attributes_pedersen_commitments()
            .iter()
            .map(|c| c.to_bs58())
            .collect::<Vec<_>>();

        let plaintext = issued_credential_plaintext(
            self.current_epoch as u32,
            request_body.tx_hash,
            blinded_signature,
            &encoded_commitments,
            &request_body.public_attributes_plain,
        );

        let signature = self.identity_key_pair.private_key().sign(plaintext);

        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database
        let credential_id = self
            .storage
            .store_issued_credential(
                self.current_epoch as u32,
                request_body.tx_hash,
                blinded_signature,
                signature,
                encoded_commitments,
                request_body.public_attributes_plain,
            )
            .await?;

        Ok(credential_id)
    }

    // TODO: figure out what exact data we need here
    pub async fn store_issued_credential(
        &self,
        request_body: BlindSignRequestBody,
        blinded_signature: &BlindedSignature,
    ) -> Result<BlindedSignatureResponse> {
        // note: we have a UNIQUE constraint on the tx_hash column of the credential
        // and so if the api is processing request for the same hash at the same time,
        // only one of them will be successfully inserted to the database

        // here we will be storing credential
        // let credential_id = self.storage.
        // self.storage
        //     .update_epoch_credentials_entry(self.current_epoch, credential_id)
        //     .await?;
        todo!()
    }

    #[deprecated]
    pub async fn encrypt_and_store(
        &self,
        tx_hash: &str,
        remote_key: &encryption::PublicKey,
        signature: &BlindedSignature,
    ) -> Result<BlindedSignatureResponse> {
        todo!()
        // let (keypair, shared_key) = {
        //     let mut rng = OsRng;
        //     new_ephemeral_shared_key::<
        //         NymApiCredentialEncryptionAlgorithm,
        //         NymApiCredentialHkdfAlgorithm,
        //         _,
        //     >(&mut rng, remote_key)
        // };
        //
        // let chunk_data = signature.to_bytes();
        //
        // let zero_iv = stream_cipher::zero_iv::<NymApiCredentialEncryptionAlgorithm>();
        // let encrypted_data = stream_cipher::encrypt::<NymApiCredentialEncryptionAlgorithm>(
        //     &shared_key,
        //     &zero_iv,
        //     &chunk_data,
        // );
        //
        // let response =
        //     BlindedSignatureResponse::new(encrypted_data, keypair.public_key().to_bytes());
        //
        // // Atomically insert data, only if there is no signature stored in the meantime
        // // This prevents race conditions on storing two signatures for the same deposit transaction
        //
        // // TODO: JS: how is it atomic? don't we need a lock or something?
        // if self
        //     .storage
        //     .insert_blinded_signature_response(tx_hash, &response.to_base58_string())
        //     .await
        //     .is_err()
        // {
        //     Ok(self
        //         .already_issued(tx_hash)
        //         .await?
        //         .expect("The signature was expected to be there"))
        // } else {
        //     Ok(response)
        // }
    }

    pub async fn verification_key(&self, epoch_id: EpochId) -> Result<VerificationKey> {
        self.comm_channel
            .aggregated_verification_key(epoch_id)
            .await
    }
}
